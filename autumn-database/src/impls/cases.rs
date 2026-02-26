use anyhow::Context as _;

use crate::{
    database::Database,
    impls::warnings::now_unix_secs,
    model::cases::{CaseEvent, CaseSummary, ModerationCase},
};

pub struct NewCase<'a> {
    pub guild_id: u64,
    pub target_user_id: Option<u64>,
    pub moderator_user_id: u64,
    pub action: &'a str,
    pub reason: &'a str,
    pub status: &'a str,
    pub duration_seconds: Option<u64>,
}

pub async fn ensure_case_schema_compat(db: &Database) -> anyhow::Result<()> {
    sqlx::query(
        "ALTER TABLE mod_cases ADD COLUMN IF NOT EXISTS case_code TEXT NOT NULL DEFAULT 'M'",
    )
    .execute(db.pool())
    .await?;

    sqlx::query(
        "ALTER TABLE mod_cases ADD COLUMN IF NOT EXISTS action_case_number BIGINT NOT NULL DEFAULT 0",
    )
    .execute(db.pool())
    .await?;

    sqlx::query(
        "CREATE UNIQUE INDEX IF NOT EXISTS mod_cases_guild_case_code_number_idx
         ON mod_cases (guild_id, case_code, action_case_number)",
    )
    .execute(db.pool())
    .await?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS guild_mod_config (
            guild_id BIGINT PRIMARY KEY,
            modlog_channel_id BIGINT
        )",
    )
    .execute(db.pool())
    .await?;

    sqlx::query(
        "WITH ranked AS (
            SELECT
                id,
                CASE
                    WHEN action = 'warn' THEN 'W'
                    WHEN action = 'ban' THEN 'B'
                    WHEN action = 'kick' THEN 'K'
                    WHEN action = 'timeout' THEN 'T'
                    WHEN action = 'unban' THEN 'UB'
                    WHEN action = 'untimeout' THEN 'UT'
                    WHEN action = 'unwarn' THEN 'UW'
                    WHEN action = 'unwarn_all' THEN 'UWA'
                    WHEN action = 'purge' THEN 'P'
                    WHEN action = 'terminate' THEN 'TR'
                    ELSE 'M'
                END AS resolved_code,
                ROW_NUMBER() OVER (
                    PARTITION BY guild_id,
                    CASE
                        WHEN action = 'warn' THEN 'W'
                        WHEN action = 'ban' THEN 'B'
                        WHEN action = 'kick' THEN 'K'
                        WHEN action = 'timeout' THEN 'T'
                        WHEN action = 'unban' THEN 'UB'
                        WHEN action = 'untimeout' THEN 'UT'
                        WHEN action = 'unwarn' THEN 'UW'
                        WHEN action = 'unwarn_all' THEN 'UWA'
                        WHEN action = 'purge' THEN 'P'
                        WHEN action = 'terminate' THEN 'TR'
                        ELSE 'M'
                    END
                    ORDER BY created_at ASC, id ASC
                ) AS resolved_number
            FROM mod_cases
        )
        UPDATE mod_cases AS mc
        SET
            case_code = ranked.resolved_code,
            action_case_number = ranked.resolved_number
        FROM ranked
        WHERE mc.id = ranked.id AND (mc.action_case_number = 0 OR mc.case_code = 'M')",
    )
    .execute(db.pool())
    .await?;

    Ok(())
}

pub struct CaseFilters<'a> {
    pub target_user_id: Option<u64>,
    pub moderator_user_id: Option<u64>,
    pub action: Option<&'a str>,
    pub limit: u32,
}

#[derive(sqlx::FromRow)]
struct CaseSummaryRow {
    case_number: i64,
    case_code: String,
    action_case_number: i64,
    target_user_id: Option<i64>,
    moderator_user_id: i64,
    action: String,
    reason: String,
    duration_seconds: Option<i64>,
    created_at: i64,
}

#[derive(sqlx::FromRow)]
struct ModerationCaseRow {
    id: i64,
    case_number: i64,
    case_code: String,
    action_case_number: i64,
    guild_id: i64,
    target_user_id: Option<i64>,
    moderator_user_id: i64,
    action: String,
    reason: String,
    status: String,
    duration_seconds: Option<i64>,
    created_at: i64,
    updated_at: i64,
}

#[derive(sqlx::FromRow)]
struct CaseEventRow {
    event_type: String,
    actor_user_id: i64,
    old_reason: Option<String>,
    new_reason: Option<String>,
    note: Option<String>,
    created_at: i64,
}

pub async fn create_case(db: &Database, new_case: NewCase<'_>) -> anyhow::Result<CaseSummary> {
    let guild_id_i64 = i64::try_from(new_case.guild_id).context("guild_id out of i64 range")?;
    let target_user_id_i64 = new_case
        .target_user_id
        .map(i64::try_from)
        .transpose()
        .context("target_user_id out of i64 range")?;
    let moderator_user_id_i64 =
        i64::try_from(new_case.moderator_user_id).context("moderator_user_id out of i64 range")?;
    let duration_seconds_i64 = new_case
        .duration_seconds
        .map(i64::try_from)
        .transpose()
        .context("duration_seconds out of i64 range")?;
    let now = i64::try_from(now_unix_secs()).context("now out of i64 range")?;
    let case_code = action_code(new_case.action);

    let mut tx = db.pool().begin().await?;

    sqlx::query("SELECT pg_advisory_xact_lock($1)")
        .bind(guild_id_i64)
        .execute(&mut *tx)
        .await?;

    let next_case_number: i64 = sqlx::query_scalar(
        "SELECT COALESCE(MAX(case_number), 0) + 1 FROM mod_cases WHERE guild_id = $1",
    )
    .bind(guild_id_i64)
    .fetch_one(&mut *tx)
    .await?;

    let next_action_case_number: i64 = sqlx::query_scalar(
        "SELECT COALESCE(MAX(action_case_number), 0) + 1
         FROM mod_cases
         WHERE guild_id = $1 AND case_code = $2",
    )
    .bind(guild_id_i64)
    .bind(case_code)
    .fetch_one(&mut *tx)
    .await?;

    let case_row: ModerationCaseRow = sqlx::query_as(
        "INSERT INTO mod_cases (
            guild_id,
            case_number,
            case_code,
            action_case_number,
            target_user_id,
            moderator_user_id,
            action,
            reason,
            status,
            duration_seconds,
            created_at,
            updated_at
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $11)
            RETURNING id, case_number, case_code, action_case_number, guild_id, target_user_id, moderator_user_id, action, reason, status, duration_seconds, created_at, updated_at",
    )
    .bind(guild_id_i64)
    .bind(next_case_number)
        .bind(case_code)
        .bind(next_action_case_number)
    .bind(target_user_id_i64)
    .bind(moderator_user_id_i64)
    .bind(new_case.action)
    .bind(new_case.reason)
    .bind(new_case.status)
    .bind(duration_seconds_i64)
    .bind(now)
    .fetch_one(&mut *tx)
    .await?;

    sqlx::query(
        "INSERT INTO mod_case_events (
            case_id,
            guild_id,
            event_type,
            actor_user_id,
            note,
            created_at
         ) VALUES ($1, $2, 'created', $3, $4, $5)",
    )
    .bind(case_row.id)
    .bind(guild_id_i64)
    .bind(moderator_user_id_i64)
    .bind(Some("Case created"))
    .bind(now)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    to_case_summary(case_row)
}

pub async fn list_recent_cases(
    db: &Database,
    guild_id: u64,
    filters: CaseFilters<'_>,
) -> anyhow::Result<Vec<CaseSummary>> {
    let guild_id_i64 = i64::try_from(guild_id).context("guild_id out of i64 range")?;
    let target_user_id_i64 = filters
        .target_user_id
        .map(i64::try_from)
        .transpose()
        .context("target_user_id out of i64 range")?;
    let moderator_user_id_i64 = filters
        .moderator_user_id
        .map(i64::try_from)
        .transpose()
        .context("moderator_user_id out of i64 range")?;
    let limit_i64 = i64::from(filters.limit.clamp(1, 200));

    let rows: Vec<CaseSummaryRow> = sqlx::query_as(
        "SELECT case_number, case_code, action_case_number, target_user_id, moderator_user_id, action, reason, duration_seconds, created_at
         FROM mod_cases
         WHERE guild_id = $1
           AND ($2::BIGINT IS NULL OR target_user_id = $2)
           AND ($3::BIGINT IS NULL OR moderator_user_id = $3)
           AND ($4::TEXT IS NULL OR LOWER(action) = LOWER($4))
         ORDER BY case_number DESC
         LIMIT $5",
    )
    .bind(guild_id_i64)
    .bind(target_user_id_i64)
    .bind(moderator_user_id_i64)
    .bind(filters.action)
    .bind(limit_i64)
    .fetch_all(db.pool())
    .await?;

    let mut out = Vec::with_capacity(rows.len());
    for row in rows {
        out.push(CaseSummary {
            case_number: u64::try_from(row.case_number).context("case_number out of u64 range")?,
            case_code: row.case_code,
            action_case_number: u64::try_from(row.action_case_number)
                .context("action_case_number out of u64 range")?,
            target_user_id: row
                .target_user_id
                .map(u64::try_from)
                .transpose()
                .context("target_user_id row out of u64 range")?,
            moderator_user_id: u64::try_from(row.moderator_user_id)
                .context("moderator_user_id row out of u64 range")?,
            action: row.action,
            reason: row.reason,
            duration_seconds: row
                .duration_seconds
                .map(u64::try_from)
                .transpose()
                .context("duration_seconds row out of u64 range")?,
            created_at: u64::try_from(row.created_at).context("created_at row out of u64 range")?,
        });
    }

    Ok(out)
}

pub async fn get_case_by_label(
    db: &Database,
    guild_id: u64,
    case_code: &str,
    action_case_number: u64,
) -> anyhow::Result<Option<ModerationCase>> {
    let guild_id_i64 = i64::try_from(guild_id).context("guild_id out of i64 range")?;
    let action_case_number_i64 =
        i64::try_from(action_case_number).context("action_case_number out of i64 range")?;

    let row: Option<ModerationCaseRow> = sqlx::query_as(
        "SELECT id, case_number, case_code, action_case_number, guild_id, target_user_id, moderator_user_id, action, reason, status, duration_seconds, created_at, updated_at
         FROM mod_cases
         WHERE guild_id = $1 AND case_code = $2 AND action_case_number = $3",
    )
    .bind(guild_id_i64)
    .bind(case_code)
    .bind(action_case_number_i64)
    .fetch_optional(db.pool())
    .await?;

    row.map(to_moderation_case).transpose()
}

pub async fn get_case_events(
    db: &Database,
    guild_id: u64,
    case_code: &str,
    action_case_number: u64,
) -> anyhow::Result<Vec<CaseEvent>> {
    let guild_id_i64 = i64::try_from(guild_id).context("guild_id out of i64 range")?;
    let action_case_number_i64 =
        i64::try_from(action_case_number).context("action_case_number out of i64 range")?;

    let case_id: Option<i64> = sqlx::query_scalar(
        "SELECT id
         FROM mod_cases
         WHERE guild_id = $1 AND case_code = $2 AND action_case_number = $3",
    )
    .bind(guild_id_i64)
    .bind(case_code)
    .bind(action_case_number_i64)
    .fetch_optional(db.pool())
    .await?;

    let Some(case_id) = case_id else {
        return Ok(Vec::new());
    };

    let rows: Vec<CaseEventRow> = sqlx::query_as(
        "SELECT event_type, actor_user_id, old_reason, new_reason, note, created_at
         FROM mod_case_events
         WHERE guild_id = $1 AND case_id = $2
         ORDER BY created_at ASC, id ASC",
    )
    .bind(guild_id_i64)
    .bind(case_id)
    .fetch_all(db.pool())
    .await?;

    let mut out = Vec::with_capacity(rows.len());
    for row in rows {
        out.push(CaseEvent {
            event_type: row.event_type,
            actor_user_id: u64::try_from(row.actor_user_id)
                .context("actor_user_id row out of u64 range")?,
            old_reason: row.old_reason,
            new_reason: row.new_reason,
            note: row.note,
            created_at: u64::try_from(row.created_at).context("created_at row out of u64 range")?,
        });
    }

    Ok(out)
}

pub async fn update_case_reason(
    db: &Database,
    guild_id: u64,
    case_code: &str,
    action_case_number: u64,
    actor_user_id: u64,
    new_reason: &str,
) -> anyhow::Result<Option<ModerationCase>> {
    let guild_id_i64 = i64::try_from(guild_id).context("guild_id out of i64 range")?;
    let action_case_number_i64 =
        i64::try_from(action_case_number).context("action_case_number out of i64 range")?;
    let actor_user_id_i64 =
        i64::try_from(actor_user_id).context("actor_user_id out of i64 range")?;
    let now = i64::try_from(now_unix_secs()).context("now out of i64 range")?;

    let mut tx = db.pool().begin().await?;

    let existing: Option<(i64, String)> = sqlx::query_as(
        "SELECT id, reason
         FROM mod_cases
            WHERE guild_id = $1 AND case_code = $2 AND action_case_number = $3
         FOR UPDATE",
    )
    .bind(guild_id_i64)
    .bind(case_code)
    .bind(action_case_number_i64)
    .fetch_optional(&mut *tx)
    .await?;

    let Some((case_id, old_reason)) = existing else {
        tx.rollback().await?;
        return Ok(None);
    };

    let updated: ModerationCaseRow = sqlx::query_as(
        "UPDATE mod_cases
         SET reason = $1, updated_at = $2
         WHERE id = $3
            RETURNING id, case_number, case_code, action_case_number, guild_id, target_user_id, moderator_user_id, action, reason, status, duration_seconds, created_at, updated_at",
    )
    .bind(new_reason)
    .bind(now)
    .bind(case_id)
    .fetch_one(&mut *tx)
    .await?;

    sqlx::query(
        "INSERT INTO mod_case_events (
            case_id,
            guild_id,
            event_type,
            actor_user_id,
            old_reason,
            new_reason,
            note,
            created_at
         ) VALUES ($1, $2, 'reason_updated', $3, $4, $5, $6, $7)",
    )
    .bind(case_id)
    .bind(guild_id_i64)
    .bind(actor_user_id_i64)
    .bind(Some(old_reason))
    .bind(Some(new_reason.to_owned()))
    .bind(Some("Reason edited"))
    .bind(now)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok(Some(to_moderation_case(updated)?))
}

pub async fn add_case_note(
    db: &Database,
    guild_id: u64,
    case_code: &str,
    action_case_number: u64,
    actor_user_id: u64,
    note: &str,
) -> anyhow::Result<bool> {
    let guild_id_i64 = i64::try_from(guild_id).context("guild_id out of i64 range")?;
    let action_case_number_i64 =
        i64::try_from(action_case_number).context("action_case_number out of i64 range")?;
    let actor_user_id_i64 =
        i64::try_from(actor_user_id).context("actor_user_id out of i64 range")?;
    let now = i64::try_from(now_unix_secs()).context("now out of i64 range")?;

    let mut tx = db.pool().begin().await?;

    let case_id: Option<i64> = sqlx::query_scalar(
        "SELECT id
         FROM mod_cases
         WHERE guild_id = $1 AND case_code = $2 AND action_case_number = $3
         FOR UPDATE",
    )
    .bind(guild_id_i64)
    .bind(case_code)
    .bind(action_case_number_i64)
    .fetch_optional(&mut *tx)
    .await?;

    let Some(case_id) = case_id else {
        tx.rollback().await?;
        return Ok(false);
    };

    sqlx::query(
        "INSERT INTO mod_case_events (
            case_id,
            guild_id,
            event_type,
            actor_user_id,
            note,
            created_at
         ) VALUES ($1, $2, 'note_added', $3, $4, $5)",
    )
    .bind(case_id)
    .bind(guild_id_i64)
    .bind(actor_user_id_i64)
    .bind(Some(note.to_owned()))
    .bind(now)
    .execute(&mut *tx)
    .await?;

    sqlx::query("UPDATE mod_cases SET updated_at = $1 WHERE id = $2")
        .bind(now)
        .bind(case_id)
        .execute(&mut *tx)
        .await?;

    tx.commit().await?;
    Ok(true)
}

fn to_case_summary(row: ModerationCaseRow) -> anyhow::Result<CaseSummary> {
    Ok(CaseSummary {
        case_number: u64::try_from(row.case_number).context("case_number out of u64 range")?,
        case_code: row.case_code,
        action_case_number: u64::try_from(row.action_case_number)
            .context("action_case_number row out of u64 range")?,
        target_user_id: row
            .target_user_id
            .map(u64::try_from)
            .transpose()
            .context("target_user_id row out of u64 range")?,
        moderator_user_id: u64::try_from(row.moderator_user_id)
            .context("moderator_user_id row out of u64 range")?,
        action: row.action,
        reason: row.reason,
        duration_seconds: row
            .duration_seconds
            .map(u64::try_from)
            .transpose()
            .context("duration_seconds row out of u64 range")?,
        created_at: u64::try_from(row.created_at).context("created_at row out of u64 range")?,
    })
}

fn to_moderation_case(row: ModerationCaseRow) -> anyhow::Result<ModerationCase> {
    Ok(ModerationCase {
        id: u64::try_from(row.id).context("id row out of u64 range")?,
        case_number: u64::try_from(row.case_number).context("case_number row out of u64 range")?,
        case_code: row.case_code,
        action_case_number: u64::try_from(row.action_case_number)
            .context("action_case_number row out of u64 range")?,
        guild_id: u64::try_from(row.guild_id).context("guild_id row out of u64 range")?,
        target_user_id: row
            .target_user_id
            .map(u64::try_from)
            .transpose()
            .context("target_user_id row out of u64 range")?,
        moderator_user_id: u64::try_from(row.moderator_user_id)
            .context("moderator_user_id row out of u64 range")?,
        action: row.action,
        reason: row.reason,
        status: row.status,
        duration_seconds: row
            .duration_seconds
            .map(u64::try_from)
            .transpose()
            .context("duration_seconds row out of u64 range")?,
        created_at: u64::try_from(row.created_at).context("created_at row out of u64 range")?,
        updated_at: u64::try_from(row.updated_at).context("updated_at row out of u64 range")?,
    })
}

fn action_code(action: &str) -> &'static str {
    match action {
        "warn" => "W",
        "ban" => "B",
        "kick" => "K",
        "timeout" => "T",
        "unban" => "UB",
        "untimeout" => "UT",
        "unwarn" => "UW",
        "unwarn_all" => "UWA",
        "purge" => "P",
        "terminate" => "TR",
        "word_filter_timeout" | "word_filter_delete" | "word_filter_log" | "word_filter_warn" => {
            "WF"
        }
        "auto_timeout" => "AT",
        _ => "M",
    }
}
