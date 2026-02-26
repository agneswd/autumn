use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::Context as _;

use crate::database::Database;
use crate::model::escalation::EscalationConfig;

// ---------------------------------------------------------------------------
// Escalation tiers: number of past timeouts → duration in seconds
// ---------------------------------------------------------------------------

/// Returns the timeout duration (in seconds) based on how many timeouts
/// the user already has within the escalation window.
///
/// Tier 0 (1st timeout) → 5 minutes
/// Tier 1 (2nd timeout) → 30 minutes
/// Tier 2 (3rd timeout) → 2 hours
/// Tier 3 (4th timeout) → 1 day
/// Tier 4+ (5th+ timeout) → 1 week
pub fn escalation_timeout_seconds(previous_timeout_count: i64) -> i64 {
    match previous_timeout_count {
        0 => 300,     // 5 minutes
        1 => 1_800,   // 30 minutes
        2 => 7_200,   // 2 hours
        3 => 86_400,  // 1 day
        _ => 604_800, // 1 week
    }
}

// ---------------------------------------------------------------------------
// Config CRUD
// ---------------------------------------------------------------------------

pub async fn get_escalation_config(
    db: &Database,
    guild_id: u64,
) -> anyhow::Result<Option<EscalationConfig>> {
    let guild_id_i64 = i64::try_from(guild_id).context("guild_id out of i64 range")?;

    let row = sqlx::query_as::<_, EscalationConfig>(
        "SELECT guild_id, enabled, warn_threshold, warn_window_seconds, timeout_window_seconds \
         FROM escalation_config WHERE guild_id = $1",
    )
    .bind(guild_id_i64)
    .fetch_optional(db.pool())
    .await?;

    Ok(row)
}

/// Get the escalation config only if it is enabled.
pub async fn get_escalation_if_enabled(
    db: &Database,
    guild_id: u64,
) -> anyhow::Result<Option<EscalationConfig>> {
    let guild_id_i64 = i64::try_from(guild_id).context("guild_id out of i64 range")?;

    let row = sqlx::query_as::<_, EscalationConfig>(
        "SELECT guild_id, enabled, warn_threshold, warn_window_seconds, timeout_window_seconds \
         FROM escalation_config WHERE guild_id = $1 AND enabled = TRUE",
    )
    .bind(guild_id_i64)
    .fetch_optional(db.pool())
    .await?;

    Ok(row)
}

pub async fn set_escalation_enabled(
    db: &Database,
    guild_id: u64,
    enabled: bool,
) -> anyhow::Result<()> {
    let guild_id_i64 = i64::try_from(guild_id).context("guild_id out of i64 range")?;

    sqlx::query(
        "INSERT INTO escalation_config (guild_id, enabled) VALUES ($1, $2) \
         ON CONFLICT (guild_id) DO UPDATE SET enabled = $2",
    )
    .bind(guild_id_i64)
    .bind(enabled)
    .execute(db.pool())
    .await?;

    Ok(())
}

pub async fn set_warn_threshold(
    db: &Database,
    guild_id: u64,
    threshold: i32,
) -> anyhow::Result<()> {
    let guild_id_i64 = i64::try_from(guild_id).context("guild_id out of i64 range")?;

    sqlx::query(
        "INSERT INTO escalation_config (guild_id, warn_threshold) VALUES ($1, $2) \
         ON CONFLICT (guild_id) DO UPDATE SET warn_threshold = $2",
    )
    .bind(guild_id_i64)
    .bind(threshold)
    .execute(db.pool())
    .await?;

    Ok(())
}

pub async fn set_warn_window(
    db: &Database,
    guild_id: u64,
    window_seconds: i64,
) -> anyhow::Result<()> {
    let guild_id_i64 = i64::try_from(guild_id).context("guild_id out of i64 range")?;

    sqlx::query(
        "INSERT INTO escalation_config (guild_id, warn_window_seconds) VALUES ($1, $2) \
         ON CONFLICT (guild_id) DO UPDATE SET warn_window_seconds = $2",
    )
    .bind(guild_id_i64)
    .bind(window_seconds)
    .execute(db.pool())
    .await?;

    Ok(())
}

pub async fn set_timeout_window(
    db: &Database,
    guild_id: u64,
    window_seconds: i64,
) -> anyhow::Result<()> {
    let guild_id_i64 = i64::try_from(guild_id).context("guild_id out of i64 range")?;

    sqlx::query(
        "INSERT INTO escalation_config (guild_id, timeout_window_seconds) VALUES ($1, $2) \
         ON CONFLICT (guild_id) DO UPDATE SET timeout_window_seconds = $2",
    )
    .bind(guild_id_i64)
    .bind(window_seconds)
    .execute(db.pool())
    .await?;

    Ok(())
}

// ---------------------------------------------------------------------------
// Counting helpers (used by the escalation check)
// ---------------------------------------------------------------------------

fn now_unix_secs() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs()) as i64
}

/// Count warnings for a user within a guild in the given time window.
pub async fn count_warnings_in_window(
    db: &Database,
    guild_id: u64,
    user_id: u64,
    window_seconds: i64,
) -> anyhow::Result<i64> {
    let guild_id_i64 = i64::try_from(guild_id).context("guild_id out of i64 range")?;
    let user_id_i64 = i64::try_from(user_id).context("user_id out of i64 range")?;
    let since = now_unix_secs() - window_seconds;

    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM warnings \
         WHERE guild_id = $1 AND user_id = $2 AND warned_at >= $3",
    )
    .bind(guild_id_i64)
    .bind(user_id_i64)
    .bind(since)
    .fetch_one(db.pool())
    .await?;

    Ok(count)
}

/// Count timeout cases for a user within a guild in the given time window.
/// Includes both manual timeouts, auto-timeouts, and word-filter timeouts.
pub async fn count_timeouts_in_window(
    db: &Database,
    guild_id: u64,
    user_id: u64,
    window_seconds: i64,
) -> anyhow::Result<i64> {
    let guild_id_i64 = i64::try_from(guild_id).context("guild_id out of i64 range")?;
    let user_id_i64 = i64::try_from(user_id).context("user_id out of i64 range")?;
    let since = now_unix_secs() - window_seconds;

    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM moderation_cases \
         WHERE guild_id = $1 AND target_user_id = $2 AND created_at >= $3 \
         AND action IN ('timeout', 'auto_timeout', 'word_filter_timeout')",
    )
    .bind(guild_id_i64)
    .bind(user_id_i64)
    .bind(since)
    .fetch_one(db.pool())
    .await?;

    Ok(count)
}
