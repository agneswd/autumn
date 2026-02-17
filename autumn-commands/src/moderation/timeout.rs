use std::time::{Duration, SystemTime, UNIX_EPOCH};

use tracing::error;

use poise::serenity_prelude as serenity;

use crate::CommandMeta;
use crate::moderation::embeds::{
    guild_only_message, moderation_action_embed, permission_denied_message, target_profile_from_user,
    usage_message,
};
use autumn_core::{Context, Error};
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
        ctx.say(permission_denied_message()).await?;
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

    let parsed_duration = duration
        .as_deref()
        .and_then(parse_duration_seconds)
        .unwrap_or(DEFAULT_TIMEOUT_SECS);
    let duration_label = duration.unwrap_or_else(|| "10m".to_owned());

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

    let target_profile = target_profile_from_user(&user);
    let embed = moderation_action_embed(
        &target_profile,
        user.id,
        "timed out",
        reason.as_deref(),
        Some(&duration_label),
    );
    ctx.send(poise::CreateReply::default().embed(embed)).await?;

    Ok(())
}
