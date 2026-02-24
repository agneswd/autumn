use std::time::{Duration, SystemTime, UNIX_EPOCH};

use tracing::error;

use poise::serenity_prelude as serenity;

use crate::CommandMeta;
use crate::moderation::embeds::{
    guild_only_message, is_missing_permissions_error, moderation_action_embed,
    moderation_bot_target_message, send_moderation_target_dm_for_guild, target_profile_from_user,
    usage_message,
};
use crate::moderation::logging::create_case_and_publish;
use autumn_core::{Context, Error};
use autumn_database::impls::cases::NewCase;
use autumn_utils::formatting::format_compact_duration;
use autumn_utils::parse::{has_duration_unit, parse_duration_seconds};
use autumn_utils::permissions::has_user_permission;

pub const META: CommandMeta = CommandMeta {
    name: "timeout",
    desc: "Timeout a user for a duration (default: 10m).",
    category: "moderation",
    usage: "!timeout <user> [duration] [reason]",
};

const DEFAULT_TIMEOUT_SECS: u64 = 10 * 60;

fn is_explicit_unit_duration_token(raw: &str) -> bool {
    has_duration_unit(raw) && parse_duration_seconds(raw).is_some()
}

fn split_timeout_duration_and_reason(
    duration: Option<&str>,
    reason: Option<&str>,
) -> (Option<String>, Option<String>) {
    let mut duration_parts = Vec::new();
    if let Some(raw_duration) = duration.map(str::trim).filter(|value| !value.is_empty()) {
        duration_parts.push(raw_duration.to_owned());
    }

    let mut reason_tokens = Vec::new();
    if let Some(rest) = reason {
        let mut tokens = rest.split_whitespace();
        let collect_more_duration = !duration_parts.is_empty();

        while let Some(token) = tokens.next() {
            if token == "--" {
                reason_tokens.extend(tokens.map(str::to_owned));
                break;
            }

            if collect_more_duration && is_explicit_unit_duration_token(token) {
                duration_parts.push(token.to_owned());
                continue;
            }

            reason_tokens.push(token.to_owned());
            reason_tokens.extend(tokens.map(str::to_owned));
            break;
        }
    }

    let parsed_duration_input = if duration_parts.is_empty() {
        None
    } else {
        Some(duration_parts.join(" "))
    };

    let parsed_reason = if reason_tokens.is_empty() {
        None
    } else {
        Some(reason_tokens.join(" "))
    };

    (parsed_duration_input, parsed_reason)
}

#[poise::command(prefix_command, slash_command, category = "Moderation")]
pub async fn timeout(
    ctx: Context<'_>,
    #[description = "The user to timeout"] user: Option<serenity::User>,
    #[description = "Duration (e.g. 10m, 2h)"] duration: Option<String>,
    #[description = "Reason for timeout"]
    #[rest]
    reason: Option<String>,
) -> Result<(), Error> {
    let Some(guild_id) = ctx.guild_id() else {
        ctx.say(guild_only_message()).await?;
        return Ok(());
    };

    if !has_user_permission(
        ctx.http(),
        guild_id,
        ctx.author().id,
        serenity::Permissions::MODERATE_MEMBERS,
    )
    .await?
    {
        return Ok(());
    }

    let Some(user) = user else {
        ctx.say(usage_message(META.usage)).await?;
        return Ok(());
    };

    if user.bot {
        ctx.say(moderation_bot_target_message()).await?;
        return Ok(());
    }

    if user.id == ctx.author().id {
        ctx.say("You can't timeout yourself.").await?;
        return Ok(());
    }

    let (duration_input, parsed_reason) =
        split_timeout_duration_and_reason(duration.as_deref(), reason.as_deref());

    let parsed_duration = match duration_input.as_deref().map(str::trim) {
        Some(raw) if !raw.is_empty() => {
            let Some(seconds) = parse_duration_seconds(raw) else {
                ctx.say(format!(
                    "Invalid duration. Usage: `{}` (examples: 30s, 10m, 2h, 1d)",
                    META.usage
                ))
                .await?;
                return Ok(());
            };
            seconds
        }
        _ => DEFAULT_TIMEOUT_SECS,
    };
    let duration_label = format_compact_duration(parsed_duration);

    let until_system_time = SystemTime::now()
        .checked_add(Duration::from_secs(parsed_duration))
        .unwrap_or(SystemTime::now());
    let until_unix = until_system_time
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs()) as i64;
    let until = serenity::Timestamp::from_unix_timestamp(until_unix)?;

    let edit = serenity::EditMember::new().disable_communication_until_datetime(until);
    let timeout_result = guild_id.edit_member(ctx.http(), user.id, edit).await;

    if let Err(source) = timeout_result {
        if !is_missing_permissions_error(&source) {
            error!(?source, "timeout request failed");
        }
        ctx.say("I couldn't timeout that user. Check role hierarchy and permissions.")
            .await?;
        return Ok(());
    }

    let case_reason = parsed_reason
        .as_deref()
        .unwrap_or("No reason provided")
        .to_owned();

    let _ = send_moderation_target_dm_for_guild(
        ctx.http(),
        &user,
        guild_id,
        "timed out",
        Some(&case_reason),
        Some(&duration_label),
    )
    .await;

    let case_label = create_case_and_publish(
        &ctx,
        guild_id,
        NewCase {
            guild_id: guild_id.get(),
            target_user_id: Some(user.id.get()),
            moderator_user_id: ctx.author().id.get(),
            action: "timeout",
            reason: &case_reason,
            status: "active",
            duration_seconds: Some(parsed_duration),
        },
    )
    .await;

    let target_profile = target_profile_from_user(&user);
    let mut embed = moderation_action_embed(
        &target_profile,
        user.id,
        "timed out",
        parsed_reason.as_deref(),
        Some(&duration_label),
    );
    if let Some(case_label) = case_label {
        embed = embed.footer(serenity::CreateEmbedFooter::new(format!("#{}", case_label)));
    }
    ctx.send(poise::CreateReply::default().embed(embed)).await?;

    Ok(())
}
