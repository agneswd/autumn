use std::time::{Duration, SystemTime, UNIX_EPOCH};

use tracing::error;

use poise::serenity_prelude as serenity;

use crate::CommandMeta;
use crate::moderation::embeds::{
    guild_only_message, moderation_action_embed, target_profile_from_user, usage_message,
};
use crate::moderation::logging::create_case_and_publish;
use autumn_core::{Context, Error};
use autumn_database::impls::cases::NewCase;
use autumn_utils::formatting::format_compact_duration;
use autumn_utils::parse::parse_duration_seconds;
use autumn_utils::permissions::has_user_permission;

pub const META: CommandMeta = CommandMeta {
    name: "timeout",
    desc: "Timeout a user for a duration (default: 10m).",
    category: "moderation",
    usage: "!timeout <user> [duration] [reason]",
};

const DEFAULT_TIMEOUT_SECS: u64 = 10 * 60;

#[poise::command(prefix_command, slash_command, category = "Moderation")]
pub async fn timeout(
    ctx: Context<'_>,
    #[description = "The user to timeout"] user: Option<serenity::User>,
    #[description = "Duration (e.g. 10m, 2h)"] duration: Option<String>,
    #[description = "Reason for timeout"] #[rest] reason: Option<String>,
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

    if user.id == ctx.author().id {
        ctx.say("You can't timeout yourself.").await?;
        return Ok(());
    }

    let parsed_duration = match duration.as_deref().map(str::trim) {
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
        error!(?source, "timeout request failed");
        ctx.say("I couldn't timeout that user. Check role hierarchy and permissions.")
            .await?;
        return Ok(());
    }

    let case_reason = reason
        .as_deref()
        .unwrap_or("No reason provided")
        .to_owned();
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
        reason.as_deref(),
        Some(&duration_label),
    );
    if let Some(case_label) = case_label {
        embed = embed.footer(serenity::CreateEmbedFooter::new(format!("#{}", case_label)));
    }
    ctx.send(poise::CreateReply::default().embed(embed)).await?;

    Ok(())
}
