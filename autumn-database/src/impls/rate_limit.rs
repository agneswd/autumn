use crate::cache::llm_mention_rate_limit_key;
use crate::database::Database;

pub async fn llm_mention_within_limit(
    db: &Database,
    guild_id: u64,
    channel_id: u64,
    user_id: u64,
) -> anyhow::Result<bool> {
    let cache = db.cache();
    let key = llm_mention_rate_limit_key(cache, guild_id, channel_id, user_id);
    let count = cache
        .increment_with_window(&key, cache.llm_rate_limit_window())
        .await?;
    let max_hits = cache.llm_rate_limit_max_hits();

    if count > max_hits {
        cache.record_rate_limit_block();
    }

    Ok(count <= max_hits)
}
