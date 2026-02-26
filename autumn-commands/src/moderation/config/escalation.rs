use poise::serenity_prelude as serenity;

use crate::CommandMeta;
use crate::moderation::embeds::guild_only_message;
use autumn_core::{Context, Error};
use autumn_database::impls::escalation::{
    get_escalation_config, set_escalation_enabled, set_timeout_window, set_warn_threshold,
    set_warn_window,
};
use autumn_utils::embed::DEFAULT_EMBED_COLOR;
use autumn_utils::formatting::format_compact_duration;
use autumn_utils::parse::parse_duration_seconds;
use autumn_utils::permissions::has_user_permission;

pub const META: CommandMeta = CommandMeta {
    name: "escalation",
    desc: "Configure automatic warn → timeout escalation.",
    category: "moderation",
    usage: "!escalation <enable|disable|set>",
};

/// Configure automatic warn → timeout escalation.
#[poise::command(
    prefix_command,
    slash_command,
    category = "Moderation",
    subcommands("enable", "disable", "set")
)]
pub async fn escalation(ctx: Context<'_>) -> Result<(), Error> {
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

    let config = get_escalation_config(&ctx.data().db, guild_id.get()).await?;

    let (enabled, threshold, warn_window, timeout_window) = match &config {
        Some(cfg) => (
            cfg.enabled,
            cfg.warn_threshold,
            cfg.warn_window_seconds,
            cfg.timeout_window_seconds,
        ),
        None => (false, 3, 86400, 604800),
    };

    let status = if enabled { "Enabled" } else { "Disabled" };

    let embed = serenity::CreateEmbed::new()
        .title("Escalation Config")
        .description(format!(
            "**Status :** {}\n\
             **Warn Threshold :** {} warning(s)\n\
             **Warn Window :** {}\n\
             **Timeout Window :** {}\n\n\
             When a user reaches **{}** warning(s) within **{}**, they are \
             automatically timed out. Repeated timeouts within **{}** escalate \
             in duration.",
            status,
            threshold,
            format_compact_duration(warn_window as u64),
            format_compact_duration(timeout_window as u64),
            threshold,
            format_compact_duration(warn_window as u64),
            format_compact_duration(timeout_window as u64),
        ))
        .color(DEFAULT_EMBED_COLOR)
        .footer(serenity::CreateEmbedFooter::new(
            "Subcommands: enable, disable, set warns/warnwindow/timeoutwindow",
        ));

    ctx.send(poise::CreateReply::default().embed(embed)).await?;

    Ok(())
}

/// Enable automatic escalation.
#[poise::command(prefix_command, slash_command, category = "Moderation")]
pub async fn enable(ctx: Context<'_>) -> Result<(), Error> {
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

    set_escalation_enabled(&ctx.data().db, guild_id.get(), true).await?;
    ctx.say("Automatic escalation has been **enabled**.")
        .await?;

    Ok(())
}

/// Disable automatic escalation.
#[poise::command(prefix_command, slash_command, category = "Moderation")]
pub async fn disable(ctx: Context<'_>) -> Result<(), Error> {
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

    set_escalation_enabled(&ctx.data().db, guild_id.get(), false).await?;
    ctx.say("Automatic escalation has been **disabled**.")
        .await?;

    Ok(())
}

/// Set escalation parameters.
#[poise::command(
    prefix_command,
    slash_command,
    category = "Moderation",
    subcommands("warns", "warnwindow", "timeoutwindow")
)]
pub async fn set(ctx: Context<'_>) -> Result<(), Error> {
    ctx.say(
        "Usage:\n\
         `!escalation set warns <count>` — set warning threshold\n\
         `!escalation set warnwindow <duration>` — set warn counting window (e.g. `24h`, `7d`)\n\
         `!escalation set timeoutwindow <duration>` — set timeout escalation window (e.g. `7d`, `30d`)",
    )
    .await?;

    Ok(())
}

/// Set the warning threshold before auto-timeout triggers.
#[poise::command(prefix_command, slash_command, category = "Moderation")]
pub async fn warns(
    ctx: Context<'_>,
    #[description = "Number of warnings before auto-timeout"]
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

    let Some(raw) = input.as_deref().map(str::trim).filter(|s| !s.is_empty()) else {
        ctx.say("Usage: `!escalation set warns <count>` (e.g. `3`)")
            .await?;
        return Ok(());
    };

    let Ok(count) = raw.parse::<i32>() else {
        ctx.say("Invalid number. Usage: `!escalation set warns <count>` (e.g. `3`)")
            .await?;
        return Ok(());
    };

    if !(1..=100).contains(&count) {
        ctx.say("Threshold must be between 1 and 100.").await?;
        return Ok(());
    }

    set_warn_threshold(&ctx.data().db, guild_id.get(), count).await?;
    ctx.say(format!(
        "Warning threshold set to **{}** warning(s).",
        count
    ))
    .await?;

    Ok(())
}

/// Set the time window for counting warnings.
#[poise::command(prefix_command, slash_command, category = "Moderation")]
pub async fn warnwindow(
    ctx: Context<'_>,
    #[description = "Duration (e.g. 24h, 7d)"]
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

    let Some(raw) = input.as_deref().map(str::trim).filter(|s| !s.is_empty()) else {
        ctx.say("Usage: `!escalation set warnwindow <duration>` (e.g. `24h`, `7d`)")
            .await?;
        return Ok(());
    };

    let Some(seconds) = parse_duration_seconds(raw) else {
        ctx.say("Invalid duration. Examples: `1h`, `24h`, `7d`, `30d`")
            .await?;
        return Ok(());
    };

    let seconds_i64 = seconds as i64;
    set_warn_window(&ctx.data().db, guild_id.get(), seconds_i64).await?;
    ctx.say(format!(
        "Warn window set to **{}**.",
        format_compact_duration(seconds)
    ))
    .await?;

    Ok(())
}

/// Set the time window for timeout escalation.
#[poise::command(prefix_command, slash_command, category = "Moderation")]
pub async fn timeoutwindow(
    ctx: Context<'_>,
    #[description = "Duration (e.g. 7d, 30d)"]
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

    let Some(raw) = input.as_deref().map(str::trim).filter(|s| !s.is_empty()) else {
        ctx.say("Usage: `!escalation set timeoutwindow <duration>` (e.g. `7d`, `30d`)")
            .await?;
        return Ok(());
    };

    let Some(seconds) = parse_duration_seconds(raw) else {
        ctx.say("Invalid duration. Examples: `7d`, `14d`, `30d`")
            .await?;
        return Ok(());
    };

    let seconds_i64 = seconds as i64;
    set_timeout_window(&ctx.data().db, guild_id.get(), seconds_i64).await?;
    ctx.say(format!(
        "Timeout escalation window set to **{}**.",
        format_compact_duration(seconds)
    ))
    .await?;

    Ok(())
}
