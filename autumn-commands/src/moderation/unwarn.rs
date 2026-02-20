use poise::serenity_prelude as serenity;
use std::time::Duration;

use crate::CommandMeta;
use crate::moderation::embeds::{
    guild_only_message, usage_message,
};
use crate::moderation::logging::create_case_and_publish;
use autumn_core::{Context, Error};
use autumn_database::impls::cases::NewCase;
use autumn_database::impls::warnings::{clear_warnings, remove_warning_by_number};
use autumn_utils::confirmation::{prompt_confirm_decline, resolve_confirmation_result};
use autumn_utils::permissions::has_user_permission;

pub const META: CommandMeta = CommandMeta {
    name: "unwarn",
    desc: "Remove a warning from a user.",
    category: "moderation",
    usage: "!unwarn <user> <warn_number|all>",
};

const UNWARN_ALL_CONFIRM_TIMEOUT_SECS: u64 = 30;

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
        let confirm_embed = serenity::CreateEmbed::new().description(format!(
            "This will clear all warnings for <@{}>.",
            user.id.get()
        ));

        let confirmation = prompt_confirm_decline(
            ctx,
            "Confirm warning clear",
            confirm_embed,
            Duration::from_secs(UNWARN_ALL_CONFIRM_TIMEOUT_SECS),
        )
        .await?;

        let Some(interaction) = resolve_confirmation_result(
            ctx,
            confirmation,
            "Timed out",
            "Warning clear cancelled.",
            "Clearing warnings...",
        )
        .await?
        else {
            return Ok(());
        };

        let removed = clear_warnings(&ctx.data().db, guild_id.get(), user.id.get()).await?;
        let case_reason = "No reason provided".to_owned();
        let _case_label = create_case_and_publish(
            &ctx,
            guild_id,
            NewCase {
                guild_id: guild_id.get(),
                target_user_id: Some(user.id.get()),
                moderator_user_id: ctx.author().id.get(),
                action: "unwarn_all",
                reason: &case_reason,
                status: "active",
                duration_seconds: None,
            },
        )
        .await;

        interaction
            .edit_response(
                ctx.http(),
                serenity::EditInteractionResponse::new().content(format!(
                    "Removed {} warning(s) for {}.",
                    removed, target_label
                ))
                .embeds(vec![]),
            )
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
        let case_reason = "No reason provided".to_owned();
        let _case_label = create_case_and_publish(
            &ctx,
            guild_id,
            NewCase {
                guild_id: guild_id.get(),
                target_user_id: Some(user.id.get()),
                moderator_user_id: ctx.author().id.get(),
                action: "unwarn",
                reason: &case_reason,
                status: "active",
                duration_seconds: None,
            },
        )
        .await;

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