use poise::serenity_prelude as serenity;

use crate::CommandMeta;
use crate::moderation::embeds::{guild_only_message, usage_message};
use autumn_core::{Context, Error};
use autumn_database::impls::ai_config::{get_llm_enabled, set_llm_enabled};
use autumn_utils::permissions::has_user_permission;

pub const META: CommandMeta = CommandMeta {
    name: "aitoggle",
    desc: "Enable or disable AI mention replies for this server.",
    category: "moderation",
    usage: "!aitoggle <on|off|status>",
};

#[poise::command(prefix_command, slash_command, category = "Moderation")]
pub async fn aitoggle(
    ctx: Context<'_>,
    #[description = "Desired state: on, off, or status"] state: Option<String>,
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

    let Some(raw_state) = state.as_deref().map(str::trim) else {
        ctx.say(usage_message(META.usage)).await?;
        return Ok(());
    };

    if raw_state.eq_ignore_ascii_case("status") {
        let enabled = get_llm_enabled(&ctx.data().db, guild_id.get()).await?;
        ctx.say(format!(
            "AI mention replies are currently **{}** for this server.",
            if enabled { "enabled" } else { "disabled" }
        ))
        .await?;
        return Ok(());
    }

    let new_state = if raw_state.eq_ignore_ascii_case("on") {
        true
    } else if raw_state.eq_ignore_ascii_case("off") {
        false
    } else {
        ctx.say(usage_message(META.usage)).await?;
        return Ok(());
    };

    set_llm_enabled(&ctx.data().db, guild_id.get(), new_state).await?;
    ctx.say(format!(
        "AI mention replies are now **{}** for this server.",
        if new_state { "enabled" } else { "disabled" }
    ))
    .await?;

    Ok(())
}
