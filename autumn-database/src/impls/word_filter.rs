use anyhow::Context as _;

use crate::cache::{
    CONFIG_CACHE_TTL, WORD_LIST_CACHE_TTL, invalidate_word_filter, word_filter_config_key,
    word_filter_words_key,
};
use crate::database::Database;
use crate::model::word_filter::{WordFilterConfig, WordFilterWord};

/// Curated preset list of commonly offensive words that would not be allowed
/// in most communities. These are loaded on demand via `load_preset_words`.
pub const PRESET_WORDS: &[&str] = &[
    "nigger",
    "nigga",
    "faggot",
    "fag",
    "retard",
    "retarded",
    "tranny",
    "kike",
    "spic",
    "wetback",
    "chink",
    "gook",
    "coon",
    "darkie",
    "paki",
    "beaner",
    "cracker",
    "dyke",
    "homo",
    "shemale",
    "twink",
    "negro",
    "raghead",
    "towelhead",
    "sandnigger",
    "zipperhead",
    "slant",
    "jap",
    "redskin",
    "squaw",
    "chinaman",
    "gringo",
    "wop",
    "dago",
    "kraut",
    "honky",
    "halfbreed",
    "mongoloid",
    "tard",
    "sperg",
    "autist",
];

// ---------------------------------------------------------------------------
// Config CRUD
// ---------------------------------------------------------------------------

pub async fn get_word_filter_config(
    db: &Database,
    guild_id: u64,
) -> anyhow::Result<Option<WordFilterConfig>> {
    let cache_key = word_filter_config_key(db.cache(), guild_id);
    db.cache()
        .get_or_load_json(&cache_key, CONFIG_CACHE_TTL, || async {
            let guild_id_i64 = i64::try_from(guild_id).context("guild_id out of i64 range")?;

            let row = sqlx::query_as::<_, (bool, String)>(
                "SELECT enabled, action FROM word_filter_config WHERE guild_id = $1",
            )
            .bind(guild_id_i64)
            .fetch_optional(db.pool())
            .await?;

            Ok(row.map(|(enabled, action)| WordFilterConfig {
                guild_id,
                enabled,
                action,
            }))
        })
        .await
}

pub async fn set_word_filter_enabled(
    db: &Database,
    guild_id: u64,
    enabled: bool,
) -> anyhow::Result<()> {
    let guild_id_i64 = i64::try_from(guild_id).context("guild_id out of i64 range")?;

    sqlx::query(
        "INSERT INTO word_filter_config (guild_id, enabled)
         VALUES ($1, $2)
         ON CONFLICT (guild_id) DO UPDATE SET enabled = EXCLUDED.enabled",
    )
    .bind(guild_id_i64)
    .bind(enabled)
    .execute(db.pool())
    .await?;

    invalidate_word_filter(db.cache(), guild_id).await?;

    Ok(())
}

pub async fn set_word_filter_action(
    db: &Database,
    guild_id: u64,
    action: &str,
) -> anyhow::Result<()> {
    let guild_id_i64 = i64::try_from(guild_id).context("guild_id out of i64 range")?;

    sqlx::query(
        "INSERT INTO word_filter_config (guild_id, action)
         VALUES ($1, $2)
         ON CONFLICT (guild_id) DO UPDATE SET action = EXCLUDED.action",
    )
    .bind(guild_id_i64)
    .bind(action)
    .execute(db.pool())
    .await?;

    invalidate_word_filter(db.cache(), guild_id).await?;

    Ok(())
}

// ---------------------------------------------------------------------------
// Word CRUD
// ---------------------------------------------------------------------------

/// Add a word to the guild's filter list. Returns `true` if inserted, `false`
/// if the word already existed (duplicate).
pub async fn add_filter_word(
    db: &Database,
    guild_id: u64,
    word: &str,
    is_preset: bool,
) -> anyhow::Result<bool> {
    add_filter_word_internal(db, guild_id, word, is_preset, true).await
}

async fn add_filter_word_internal(
    db: &Database,
    guild_id: u64,
    word: &str,
    is_preset: bool,
    invalidate_cache: bool,
) -> anyhow::Result<bool> {
    let guild_id_i64 = i64::try_from(guild_id).context("guild_id out of i64 range")?;
    let lower = word.to_lowercase();

    let result = sqlx::query(
        "INSERT INTO word_filter_words (guild_id, word, is_preset)
         VALUES ($1, $2, $3)
         ON CONFLICT (guild_id, word) DO NOTHING",
    )
    .bind(guild_id_i64)
    .bind(&lower)
    .bind(is_preset)
    .execute(db.pool())
    .await?;

    if invalidate_cache {
        invalidate_word_filter(db.cache(), guild_id).await?;
    }

    Ok(result.rows_affected() > 0)
}

/// Remove a word from the guild's filter list. Returns `true` if removed.
pub async fn remove_filter_word(db: &Database, guild_id: u64, word: &str) -> anyhow::Result<bool> {
    let guild_id_i64 = i64::try_from(guild_id).context("guild_id out of i64 range")?;
    let lower = word.to_lowercase();

    let result = sqlx::query("DELETE FROM word_filter_words WHERE guild_id = $1 AND word = $2")
        .bind(guild_id_i64)
        .bind(&lower)
        .execute(db.pool())
        .await?;

    invalidate_word_filter(db.cache(), guild_id).await?;

    Ok(result.rows_affected() > 0)
}

/// List all filtered words for a guild (both preset and custom).
pub async fn list_filter_words(
    db: &Database,
    guild_id: u64,
) -> anyhow::Result<Vec<WordFilterWord>> {
    let guild_id_i64 = i64::try_from(guild_id).context("guild_id out of i64 range")?;

    let rows = sqlx::query_as::<_, (i64, i64, String, bool, i64)>(
        "SELECT id, guild_id, word, is_preset, created_at
         FROM word_filter_words
         WHERE guild_id = $1
         ORDER BY word ASC",
    )
    .bind(guild_id_i64)
    .fetch_all(db.pool())
    .await?;

    rows.into_iter()
        .map(|(id, gid, word, is_preset, created_at)| {
            Ok(WordFilterWord {
                id: u64::try_from(id).context("id out of u64 range")?,
                guild_id: u64::try_from(gid).context("guild_id out of u64 range")?,
                word,
                is_preset,
                created_at: u64::try_from(created_at).context("created_at out of u64 range")?,
            })
        })
        .collect()
}

/// Get just the word strings for a guild (used by the event handler for fast matching).
pub async fn get_all_filter_words_for_guild(
    db: &Database,
    guild_id: u64,
) -> anyhow::Result<Vec<String>> {
    let cache_key = word_filter_words_key(db.cache(), guild_id);
    db.cache()
        .get_or_load_json(&cache_key, WORD_LIST_CACHE_TTL, || async {
            let guild_id_i64 = i64::try_from(guild_id).context("guild_id out of i64 range")?;

            let words: Vec<String> =
                sqlx::query_scalar("SELECT word FROM word_filter_words WHERE guild_id = $1")
                    .bind(guild_id_i64)
                    .fetch_all(db.pool())
                    .await?;

            Ok(words)
        })
        .await
}

/// Load all preset words into the guild's filter list. Duplicates are skipped.
/// Returns the number of newly inserted words.
pub async fn load_preset_words(db: &Database, guild_id: u64) -> anyhow::Result<u64> {
    let mut inserted: u64 = 0;
    for word in PRESET_WORDS {
        if add_filter_word_internal(db, guild_id, word, true, false).await? {
            inserted += 1;
        }
    }

    if inserted > 0 {
        invalidate_word_filter(db.cache(), guild_id).await?;
    }

    Ok(inserted)
}

/// Remove all preset words from the guild's filter list.
pub async fn clear_preset_words(db: &Database, guild_id: u64) -> anyhow::Result<u64> {
    let guild_id_i64 = i64::try_from(guild_id).context("guild_id out of i64 range")?;

    let result =
        sqlx::query("DELETE FROM word_filter_words WHERE guild_id = $1 AND is_preset = TRUE")
            .bind(guild_id_i64)
            .execute(db.pool())
            .await?;

    invalidate_word_filter(db.cache(), guild_id).await?;

    Ok(result.rows_affected())
}

/// Check whether the word filter is enabled for a guild and return the config.
pub async fn get_word_filter_if_enabled(
    db: &Database,
    guild_id: u64,
) -> anyhow::Result<Option<WordFilterConfig>> {
    let config = get_word_filter_config(db, guild_id).await?;
    match config {
        Some(cfg) if cfg.enabled => Ok(Some(cfg)),
        _ => Ok(None),
    }
}
