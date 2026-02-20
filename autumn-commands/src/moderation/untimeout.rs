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
    name: "untimeout",
    desc: "Remove timeout from a user.",
    category: "moderation",
    usage: "!untimeout <user> [reason]",
};

#[poise::command(prefix_command, slash_command, category = "Moderation")]
pub async fn untimeout(
    ctx: Context<'_>,
    #[description = "The user to untimeout"] user: Option<serenity::User>,
    #[description = "Reason for removing timeout"] #[rest] reason: Option<String>,
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

    let edit = serenity::EditMember::new().enable_communication();
    let untimeout_result = guild_id.edit_member(ctx.http(), user.id, edit).await;

    if let Err(source) = untimeout_result {
        error!(?source, "untimeout request failed");
        ctx.say("I couldn't remove timeout from that user. Check permissions.")
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
            action: "untimeout",
            reason: &case_reason,
            status: "active",
            duration_seconds: None,
        },
    )
    .await;

    let target_profile = target_profile_from_user(&user);
    let mut embed = moderation_action_embed(&target_profile, user.id, "untimed out", reason.as_deref(), None);
    if let Some(case_label) = case_label {
        embed = embed.footer(serenity::CreateEmbedFooter::new(format!("#{}", case_label)));
    }
    ctx.send(poise::CreateReply::default().embed(embed)).await?;

    Ok(())
}
