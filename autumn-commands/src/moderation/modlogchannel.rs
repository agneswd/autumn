use poise::serenity_prelude as serenity;

use crate::CommandMeta;
use crate::moderation::embeds::guild_only_message;
use autumn_core::{Context, Error};
use autumn_database::impls::modlog_config::{
    clear_modlog_channel_id, get_modlog_channel_id, set_modlog_channel_id,
};
use autumn_utils::permissions::has_user_permission;

pub const META: CommandMeta = CommandMeta {
    name: "modlogchannel",
    desc: "Set or view the moderation log channel.",
    category: "moderation",
    usage: "!modlogchannel [#channel|channel_id|clear]",
};

#[poise::command(prefix_command, slash_command, category = "Moderation")]
pub async fn modlogchannel(
    ctx: Context<'_>,
    #[description = "Channel mention/id, or 'clear'"]
    #[rest]
    input: Option<String>,
) -> Result<(), Error> {
    let Some(guild_id) = ctx.guild_id() else {
        ctx.say(guild_only_message()).await?;
        return Ok(());
    };

    if !has_user_permission(
        ctx.http(),
        guild_id,
        ctx.author().id,
        serenity::Permissions::MANAGE_GUILD,
    )
    .await?
    {
        return Ok(());
    }

    if let Some(input) = input
        .as_deref()
        .map(str::trim)
        .filter(|entry| !entry.is_empty())
    {
        if input.eq_ignore_ascii_case("clear") {
            clear_modlog_channel_id(&ctx.data().db, guild_id.get()).await?;
            ctx.say("Modlog channel cleared.").await?;
            return Ok(());
        }

        if let Some(channel_id) = parse_channel_id(input) {
            set_modlog_channel_id(&ctx.data().db, guild_id.get(), channel_id).await?;
            ctx.say(format!("Modlog channel set to <#{}>.", channel_id))
                .await?;
            return Ok(());
        }

        ctx.say("Provide a valid channel mention/id, or `clear`.")
            .await?;
        return Ok(());
    }

    let current = get_modlog_channel_id(&ctx.data().db, guild_id.get()).await?;
    if let Some(channel_id) = current {
        ctx.say(format!("Current modlog channel: <#{}>", channel_id))
            .await?;
    } else {
        ctx.say("No modlog channel configured.").await?;
    }

    Ok(())
}

fn parse_channel_id(raw: &str) -> Option<u64> {
    if let Ok(id) = raw.parse::<u64>() {
        return Some(id);
    }

    if raw.starts_with("<#") && raw.ends_with('>') {
        return raw
            .trim_start_matches("<#")
            .trim_end_matches('>')
            .parse::<u64>()
            .ok();
    }

    None
}
