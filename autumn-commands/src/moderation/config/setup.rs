use poise::serenity_prelude as serenity;

use crate::CommandMeta;
use crate::moderation::embeds::guild_only_message;
use autumn_core::{Context, Error};
use autumn_database::impls::escalation::{
    set_escalation_enabled, set_timeout_window, set_warn_threshold, set_warn_window,
};
use autumn_database::impls::modlog_config::set_modlog_channel_id;
use autumn_database::impls::userlog_config::set_userlog_channel_id;
use autumn_database::impls::word_filter::{
    load_preset_words, set_word_filter_action, set_word_filter_enabled,
};
use autumn_utils::embed::DEFAULT_EMBED_COLOR;
use autumn_utils::permissions::has_user_permission;

pub const META: CommandMeta = CommandMeta {
    name: "setup",
    desc: "Apply a moderation configuration preset to this server.",
    category: "moderation",
    usage: "!setup <basic|standard|strict> [#modlog-channel] [#userlog-channel]",
};

enum SetupPreset {
    Basic,
    Standard,
    Strict,
}

impl SetupPreset {
    fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "basic" => Some(Self::Basic),
            "standard" => Some(Self::Standard),
            "strict" => Some(Self::Strict),
            _ => None,
        }
    }

    fn name(&self) -> &'static str {
        match self {
            Self::Basic => "Basic",
            Self::Standard => "Standard",
            Self::Strict => "Strict",
        }
    }

    fn escalation_enabled(&self) -> bool {
        !matches!(self, Self::Basic)
    }

    fn escalation_threshold(&self) -> i32 {
        match self {
            Self::Basic | Self::Standard => 3,
            Self::Strict => 2,
        }
    }

    // Warn counting window in seconds.
    fn warn_window_secs(&self) -> i64 {
        match self {
            Self::Basic | Self::Standard => 86_400, // 24h
            Self::Strict => 604_800,                // 7d
        }
    }

    // Timeout escalation tier window in seconds.
    fn timeout_window_secs(&self) -> i64 {
        match self {
            Self::Basic | Self::Standard => 604_800, // 7d
            Self::Strict => 2_592_000,               // 30d
        }
    }

    fn wordfilter_enabled(&self) -> bool {
        !matches!(self, Self::Basic)
    }

    fn wordfilter_action(&self) -> &'static str {
        match self {
            Self::Basic => "log_only",
            Self::Standard => "warn_and_log",
            Self::Strict => "timeout_delete_and_log",
        }
    }

    fn load_preset_words(&self) -> bool {
        matches!(self, Self::Strict)
    }
}

/// Apply a moderation configuration preset to this server.
#[poise::command(prefix_command, slash_command, category = "Moderation")]
pub async fn setup(
    ctx: Context<'_>,
    #[description = "Preset: basic, standard, or strict"] preset: Option<String>,
    #[description = "Modlog channel (mention or ID; auto-created if omitted)"] modlog_input: Option<
        String,
    >,
    #[description = "Userlog channel (mention or ID; auto-created if omitted)"]
    userlog_input: Option<String>,
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

    let Some(raw_preset) = preset.as_deref().map(str::trim).filter(|s| !s.is_empty()) else {
        ctx.say(
            "**Autumn Setup Presets**\n\n\
             \u{2022} `basic` \u{2014} Modlog + userlog channels. Escalation and word filter disabled.\n\
             \u{2022} `standard` \u{2014} Modlog + userlog + escalation (3 warns in 24h \u{2192} auto-timeout, 7d tier window) + word filter (warn action).\n\
             \u{2022} `strict` \u{2014} Modlog + userlog + escalation (2 warns in 7d \u{2192} auto-timeout, 30d tier window) + word filter (timeout action) + preset word list loaded.\n\n\
             **Usage:** `!setup <basic|standard|strict> [#modlog-channel] [#userlog-channel]`\n\
             If channels are omitted, Autumn finds or creates `mod-logs` and `user-logs`.",
        )
        .await?;
        return Ok(());
    };

    let Some(preset) = SetupPreset::from_str(raw_preset) else {
        ctx.say(
            "Unknown preset. Use `basic`, `standard`, or `strict`.\n\nRun `!setup` for details.",
        )
        .await?;
        return Ok(());
    };

    let modlog_provided = modlog_input
        .as_deref()
        .and_then(|s| parse_channel_id(s.trim()));
    let userlog_provided = userlog_input
        .as_deref()
        .and_then(|s| parse_channel_id(s.trim()));

    let modlog_id =
        match resolve_or_create_channel(ctx.http(), guild_id, modlog_provided, "mod-logs").await {
            Ok(id) => id,
            Err(e) => {
                ctx.say(format!(
                    "Could not resolve or create `mod-logs` channel: {e}\n\
                 Pass the channel explicitly: `!setup {raw_preset} #mod-logs #user-logs`"
                ))
                .await?;
                return Ok(());
            }
        };

    let userlog_id = match resolve_or_create_channel(
        ctx.http(),
        guild_id,
        userlog_provided,
        "user-logs",
    )
    .await
    {
        Ok(id) => id,
        Err(e) => {
            ctx.say(format!(
                "Could not resolve or create `user-logs` channel: {e}\n\
                 Pass the channel explicitly: `!setup {raw_preset} #mod-logs #user-logs`"
            ))
            .await?;
            return Ok(());
        }
    };

    let db = &ctx.data().db;
    let gid = guild_id.get();

    // Channels
    set_modlog_channel_id(db, gid, modlog_id.get()).await?;
    set_userlog_channel_id(db, gid, userlog_id.get()).await?;

    // Escalation
    set_escalation_enabled(db, gid, preset.escalation_enabled()).await?;
    if preset.escalation_enabled() {
        set_warn_threshold(db, gid, preset.escalation_threshold()).await?;
        set_warn_window(db, gid, preset.warn_window_secs()).await?;
        set_timeout_window(db, gid, preset.timeout_window_secs()).await?;
    }

    // Word filter
    set_word_filter_enabled(db, gid, preset.wordfilter_enabled()).await?;
    if preset.wordfilter_enabled() {
        set_word_filter_action(db, gid, preset.wordfilter_action()).await?;
    }

    let preset_words_loaded: u64 = if preset.load_preset_words() {
        load_preset_words(db, gid).await?
    } else {
        0
    };

    // Build summary embed.
    let escalation_desc = if preset.escalation_enabled() {
        format!(
            "Enabled — {} warn(s) in {} \u{2192} auto-timeout (tier window {})",
            preset.escalation_threshold(),
            secs_to_label(preset.warn_window_secs() as u64),
            secs_to_label(preset.timeout_window_secs() as u64),
        )
    } else {
        "Disabled".to_owned()
    };

    let wordfilter_desc = if preset.wordfilter_enabled() {
        let action_label = match preset.wordfilter_action() {
            "warn_and_log" => "Warn, Delete and Log",
            "timeout_delete_and_log" => "Timeout, Delete and Log",
            _ => "Only Log",
        };
        if preset_words_loaded > 0 {
            format!("Enabled — {action_label} — {preset_words_loaded} preset word(s) loaded")
        } else {
            format!("Enabled \u{2014} {action_label}")
        }
    } else {
        "Disabled".to_owned()
    };

    let embed = serenity::CreateEmbed::new()
        .title(format!("{} Preset Applied", preset.name()))
        .description(format!(
            "**Modlog Channel :** <#{}>\n\
             **Userlog Channel :** <#{}>\n\
             **Escalation :** {escalation_desc}\n\
             **Word Filter :** {wordfilter_desc}",
            modlog_id.get(),
            userlog_id.get(),
        ))
        .color(DEFAULT_EMBED_COLOR)
        .footer(serenity::CreateEmbedFooter::new(
            "You can fine-tune these settings with !modlogchannel, !userlogchannel, !escalation, and !wordfilter.",
        ));

    ctx.send(poise::CreateReply::default().embed(embed)).await?;

    Ok(())
}

async fn resolve_or_create_channel(
    http: &serenity::Http,
    guild_id: serenity::GuildId,
    provided: Option<u64>,
    fallback_name: &str,
) -> anyhow::Result<serenity::ChannelId> {
    if let Some(id) = provided {
        return Ok(serenity::ChannelId::new(id));
    }

    let channels = guild_id.channels(http).await?;
    if let Some(channel) = channels
        .values()
        .find(|c| c.kind == serenity::ChannelType::Text && c.name == fallback_name)
    {
        return Ok(channel.id);
    }

    let created = guild_id
        .create_channel(
            http,
            serenity::CreateChannel::new(fallback_name).kind(serenity::ChannelType::Text),
        )
        .await?;

    Ok(created.id)
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

fn secs_to_label(secs: u64) -> String {
    if secs.is_multiple_of(86_400) {
        format!("{}d", secs / 86_400)
    } else {
        format!("{}h", secs / 3_600)
    }
}
