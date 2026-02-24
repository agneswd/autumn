use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::Context as _;

use crate::{database::Database, model::llm_chat::LlmChatEntry};

#[derive(sqlx::FromRow)]
struct LlmChatRow {
    user_id: i64,
    display_name: Option<String>,
    role: String,
    content: String,
    created_at: i64,
}

pub async fn insert_llm_chat_message(
    db: &Database,
    guild_id: u64,
    channel_id: u64,
    user_id: u64,
    display_name: Option<&str>,
    role: &str,
    content: &str,
) -> anyhow::Result<()> {
    let guild_id_i64 = i64::try_from(guild_id).context("guild_id out of i64 range")?;
    let channel_id_i64 = i64::try_from(channel_id).context("channel_id out of i64 range")?;
    let user_id_i64 = i64::try_from(user_id).context("user_id out of i64 range")?;
    let created_at_i64 = i64::try_from(now_unix_secs()).context("created_at out of i64 range")?;

    sqlx::query(
           "INSERT INTO llm_chat_history (guild_id, channel_id, user_id, display_name, role, content, created_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7)",
    )
    .bind(guild_id_i64)
    .bind(channel_id_i64)
    .bind(user_id_i64)
        .bind(display_name)
    .bind(role)
    .bind(content)
    .bind(created_at_i64)
    .execute(db.pool())
    .await?;

    Ok(())
}

pub async fn list_recent_llm_chat_messages(
    db: &Database,
    guild_id: u64,
    channel_id: u64,
    limit: u32,
) -> anyhow::Result<Vec<LlmChatEntry>> {
    let guild_id_i64 = i64::try_from(guild_id).context("guild_id out of i64 range")?;
    let channel_id_i64 = i64::try_from(channel_id).context("channel_id out of i64 range")?;
    let limit_i64 = i64::from(limit.clamp(1, 200));

    let rows: Vec<LlmChatRow> = sqlx::query_as(
        "SELECT user_id, display_name, role, content, created_at
         FROM llm_chat_history
         WHERE guild_id = $1 AND channel_id = $2
         ORDER BY created_at DESC, id DESC
         LIMIT $3",
    )
    .bind(guild_id_i64)
    .bind(channel_id_i64)
    .bind(limit_i64)
    .fetch_all(db.pool())
    .await?;

    let mut out = Vec::with_capacity(rows.len());
    for row in rows {
        out.push(LlmChatEntry {
            user_id: u64::try_from(row.user_id).context("user_id row out of u64 range")?,
            display_name: row.display_name,
            role: row.role,
            content: row.content,
            created_at: u64::try_from(row.created_at).context("created_at row out of u64 range")?,
        });
    }

    Ok(out)
}

fn now_unix_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs())
}
