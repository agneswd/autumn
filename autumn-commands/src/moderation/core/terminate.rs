use std::time::Duration;

use tracing::error;

use poise::serenity_prelude as serenity;

use crate::CommandMeta;
use crate::moderation::embeds::{
    guild_only_message, is_missing_permissions_error, moderation_action_embed,
    moderation_bot_target_message, moderation_self_action_message, target_profile_from_user,
    usage_message,
};
use crate::moderation::logging::create_case_and_publish;
use autumn_core::{Context, Error};
use autumn_database::impls::cases::NewCase;
use autumn_utils::confirmation::{ConfirmationResult, prompt_confirm_decline};
use autumn_utils::parse::parse_duration_seconds;
use autumn_utils::permissions::has_user_permission;

pub const META: CommandMeta = CommandMeta {
    name: "terminate",
    desc: "Ban a user and purge their messages (DANGER)",
    category: "moderation",
    usage: "!terminate <user> [period] [reason]",
};

const SECONDS_PER_DAY: u64 = 86_400;
const MAX_NATIVE_BAN_DELETE_DAYS: u8 = 7;
const TERMINATE_CONFIRM_TIMEOUT_SECS: u64 = 30;

#[poise::command(prefix_command, slash_command, category = "Moderation")]
pub async fn terminate(
    ctx: Context<'_>,
    #[description = "The user to terminate"] user: Option<serenity::User>,
    #[description = "Purge period (e.g. 7d) or reason"] period_or_reason: Option<String>,
    #[description = "Reason"]
    #[rest]
    reason_rest: Option<String>,
) -> Result<(), Error> {
    let Some(guild_id) = ctx.guild_id() else {
        ctx.say(guild_only_message()).await?;
        return Ok(());
    };

    let required_permissions =
        serenity::Permissions::BAN_MEMBERS | serenity::Permissions::MANAGE_MESSAGES;
    if !has_user_permission(ctx.http(), guild_id, ctx.author().id, required_permissions).await? {
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
        ctx.say(moderation_self_action_message("terminate")).await?;
        return Ok(());
    }

    let default_duration_secs = u64::from(MAX_NATIVE_BAN_DELETE_DAYS) * SECONDS_PER_DAY;
    let (purge_duration_secs, cutoff_display, reason) = match period_or_reason.as_deref() {
        Some(first) => {
            let Some(duration_secs) = parse_duration_seconds(first) else {
                ctx.say(format!(
                    "Invalid purge period. Usage: `{}` (examples: 30s, 10m, 2h, 7d)",
                    META.usage
                ))
                .await?;
                return Ok(());
            };

            if duration_secs > u64::from(MAX_NATIVE_BAN_DELETE_DAYS) * SECONDS_PER_DAY {
                ctx.say(format!(
                    "Invalid purge period. Max is 7d. Usage: `{}` (examples: 30s, 10m, 2h, 7d)",
                    META.usage
                ))
                .await?;
                return Ok(());
            }

            (duration_secs, first.to_owned(), reason_rest)
        }
        None => (
            default_duration_secs,
            format!("{}d (default)", MAX_NATIVE_BAN_DELETE_DAYS),
            None,
        ),
    };

    let native_delete_days = purge_duration_secs
        .div_ceil(SECONDS_PER_DAY)
        .min(u64::from(MAX_NATIVE_BAN_DELETE_DAYS)) as u8;

    let target_profile = target_profile_from_user(&user);
    let confirmation_embed = moderation_action_embed(
        &target_profile,
        user.id,
        "queued for termination",
        reason.as_deref(),
        None,
    );

    let confirmation_result = prompt_confirm_decline(
        ctx,
        format!(
            "Ban and purge pending moderator confirmation.\nPeriod: {}",
            cutoff_display
        ),
        confirmation_embed,
        Duration::from_secs(TERMINATE_CONFIRM_TIMEOUT_SECS),
    )
    .await?;

    let interaction = match confirmation_result {
        ConfirmationResult::TimedOut(message) => {
            let timeout_embed = moderation_action_embed(
                &target_profile,
                user.id,
                "left unchanged",
                Some("Timed out"),
                None,
            );

            message
                .channel_id
                .edit_message(
                    ctx.http(),
                    message.id,
                    serenity::EditMessage::new()
                        .content("Timed out")
                        .embed(timeout_embed)
                        .components(vec![]),
                )
                .await?;
            return Ok(());
        }
        ConfirmationResult::Declined(interaction) => {
            interaction
                .create_response(
                    ctx.http(),
                    serenity::CreateInteractionResponse::UpdateMessage(
                        serenity::CreateInteractionResponseMessage::new()
                            .content("Termination cancelled.")
                            .embed(moderation_action_embed(
                                &target_profile,
                                user.id,
                                "left unchanged",
                                Some("Termination cancelled."),
                                None,
                            ))
                            .components(vec![]),
                    ),
                )
                .await?;
            return Ok(());
        }
        ConfirmationResult::Confirmed(interaction) => interaction,
    };

    interaction
        .create_response(
            ctx.http(),
            serenity::CreateInteractionResponse::UpdateMessage(
                serenity::CreateInteractionResponseMessage::new()
                    .content("Terminating...")
                    .embed(moderation_action_embed(
                        &target_profile,
                        user.id,
                        "queued for termination",
                        reason.as_deref(),
                        None,
                    ))
                    .components(vec![]),
            ),
        )
        .await?;

    if let Err(source) = guild_id
        .ban_with_reason(
            ctx.http(),
            user.id,
            native_delete_days,
            reason.as_deref().unwrap_or("No reason provided"),
        )
        .await
    {
        if !is_missing_permissions_error(&source) {
            error!(?source, "terminate ban failed");
        }
        interaction
            .edit_response(
                ctx.http(),
                serenity::EditInteractionResponse::new()
                    .content("Ban failed. Check hierarchy and permissions.")
                    .embed(moderation_action_embed(
                        &target_profile,
                        user.id,
                        "not terminated",
                        Some("Ban failed. Check hierarchy and permissions."),
                        None,
                    )),
            )
            .await?;
        return Ok(());
    }

    interaction
        .edit_response(
            ctx.http(),
            serenity::EditInteractionResponse::new()
                .content(format!(
                    "Ban applied. Discord native cleanup executed (up to {} day(s)).",
                    native_delete_days
                ))
                .embed(moderation_action_embed(
                    &target_profile,
                    user.id,
                    "termination in progress",
                    reason.as_deref(),
                    None,
                )),
        )
        .await?;

    let final_content = format!(
        "Ban applied. Native cleanup done.\nPurge period: last {} day(s)",
        native_delete_days
    );

    let case_reason = reason.as_deref().unwrap_or("No reason provided").to_owned();

    let case_label = create_case_and_publish(
        &ctx,
        guild_id,
        NewCase {
            guild_id: guild_id.get(),
            target_user_id: Some(user.id.get()),
            moderator_user_id: ctx.author().id.get(),
            action: "terminate",
            reason: &case_reason,
            status: "active",
            duration_seconds: Some(purge_duration_secs),
        },
    )
    .await;

    let mut final_embed = moderation_action_embed(
        &target_profile,
        user.id,
        "terminated",
        reason.as_deref(),
        None,
    );

    if let Some(case_label) = case_label {
        final_embed =
            final_embed.footer(serenity::CreateEmbedFooter::new(format!("#{}", case_label)));
    }

    interaction
        .edit_response(
            ctx.http(),
            serenity::EditInteractionResponse::new()
                .content(final_content)
                .embed(final_embed),
        )
        .await?;

    Ok(())
}
