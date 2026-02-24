use tracing::error;

use poise::serenity_prelude as serenity;

use autumn_core::Context;
use autumn_database::impls::cases::{NewCase, create_case};
use autumn_database::impls::modlog_config::get_modlog_channel_id;
use autumn_database::model::cases::CaseSummary;
use autumn_utils::embed::DEFAULT_EMBED_COLOR;
use autumn_utils::formatting::{action_display_name, format_case_label, format_compact_duration};

/// Orchestrator: create moderation case and publish to optional modlog channel.
pub async fn create_case_and_publish(
    ctx: &Context<'_>,
    guild_id: serenity::GuildId,
    new_case: NewCase<'_>,
) -> Option<String> {
    let case = match create_case(&ctx.data().db, new_case).await {
        Ok(case) => case,
        Err(source) => {
            error!(?source, "failed to create moderation case");
            return None;
        }
    };

    if let Err(source) = publish_case_to_modlog_channel(ctx, guild_id, &case).await {
        error!(
            ?source,
            "failed to publish case to configured modlog channel"
        );
    }

    Some(format_case_label(&case.case_code, case.action_case_number))
}

async fn publish_case_to_modlog_channel(
    ctx: &Context<'_>,
    guild_id: serenity::GuildId,
    case: &CaseSummary,
) -> Result<(), serenity::Error> {
    let channel_id = match get_modlog_channel_id(&ctx.data().db, guild_id.get()).await {
        Ok(channel_id) => channel_id,
        Err(source) => {
            error!(?source, "failed to read modlog channel config");
            None
        }
    };

    let Some(channel_id) = channel_id else {
        return Ok(());
    };

    let action_name = action_display_name(&case.action);
    let mut fields = Vec::new();
    fields.push(format!("**Action :** {}", action_name));

    if let Some(target_user_id) = case.target_user_id {
        fields.push(format!("**Target :** <@{}>", target_user_id));
    }

    fields.push(format!(
        "**Reason :** {}",
        case.reason.replace('@', "@\u{200B}")
    ));

    if let Some(duration_seconds) = case.duration_seconds {
        fields.push(format!(
            "**Duration :** {}",
            format_compact_duration(duration_seconds)
        ));
    }

    fields.push(format!("**Moderator :** <@{}>", case.moderator_user_id));

    fields.push(format!(
        "**When :** <t:{}:R> • <t:{}:f>",
        case.created_at, case.created_at,
    ));

    let description = fields.join("\n");

    let embed = serenity::CreateEmbed::new()
        .color(DEFAULT_EMBED_COLOR)
        .title(format!(
            "#{}",
            format_case_label(&case.case_code, case.action_case_number)
        ))
        .description(description);

    serenity::ChannelId::new(channel_id)
        .send_message(ctx.http(), serenity::CreateMessage::new().embed(embed))
        .await?;

    Ok(())
}
