use poise::serenity_prelude as serenity;

use autumn_utils::embed::DEFAULT_EMBED_COLOR;

#[derive(Clone, Debug)]
pub struct TargetProfile {
    pub display_name: String,
    pub avatar_url: Option<String>,
}

pub fn target_profile_from_user(user: &serenity::User) -> TargetProfile {
    TargetProfile {
        display_name: user
            .global_name
            .clone()
            .unwrap_or_else(|| user.name.clone()),
        avatar_url: Some(user.face()),
    }
}

pub async fn fetch_target_profile(
    http: &serenity::Http,
    user_id: serenity::UserId,
) -> TargetProfile {
    match http.get_user(user_id).await {
        Ok(user) => target_profile_from_user(&user),
        Err(_) => TargetProfile {
            display_name: format!("User {}", user_id.get()),
            avatar_url: None,
        },
    }
}

pub fn moderation_action_embed(
    target_profile: &TargetProfile,
    target_user_id: serenity::UserId,
    action_past_tense: &str,
    reason: Option<&str>,
    duration: Option<&str>,
) -> serenity::CreateEmbed {
    let reason = reason
        .unwrap_or("No reason provided")
        .replace('@', "@\u{200B}");

    let description = match duration {
        Some(duration) => format!(
            "**Target :** <@{}>\n**Reason :** {}\n**Duration :** {}",
            target_user_id.get(),
            reason,
            duration
        ),
        None => format!(
            "**Target :** <@{}>\n**Reason :** {}",
            target_user_id.get(),
            reason
        ),
    };

    let mut embed = serenity::CreateEmbed::new()
        .color(DEFAULT_EMBED_COLOR)
        .description(description);

    if let Some(url) = target_profile.avatar_url.as_deref() {
        embed = embed.author(
            serenity::CreateEmbedAuthor::new(format!(
                "{} has been {}",
                target_profile.display_name, action_past_tense
            ))
            .icon_url(url),
        );
    } else {
        embed = embed.title(format!(
            "{} has been {}",
            target_profile.display_name, action_past_tense
        ));
    }

    embed
}

pub fn moderation_target_dm_embed(
    guild_name: &str,
    action_past_tense: &str,
    reason: Option<&str>,
    duration: Option<&str>,
) -> serenity::CreateEmbed {
    let mut details = Vec::new();

    if let Some(reason) = reason {
        let clean_reason = reason.replace('@', "@\u{200B}");
        details.push(format!("**Reason :** {}", clean_reason));
    }

    if let Some(duration) = duration {
        details.push(format!("**Duration :** {}", duration));
    }

    let description = if details.is_empty() {
        "No additional details were provided.".to_owned()
    } else {
        details.join("\n")
    };

    serenity::CreateEmbed::new()
        .color(DEFAULT_EMBED_COLOR)
        .title(format!(
            "You have been {} in {}",
            action_past_tense, guild_name
        ))
        .description(description)
}

pub async fn send_moderation_target_dm(
    http: &serenity::Http,
    target_user: &serenity::User,
    guild_name: &str,
    action_past_tense: &str,
    reason: Option<&str>,
    duration: Option<&str>,
) -> Result<(), serenity::Error> {
    let dm_channel = target_user.create_dm_channel(http).await?;
    dm_channel
        .send_message(
            http,
            serenity::CreateMessage::new().embed(moderation_target_dm_embed(
                guild_name,
                action_past_tense,
                reason,
                duration,
            )),
        )
        .await?;

    Ok(())
}

pub async fn send_moderation_target_dm_for_guild(
    http: &serenity::Http,
    target_user: &serenity::User,
    guild_id: serenity::GuildId,
    action_past_tense: &str,
    reason: Option<&str>,
    duration: Option<&str>,
) -> Result<(), serenity::Error> {
    let guild_name = match guild_id.to_partial_guild(http).await {
        Ok(guild) => guild.name,
        Err(_) => format!("Server {}", guild_id.get()),
    };

    send_moderation_target_dm(
        http,
        target_user,
        &guild_name,
        action_past_tense,
        reason,
        duration,
    )
    .await
}

pub fn usage_message(usage: &str) -> String {
    format!("Usage: `{usage}`")
}

pub fn guild_only_message() -> &'static str {
    "This command only works in servers."
}

pub fn moderation_self_action_message(action: &str) -> String {
    format!("You can't {action} yourself.")
}

pub fn moderation_bot_target_message() -> &'static str {
    "You can't use moderation actions on bots or application accounts."
}

pub fn is_missing_permissions_error(source: &serenity::Error) -> bool {
    matches!(
        source,
        serenity::Error::Http(serenity::HttpError::UnsuccessfulRequest(response))
            if response.status_code.as_u16() == 403 || response.error.code == 50013
    )
}

pub fn warnings_window_label_days(days: u64) -> String {
    format!("last {} day(s)", days)
}
