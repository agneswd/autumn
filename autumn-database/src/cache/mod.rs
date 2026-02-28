mod noop_store;
mod redis_store;

use std::future::Future;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
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
    stats: Arc<CacheStatsInner>,
    llm_rate_limit_window: Duration,
    llm_rate_limit_max_hits: u64,
}

#[derive(Debug, Default)]
struct CacheStatsInner {
    hit: AtomicU64,
    miss: AtomicU64,
    set: AtomicU64,
    del: AtomicU64,
    error: AtomicU64,
    fallback_load: AtomicU64,
    ratelimit_checks: AtomicU64,
    ratelimit_blocks: AtomicU64,
}

#[derive(Clone, Debug, Default)]
pub struct CacheStatsSnapshot {
    pub hit: u64,
    pub miss: u64,
    pub set: u64,
    pub del: u64,
    pub error: u64,
    pub fallback_load: u64,
    pub ratelimit_checks: u64,
    pub ratelimit_blocks: u64,
}

impl CacheStatsInner {
    fn snapshot(&self) -> CacheStatsSnapshot {
        CacheStatsSnapshot {
            hit: self.hit.load(Ordering::Relaxed),
            miss: self.miss.load(Ordering::Relaxed),
            set: self.set.load(Ordering::Relaxed),
            del: self.del.load(Ordering::Relaxed),
            error: self.error.load(Ordering::Relaxed),
            fallback_load: self.fallback_load.load(Ordering::Relaxed),
            ratelimit_checks: self.ratelimit_checks.load(Ordering::Relaxed),
            ratelimit_blocks: self.ratelimit_blocks.load(Ordering::Relaxed),
        }
    }
}

pub const CONFIG_CACHE_TTL: Duration = Duration::from_secs(15 * 60);
pub const WORD_LIST_CACHE_TTL: Duration = Duration::from_secs(5 * 60);
pub const DEFAULT_LLM_MENTION_RATE_LIMIT_WINDOW: Duration = Duration::from_secs(10);
pub const DEFAULT_LLM_MENTION_RATE_LIMIT_MAX_HITS: u64 = 2;

impl CacheService {
    pub fn disabled(prefix: impl Into<String>) -> Self {
        Self {
            key_prefix: prefix.into(),
            backend: CacheBackend::Disabled(NoopCacheStore),
            stats: Arc::new(CacheStatsInner::default()),
            llm_rate_limit_window: DEFAULT_LLM_MENTION_RATE_LIMIT_WINDOW,
            llm_rate_limit_max_hits: DEFAULT_LLM_MENTION_RATE_LIMIT_MAX_HITS,
        }
    }

    pub fn redis(redis_url: &str, prefix: impl Into<String>) -> anyhow::Result<Self> {
        Ok(Self {
            key_prefix: prefix.into(),
            backend: CacheBackend::Redis(RedisCacheStore::from_url(redis_url)?),
            stats: Arc::new(CacheStatsInner::default()),
            llm_rate_limit_window: DEFAULT_LLM_MENTION_RATE_LIMIT_WINDOW,
            llm_rate_limit_max_hits: DEFAULT_LLM_MENTION_RATE_LIMIT_MAX_HITS,
        })
    }

    pub fn configure_llm_rate_limit(&mut self, window: Duration, max_hits: u64) {
        let (window, max_hits) = normalize_llm_rate_limit(window, max_hits);
        self.llm_rate_limit_window = window;
        self.llm_rate_limit_max_hits = max_hits;
    }

    pub fn llm_rate_limit_window(&self) -> Duration {
        self.llm_rate_limit_window
    }

    pub fn llm_rate_limit_max_hits(&self) -> u64 {
        self.llm_rate_limit_max_hits
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
        }
        .inspect_err(|_| {
            self.stats.error.fetch_add(1, Ordering::Relaxed);
        })?;

        match value {
            Some(bytes) => {
                let parsed = serde_json::from_slice(&bytes)
                    .map_err(|e| {
                        anyhow::anyhow!("failed to deserialize cache value for `{key}`: {e}")
                    })
                    .inspect_err(|_| {
                        self.stats.error.fetch_add(1, Ordering::Relaxed);
                    })?;
                self.stats.hit.fetch_add(1, Ordering::Relaxed);
                Ok(Some(parsed))
            }
            None => {
                self.stats.miss.fetch_add(1, Ordering::Relaxed);
                Ok(None)
            }
        }
    }

    pub async fn set_json<T>(&self, key: &str, value: &T, ttl: Duration) -> anyhow::Result<()>
    where
        T: Serialize,
    {
        let ttl_seconds = ttl.as_secs().max(1);
        let payload = serde_json::to_vec(value)
            .map_err(|e| anyhow::anyhow!("failed to serialize cache value for `{key}`: {e}"))?;

        let result = match &self.backend {
            CacheBackend::Disabled(store) => store.set(key, payload, ttl_seconds).await,
            CacheBackend::Redis(store) => store.set(key, payload, ttl_seconds).await,
        };

        match result {
            Ok(()) => {
                self.stats.set.fetch_add(1, Ordering::Relaxed);
                Ok(())
            }
            Err(e) => {
                self.stats.error.fetch_add(1, Ordering::Relaxed);
                Err(e)
            }
        }
    }

    pub async fn del(&self, key: &str) -> anyhow::Result<()> {
        let result = match &self.backend {
            CacheBackend::Disabled(store) => store.del(key).await,
            CacheBackend::Redis(store) => store.del(key).await,
        };

        match result {
            Ok(()) => {
                self.stats.del.fetch_add(1, Ordering::Relaxed);
                Ok(())
            }
            Err(e) => {
                self.stats.error.fetch_add(1, Ordering::Relaxed);
                Err(e)
            }
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

        self.stats.fallback_load.fetch_add(1, Ordering::Relaxed);

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

    pub async fn increment_with_window(&self, key: &str, window: Duration) -> anyhow::Result<u64> {
        self.stats.ratelimit_checks.fetch_add(1, Ordering::Relaxed);
        let window_seconds = window.as_secs().max(1);
        let result = match &self.backend {
            CacheBackend::Disabled(store) => store.increment_with_window(key, window_seconds).await,
            CacheBackend::Redis(store) => store.increment_with_window(key, window_seconds).await,
        };

        match result {
            Ok(value) => Ok(value),
            Err(e) => {
                self.stats.error.fetch_add(1, Ordering::Relaxed);
                Err(e)
            }
        }
    }

    pub fn record_rate_limit_block(&self) {
        self.stats.ratelimit_blocks.fetch_add(1, Ordering::Relaxed);
    }

    pub fn stats_snapshot(&self) -> CacheStatsSnapshot {
        self.stats.snapshot()
    }

    pub async fn ping(&self) -> anyhow::Result<()> {
        match &self.backend {
            CacheBackend::Disabled(store) => store.ping().await,
            CacheBackend::Redis(store) => store.ping().await,
        }
    }

    pub fn is_redis_enabled(&self) -> bool {
        matches!(self.backend, CacheBackend::Redis(_))
    }
}

fn normalize_llm_rate_limit(window: Duration, max_hits: u64) -> (Duration, u64) {
    let window_seconds = window.as_secs().clamp(1, 3600);
    let normalized_hits = max_hits.max(1);
    (Duration::from_secs(window_seconds), normalized_hits)
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

pub fn llm_mention_rate_limit_key(
    cache: &CacheService,
    guild_id: u64,
    channel_id: u64,
    user_id: u64,
) -> String {
    cache.key(format!(
        "guild:{guild_id}:channel:{channel_id}:user:{user_id}:ratelimit:llm_mention"
    ))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn llm_key_generation_is_stable() {
        let cache = CacheService::disabled("autumn:test");
        let key = llm_mention_rate_limit_key(&cache, 1, 2, 3);
        assert_eq!(
            key,
            "autumn:test:guild:1:channel:2:user:3:ratelimit:llm_mention"
        );
    }

    #[test]
    fn normalize_llm_rate_limit_applies_bounds() {
        let (window, max_hits) = normalize_llm_rate_limit(Duration::from_secs(0), 0);
        assert_eq!(window, Duration::from_secs(1));
        assert_eq!(max_hits, 1);

        let (window, max_hits) = normalize_llm_rate_limit(Duration::from_secs(7200), 5);
        assert_eq!(window, Duration::from_secs(3600));
        assert_eq!(max_hits, 5);
    }
}
