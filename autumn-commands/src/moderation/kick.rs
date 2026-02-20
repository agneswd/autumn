use tracing::error;

use poise::serenity_prelude as serenity;

use crate::CommandMeta;
use crate::moderation::embeds::{
    guild_only_message, moderation_action_embed, target_profile_from_user, usage_message,
};
use crate::moderation::logging::create_case_and_publish;
use autumn_core::{Context, Error};
use autumn_database::impls::cases::NewCase;
use autumn_utils::permissions::has_user_permission;

pub const META: CommandMeta = CommandMeta {
    name: "kick",
    desc: "Kick a user from the server.",
    category: "moderation",
    usage: "!kick <user> [reason]",
};

#[poise::command(prefix_command, slash_command, category = "Moderation")]
pub async fn kick(
    ctx: Context<'_>,
    #[description = "The user to kick"] user: Option<serenity::User>,
    #[description = "Reason for the kick"] #[rest] reason: Option<String>,
) -> Result<(), Error> {
    let Some(guild_id) = ctx.guild_id() else {
        ctx.say(guild_only_message()).await?;
        return Ok(());
    };

    if !has_user_permission(
        ctx.http(),
        guild_id,
        ctx.author().id,
        serenity::Permissions::KICK_MEMBERS,
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
        ctx.say("You can't kick yourself.").await?;
        return Ok(());
    }

    let kick_result = guild_id.kick_with_reason(
        ctx.http(),
        user.id,
        reason.as_deref().unwrap_or("No reason provided"),
    )
    .await;

    if let Err(source) = kick_result {
        error!(?source, "kick request failed");
        ctx.say("I couldn't kick that user. Check role hierarchy and permissions.")
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
            action: "kick",
            reason: &case_reason,
            status: "active",
            duration_seconds: None,
        },
    )
    .await;

    let target_profile = target_profile_from_user(&user);
    let mut embed = moderation_action_embed(&target_profile, user.id, "kicked", reason.as_deref(), None);
    if let Some(case_label) = case_label {
        embed = embed.footer(serenity::CreateEmbedFooter::new(format!("#{}", case_label)));
    }
    ctx.send(poise::CreateReply::default().embed(embed)).await?;

    Ok(())
}
