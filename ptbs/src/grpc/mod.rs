//! Chain reads over a fullnode's gRPC API: what PTB building needs — object
//! arguments, gas coins, coin metadata, dynamic fields — plus typed reads of
//! Move objects, on the SDK's `GrpcClient`.
//!
//! gRPC serves objects as BCS only, so there is no counterpart to the
//! JSON-RPC client's JSON reads (`parsed_object`, `json_dynamic_field`):
//! callers decode objects into [`crate::mirrors`] with
//! [`IotaClientExt::move_object`] and [`IotaClientExt::dynamic_field`].
//!
//! The failure model matches the JSON-RPC client's. A failure of the call
//! itself — transport, a status for the whole request, no answer within
//! [`READ_TIMEOUT`] — is transient: [`IotaClientExt::get_object_arg_within`]
//! retries it within its budget. An error about one object (not found,
//! deleted) is the answer and comes back at once. The gRPC client sets no
//! request timeout, so every call here is bounded by [`READ_TIMEOUT`], the
//! JSON-RPC client's default.
//!
//! [`IotaClientExt`] is the public surface; each submodule implements its
//! share as a private extension trait on `GrpcClient` that the public impl
//! delegates to.

use std::{
    future::{Future, IntoFuture},
    time::Duration,
};

use futures_util::future::BoxFuture;
use iota_sdk_grpc_client::{GrpcError, GrpcResult, tonic};
use iota_sdk_types::{Address, Input, ObjectId, ObjectReference, StructTag, TypeTag};
use serde::{Serialize, de::DeserializeOwned};

use crate::{
    dynamic_field::DynamicFieldInfo,
    object_source::ObjectArgSource,
    utils::{GasCoinDetails, IntoObjectId, RetryHook},
};

mod coins;
mod dynamic_fields;
mod isolated_scan;
#[cfg(any(test, feature = "test-helpers"))]
pub mod mock_node;
mod object_args;
#[cfg(test)]
mod tests;

use coins::CoinReads;
use dynamic_fields::FieldReads;
pub use iota_sdk_grpc_client::GrpcClient;
pub use isolated_scan::isolated_markets;
use object_args::ObjectReads;

/// How long one gRPC call may take before it counts as failed.
pub const READ_TIMEOUT: Duration = Duration::from_secs(60);

/// What PTB building reads from the chain, over gRPC.
pub trait IotaClientExt {
    /// An object as a transaction input, from its owner: a shared object as
    /// immutable — make it mutable with `.set_mutable(true)` — anything else
    /// by its current reference.
    fn get_object_arg(
        &self,
        object_id: impl IntoObjectId,
    ) -> impl Future<Output = anyhow::Result<Input>> + Send;

    /// Like [`Self::get_object_arg`], but retries failed calls with backoff
    /// until `budget` is spent, calling `on_retry` on each. Errors about the
    /// object itself (not found, deleted) fail at once.
    fn get_object_arg_within(
        &self,
        object_id: impl IntoObjectId,
        budget: Duration,
        on_retry: Option<&RetryHook>,
    ) -> impl Future<Output = anyhow::Result<Input>> + Send;

    /// The current reference and balance of one of `address`'s gas coins,
    /// looked up among those [`Self::get_gas_coin_details`] returns.
    fn get_gas_coin_detail(
        &self,
        address: Address,
        reference: ObjectReference,
    ) -> impl Future<Output = anyhow::Result<GasCoinDetails>> + Send;

    /// `address`'s gas coins, the first 100 in object-id order — the page
    /// the JSON-RPC `get_coins` call returned.
    fn get_gas_coin_details(
        &self,
        address: Address,
    ) -> impl Future<Output = anyhow::Result<Vec<GasCoinDetails>>> + Send;

    /// `address`'s coins of `coin_type`, the first 100 in object-id order,
    /// as [`Self::get_gas_coin_details`] lists the gas coin's.
    fn get_coin_details(
        &self,
        address: Address,
        coin_type: TypeTag,
    ) -> impl Future<Output = anyhow::Result<Vec<GasCoinDetails>>> + Send;

    /// The `CoinMetadata` object ID of `coin_type`.
    fn get_coin_metadata(
        &self,
        coin_type: impl Into<String>,
    ) -> impl Future<Output = anyhow::Result<ObjectId>> + Send;

    /// Every dynamic field of `parent_object_id`.
    fn get_dynamic_fields(
        &self,
        parent_object_id: ObjectId,
    ) -> impl Future<Output = anyhow::Result<Vec<DynamicFieldInfo>>> + Send;

    /// A Move object's type and its contents decoded as `T`.
    fn move_object<T: DeserializeOwned>(
        &self,
        id: ObjectId,
    ) -> impl Future<Output = anyhow::Result<(StructTag, T)>> + Send;

    /// The value of `parent`'s dynamic field named `name` (of type
    /// `name_type`), decoded as `V`, or `None` when it has no such field.
    /// Object fields are not covered: their name is wrapped.
    fn dynamic_field<V: DeserializeOwned>(
        &self,
        parent: ObjectId,
        name_type: TypeTag,
        name: &impl Serialize,
    ) -> impl Future<Output = anyhow::Result<Option<V>>> + Send;
}

impl IotaClientExt for GrpcClient {
    fn get_object_arg(
        &self,
        object_id: impl IntoObjectId,
    ) -> impl Future<Output = anyhow::Result<Input>> + Send {
        return self.get_object_arg_within(object_id, Duration::ZERO, None);
    }

    fn get_object_arg_within(
        &self,
        object_id: impl IntoObjectId,
        budget: Duration,
        on_retry: Option<&RetryHook>,
    ) -> impl Future<Output = anyhow::Result<Input>> + Send {
        let object_id = object_id.into_object_id();
        return async move {
            return self.fetch_object_arg(object_id?, budget, on_retry).await;
        };
    }

    fn get_gas_coin_detail(
        &self,
        address: Address,
        reference: ObjectReference,
    ) -> impl Future<Output = anyhow::Result<GasCoinDetails>> + Send {
        return self.fetch_gas_coin(address, reference);
    }

    fn get_gas_coin_details(
        &self,
        address: Address,
    ) -> impl Future<Output = anyhow::Result<Vec<GasCoinDetails>>> + Send {
        return self.fetch_gas_coins(address);
    }

    fn get_coin_details(
        &self,
        address: Address,
        coin_type: TypeTag,
    ) -> impl Future<Output = anyhow::Result<Vec<GasCoinDetails>>> + Send {
        return self.fetch_coins(address, coin_type);
    }

    fn get_coin_metadata(
        &self,
        coin_type: impl Into<String>,
    ) -> impl Future<Output = anyhow::Result<ObjectId>> + Send {
        return self.fetch_coin_metadata_id(coin_type.into());
    }

    fn get_dynamic_fields(
        &self,
        parent_object_id: ObjectId,
    ) -> impl Future<Output = anyhow::Result<Vec<DynamicFieldInfo>>> + Send {
        return self.fetch_dynamic_fields(parent_object_id);
    }

    fn move_object<T: DeserializeOwned>(
        &self,
        id: ObjectId,
    ) -> impl Future<Output = anyhow::Result<(StructTag, T)>> + Send {
        return self.fetch_move_object(id);
    }

    fn dynamic_field<V: DeserializeOwned>(
        &self,
        parent: ObjectId,
        name_type: TypeTag,
        name: &impl Serialize,
    ) -> impl Future<Output = anyhow::Result<Option<V>>> + Send {
        let name = bcs::to_bytes(name);
        return async move {
            return self.fetch_dynamic_field(parent, &name_type, &name?).await;
        };
    }
}

impl ObjectArgSource for GrpcClient {
    fn object_arg_within<'a>(
        &'a self,
        object_id: ObjectId,
        budget: Duration,
        on_retry: Option<&'a RetryHook>,
    ) -> BoxFuture<'a, anyhow::Result<Input>> {
        return Box::pin(self.fetch_object_arg(object_id, budget, on_retry));
    }
}

/// A gRPC query that gives up after [`READ_TIMEOUT`]. A timeout is a failure
/// of the call, like a dropped connection.
trait Timed<T> {
    fn timed(self) -> impl Future<Output = GrpcResult<T>> + Send;
}

impl<T: Send, Q> Timed<T> for Q
where
    Q: IntoFuture<Output = GrpcResult<T>>,
    Q::IntoFuture: Send,
{
    fn timed(self) -> impl Future<Output = GrpcResult<T>> + Send {
        let query = self.into_future();
        return async move {
            match tokio::time::timeout(READ_TIMEOUT, query).await {
                Ok(result) => return result,
                Err(_) => {
                    return Err(GrpcError::from(tonic::Status::deadline_exceeded(format!(
                        "no response within {READ_TIMEOUT:?}"
                    ))));
                }
            }
        };
    }
}
