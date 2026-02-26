use std::time::{Duration, SystemTime, UNIX_EPOCH};

use poise::serenity_prelude as serenity;
use tracing::{error, warn};

use autumn_commands::moderation::escalation_check::check_and_escalate;
use autumn_commands::moderation::send_moderation_target_dm_for_guild;
use autumn_core::Data;
use autumn_database::impls::cases::{NewCase, create_case};
use autumn_database::impls::modlog_config::get_modlog_channel_id;
use autumn_database::impls::warnings::record_warning;
use autumn_database::impls::word_filter::{
    get_all_filter_words_for_guild, get_word_filter_if_enabled,
};
use autumn_utils::embed::DEFAULT_EMBED_COLOR;
use autumn_utils::formatting::{format_case_label, format_compact_duration};

/// Check an incoming message against the guild's word filter and execute the
/// configured action when a match is found.
pub async fn handle_message_word_filter(
    ctx: &serenity::Context,
    data: &Data,
    message: &serenity::Message,
) {
    // Ignore bots and webhooks.
    if message.author.bot || message.webhook_id.is_some() {
        return;
    }

    let Some(guild_id) = message.guild_id else {
        return;
    };

    // Check if the word filter is enabled for this guild.
    let config = match get_word_filter_if_enabled(&data.db, guild_id.get()).await {
        Ok(Some(cfg)) => cfg,
        Ok(None) => return,
        Err(source) => {
            error!(?source, "failed to read word filter config");
            return;
        }
    };

    // Fetch the guild's filtered words.
    let words = match get_all_filter_words_for_guild(&data.db, guild_id.get()).await {
        Ok(w) => w,
        Err(source) => {
            error!(?source, "failed to load word filter list");
            return;
        }
    };

    if words.is_empty() {
        return;
    }

    // Check if any filtered word appears as a whole word in the message content.
    let content_lower = message.content.to_lowercase();
    let matched_word = words.iter().find(|w| {
        // Match the word only at word boundaries to avoid false positives
        // (e.g. "fag" should not match "leafage").
        content_lower
            .split(|c: char| !c.is_alphanumeric())
            .any(|token| token == w.as_str())
    });

    let Some(matched_word) = matched_word else {
        return;
    };

    let matched_word = matched_word.clone();
    let action = config.action.as_str();
    let bot_user_id = ctx.cache.current_user().id.get();

    // Suppress this message from user-log recording if it will be deleted.
    if matches!(
        action,
        "delete_and_log" | "warn_and_log" | "timeout_delete_and_log"
    ) {
        let mut suppressed = data.suppressed_deletes.write().await;
        suppressed.insert(message.id.get());
    }

    // Execute the configured action.
    match action {
        "delete_and_log" => {
            if let Err(source) = message.delete(&ctx.http).await {
                if !is_missing_permissions(&source) {
                    error!(?source, "failed to delete filtered message");
                } else {
                    warn!("missing permissions to delete filtered message");
                }
            }
        }
        "warn_and_log" => {
            if let Err(source) = message.delete(&ctx.http).await {
                if !is_missing_permissions(&source) {
                    error!(?source, "failed to delete filtered message");
                } else {
                    warn!("missing permissions to delete filtered message");
                }
            }

            // Issue a warning for the user.
            let warn_reason = format!("Word filter: {}", matched_word);
            if let Err(source) = record_warning(
                &data.db,
                guild_id.get(),
                message.author.id.get(),
                bot_user_id,
                &warn_reason,
            )
            .await
            {
                error!(
                    ?source,
                    "failed to record warning for word filter violation"
                );
            }

            // DM the user about the warning.
            let _ = send_moderation_target_dm_for_guild(
                &ctx.http,
                &message.author,
                guild_id,
                "warned",
                Some(&warn_reason),
                None,
            )
            .await;

            // Check for automatic escalation (warn threshold → auto-timeout).
            check_and_escalate(&ctx.http, &data.db, guild_id, &message.author, bot_user_id).await;
        }
        "timeout_delete_and_log" => {
            if let Err(source) = message.delete(&ctx.http).await {
                if !is_missing_permissions(&source) {
                    error!(?source, "failed to delete filtered message");
                } else {
                    warn!("missing permissions to delete filtered message");
                }
            }

            // Apply a 5-minute timeout.
            let timeout_duration = Duration::from_secs(300);
            let until_system_time = SystemTime::now()
                .checked_add(timeout_duration)
                .unwrap_or(SystemTime::now());
            let until_unix = until_system_time
                .duration_since(UNIX_EPOCH)
                .map_or(0, |d| d.as_secs()) as i64;

            if let Ok(until) = serenity::Timestamp::from_unix_timestamp(until_unix) {
                let edit = serenity::EditMember::new().disable_communication_until_datetime(until);
                if let Err(source) = guild_id
                    .edit_member(&ctx.http, message.author.id, edit)
                    .await
                {
                    if !is_missing_permissions(&source) {
                        error!(?source, "failed to timeout user for word filter violation");
                    } else {
                        warn!(
                            user_id = %message.author.id,
                            "missing permissions to timeout user for word filter violation \
                             (check role hierarchy)"
                        );
                    }
                }
            }

            // DM the user about the timeout.
            let _ = send_moderation_target_dm_for_guild(
                &ctx.http,
                &message.author,
                guild_id,
                "timed out",
                Some(&format!("Word filter: {}", matched_word)),
                Some("5m"),
            )
            .await;
        }
        // "log_only" or anything else — no message action needed.
        _ => {}
    }

    // Create a moderation case for the violation.
    let reason = matched_word.clone();
    let case_action = match action {
        "timeout_delete_and_log" => "word_filter_timeout",
        "delete_and_log" => "word_filter_delete",
        "warn_and_log" => "word_filter_warn",
        _ => "word_filter_log",
    };

    let new_case = NewCase {
        guild_id: guild_id.get(),
        target_user_id: Some(message.author.id.get()),
        moderator_user_id: bot_user_id,
        action: case_action,
        reason: &reason,
        status: "completed",
        duration_seconds: if action == "timeout_delete_and_log" {
            Some(300)
        } else {
            None
        },
    };

    let case = match create_case(&data.db, new_case).await {
        Ok(case) => case,
        Err(source) => {
            error!(?source, "failed to create word filter case");
            return;
        }
    };

    // Publish to modlog channel.
    if let Err(source) =
        publish_word_filter_to_modlog(ctx, data, guild_id, &case, &matched_word, action).await
    {
        error!(
            ?source,
            "failed to publish word filter case to modlog channel"
        );
    }
}

async fn publish_word_filter_to_modlog(
    ctx: &serenity::Context,
    data: &Data,
    guild_id: serenity::GuildId,
    case: &autumn_database::model::cases::CaseSummary,
    matched_word: &str,
    action: &str,
) -> Result<(), serenity::Error> {
    let channel_id = match get_modlog_channel_id(&data.db, guild_id.get()).await {
        Ok(Some(id)) => id,
        Ok(None) => return Ok(()),
        Err(source) => {
            error!(?source, "failed to read modlog channel for word filter");
            return Ok(());
        }
    };

    let case_label = format_case_label(&case.case_code, case.action_case_number);

    let action_label = match action {
        "timeout_delete_and_log" => "Timeout, Delete & Log",
        "delete_and_log" => "Delete & Log",
        "warn_and_log" => "Warn, Delete & Log",
        _ => "Log Only",
    };

    let mut fields = Vec::new();
    fields.push(format!(
        "**User :** <@{}>",
        case.target_user_id.unwrap_or(0)
    ));
    fields.push(format!("**Violation :** {}", matched_word));
    fields.push(format!("**Action Taken :** {}", action_label));

    if let Some(duration_seconds) = case.duration_seconds {
        fields.push(format!(
            "**Timeout Duration :** {}",
            format_compact_duration(duration_seconds)
        ));
    }

    // Blank line separator before metadata.
    fields.push(String::new());

    fields.push(format!("**When :** <t:{}:R>", case.created_at));

    let title = format!("Word Filter Violation - #{}", case_label);
    let description = fields.join("\n");

    let embed = serenity::CreateEmbed::new()
        .color(DEFAULT_EMBED_COLOR)
        .title(title)
        .description(description);

    serenity::ChannelId::new(channel_id)
        .send_message(&ctx.http, serenity::CreateMessage::new().embed(embed))
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
