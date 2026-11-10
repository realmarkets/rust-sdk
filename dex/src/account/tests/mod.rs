//! [`Accounts`] against a [`MockNode`] holding a deployed exchange. Reads
//! decode the objects the chain would serve; each operation's transaction is
//! checked as the node received it: its commands, gas and signature.

use std::str::FromStr;

use crypto::{Signer, raw::RawSigner};
use iota_sdk_crypto::{IotaVerifier, simple::SimpleVerifier};
use iota_sdk_types::{
    Address, Argument, Command, Identifier, Input, Object, ObjectId, Owner,
    ProgrammableTransaction, StructTag, Transaction, TransactionKind, TransactionV1, TypeTag,
};
use ptbs::{
    grpc::mock_node::{MockNode, Submission, field_object, gas_coin, move_object},
    mirrors,
};
use serde::Serialize;

use super::{Accounts, AccountsConfig};

mod creation;
mod deposits;
mod lookups;
mod withdrawals;

/// The sender of every transaction here.
const KEY: &str = "iotaprivkey1qrsd2u9ztqr6z2y78sp2ch8kcyak9cx3ezphkpp4acqaqruajxr6qye2645";

const EXCHANGE_PACKAGE: Address = Address::new([0xe1; 32]);
/// The registry package's latest version, which Move calls target.
const REGISTRY_PACKAGE: Address = Address::new([0xe2; 32]);
/// The registry package's first version, which defines its types.
const REGISTRY_ORIGIN: Address = Address::new([0xe3; 32]);
const SOUL_BOUND_PACKAGE: Address = Address::new([0xe4; 32]);
const FAUCET_PACKAGE: Address = Address::new([0xe5; 32]);
const FIXED18_PACKAGE: Address = Address::new([0xe6; 32]);
const STABLE_PACKAGE: Address = Address::new([0xe7; 32]);

const EXCHANGE: ObjectId = ObjectId::new([0x10; 32]);
const REGISTRY: ObjectId = ObjectId::new([0x11; 32]);
const ADDRESS_REGISTRY: ObjectId = ObjectId::new([0x12; 32]);
const VAULT: ObjectId = ObjectId::new([0x13; 32]);
const FAUCET: ObjectId = ObjectId::new([0x14; 32]);
const TREASURY: ObjectId = ObjectId::new([0x15; 32]);
const COIN_METADATA: ObjectId = ObjectId::new([0x16; 32]);

/// The registered account's objects.
const ACCOUNT_STATIC: ObjectId = ObjectId::new([0x20; 32]);
const ACCOUNT_OBJECT: ObjectId = ObjectId::new([0x21; 32]);
const MANAGER_CAP: ObjectId = ObjectId::new([0x22; 32]);
const SOUL_BOUND: ObjectId = ObjectId::new([0x23; 32]);

const GAS_COIN: ObjectId = ObjectId::new([0x90; 32]);

fn signer() -> Signer {
    return RawSigner::new(KEY).unwrap().into();
}

fn sender() -> Address {
    return Address::from_str(&signer().address()).unwrap();
}

fn stable() -> TypeTag {
    return TypeTag::Struct(Box::new(struct_type(
        STABLE_PACKAGE,
        "stable",
        "STABLE",
        vec![],
    )));
}

fn id_type() -> TypeTag {
    return TypeTag::from(StructTag::new_id());
}

fn struct_type(package: Address, module: &str, name: &str, params: Vec<TypeTag>) -> StructTag {
    return StructTag::new(
        package,
        Identifier::new(module).unwrap(),
        Identifier::new(name).unwrap(),
        params,
    );
}

fn shared(type_: StructTag, value: &impl Serialize) -> Object {
    return move_object(type_, Owner::Shared(1u64.into()), 2, value);
}

fn account_type() -> StructTag {
    return struct_type(EXCHANGE_PACKAGE, "account", "Account", vec![stable()]);
}

/// An account with no positions and `cross_collateral` credited.
fn account(cross_collateral: u64) -> mirrors::Account {
    return mirrors::Account {
        id: ACCOUNT_OBJECT,
        account_id: ACCOUNT_STATIC,
        version: 1,
        manager: MANAGER_CAP,
        active_cross: 0,
        active_isolated: 0,
        cross_collateral: mirrors::Collateral {
            value: cross_collateral,
            is_neg: false,
        },
        locked_for_withdrawal: mirrors::Collateral {
            value: 0,
            is_neg: false,
        },
        can_deposit: true,
        can_withdraw: true,
        can_trade: true,
        maker_fee_tier: 0,
        taker_fee_tier: 0,
        discount_tier: 0,
        rate_limit_exempt: false,
    };
}

/// The exchange's entry resolving `static_id` to `object_id`.
fn exchange_entry(static_id: ObjectId, object_id: ObjectId) -> Object {
    return field_object(EXCHANGE, id_type(), &static_id, id_type(), &object_id);
}

/// A node holding the exchange's shared objects and one gas coin of the
/// sender's.
fn deployed() -> MockNode {
    let node = MockNode::default();
    node.insert(shared(
        struct_type(EXCHANGE_PACKAGE, "exchange", "Exchange", vec![]),
        &EXCHANGE,
    ));
    node.insert(shared(
        struct_type(REGISTRY_ORIGIN, "registry", "AccountRegistry", vec![]),
        &mirrors::AccountRegistry { id: REGISTRY },
    ));
    node.insert(shared(
        struct_type(
            REGISTRY_ORIGIN,
            "address_registry",
            "AddressRegistry",
            vec![],
        ),
        &ADDRESS_REGISTRY,
    ));
    node.insert(shared(
        struct_type(Address::FRAMEWORK, "clock", "Clock", vec![]),
        &(ObjectId::from_str("0x6").unwrap(), 0u64),
    ));
    node.insert(shared(
        struct_type(EXCHANGE_PACKAGE, "vault", "Vault", vec![stable()]),
        &VAULT,
    ));
    insert_faucet(&node);
    node.insert(gas_coin(sender(), GAS_COIN, 5_000_000_000));
    return node;
}

/// The faucet, with the sender as its one VIP, its treasury and the stable
/// coin's metadata.
fn insert_faucet(node: &MockNode) {
    node.insert(shared(
        struct_type(FAUCET_PACKAGE, "faucet", "Faucet", vec![]),
        &mirrors::Faucet {
            id: FAUCET,
            cap: ObjectId::new([0x17; 32]),
            cooldown_ms: 60_000,
            mint_amount: 1_000,
            empty_threshold: 10,
            active_positions_check: false,
            decimals: 6,
            vip_senders: mirrors::VecSet {
                contents: vec![sender()],
            },
        },
    ));
    node.insert(shared(
        struct_type(STABLE_PACKAGE, "stable", "Treasury", vec![]),
        &TREASURY,
    ));
    node.insert(move_object(
        StructTag::new(
            Address::FRAMEWORK,
            Identifier::new("coin").unwrap(),
            Identifier::new("CoinMetadata").unwrap(),
            vec![stable()],
        ),
        Owner::Immutable,
        1,
        &mirrors::CoinMetadata {
            id: COIN_METADATA,
            decimals: 6,
            name: "Stable".to_string(),
            symbol: "USDT".to_string(),
            description: "test stable coin".to_string(),
            icon_url: None,
        },
    ));
}

/// Register `account` as the sender's account at `index`: its registry
/// entry, its soul-bound manager cap, the exchange's entry for it and the
/// account object.
fn register(node: &MockNode, index: u32, account: &mirrors::Account) {
    node.insert(field_object(
        REGISTRY,
        TypeTag::from(struct_type(
            REGISTRY_ORIGIN,
            "registry",
            "Key",
            vec![stable()],
        )),
        &mirrors::RegistryKey {
            sender: sender(),
            index,
        },
        TypeTag::from(struct_type(REGISTRY_ORIGIN, "registry", "Val", vec![])),
        &mirrors::RegistryVal {
            account: ACCOUNT_STATIC,
            manager: SOUL_BOUND,
        },
    ));
    node.insert(shared(
        soul_bound_type(),
        &mirrors::SoulBound {
            id: SOUL_BOUND,
            sender: sender(),
            inner: Some(mirrors::ManagerCap {
                id: MANAGER_CAP,
                account_id: ACCOUNT_STATIC,
            }),
        },
    ));
    node.insert(exchange_entry(ACCOUNT_STATIC, ACCOUNT_OBJECT));
    node.insert(shared(account_type(), account));
}

fn soul_bound_type() -> StructTag {
    let manager_cap = struct_type(EXCHANGE_PACKAGE, "account", "ManagerCap", vec![]);
    return struct_type(
        SOUL_BOUND_PACKAGE,
        "soul_bound",
        "SoulBound",
        vec![TypeTag::from(manager_cap)],
    );
}

fn config() -> AccountsConfig {
    return AccountsConfig {
        real_markets_package_address: EXCHANGE_PACKAGE.to_string(),
        soul_bound_package_address: SOUL_BOUND_PACKAGE.to_string(),
        account_registry_package_address: REGISTRY_PACKAGE.to_string(),
        account_registry_object_id: REGISTRY.to_string(),
        address_registry_object_id: ADDRESS_REGISTRY.to_string(),
        exchange_object_id: EXCHANGE.to_string(),
        vault_object_id: VAULT.to_string(),
        faucet_package_address: FAUCET_PACKAGE.to_string(),
        faucet_object_id: FAUCET.to_string(),
        stable_treasury_object_id: TREASURY.to_string(),
        stable_coin_metadata_object_id: COIN_METADATA.to_string(),
        fixed18_package_address: FIXED18_PACKAGE.to_string(),
        coin_type: stable().to_string(),
    };
}

async fn accounts(node: &MockNode) -> Accounts {
    return Accounts::new(node.start().await, config(), signer())
        .await
        .unwrap();
}

trait OnlySubmission {
    /// The one transaction the node executed; panics on none or several.
    fn only_submission(&self) -> Submission;
}

impl OnlySubmission for MockNode {
    fn only_submission(&self) -> Submission {
        let [submission] = <[Submission; 1]>::try_from(self.submissions()).unwrap();
        return submission;
    }
}

/// What a test checks of a transaction the node received.
trait Received {
    fn v1(&self) -> &TransactionV1;

    fn ptb(&self) -> &ProgrammableTransaction;

    /// The input `argument` refers to, or `None` for a command result.
    fn input(&self, argument: Argument) -> Option<&Input>;

    /// Each command: `module::function` for a Move call, else its kind.
    fn commands(&self) -> Vec<String>;

    /// The checks every submission must pass: sent by the sender, signed by
    /// it over this transaction, asking for what the response is read for
    /// and to be answered once checkpointed.
    fn assert_well_formed(&self);
}

impl Received for Submission {
    fn v1(&self) -> &TransactionV1 {
        let Transaction::V1(transaction) = &self.transaction else {
            panic!("unexpected transaction version");
        };
        return transaction;
    }

    fn ptb(&self) -> &ProgrammableTransaction {
        let TransactionKind::Programmable(ptb) = &self.v1().kind else {
            panic!("not a programmable transaction");
        };
        return ptb;
    }

    fn input(&self, argument: Argument) -> Option<&Input> {
        let Argument::Input(index) = argument else {
            return None;
        };
        return Some(&self.ptb().inputs[usize::from(index)]);
    }

    fn commands(&self) -> Vec<String> {
        return self
            .ptb()
            .commands
            .iter()
            .map(|command| match command {
                Command::MoveCall(call) => format!("{}::{}", call.module, call.function),
                Command::MergeCoins(_) => "MergeCoins".to_string(),
                Command::SplitCoins(_) => "SplitCoins".to_string(),
                other => format!("{other:?}"),
            })
            .collect();
    }

    fn assert_well_formed(&self) {
        assert_eq!(self.v1().sender, sender());
        assert_eq!(self.signatures.len(), 1);
        SimpleVerifier
            .verify_transaction(&self.transaction, &self.signatures[0])
            .unwrap();
        assert_eq!(self.checkpoint_inclusion_timeout_ms, Some(30_000));
        // a mask is a set: the client sends its paths in its own order
        let mut read_mask = self.read_mask.clone();
        read_mask.sort();
        assert_eq!(
            read_mask,
            vec![
                "effects.bcs",
                "events.events.bcs",
                "output_objects.bcs",
                "transaction.digest",
            ]
        );
    }
}
