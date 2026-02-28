use deadpool_redis::{Config, Pool, Runtime};
use redis::AsyncCommands;

#[derive(Clone, Debug)]
pub struct RedisCacheStore {
    pool: Pool,
}

impl RedisCacheStore {
    pub fn from_url(redis_url: &str) -> anyhow::Result<Self> {
        let config = Config::from_url(redis_url);
        let pool = config
            .create_pool(Some(Runtime::Tokio1))
            .map_err(|e| anyhow::anyhow!("failed to create redis pool: {e}"))?;

        Ok(Self { pool })
    }

    pub async fn get(&self, key: &str) -> anyhow::Result<Option<Vec<u8>>> {
        let mut conn = self
            .pool
            .get()
            .await
            .map_err(|e| anyhow::anyhow!("failed to get redis connection: {e}"))?;

        let value = conn
            .get::<_, Option<Vec<u8>>>(key)
            .await
            .map_err(|e| anyhow::anyhow!("redis GET failed for key `{key}`: {e}"))?;

        Ok(value)
    }

    pub async fn set(&self, key: &str, value: Vec<u8>, ttl_seconds: u64) -> anyhow::Result<()> {
        let mut conn = self
            .pool
            .get()
            .await
            .map_err(|e| anyhow::anyhow!("failed to get redis connection: {e}"))?;

        conn.set_ex::<_, _, ()>(key, value, ttl_seconds)
            .await
            .map_err(|e| anyhow::anyhow!("redis SETEX failed for key `{key}`: {e}"))?;

        Ok(())
    }

    pub async fn del(&self, key: &str) -> anyhow::Result<()> {
        let mut conn = self
            .pool
            .get()
            .await
            .map_err(|e| anyhow::anyhow!("failed to get redis connection: {e}"))?;

        let _ = conn
            .del::<_, u64>(key)
            .await
            .map_err(|e| anyhow::anyhow!("redis DEL failed for key `{key}`: {e}"))?;

        Ok(())
    }
}
