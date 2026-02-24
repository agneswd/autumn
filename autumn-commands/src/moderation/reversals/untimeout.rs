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
use autumn_utils::permissions::has_user_permission;

pub const META: CommandMeta = CommandMeta {
    name: "untimeout",
    desc: "Remove timeout from a user.",
    category: "moderation",
    usage: "!untimeout <user> [reason]",
};

#[poise::command(prefix_command, slash_command, category = "Moderation")]
pub async fn untimeout(
    ctx: Context<'_>,
    #[description = "The user to untimeout"] user: Option<serenity::User>,
    #[description = "Reason for removing timeout"]
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

    let member = match guild_id.member(ctx.http(), user.id).await {
        Ok(member) => member,
        Err(source) => {
            error!(?source, "failed to fetch member before untimeout");
            ctx.say("I couldn't find that member in this server.")
                .await?;
            return Ok(());
        }
    };

    let now = serenity::Timestamp::now().unix_timestamp();
    let is_timed_out = member
        .communication_disabled_until
        .map(|until| until.unix_timestamp() > now)
        .unwrap_or(false);

    if !is_timed_out {
        ctx.say("That user is not currently timed out.").await?;
        return Ok(());
    }

    let edit = serenity::EditMember::new().enable_communication();
    let untimeout_result = guild_id.edit_member(ctx.http(), user.id, edit).await;

    if let Err(source) = untimeout_result {
        if !is_missing_permissions_error(&source) {
            error!(?source, "untimeout request failed");
        }
        ctx.say("I couldn't remove timeout from that user. Check permissions.")
            .await?;
        return Ok(());
    }

    let case_reason = reason.as_deref().unwrap_or("No reason provided").to_owned();

    let _ = send_moderation_target_dm_for_guild(
        ctx.http(),
        &user,
        guild_id,
        "untimed out",
        Some(&case_reason),
        None,
    )
    .await;

    let case_label = create_case_and_publish(
        &ctx,
        guild_id,
        NewCase {
            guild_id: guild_id.get(),
            target_user_id: Some(user.id.get()),
            moderator_user_id: ctx.author().id.get(),
            action: "untimeout",
            reason: &case_reason,
            status: "active",
            duration_seconds: None,
        },
    )
    .await;

    let target_profile = target_profile_from_user(&user);
    let mut embed = moderation_action_embed(
        &target_profile,
        user.id,
        "untimed out",
        reason.as_deref(),
        None,
    );
    if let Some(case_label) = case_label {
        embed = embed.footer(serenity::CreateEmbedFooter::new(format!("#{}", case_label)));
    }
    ctx.send(poise::CreateReply::default().embed(embed)).await?;

    Ok(())
}
