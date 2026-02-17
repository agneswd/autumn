use tokio::time::{Duration, sleep};
use tracing::error;

use poise::serenity_prelude as serenity;

use crate::CommandMeta;
use autumn_core::{Context, Error};
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
        ctx.say("You are not permitted to use this command.").await?;
        return Ok(());
    }

    let amount = requested.min(MAX_PURGE);
    let delete_count = amount.saturating_add(1).min(MAX_PURGE);

    let channel_id = ctx.channel_id();
    let messages = channel_id
        .messages(ctx.http(), serenity::GetMessages::new().limit(delete_count as u8))
        .await?;

    let ids: Vec<serenity::MessageId> = messages.into_iter().map(|message| message.id).collect();

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
        error!(?source, "purge delete request failed");
        ctx.say("I couldn't delete messages. I likely need the 'Manage Messages' permission.")
            .await?;
        return Ok(());
    }

    let confirmation = ctx.say(format!("Purged {} message(s).", amount)).await?;
    sleep(Duration::from_secs(3)).await;
    let _ = confirmation
        .message()
        .await?
        .delete(ctx.http())
        .await;

    Ok(())
}
