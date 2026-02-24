use tokio::time::{Duration, sleep};
use tracing::error;

use poise::serenity_prelude as serenity;

use crate::CommandMeta;
use crate::moderation::embeds::is_missing_permissions_error;
use crate::moderation::logging::create_case_and_publish;
use autumn_core::{Context, Error};
use autumn_database::impls::cases::NewCase;
use autumn_utils::permissions::has_user_permission;

pub const META: CommandMeta = CommandMeta {
    name: "purge",
    desc: "Delete the latest messages in this channel.",
    category: "moderation",
    usage: "!purge <amount>",
};

const MAX_PURGE: u16 = 100;

#[poise::command(prefix_command, slash_command, category = "Moderation")]
pub async fn purge(
    ctx: Context<'_>,
    #[description = "Amount of messages to purge"] amount: Option<u16>,
) -> Result<(), Error> {
    let Some(guild_id) = ctx.guild_id() else {
        ctx.say("This command only works in servers.").await?;
        return Ok(());
    };

    let Some(requested) = amount else {
        ctx.say(format!("Usage: `{}`", META.usage)).await?;
        return Ok(());
    };

    if requested == 0 {
        ctx.say("Amount must be at least 1.").await?;
        return Ok(());
    }

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

    let delete_count = requested.saturating_add(1).min(MAX_PURGE);

    let channel_id = ctx.channel_id();
    let messages = channel_id
        .messages(
            ctx.http(),
            serenity::GetMessages::new().limit(delete_count as u8),
        )
        .await?;

    let ids: Vec<serenity::MessageId> = messages.into_iter().map(|message| message.id).collect();
    let deleted_count = ids.len();

    if ids.is_empty() {
        ctx.say("No messages found to delete.").await?;
        return Ok(());
    }

    let delete_result = if ids.len() == 1 {
        channel_id.delete_message(ctx.http(), ids[0]).await
    } else {
        channel_id.delete_messages(ctx.http(), ids).await
    };

    if let Err(source) = delete_result {
        if !is_missing_permissions_error(&source) {
            error!(?source, "purge delete request failed");
        }
        ctx.say("I couldn't delete messages. I likely need the 'Manage Messages' permission.")
            .await?;
        return Ok(());
    }

    let case_reason = "No reason provided".to_owned();
    let _case_label = create_case_and_publish(
        &ctx,
        guild_id,
        NewCase {
            guild_id: guild_id.get(),
            target_user_id: None,
            moderator_user_id: ctx.author().id.get(),
            action: "purge",
            reason: &case_reason,
            status: "active",
            duration_seconds: None,
        },
    )
    .await;

    let success_message = channel_id
        .say(ctx.http(), format!("Purged {} message(s).", deleted_count))
        .await?;

    sleep(Duration::from_secs(3)).await;
    let _ = success_message.delete(ctx.http()).await;

    Ok(())
}
