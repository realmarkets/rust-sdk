use std::{
    future::Future,
    time::{Duration, Instant},
};

use anyhow::{Context, anyhow, bail};
use iota_sdk_grpc_client::{GrpcClient, GrpcResult};
use iota_sdk_grpc_types::v1::object::Object as ProtoObject;
use iota_sdk_types::{
    Input, MoveStruct, Object, ObjectData, ObjectId, Owner, SharedObjectReference, StructTag,
};
use serde::de::DeserializeOwned;
use tracing::warn;

use super::Timed;
use crate::utils::{RetryHook, backoff_for};

/// Single-object reads.
pub(super) trait ObjectReads {
    /// The object `object_id`, or `None` when the node has no such object.
    fn fetch_object(
        &self,
        object_id: ObjectId,
    ) -> impl Future<Output = anyhow::Result<Option<Object>>> + Send;

    /// `object_id` as a transaction input, retrying failed calls with
    /// backoff until `budget` is spent; an error about the object itself
    /// fails at once.
    fn fetch_object_arg(
        &self,
        object_id: ObjectId,
        budget: Duration,
        on_retry: Option<&RetryHook>,
    ) -> impl Future<Output = anyhow::Result<Input>> + Send;

    /// A Move object's type and its contents decoded as `T`.
    fn fetch_move_object<T: DeserializeOwned>(
        &self,
        id: ObjectId,
    ) -> impl Future<Output = anyhow::Result<(StructTag, T)>> + Send;
}

impl ObjectReads for GrpcClient {
    async fn fetch_object(&self, object_id: ObjectId) -> anyhow::Result<Option<Object>> {
        return self
            .objects([object_id])
            .timed()
            .await?
            .into_inner()
            .into_single_object(object_id);
    }

    async fn fetch_object_arg(
        &self,
        object_id: ObjectId,
        budget: Duration,
        on_retry: Option<&RetryHook>,
    ) -> anyhow::Result<Input> {
        let started = Instant::now();
        let mut attempt = 0u32;
        let results = loop {
            attempt += 1;
            let err = match self.objects([object_id]).timed().await {
                Ok(response) => break response.into_inner(),
                Err(e) => e,
            };
            let backoff = backoff_for(attempt);
            if started.elapsed() + backoff > budget {
                return Err(err.into());
            }
            warn!(%object_id, attempt, err = %err, "object fetch failed, retrying");
            if let Some(hook) = on_retry {
                hook(&object_id.to_string(), attempt);
            }
            tokio::time::sleep(backoff).await;
        };

        return results
            .into_single_object(object_id)?
            .ok_or_else(|| anyhow!("object {object_id} not found"))?
            .object_arg();
    }

    async fn fetch_move_object<T: DeserializeOwned>(
        &self,
        id: ObjectId,
    ) -> anyhow::Result<(StructTag, T)> {
        let object = self
            .fetch_object(id)
            .await?
            .ok_or_else(|| anyhow!("object {id} not found"))?;
        let move_struct = object.move_struct()?;
        let value = bcs::from_bytes::<T>(move_struct.contents()).with_context(|| {
            format!(
                "object {id} of type {} does not decode as {}",
                move_struct.struct_tag(),
                std::any::type_name::<T>()
            )
        })?;
        return Ok((move_struct.struct_tag().clone(), value));
    }
}

/// The per-object results of a single-id `objects` request.
trait SingleObject {
    fn into_single_object(self, object_id: ObjectId) -> anyhow::Result<Option<Object>>;
}

impl SingleObject for Vec<GrpcResult<ProtoObject>> {
    // the client checks one result per requested id, in order, so a
    // single-id request yields exactly one
    fn into_single_object(mut self, object_id: ObjectId) -> anyhow::Result<Option<Object>> {
        let result = self
            .pop()
            .ok_or_else(|| anyhow!("no result for object {object_id}"))?;
        match result {
            Ok(object) => return Ok(Some(object.object()?)),
            Err(e) if e.is_not_found() => return Ok(None),
            Err(e) => return Err(e.into()),
        }
    }
}

pub(super) trait ObjectExt {
    /// The object as a transaction input, by its owner: a shared object by
    /// its initial shared version, immutable until the caller says otherwise;
    /// any other by its current reference.
    fn object_arg(&self) -> anyhow::Result<Input>;

    /// The Move struct this object holds; a package is an error.
    fn move_struct(&self) -> anyhow::Result<&MoveStruct>;
}

impl ObjectExt for Object {
    fn object_arg(&self) -> anyhow::Result<Input> {
        let object_ref = self.object_ref();
        match self.owner() {
            Owner::Shared(initial_shared_version) => {
                return Ok(Input::Shared(SharedObjectReference {
                    object_id: object_ref.object_id,
                    initial_shared_version: *initial_shared_version,
                    mutable: false,
                }));
            }
            Owner::Immutable | Owner::Address(_) | Owner::Object(_) => {
                return Ok(Input::ImmutableOrOwned(object_ref));
            }
            _ => bail!("unsupported object owner"),
        }
    }

    fn move_struct(&self) -> anyhow::Result<&MoveStruct> {
        match self.data() {
            ObjectData::Struct(move_struct) => return Ok(move_struct),
            _ => bail!("object {} is not a Move object", self.id()),
        }
    }
}
