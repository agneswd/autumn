use anyhow::Context as _;

use crate::{database::Database, impls::warnings::now_unix_secs, model::notes::UserNote};

#[derive(sqlx::FromRow)]
struct UserNoteRow {
    id: i64,
    guild_id: i64,
    target_user_id: i64,
    author_user_id: i64,
    content: String,
    created_at: i64,
    updated_at: i64,
    deleted_at: Option<i64>,
}

pub async fn add_user_note(
    db: &Database,
    guild_id: u64,
    target_user_id: u64,
    author_user_id: u64,
    content: &str,
) -> anyhow::Result<UserNote> {
    let guild_id_i64 = i64::try_from(guild_id).context("guild_id out of i64 range")?;
    let target_user_id_i64 =
        i64::try_from(target_user_id).context("target_user_id out of i64 range")?;
    let author_user_id_i64 =
        i64::try_from(author_user_id).context("author_user_id out of i64 range")?;
    let now = i64::try_from(now_unix_secs()).context("now out of i64 range")?;

    let row: UserNoteRow = sqlx::query_as(
        "INSERT INTO user_notes (
            guild_id,
            target_user_id,
            author_user_id,
            content,
            created_at,
            updated_at
         ) VALUES ($1, $2, $3, $4, $5, $5)
         RETURNING id, guild_id, target_user_id, author_user_id, content, created_at, updated_at, deleted_at",
    )
    .bind(guild_id_i64)
    .bind(target_user_id_i64)
    .bind(author_user_id_i64)
    .bind(content)
    .bind(now)
    .fetch_one(db.pool())
    .await?;

    to_user_note(row)
}

pub async fn list_user_notes(
    db: &Database,
    guild_id: u64,
    target_user_id: u64,
) -> anyhow::Result<Vec<UserNote>> {
    let guild_id_i64 = i64::try_from(guild_id).context("guild_id out of i64 range")?;
    let target_user_id_i64 =
        i64::try_from(target_user_id).context("target_user_id out of i64 range")?;

    let rows: Vec<UserNoteRow> = sqlx::query_as(
        "SELECT id, guild_id, target_user_id, author_user_id, content, created_at, updated_at, deleted_at
         FROM user_notes
         WHERE guild_id = $1 AND target_user_id = $2 AND deleted_at IS NULL
         ORDER BY created_at DESC, id DESC",
    )
    .bind(guild_id_i64)
    .bind(target_user_id_i64)
    .fetch_all(db.pool())
    .await?;

    rows.into_iter().map(to_user_note).collect()
}

pub async fn get_user_note(
    db: &Database,
    guild_id: u64,
    note_id: u64,
) -> anyhow::Result<Option<UserNote>> {
    let guild_id_i64 = i64::try_from(guild_id).context("guild_id out of i64 range")?;
    let note_id_i64 = i64::try_from(note_id).context("note_id out of i64 range")?;

    let row: Option<UserNoteRow> = sqlx::query_as(
        "SELECT id, guild_id, target_user_id, author_user_id, content, created_at, updated_at, deleted_at
         FROM user_notes
         WHERE guild_id = $1 AND id = $2 AND deleted_at IS NULL",
    )
    .bind(guild_id_i64)
    .bind(note_id_i64)
    .fetch_optional(db.pool())
    .await?;

    row.map(to_user_note).transpose()
}

pub async fn edit_user_note(
    db: &Database,
    guild_id: u64,
    note_id: u64,
    content: &str,
) -> anyhow::Result<bool> {
    let guild_id_i64 = i64::try_from(guild_id).context("guild_id out of i64 range")?;
    let note_id_i64 = i64::try_from(note_id).context("note_id out of i64 range")?;
    let now = i64::try_from(now_unix_secs()).context("now out of i64 range")?;

    let updated = sqlx::query(
        "UPDATE user_notes
         SET content = $1, updated_at = $2
         WHERE guild_id = $3 AND id = $4 AND deleted_at IS NULL",
    )
    .bind(content)
    .bind(now)
    .bind(guild_id_i64)
    .bind(note_id_i64)
    .execute(db.pool())
    .await?
    .rows_affected();

    Ok(updated > 0)
}

pub async fn delete_user_note(db: &Database, guild_id: u64, note_id: u64) -> anyhow::Result<bool> {
    let guild_id_i64 = i64::try_from(guild_id).context("guild_id out of i64 range")?;
    let note_id_i64 = i64::try_from(note_id).context("note_id out of i64 range")?;
    let now = i64::try_from(now_unix_secs()).context("now out of i64 range")?;

    let updated = sqlx::query(
        "UPDATE user_notes
         SET deleted_at = $1, updated_at = $1
         WHERE guild_id = $2 AND id = $3 AND deleted_at IS NULL",
    )
    .bind(now)
    .bind(guild_id_i64)
    .bind(note_id_i64)
    .execute(db.pool())
    .await?
    .rows_affected();

    Ok(updated > 0)
}

pub async fn clear_user_notes(
    db: &Database,
    guild_id: u64,
    target_user_id: u64,
) -> anyhow::Result<u64> {
    let guild_id_i64 = i64::try_from(guild_id).context("guild_id out of i64 range")?;
    let target_user_id_i64 =
        i64::try_from(target_user_id).context("target_user_id out of i64 range")?;
    let now = i64::try_from(now_unix_secs()).context("now out of i64 range")?;

    let updated = sqlx::query(
        "UPDATE user_notes
         SET deleted_at = $1, updated_at = $1
         WHERE guild_id = $2 AND target_user_id = $3 AND deleted_at IS NULL",
    )
    .bind(now)
    .bind(guild_id_i64)
    .bind(target_user_id_i64)
    .execute(db.pool())
    .await?
    .rows_affected();

    Ok(updated)
}

fn to_user_note(row: UserNoteRow) -> anyhow::Result<UserNote> {
    Ok(UserNote {
        id: u64::try_from(row.id).context("id row out of u64 range")?,
        guild_id: u64::try_from(row.guild_id).context("guild_id row out of u64 range")?,
        target_user_id: u64::try_from(row.target_user_id)
            .context("target_user_id row out of u64 range")?,
        author_user_id: u64::try_from(row.author_user_id)
            .context("author_user_id row out of u64 range")?,
        content: row.content,
        created_at: u64::try_from(row.created_at).context("created_at row out of u64 range")?,
        updated_at: u64::try_from(row.updated_at).context("updated_at row out of u64 range")?,
        deleted_at: row
            .deleted_at
            .map(u64::try_from)
            .transpose()
            .context("deleted_at row out of u64 range")?,
    })
}
