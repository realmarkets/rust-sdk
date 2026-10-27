//! Signing a built PTB and executing it on the node.
//!
//! The node answers only once it has checkpointed the transaction (or the
//! wait runs out), so the objects it creates and changes are readable from
//! it by the next call. Gas is the signer's first 100 gas coins by object
//! id, with the whole balance as budget up to a cap.

use std::{collections::HashSet, str::FromStr};

use anyhow::{Context, anyhow, bail};
use iota_sdk_grpc_client::read_mask_fields::TransactionField;
use iota_sdk_grpc_types::v1::transaction::ExecutedTransaction;
use iota_sdk_types::{
    Address, Event, ExecutionStatus, GasPayment, IdOperation, Intent, Object, ObjectData, ObjectId,
    SignedTransaction, StructTag, Transaction, TransactionExpiration, TransactionKind,
    TransactionV1, UserSignature,
};
use ptbs::{
    TransactionBuilder,
    grpc::{IotaClientExt, READ_TIMEOUT},
};
use serde::de::DeserializeOwned;

use super::{Accounts, TransactionDigest};

/// The most gas one transaction may be given, in nanos.
const GAS_BUDGET_CAP: u64 = 49_999_999_999;

/// The gas price offered, in nanos per unit.
const GAS_PRICE: u64 = 1_000;

/// How long the node may wait for the transaction to be checkpointed before
/// it answers; it caps the wait at its own configured maximum.
const CHECKPOINT_INCLUSION_TIMEOUT_MS: u64 = 30_000;

/// What the execution must return: the digest, the effects for the status
/// and the created ids, the events, and the objects the transaction wrote.
const EXECUTE_READ_MASK: [TransactionField; 4] = [
    TransactionField::TRANSACTION_DIGEST,
    TransactionField::EFFECTS_BCS,
    TransactionField::EVENTS_EVENTS_BCS,
    TransactionField::OUTPUT_OBJECTS_BCS,
];

/// A transaction the node executed successfully.
pub(super) struct Executed {
    pub(super) digest: TransactionDigest,
    /// The objects the transaction created, as it left them.
    created: Vec<Object>,
    events: Vec<Event>,
}

impl Accounts {
    /// Sign the PTB in `builder` with the signer's gas and execute it. A
    /// transaction the node runs but that fails is an error.
    pub(super) async fn sign_and_send(
        &self,
        builder: TransactionBuilder,
    ) -> anyhow::Result<Executed> {
        let transaction = self.transaction(builder).await?;
        let signature = self
            .signer
            .sign_secure(&transaction, Intent::iota_transaction());
        let signed = SignedTransaction {
            transaction,
            signatures: vec![UserSignature::Simple(signature)],
        };

        let query = self
            .client
            .execute_transaction(signed)
            .read_mask(EXECUTE_READ_MASK)
            .checkpoint_inclusion_timeout_ms(CHECKPOINT_INCLUSION_TIMEOUT_MS);
        let executed = tokio::time::timeout(READ_TIMEOUT, query)
            .await
            .map_err(|_| anyhow!("no response to the execution within {READ_TIMEOUT:?}"))??
            .into_inner();
        return Executed::try_from(&executed);
    }

    /// The PTB in `builder` as the signer's transaction, paid for by its gas
    /// coins.
    async fn transaction(&self, builder: TransactionBuilder) -> anyhow::Result<Transaction> {
        let sender = Address::from_str(&self.signer.address())?;
        let gas_coins = self.client.get_gas_coin_details(sender).await?;
        let gas_budget: u64 = gas_coins.iter().map(|g| g.balance).sum();

        return Ok(Transaction::V1(TransactionV1 {
            kind: TransactionKind::Programmable(builder.finish()),
            sender,
            gas_payment: GasPayment {
                objects: gas_coins.iter().map(|g| g.r#ref).collect(),
                owner: sender,
                price: GAS_PRICE,
                budget: gas_budget.min(GAS_BUDGET_CAP),
            },
            expiration: TransactionExpiration::None,
        }));
    }
}

impl TryFrom<&ExecutedTransaction> for Executed {
    type Error = anyhow::Error;

    fn try_from(executed: &ExecutedTransaction) -> anyhow::Result<Executed> {
        let digest = executed.transaction()?.digest()?;
        let effects = executed.effects()?.effects()?;
        let effects = effects
            .as_opt_v1()
            .ok_or_else(|| anyhow!("transaction {digest} has effects of an unknown version"))?;
        if let ExecutionStatus::Failure { error, command } = &effects.status {
            match command {
                Some(command) => bail!("transaction {digest} failed in command {command}: {error}"),
                None => bail!("transaction {digest} failed: {error}"),
            }
        }

        let created_ids: HashSet<ObjectId> = effects
            .changed_objects
            .iter()
            .filter(|changed| changed.id_operation == IdOperation::Created)
            .map(|changed| changed.object_id)
            .collect();
        let mut created = Vec::new();
        for output in &executed.output_objects()?.objects {
            let object = output.object()?;
            if created_ids.contains(&object.id()) {
                created.push(object);
            }
        }
        // the node leaves the events out when there are none
        let events = match &executed.events {
            Some(events) => events.events()?.0,
            None => vec![],
        };

        return Ok(Executed {
            digest: digest.to_string(),
            created,
            events,
        });
    }
}

impl Executed {
    /// The created object of Move type `module::name`, with its contents
    /// decoded as `T`, or `None` when the transaction created none.
    pub(super) fn created<T: DeserializeOwned>(
        &self,
        module: &str,
        name: &str,
    ) -> anyhow::Result<Option<(ObjectId, T)>> {
        let Some(object) = self
            .created
            .iter()
            .find(|object| object.struct_tag().is_some_and(|tag| tag.is(module, name)))
        else {
            return Ok(None);
        };
        let object_id = object.id();
        let ObjectData::Struct(move_struct) = object.data() else {
            bail!("created object {object_id} is not a Move object");
        };
        let value = bcs::from_bytes(move_struct.contents())
            .with_context(|| format!("created object {object_id} is not a {module}::{name}"))?;
        return Ok(Some((object_id, value)));
    }

    /// The first event of Move type `module::name`, decoded as `T`, or
    /// `None` when the transaction emitted none.
    pub(super) fn event<T: DeserializeOwned>(
        &self,
        module: &str,
        name: &str,
    ) -> anyhow::Result<Option<T>> {
        let Some(event) = self
            .events
            .iter()
            .find(|event| event.struct_tag.is(module, name))
        else {
            return Ok(None);
        };
        let value = bcs::from_bytes(&event.contents)
            .with_context(|| format!("event {} is not a {module}::{name}", event.struct_tag))?;
        return Ok(Some(value));
    }
}

trait MoveType {
    /// Whether this is the type `name` of module `module`, at any address and
    /// with any type parameters.
    fn is(&self, module: &str, name: &str) -> bool;
}

impl MoveType for StructTag {
    fn is(&self, module: &str, name: &str) -> bool {
        return self.module().as_str() == module && self.name().as_str() == name;
    }
}
