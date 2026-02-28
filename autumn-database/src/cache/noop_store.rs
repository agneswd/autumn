#[derive(Clone, Debug, Default)]
pub struct NoopCacheStore;

impl NoopCacheStore {
    pub async fn get(&self, _key: &str) -> anyhow::Result<Option<Vec<u8>>> {
        Ok(None)
    }

    pub async fn set(&self, _key: &str, _value: Vec<u8>, _ttl_seconds: u64) -> anyhow::Result<()> {
        Ok(())
    }

    pub async fn del(&self, _key: &str) -> anyhow::Result<()> {
        Ok(())
    }
}
