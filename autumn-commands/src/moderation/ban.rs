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
    name: "ban",
    desc: "Ban a user from the server.",
    category: "moderation",
    usage: "!ban <user> [reason]",
};

#[poise::command(prefix_command, slash_command, category = "Moderation")]
pub async fn ban(
    ctx: Context<'_>,
    #[description = "The user to ban"] user: Option<serenity::User>,
    #[description = "Reason for the ban"] #[rest] reason: Option<String>,
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
        return Ok(());
    }

    let Some(user) = user else {
        ctx.say(usage_message(META.usage)).await?;
        return Ok(());
    };

    if user.id == ctx.author().id {
        ctx.say("You can't ban yourself.").await?;
        return Ok(());
    }

    let ban_result = guild_id
        .ban_with_reason(ctx.http(), user.id, 0, reason.as_deref().unwrap_or("No reason provided"))
        .await;

    if let Err(source) = ban_result {
        error!(?source, "ban request failed");
        ctx.say("I couldn't ban that user. Check role hierarchy and permissions.")
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
            action: "ban",
            reason: &case_reason,
            status: "active",
            duration_seconds: None,
        },
    )
    .await;

    let target_profile = target_profile_from_user(&user);
    let mut embed = moderation_action_embed(&target_profile, user.id, "banned", reason.as_deref(), None);
    if let Some(case_label) = case_label {
        embed = embed.footer(serenity::CreateEmbedFooter::new(format!("#{}", case_label)));
    }
    ctx.send(poise::CreateReply::default().embed(embed)).await?;

    Ok(())
}
