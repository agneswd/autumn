mod noop_store;
mod redis_store;

use std::future::Future;
use std::time::Duration;

use serde::Serialize;
use serde::de::DeserializeOwned;
use tracing::warn;

use noop_store::NoopCacheStore;
use redis_store::RedisCacheStore;

#[derive(Clone, Debug)]
enum CacheBackend {
    Disabled(NoopCacheStore),
    Redis(RedisCacheStore),
}

#[derive(Clone, Debug)]
pub struct CacheService {
    key_prefix: String,
    backend: CacheBackend,
}

pub const CONFIG_CACHE_TTL: Duration = Duration::from_secs(15 * 60);
pub const WORD_LIST_CACHE_TTL: Duration = Duration::from_secs(5 * 60);

impl CacheService {
    pub fn disabled(prefix: impl Into<String>) -> Self {
        Self {
            key_prefix: prefix.into(),
            backend: CacheBackend::Disabled(NoopCacheStore),
        }
    }

    pub fn redis(redis_url: &str, prefix: impl Into<String>) -> anyhow::Result<Self> {
        Ok(Self {
            key_prefix: prefix.into(),
            backend: CacheBackend::Redis(RedisCacheStore::from_url(redis_url)?),
        })
    }

    pub fn key(&self, suffix: impl AsRef<str>) -> String {
        format!("{}:{}", self.key_prefix, suffix.as_ref())
    }

    pub async fn get_json<T>(&self, key: &str) -> anyhow::Result<Option<T>>
    where
        T: DeserializeOwned,
    {
        let value = match &self.backend {
            CacheBackend::Disabled(store) => store.get(key).await,
            CacheBackend::Redis(store) => store.get(key).await,
        }?;

        match value {
            Some(bytes) => {
                let parsed = serde_json::from_slice(&bytes).map_err(|e| {
                    anyhow::anyhow!("failed to deserialize cache value for `{key}`: {e}")
                })?;
                Ok(Some(parsed))
            }
            None => Ok(None),
        }
    }

    pub async fn set_json<T>(&self, key: &str, value: &T, ttl: Duration) -> anyhow::Result<()>
    where
        T: Serialize,
    {
        let ttl_seconds = ttl.as_secs().max(1);
        let payload = serde_json::to_vec(value)
            .map_err(|e| anyhow::anyhow!("failed to serialize cache value for `{key}`: {e}"))?;

        match &self.backend {
            CacheBackend::Disabled(store) => store.set(key, payload, ttl_seconds).await,
            CacheBackend::Redis(store) => store.set(key, payload, ttl_seconds).await,
        }
    }

    pub async fn del(&self, key: &str) -> anyhow::Result<()> {
        match &self.backend {
            CacheBackend::Disabled(store) => store.del(key).await,
            CacheBackend::Redis(store) => store.del(key).await,
        }
    }

    pub async fn get_or_load_json<T, F, Fut>(
        &self,
        key: &str,
        ttl: Duration,
        loader: F,
    ) -> anyhow::Result<T>
    where
        T: Serialize + DeserializeOwned + Clone,
        F: FnOnce() -> Fut,
        Fut: Future<Output = anyhow::Result<T>>,
    {
        match self.get_json::<T>(key).await {
            Ok(Some(cached)) => return Ok(cached),
            Ok(None) => {}
            Err(e) => warn!(
                ?e,
                cache_key = key,
                "cache get failed; falling back to database"
            ),
        }

        let loaded = loader().await?;

        if let Err(e) = self.set_json(key, &loaded, ttl).await {
            warn!(
                ?e,
                cache_key = key,
                "cache set failed; returning database value"
            );
        }

        Ok(loaded)
    }
}

pub fn ai_config_key(cache: &CacheService, guild_id: u64) -> String {
    cache.key(format!("guild:{guild_id}:config:ai"))
}

pub fn modlog_config_key(cache: &CacheService, guild_id: u64) -> String {
    cache.key(format!("guild:{guild_id}:config:modlog"))
}

pub fn escalation_config_key(cache: &CacheService, guild_id: u64) -> String {
    cache.key(format!("guild:{guild_id}:config:escalation"))
}

pub fn word_filter_config_key(cache: &CacheService, guild_id: u64) -> String {
    cache.key(format!("guild:{guild_id}:config:word_filter"))
}

pub fn word_filter_words_key(cache: &CacheService, guild_id: u64) -> String {
    cache.key(format!("guild:{guild_id}:config:word_filter_words"))
}

pub async fn invalidate_ai_config(cache: &CacheService, guild_id: u64) -> anyhow::Result<()> {
    cache.del(&ai_config_key(cache, guild_id)).await
}

pub async fn invalidate_modlog_config(cache: &CacheService, guild_id: u64) -> anyhow::Result<()> {
    cache.del(&modlog_config_key(cache, guild_id)).await
}

pub async fn invalidate_escalation_config(
    cache: &CacheService,
    guild_id: u64,
) -> anyhow::Result<()> {
    cache.del(&escalation_config_key(cache, guild_id)).await
}

pub async fn invalidate_word_filter(cache: &CacheService, guild_id: u64) -> anyhow::Result<()> {
    cache.del(&word_filter_config_key(cache, guild_id)).await?;
    cache.del(&word_filter_words_key(cache, guild_id)).await
}
