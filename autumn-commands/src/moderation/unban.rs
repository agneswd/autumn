use tracing::error;

use poise::serenity_prelude as serenity;

use crate::CommandMeta;
use crate::moderation::embeds::{
    guild_only_message, moderation_action_embed, permission_denied_message, target_profile_from_user,
    usage_message,
};
use autumn_core::{Context, Error};
use autumn_utils::permissions::has_user_permission;

pub const META: CommandMeta = CommandMeta {
    name: "unban",
    desc: "Unban a user from the server.",
    category: "moderation",
    usage: "!unban <user> [reason]",
};

#[poise::command(prefix_command, slash_command, category = "Moderation")]
pub async fn unban(
    ctx: Context<'_>,
    #[description = "The user to unban"] user: Option<serenity::User>,
    #[description = "Reason for the unban"] #[rest] reason: Option<String>,
) -> Result<(), Error> {
    let Some(guild_id) = ctx.guild_id() else {
        ctx.say(guild_only_message()).await?;
        return Ok(());
    };

    if !has_user_permission(
        ctx.http(),
        guild_id,
        ctx.author().id,
        serenity::Permissions::BAN_MEMBERS,
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

    if let Err(source) = guild_id.unban(ctx.http(), user.id).await {
        error!(?source, "unban request failed");
        ctx.say("I couldn't unban that user. They may not be banned, or I lack permissions.")
            .await?;
        return Ok(());
    }

    let target_profile = target_profile_from_user(&user);
    let embed = moderation_action_embed(&target_profile, user.id, "unbanned", reason.as_deref(), None);
    ctx.send(poise::CreateReply::default().embed(embed)).await?;

    Ok(())
}
