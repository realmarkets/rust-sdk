use std::{collections::HashMap, sync::Arc, time::Duration};

use iota_sdk::IotaClient;
use iota_sdk_types::Input;
use tokio::sync::RwLock;

use crate::utils::{IotaClientExt, RetryHook};

#[allow(async_fn_in_trait)]
pub trait ObjectArgCacheT {
    async fn get(&mut self, key: &str) -> anyhow::Result<Input>;
}

#[derive(Clone)]
pub struct ObjectArgCache {
    client: IotaClient,
    // retry window for transport failures on cache-miss fetches
    budget: Duration,
    on_retry: Option<RetryHook>,
    // the main cache shared between task
    objects: Arc<RwLock<HashMap<String, Input>>>,
}

impl ObjectArgCache {
    pub fn new(client: IotaClient) -> ObjectArgCache {
        return ObjectArgCache {
            client,
            budget: Duration::ZERO,
            on_retry: None,
            objects: Arc::new(RwLock::new(HashMap::new())),
        };
    }

    /// retry transport failures on cache-miss fetches for up to `budget`
    pub fn with_budget(mut self, budget: Duration) -> ObjectArgCache {
        self.budget = budget;
        return self;
    }

    /// called on every failed fetch attempt, e.g. to feed a retry metric
    pub fn on_retry(mut self, hook: RetryHook) -> ObjectArgCache {
        self.on_retry = Some(hook);
        return self;
    }

    async fn read_object(&self, key: &str) -> Option<Input> {
        let read_guard = self.objects.read().await;
        read_guard.get(key).cloned()
    }

    async fn write_object(&self, key: &str) -> anyhow::Result<Input> {
        let obj = self
            .client
            .get_object_arg_within(key, self.budget, self.on_retry.as_ref())
            .await?;

        {
            let mut cache = self.objects.write().await;
            cache.insert(key.to_string(), obj.clone());
        }
        return Ok(obj);
    }
}

impl ObjectArgCacheT for ObjectArgCache {
    async fn get(&mut self, key: &str) -> anyhow::Result<Input> {
        if let Some(obj) = self.read_object(key).await {
            return Ok(obj);
        }

        return self.write_object(key).await;
    }
}

// A transient cache created when a task is launched.
// it will have it's own cache, acccessible only by this
// task to prevent locking the main cache.
#[derive(Clone)]
pub struct TransientCache {
    // an hot cache, which do not required
    // to access the inner Arc<RwLock<>> of the main
    // cache
    hot_cache: HashMap<String, Input>,
    cold_cache: ObjectArgCache,
}

impl TransientCache {
    pub fn with_cache(cold_cache: ObjectArgCache) -> TransientCache {
        return TransientCache {
            hot_cache: HashMap::new(),
            cold_cache,
        };
    }
}

impl ObjectArgCacheT for TransientCache {
    async fn get(&mut self, key: &str) -> anyhow::Result<Input> {
        // already in the top level cache, just return it
        if let Some(obj) = self.hot_cache.get(key) {
            return Ok(obj.clone());
        }

        let obj = self.cold_cache.get(key).await?;
        self.hot_cache.insert(key.to_string(), obj.clone());

        return Ok(obj);
    }
}
