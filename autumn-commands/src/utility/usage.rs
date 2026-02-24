use autumn_core::{Context, Error};
use autumn_utils::permissions::has_user_permission;
use poise::serenity_prelude as serenity;

use crate::{COMMANDS, CommandMeta};

pub const META: CommandMeta = CommandMeta {
    name: "usage",
    desc: "Show usage syntax for a specific command.",
    category: "utility",
    usage: "!usage <command>",
};

#[poise::command(prefix_command, slash_command, category = "Utility")]
pub async fn usage(
    ctx: Context<'_>,
    #[description = "Command name"] command: Option<String>,
) -> Result<(), Error> {
    let Some(raw_name) = command.as_deref() else {
        ctx.say(format!("Usage: `{}`", META.usage)).await?;
        return Ok(());
    };

    let lookup = raw_name.trim().trim_start_matches('!').to_ascii_lowercase();

    let Some(command) = COMMANDS.iter().find(|command| command.name == lookup) else {
        ctx.say(format!("Unknown command: `{}`", lookup)).await?;
        return Ok(());
    };

    if let (Some(guild_id), Some(required_permissions)) = (
        ctx.guild_id(),
        required_permissions_for_command(command.name),
    ) && !has_user_permission(ctx.http(), guild_id, ctx.author().id, required_permissions)
        .await?
    {
        return Ok(());
    }

    ctx.say(format!("Usage: `{}`", command.usage)).await?;
    Ok(())
}

fn required_permissions_for_command(command_name: &str) -> Option<serenity::Permissions> {
    match command_name {
        "ban" | "unban" => Some(serenity::Permissions::BAN_MEMBERS),
        "kick" => Some(serenity::Permissions::KICK_MEMBERS),
        "timeout" | "untimeout" => Some(serenity::Permissions::MODERATE_MEMBERS),
        "warn" | "warnings" | "unwarn" | "purge" | "permissions" | "modlogs" | "case" | "notes" => {
            Some(serenity::Permissions::MANAGE_MESSAGES)
        }
        "modlogchannel" | "aitoggle" => Some(serenity::Permissions::MANAGE_GUILD),
        "terminate" => {
            Some(serenity::Permissions::BAN_MEMBERS | serenity::Permissions::MANAGE_MESSAGES)
        }
        _ => None,
    }
}
