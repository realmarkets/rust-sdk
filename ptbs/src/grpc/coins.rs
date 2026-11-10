use std::{future::Future, str::FromStr};

use anyhow::{Context, anyhow};
use iota_sdk_grpc_client::GrpcClient;
use iota_sdk_grpc_types::v1::object::Object as ProtoObject;
use iota_sdk_types::{Address, Coin, ObjectId, ObjectReference, StructTag, TypeTag};

use super::Timed;
use crate::utils::GasCoinDetails;

/// How many gas coins come back: the page size of the JSON-RPC `get_coins`
/// call these replace.
const GAS_COIN_PAGE: usize = 100;

/// Objects per gRPC list page; the server caps it at 1000.
const LIST_PAGE_SIZE: u32 = 1000;

/// Coin reads.
pub(super) trait CoinReads {
    /// `address`'s coins of `coin_type`, the first [`GAS_COIN_PAGE`] in
    /// object-id order.
    fn fetch_coins(
        &self,
        address: Address,
        coin_type: TypeTag,
    ) -> impl Future<Output = anyhow::Result<Vec<GasCoinDetails>>> + Send;

    /// `address`'s gas coins, the first [`GAS_COIN_PAGE`] in object-id
    /// order.
    fn fetch_gas_coins(
        &self,
        address: Address,
    ) -> impl Future<Output = anyhow::Result<Vec<GasCoinDetails>>> + Send;

    /// The gas coin `reference` points at, by object id, among those
    /// [`Self::fetch_gas_coins`] returns — with its current version and
    /// balance.
    fn fetch_gas_coin(
        &self,
        address: Address,
        reference: ObjectReference,
    ) -> impl Future<Output = anyhow::Result<GasCoinDetails>> + Send;

    /// The `CoinMetadata` object ID of `coin_type`.
    fn fetch_coin_metadata_id(
        &self,
        coin_type: String,
    ) -> impl Future<Output = anyhow::Result<ObjectId>> + Send;
}

impl CoinReads for GrpcClient {
    // JSON-RPC listed coins by object id; gRPC lists them by descending
    // balance, so every coin is fetched, sorted by id and cut to the same page
    async fn fetch_coins(
        &self,
        address: Address,
        coin_type: TypeTag,
    ) -> anyhow::Result<Vec<GasCoinDetails>> {
        let objects = self
            .owned_objects(address)
            .object_type(StructTag::new_coin(coin_type))
            .page_size(LIST_PAGE_SIZE)
            .collect(None)
            .timed()
            .await?
            .into_inner();

        let mut coins = objects
            .iter()
            .map(GasCoinDetails::try_from)
            .collect::<anyhow::Result<Vec<_>>>()?;
        coins.sort_by_key(|coin| coin.r#ref.object_id);
        coins.truncate(GAS_COIN_PAGE);
        return Ok(coins);
    }

    async fn fetch_gas_coins(&self, address: Address) -> anyhow::Result<Vec<GasCoinDetails>> {
        return self
            .fetch_coins(address, TypeTag::from(StructTag::new_gas()))
            .await;
    }

    async fn fetch_gas_coin(
        &self,
        address: Address,
        reference: ObjectReference,
    ) -> anyhow::Result<GasCoinDetails> {
        return self
            .fetch_gas_coins(address)
            .await?
            .into_iter()
            .find(|coin| coin.r#ref.object_id == reference.object_id)
            .ok_or_else(|| anyhow!("gas coin not found"));
    }

    async fn fetch_coin_metadata_id(&self, coin_type: String) -> anyhow::Result<ObjectId> {
        let struct_tag = StructTag::from_str(&coin_type)
            .with_context(|| format!("invalid coin type: {coin_type}"))?;
        let response = match self.coin_info(struct_tag).timed().await {
            Ok(response) => response.into_inner(),
            Err(e) if e.is_not_found() => {
                return Err(anyhow!("coin metadata not found for type: {}", coin_type));
            }
            Err(e) => return Err(e.into()),
        };
        let metadata = response
            .metadata
            .ok_or_else(|| anyhow!("coin metadata not found for type: {}", coin_type))?;
        let id = metadata
            .id
            .ok_or_else(|| anyhow!("coin metadata has no object ID for type: {}", coin_type))?;
        return Ok(id.object_id()?);
    }
}

impl TryFrom<&ProtoObject> for GasCoinDetails {
    type Error = anyhow::Error;

    fn try_from(object: &ProtoObject) -> anyhow::Result<GasCoinDetails> {
        let object = object.object()?;
        let coin = Coin::try_from_object(&object)
            .with_context(|| format!("listed object {} is not a coin", object.id()))?;
        return Ok(GasCoinDetails {
            balance: coin.balance(),
            r#ref: object.object_ref(),
        });
    }
}
