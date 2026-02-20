use poise::serenity_prelude as serenity;

use crate::CommandMeta;
use crate::moderation::embeds::{
    guild_only_message, moderation_action_embed, target_profile_from_user, usage_message,
};
use crate::moderation::logging::create_case_and_publish;
use autumn_core::{Context, Error};
use autumn_database::impls::cases::NewCase;
use autumn_database::impls::warnings::record_warning;
use autumn_utils::permissions::has_user_permission;

pub const META: CommandMeta = CommandMeta {
    name: "warn",
    desc: "Issue a warning to a user.",
    category: "moderation",
    usage: "!warn <user> [reason]",
};

#[poise::command(prefix_command, slash_command, category = "Moderation")]
pub async fn warn(
    ctx: Context<'_>,
    #[description = "The user to warn"] user: Option<serenity::User>,
    #[description = "Reason for warning"] #[rest] reason: Option<String>,
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

    let reason = reason.unwrap_or_else(|| "No reason provided".to_owned());
    let warning = record_warning(
        &ctx.data().db,
        guild_id.get(),
        user.id.get(),
        ctx.author().id.get(),
        &reason,
    )
    .await?;

    let case_label = create_case_and_publish(
        &ctx,
        guild_id,
        NewCase {
            guild_id: guild_id.get(),
            target_user_id: Some(user.id.get()),
            moderator_user_id: ctx.author().id.get(),
            action: "warn",
            reason: &reason,
            status: "active",
            duration_seconds: None,
        },
    )
    .await;

    let action = format!("warned #{}", warning.warn_number);
    let target_profile = target_profile_from_user(&user);
    let mut embed = moderation_action_embed(&target_profile, user.id, &action, Some(&reason), None);
    if let Some(case_label) = case_label {
        embed = embed.footer(serenity::CreateEmbedFooter::new(format!("#{}", case_label)));
    }
    ctx.send(poise::CreateReply::default().embed(embed)).await?;

    Ok(())
}
