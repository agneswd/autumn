use poise::serenity_prelude as serenity;

use crate::CommandMeta;
use crate::moderation::embeds::{
    guild_only_message, permission_denied_message, usage_message,
};
use autumn_core::{Context, Error};
use autumn_database::impls::warnings::{clear_warnings, remove_warning_by_number};
use autumn_utils::permissions::has_user_permission;

pub const META: CommandMeta = CommandMeta {
    name: "unwarn",
    desc: "Remove a warning by number, or clear all warnings for a user.",
    category: "moderation",
    usage: "!unwarn <user> <warn_number|all>",
};

#[poise::command(prefix_command, slash_command, category = "Moderation")]
pub async fn unwarn(
    ctx: Context<'_>,
    #[description = "The user to modify warnings for"] user: Option<serenity::User>,
    #[description = "Warning number or 'all'"] selector: Option<String>,
) -> Result<(), Error> {
    let Some(guild_id) = ctx.guild_id() else {
        ctx.say(guild_only_message()).await?;
        return Ok(());
    };

    if !has_user_permission(
        ctx.http(),
        guild_id,
        ctx.author().id,
        serenity::Permissions::MANAGE_MESSAGES,
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
    let target_label = user
        .global_name
        .as_deref()
        .unwrap_or(&user.name)
        .to_owned();

    let Some(selector) = selector.as_deref() else {
        ctx.say(usage_message(META.usage)).await?;
        return Ok(());
    };

    if selector.eq_ignore_ascii_case("all") {
        let removed = clear_warnings(&ctx.data().db, guild_id.get(), user.id.get()).await?;
        ctx.say(format!(
            "Removed {} warning(s) for {}.",
            removed, target_label
        ))
        .await?;
        return Ok(());
    }

    let Ok(warning_number) = selector.parse::<usize>() else {
        ctx.say("Selector must be a warning number or 'all'.").await?;
        return Ok(());
    };

    if warning_number == 0 {
        ctx.say("Warning number must be 1 or greater.").await?;
        return Ok(());
    }

    let removed = remove_warning_by_number(
        &ctx.data().db,
        guild_id.get(),
        user.id.get(),
        warning_number,
    )
    .await?;

    if removed {
        ctx.say(format!(
            "Removed warning #{} for {}.",
            warning_number, target_label
        ))
        .await?;
    } else {
        ctx.say(format!(
            "Warning #{} was not found for {}.",
            warning_number, target_label
        ))
        .await?;
    }

    Ok(())
}