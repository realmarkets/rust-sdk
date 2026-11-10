use std::time::Duration;

use futures_util::future::BoxFuture;
use iota_sdk_types::{Input, ObjectId};

use crate::utils::RetryHook;

/// Where [`crate::ObjectArgCache`] fetches the object arguments it does not
/// hold yet.
///
/// The one operation a cache miss needs: an object's current reference as a
/// transaction [`Input`], retrying transient failures until `budget` is spent
/// and calling `on_retry` on each failed attempt.
///
/// The method returns a boxed future rather than being an `async fn`: an
/// `async fn` would make the trait unusable as `dyn`, and the cache — with the
/// builders holding it — would need a client type parameter instead.
pub trait ObjectArgSource: Send + Sync {
    fn object_arg_within<'a>(
        &'a self,
        object_id: ObjectId,
        budget: Duration,
        on_retry: Option<&'a RetryHook>,
    ) -> BoxFuture<'a, anyhow::Result<Input>>;
}
