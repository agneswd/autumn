//! Shared escalation check logic.
//!
//! Called after a warning is issued (from `!warn`, word filter, etc.)
//! to automatically time out a user if they have reached the configured
//! warning threshold within the configured window.

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use poise::serenity_prelude as serenity;
use tracing::{error, info, warn};

use autumn_database::Database;
use autumn_database::impls::cases::{NewCase, create_case};
use autumn_database::impls::escalation::{
    count_timeouts_in_window, count_warnings_in_window, escalation_timeout_seconds,
    get_escalation_if_enabled,
};
use autumn_database::impls::modlog_config::get_modlog_channel_id;
use autumn_utils::embed::DEFAULT_EMBED_COLOR;
use autumn_utils::formatting::{format_case_label, format_compact_duration};

use crate::moderation::embeds::send_moderation_target_dm;

/// Result of an escalation check.
pub struct EscalationResult {
    /// Whether an auto-timeout was applied.
    pub timed_out: bool,
    /// Duration of the timeout in seconds (if applied).
    pub timeout_seconds: Option<i64>,
    /// Escalation tier (0-indexed, if applied).
    pub tier: Option<i64>,
}

/// Check whether a user should be auto-timed-out after receiving a warning.
///
/// This function:
/// 1. Checks if escalation is enabled for the guild.
/// 2. Counts the user's warnings within the configured window.
/// 3. If threshold is met, counts past timeouts to determine the escalation tier.
/// 4. Applies the timeout, creates a moderation case, publishes to modlog, and DMs the user.
///
/// Returns `None` if escalation is disabled or the threshold was not met.
pub async fn check_and_escalate(
    http: &serenity::Http,
    db: &Database,
    guild_id: serenity::GuildId,
    target_user: &serenity::User,
    bot_user_id: u64,
) -> Option<EscalationResult> {
    // 1. Check if escalation is enabled.
    let config = match get_escalation_if_enabled(db, guild_id.get()).await {
        Ok(Some(cfg)) => cfg,
        Ok(None) => return None,
        Err(source) => {
            error!(?source, "failed to read escalation config");
            return None;
        }
    };

    // 2. Count warnings in window.
    let warn_count = match count_warnings_in_window(
        db,
        guild_id.get(),
        target_user.id.get(),
        config.warn_window_seconds,
    )
    .await
    {
        Ok(count) => count,
        Err(source) => {
            error!(?source, "failed to count warnings for escalation");
            return None;
        }
    };

    if warn_count < config.warn_threshold as i64 {
        return None;
    }

    // 3. Count past timeouts to determine escalation tier.
    let timeout_count = match count_timeouts_in_window(
        db,
        guild_id.get(),
        target_user.id.get(),
        config.timeout_window_seconds,
    )
    .await
    {
        Ok(count) => count,
        Err(source) => {
            error!(?source, "failed to count timeouts for escalation");
            return None;
        }
    };

    let timeout_secs = escalation_timeout_seconds(timeout_count);

    info!(
        user_id = %target_user.id,
        guild_id = %guild_id,
        warn_count,
        timeout_count,
        timeout_secs,
        "escalation triggered: auto-timeout"
    );

    // 4. Apply the timeout.
    let timeout_duration = Duration::from_secs(timeout_secs as u64);
    let until_system_time = SystemTime::now()
        .checked_add(timeout_duration)
        .unwrap_or(SystemTime::now());
    let until_unix = until_system_time
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs()) as i64;

    if let Ok(until) = serenity::Timestamp::from_unix_timestamp(until_unix) {
        let edit = serenity::EditMember::new().disable_communication_until_datetime(until);
        if let Err(source) = guild_id.edit_member(http, target_user.id, edit).await {
            if is_missing_permissions(&source) {
                warn!(
                    user_id = %target_user.id,
                    "missing permissions to auto-timeout user (check role hierarchy)"
                );
            } else {
                error!(?source, "failed to auto-timeout user");
            }
            // Still create a case to record the intent even if we couldn't apply it.
        }
    }

    // 5. Create a moderation case.
    let reason = format!(
        "Auto-escalation: {} warning(s) in {}",
        warn_count,
        format_compact_duration(config.warn_window_seconds as u64)
    );

    let new_case = NewCase {
        guild_id: guild_id.get(),
        target_user_id: Some(target_user.id.get()),
        moderator_user_id: bot_user_id,
        action: "auto_timeout",
        reason: &reason,
        status: "completed",
        duration_seconds: Some(timeout_secs as u64),
    };

    let case = match create_case(db, new_case).await {
        Ok(case) => case,
        Err(source) => {
            error!(?source, "failed to create auto-timeout case");
            return Some(EscalationResult {
                timed_out: true,
                timeout_seconds: Some(timeout_secs),
                tier: Some(timeout_count),
            });
        }
    };

    // 6. Publish to modlog channel.
    if let Err(source) =
        publish_auto_timeout_to_modlog(http, db, guild_id, &case, &reason, timeout_secs).await
    {
        error!(?source, "failed to publish auto-timeout case to modlog");
    }

    // 7. DM the user.
    let guild_name = match guild_id.to_partial_guild(http).await {
        Ok(guild) => guild.name,
        Err(_) => format!("Server {}", guild_id.get()),
    };

    let _ = send_moderation_target_dm(
        http,
        target_user,
        &guild_name,
        "automatically timed out",
        Some(&reason),
        Some(&format_compact_duration(timeout_secs as u64)),
    )
    .await;

    Some(EscalationResult {
        timed_out: true,
        timeout_seconds: Some(timeout_secs),
        tier: Some(timeout_count),
    })
}

async fn publish_auto_timeout_to_modlog(
    http: &serenity::Http,
    db: &Database,
    guild_id: serenity::GuildId,
    case: &autumn_database::model::cases::CaseSummary,
    reason: &str,
    timeout_secs: i64,
) -> Result<(), serenity::Error> {
    let channel_id = match get_modlog_channel_id(db, guild_id.get()).await {
        Ok(Some(id)) => id,
        Ok(None) => return Ok(()),
        Err(source) => {
            error!(?source, "failed to read modlog channel for auto-timeout");
            return Ok(());
        }
    };

    let case_label = format_case_label(&case.case_code, case.action_case_number);

    let mut fields = Vec::new();
    fields.push(format!(
        "**User :** <@{}>",
        case.target_user_id.unwrap_or(0)
    ));
    fields.push(format!("**Reason :** {}", reason));
    fields.push(format!(
        "**Duration :** {}",
        format_compact_duration(timeout_secs as u64)
    ));

    // Blank line separator before metadata.
    fields.push(String::new());

    fields.push(format!("**When :** <t:{}:R>", case.created_at));

    let title = format!("Auto Timeout - #{}", case_label);
    let description = fields.join("\n");

    let embed = serenity::CreateEmbed::new()
        .color(DEFAULT_EMBED_COLOR)
        .title(title)
        .description(description);

    serenity::ChannelId::new(channel_id)
        .send_message(http, serenity::CreateMessage::new().embed(embed))
        .await?;

    Ok(())
}

fn is_missing_permissions(source: &serenity::Error) -> bool {
    matches!(
        source,
        serenity::Error::Http(serenity::HttpError::UnsuccessfulRequest(response))
            if response.status_code.as_u16() == 403 || response.error.code == 50013
    )
}
