use anyhow::Context as _;

use crate::cache::{CONFIG_CACHE_TTL, invalidate_modlog_config, modlog_config_key};
use crate::database::Database;

pub async fn get_modlog_channel_id(db: &Database, guild_id: u64) -> anyhow::Result<Option<u64>> {
    let cache_key = modlog_config_key(db.cache(), guild_id);
    db.cache()
        .get_or_load_json(&cache_key, CONFIG_CACHE_TTL, || async {
            let guild_id_i64 = i64::try_from(guild_id).context("guild_id out of i64 range")?;

            let channel_id: Option<i64> = sqlx::query_scalar(
                "SELECT modlog_channel_id FROM guild_mod_config WHERE guild_id = $1",
            )
            .bind(guild_id_i64)
            .fetch_optional(db.pool())
            .await?
            .flatten();

            channel_id
                .map(u64::try_from)
                .transpose()
                .context("modlog_channel_id out of u64 range")
        })
        .await
}

pub async fn set_modlog_channel_id(
    db: &Database,
    guild_id: u64,
    channel_id: u64,
) -> anyhow::Result<()> {
    let guild_id_i64 = i64::try_from(guild_id).context("guild_id out of i64 range")?;
    let channel_id_i64 = i64::try_from(channel_id).context("channel_id out of i64 range")?;

    sqlx::query(
        "INSERT INTO guild_mod_config (guild_id, modlog_channel_id)
         VALUES ($1, $2)
         ON CONFLICT (guild_id) DO UPDATE SET modlog_channel_id = EXCLUDED.modlog_channel_id",
    )
    .bind(guild_id_i64)
    .bind(channel_id_i64)
    .execute(db.pool())
    .await?;

    invalidate_modlog_config(db.cache(), guild_id).await?;

    Ok(())
}

pub async fn clear_modlog_channel_id(db: &Database, guild_id: u64) -> anyhow::Result<()> {
    let guild_id_i64 = i64::try_from(guild_id).context("guild_id out of i64 range")?;

    sqlx::query("DELETE FROM guild_mod_config WHERE guild_id = $1")
        .bind(guild_id_i64)
        .execute(db.pool())
        .await?;

    invalidate_modlog_config(db.cache(), guild_id).await?;

    Ok(())
}
