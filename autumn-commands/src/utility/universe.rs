use autumn_core::{Context, Error};

use crate::CommandMeta;

pub const META: CommandMeta = CommandMeta {
    name: "universe",
    desc: "The answer to the universe.",
    category: "utility",
    usage: "!universe",
};

#[poise::command(prefix_command, slash_command, category = "Utility")]
pub async fn universe(ctx: Context<'_>) -> Result<(), Error> {
    ctx.say("The answer to the universe is 67 😹").await?;
    Ok(())
}
