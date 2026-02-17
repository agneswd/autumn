use autumn_core::{Context, Error};

use crate::CommandMeta;

pub const META: CommandMeta = CommandMeta {
    name: "ping",
    desc: "Replies with Pong!",
    category: "utility",
    usage: "!ping",
};

#[poise::command(prefix_command, slash_command, category = "Utility")]
pub async fn ping(ctx: Context<'_>) -> Result<(), Error> {
    ctx.say("Pong!").await?;
    Ok(())
}
