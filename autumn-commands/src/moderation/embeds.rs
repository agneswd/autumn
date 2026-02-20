use poise::serenity_prelude as serenity;

use autumn_utils::embed::DEFAULT_EMBED_COLOR;

#[derive(Clone, Debug)]
pub struct TargetProfile {
    pub display_name: String,
    pub avatar_url: Option<String>,
}

pub fn target_profile_from_user(user: &serenity::User) -> TargetProfile {
    TargetProfile {
        display_name: user.global_name.clone().unwrap_or_else(|| user.name.clone()),
        avatar_url: Some(user.face()),
    }
}

pub async fn fetch_target_profile(http: &serenity::Http, user_id: serenity::UserId) -> TargetProfile {
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
        None => format!("**Target :** <@{}>\n**Reason :** {}", target_user_id.get(), reason),
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

pub fn usage_message(usage: &str) -> String {
    format!("Usage: `{usage}`")
}

pub fn guild_only_message() -> &'static str {
    "This command only works in servers."
}

pub fn moderation_self_action_message(action: &str) -> String {
    format!("You can't {action} yourself.")
}

pub fn warnings_window_label_days(days: u64) -> String {
    format!("last {} day(s)", days)
}
