use autumn_core::{Context, Error};

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

    ctx.say(format!("Usage: `{}`", command.usage)).await?;
    Ok(())
}
