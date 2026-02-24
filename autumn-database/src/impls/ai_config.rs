use anyhow::Context as _;

use crate::database::Database;

pub async fn get_llm_enabled(db: &Database, guild_id: u64) -> anyhow::Result<bool> {
    let guild_id_i64 = i64::try_from(guild_id).context("guild_id out of i64 range")?;

    let enabled: Option<bool> =
        sqlx::query_scalar("SELECT llm_enabled FROM guild_ai_config WHERE guild_id = $1")
            .bind(guild_id_i64)
            .fetch_optional(db.pool())
            .await?
            .flatten();

    Ok(enabled.unwrap_or(true))
}

pub async fn set_llm_enabled(db: &Database, guild_id: u64, enabled: bool) -> anyhow::Result<()> {
    let guild_id_i64 = i64::try_from(guild_id).context("guild_id out of i64 range")?;

    sqlx::query(
        "INSERT INTO guild_ai_config (guild_id, llm_enabled)
         VALUES ($1, $2)
         ON CONFLICT (guild_id) DO UPDATE SET llm_enabled = EXCLUDED.llm_enabled",
    )
    .bind(guild_id_i64)
    .bind(enabled)
    .execute(db.pool())
    .await?;

    Ok(())
}
