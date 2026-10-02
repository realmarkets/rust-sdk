use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use anyhow::Context;
use iota_json_rpc_api::{IndexerApiClient as _, ReadApiClient as _};
use iota_json_rpc_types::{
    IotaData, IotaDynamicFieldInfo, IotaObjectDataOptions, IotaObjectResponse,
    IotaObjectResponseError,
};
use iota_sdk::IotaClient;
use iota_sdk_types::{
    Address, Input, ObjectId, ObjectReference, Owner, SharedObjectReference, StructTag,
};
use iota_types::dynamic_field::DynamicFieldName;
use tracing::warn;

/// called on every failed fetch attempt with the object id and attempt number
pub type RetryHook = Arc<dyn Fn(&str, u32) + Send + Sync>;

/// Trait for setting mutability on Input
pub trait Mutable {
    /// Set the mutability of this object argument
    fn set_mutable(self, mutable: bool) -> Self;
}

impl Mutable for Input {
    fn set_mutable(self, mutable: bool) -> Self {
        match self {
            Input::Shared(shared) => Input::Shared(SharedObjectReference { mutable, ..shared }),
            _ => self,
        }
    }
}

/// Helper trait to convert various types into ObjectId
pub trait IntoObjectId {
    fn into_object_id(self) -> anyhow::Result<ObjectId>;
}

impl IntoObjectId for ObjectId {
    fn into_object_id(self) -> anyhow::Result<ObjectId> {
        Ok(self)
    }
}

impl IntoObjectId for &ObjectId {
    fn into_object_id(self) -> anyhow::Result<ObjectId> {
        Ok(*self)
    }
}

impl IntoObjectId for &str {
    fn into_object_id(self) -> anyhow::Result<ObjectId> {
        use std::str::FromStr;
        ObjectId::from_str(self).map_err(|e| anyhow::anyhow!(e))
    }
}

impl IntoObjectId for String {
    fn into_object_id(self) -> anyhow::Result<ObjectId> {
        use std::str::FromStr;
        ObjectId::from_str(&self).map_err(|e| anyhow::anyhow!(e))
    }
}

impl IntoObjectId for &String {
    fn into_object_id(self) -> anyhow::Result<ObjectId> {
        use std::str::FromStr;
        ObjectId::from_str(self).map_err(|e| anyhow::anyhow!(e))
    }
}

/// Gas coin details containing balance and object reference
#[derive(Clone, Debug)]
pub struct GasCoinDetails {
    pub balance: u64,
    pub r#ref: ObjectReference,
}

/// Extension trait for IotaClient to provide helper methods for PTB building
#[allow(async_fn_in_trait)]
pub trait IotaClientExt {
    /// Fetch an object from the chain and convert it to an Input based on its Owner type.
    /// Shared objects default to immutable (mutable=false) and can be made mutable using `.set_mutable(true)`.
    ///
    /// Accepts anything that can be converted into an ObjectId (ObjectId, &ObjectId, &str, String, etc.)
    async fn get_object_arg(&self, object_id: impl IntoObjectId) -> anyhow::Result<Input>;

    /// Like [`Self::get_object_arg`], but retries transport failures with
    /// backoff until `budget` is spent. Object-level errors (not found,
    /// deleted, no owner) are deterministic and fail immediately.
    async fn get_object_arg_within(
        &self,
        object_id: impl IntoObjectId,
        budget: Duration,
        on_retry: Option<&RetryHook>,
    ) -> anyhow::Result<Input>;

    /// Get details for a specific gas coin by its object reference
    async fn get_gas_coin_detail(
        &self,
        address: Address,
        reference: ObjectReference,
    ) -> anyhow::Result<GasCoinDetails>;

    /// Get details for all gas coins owned by an address
    async fn get_gas_coin_details(&self, address: Address) -> anyhow::Result<Vec<GasCoinDetails>>;

    /// Get the coin metadata object ID for a given coin type
    async fn get_coin_metadata(&self, coin_type: impl Into<String>) -> anyhow::Result<ObjectId>;

    /// List all dynamic fields on a parent object (paginated)
    async fn get_dynamic_fields(
        &self,
        parent_object_id: ObjectId,
    ) -> anyhow::Result<Vec<IotaDynamicFieldInfo>>;

    /// Read a dynamic field's parsed JSON value
    fn json_dynamic_field<'a>(
        &'a self,
        parent: ObjectId,
        df_name: DynamicFieldName,
    ) -> impl Future<Output = anyhow::Result<serde_json::Value>> + Send + 'a;

    /// Like [`Self::json_dynamic_field`], but returns `Ok(None)` when `parent`
    /// carries no field under `df_name`. Every other failure is propagated.
    fn json_dynamic_field_opt<'a>(
        &'a self,
        parent: ObjectId,
        df_name: DynamicFieldName,
    ) -> impl Future<Output = anyhow::Result<Option<serde_json::Value>>> + Send + 'a;

    /// Read an object's parsed Move struct type and JSON value
    fn parsed_object<'a>(
        &'a self,
        id: ObjectId,
    ) -> impl Future<Output = anyhow::Result<(StructTag, serde_json::Value)>> + Send + 'a;
}

// transport errors (closed keep-alives, timeouts) are transient on a healthy
// fullnode and retried with backoff until the budget is spent; errors carried
// inside the response are deterministic and surface in the caller instead
async fn fetch_object_within(
    client: &IotaClient,
    object_id: ObjectId,
    budget: Duration,
    on_retry: Option<&RetryHook>,
) -> anyhow::Result<IotaObjectResponse> {
    let started = Instant::now();
    let mut attempt = 0u32;
    loop {
        attempt += 1;
        let err = match client
            .read_api()
            .get_object_with_options(object_id, IotaObjectDataOptions::new().with_owner())
            .await
        {
            Ok(resp) => return Ok(resp),
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
    }
}

// exponential backoff: 100ms, 200ms, ..., capped at 2s; the attempt clamp
// keeps the shift from overflowing during long retry windows
pub fn backoff_for(attempt: u32) -> Duration {
    return Duration::from_millis(std::cmp::min(50u64 << attempt.min(6), 2000));
}

impl IotaClientExt for IotaClient {
    async fn get_object_arg(&self, object_id: impl IntoObjectId) -> anyhow::Result<Input> {
        return self
            .get_object_arg_within(object_id, Duration::ZERO, None)
            .await;
    }

    async fn get_object_arg_within(
        &self,
        object_id: impl IntoObjectId,
        budget: Duration,
        on_retry: Option<&RetryHook>,
    ) -> anyhow::Result<Input> {
        let object_id = object_id.into_object_id()?;
        let resp = fetch_object_within(self, object_id, budget, on_retry).await?;

        let object = resp.object()?;
        let object_ref = object.object_ref();

        let arg = match object.owner {
            Some(Owner::Shared(initial_shared_version)) => Input::Shared(SharedObjectReference {
                object_id: object_ref.object_id,
                initial_shared_version,
                mutable: false, // Default to immutable, use .set_mutable(true) if needed
            }),
            Some(Owner::Immutable) => Input::ImmutableOrOwned(object_ref),
            Some(Owner::Address(_)) => Input::ImmutableOrOwned(object_ref),
            Some(Owner::Object(_)) => Input::ImmutableOrOwned(object_ref),
            Some(_) => anyhow::bail!("unsupported object owner"),
            None => anyhow::bail!("Object has no owner"),
        };

        return Ok(arg);
    }

    async fn get_gas_coin_detail(
        &self,
        address: Address,
        reference: ObjectReference,
    ) -> anyhow::Result<GasCoinDetails> {
        let coins = self
            .coin_read_api()
            .get_coins(address, None, None, 100)
            .await?;

        return coins
            .data
            .iter()
            .find(|&c| c.object_ref().object_id == reference.object_id)
            .map(|c| GasCoinDetails {
                balance: c.balance,
                r#ref: c.object_ref(),
            })
            .ok_or_else(|| anyhow::anyhow!("gas coin not found"));
    }

    async fn get_gas_coin_details(&self, address: Address) -> anyhow::Result<Vec<GasCoinDetails>> {
        let coins = self
            .coin_read_api()
            .get_coins(address, None, None, 100)
            .await?;

        return Ok(coins
            .data
            .iter()
            .map(|c| GasCoinDetails {
                balance: c.balance,
                r#ref: c.object_ref(),
            })
            .collect());
    }

    async fn get_coin_metadata(&self, coin_type: impl Into<String>) -> anyhow::Result<ObjectId> {
        let coin_type = coin_type.into();
        let metadata = self
            .coin_read_api()
            .get_coin_metadata(&coin_type)
            .await?
            .ok_or_else(|| anyhow::anyhow!("coin metadata not found for type: {}", coin_type))?;

        let object_id = metadata.id.ok_or_else(|| {
            anyhow::anyhow!("coin metadata has no object ID for type: {}", coin_type)
        })?;

        return Ok(object_id);
    }

    async fn get_dynamic_fields(
        &self,
        parent_object_id: ObjectId,
    ) -> anyhow::Result<Vec<IotaDynamicFieldInfo>> {
        let mut all_fields = Vec::new();
        let mut cursor = None;
        loop {
            let page = self
                .http()
                .get_dynamic_fields(parent_object_id, cursor, Some(50))
                .await?;
            all_fields.extend(page.data);
            if !page.has_next_page {
                break;
            }
            cursor = page.next_cursor;
        }
        return Ok(all_fields);
    }

    fn json_dynamic_field<'a>(
        &'a self,
        parent: ObjectId,
        df_name: DynamicFieldName,
    ) -> impl Future<Output = anyhow::Result<serde_json::Value>> + Send + 'a {
        let fut = self.json_dynamic_field_opt(parent, df_name);
        async move {
            return fut
                .await?
                .with_context(|| format!("no dynamic field on parent object {parent}"));
        }
    }

    fn json_dynamic_field_opt<'a>(
        &'a self,
        parent: ObjectId,
        df_name: DynamicFieldName,
    ) -> impl Future<Output = anyhow::Result<Option<serde_json::Value>>> + Send + 'a {
        let fut = self.http().get_dynamic_field_object(parent, df_name);
        async move {
            // a missing field arrives as an error *inside* a successful response,
            // so it has to be told apart from a real RPC or decode failure
            let object = match fut.await?.into_object() {
                Ok(object) => object,
                Err(IotaObjectResponseError::DynamicFieldNotFound { .. }) => return Ok(None),
                Err(e) => return Err(e.into()),
            };

            return Ok(Some(
                object
                    .content
                    .context("missing content")?
                    .try_into_move()
                    .context("not a Move struct")?
                    .fields
                    .to_json_value()["value"]
                    .take(),
            ));
        }
    }

    fn parsed_object<'a>(
        &'a self,
        id: ObjectId,
    ) -> impl Future<Output = anyhow::Result<(StructTag, serde_json::Value)>> + Send + 'a {
        let fut = self
            .http()
            .get_object(id, Some(IotaObjectDataOptions::new().with_content()));
        async move {
            let parsed_obj = fut
                .await?
                .into_object()?
                .content
                .context("missing content")?
                .try_into_move()
                .context("not a Move struct")?;
            Ok((parsed_obj.struct_tag, parsed_obj.fields.to_json_value()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backoff_doubles_then_caps_at_two_seconds() {
        assert_eq!(backoff_for(1), Duration::from_millis(100));
        assert_eq!(backoff_for(2), Duration::from_millis(200));
        assert_eq!(backoff_for(3), Duration::from_millis(400));
        assert_eq!(backoff_for(4), Duration::from_millis(800));
        assert_eq!(backoff_for(5), Duration::from_millis(1600));
        assert_eq!(backoff_for(6), Duration::from_millis(2000));
        // high attempt counts during a long retry window must not overflow
        assert_eq!(backoff_for(1800), Duration::from_millis(2000));
    }
}
