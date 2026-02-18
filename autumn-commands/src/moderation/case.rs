use poise::serenity_prelude as serenity;
use tracing::error;

use crate::CommandMeta;
use crate::moderation::embeds::{guild_only_message, permission_denied_message, usage_message};
use autumn_core::{Context, Error};
use autumn_database::impls::cases::{
    add_case_note, get_case_by_label, get_case_events, update_case_reason,
};
use autumn_utils::embed::DEFAULT_EMBED_COLOR;
use autumn_utils::formatting::{
    action_display_name, event_display_name, format_compact_duration, parse_case_label,
};
use autumn_utils::permissions::has_user_permission;

pub const META: CommandMeta = CommandMeta {
    name: "case",
    desc: "View or edit a moderation case.",
    category: "moderation",
    usage: "!case <case_id> [reason|note] [text]",
};

#[poise::command(prefix_command, slash_command, category = "Moderation")]
pub async fn case(
    ctx: Context<'_>,
    #[description = "Case id (e.g. W1, B3)"] case_id: Option<String>,
    #[description = "Optional action: reason or note"] action: Option<String>,
    #[description = "Text for the selected action"] #[rest] value: Option<String>,
) -> Result<(), Error> {
    let Some(guild_id) = ctx.guild_id() else {
        ctx.say(guild_only_message()).await?;
        return Ok(());
    };

    if !has_user_permission(
        ctx.http(),
        guild_id,
        ctx.author().id,
        serenity::Permissions::MANAGE_MESSAGES,
    )
    .await?
    {
        ctx.say(permission_denied_message()).await?;
        return Ok(());
    }

    let Some(case_id) = case_id else {
        ctx.say(usage_message(META.usage)).await?;
        return Ok(());
    };

    let Some((case_code, action_case_number)) = parse_case_label(&case_id) else {
        ctx.say("Case id must look like W1, B2, K3, etc.").await?;
        return Ok(());
    };

    if let Some(action) = action.as_deref().map(str::trim) {
        if action.eq_ignore_ascii_case("reason") {
            let Some(new_reason) = value.map(|entry| entry.trim().to_owned()) else {
                ctx.say("Usage: `!case <case_id> reason <new reason>`").await?;
                return Ok(());
            };

            if new_reason.is_empty() {
                ctx.say("Reason cannot be empty.").await?;
                return Ok(());
            }

            let updated = update_case_reason(
                &ctx.data().db,
                guild_id.get(),
                &case_code,
                action_case_number,
                ctx.author().id.get(),
                &new_reason,
            )
            .await;

            let updated = match updated {
                Ok(updated) => updated,
                Err(source) => {
                    error!(?source, "case reason update failed");
                    ctx.say("Failed to update case reason.").await?;
                    return Ok(());
                }
            };

            if updated.is_none() {
                ctx.say(format!("Case {}{} was not found.", case_code, action_case_number))
                    .await?;
                return Ok(());
            }

            ctx.say(format!("Updated reason for #{}{}.", case_code, action_case_number))
                .await?;
            return Ok(());
        }

        if action.eq_ignore_ascii_case("note") {
            let Some(note) = value.map(|entry| entry.trim().to_owned()) else {
                ctx.say("Usage: `!case <case_id> note <note text>`").await?;
                return Ok(());
            };

            if note.is_empty() {
                ctx.say("Note cannot be empty.").await?;
                return Ok(());
            }

            let added = add_case_note(
                &ctx.data().db,
                guild_id.get(),
                &case_code,
                action_case_number,
                ctx.author().id.get(),
                &note,
            )
            .await;

            let added = match added {
                Ok(added) => added,
                Err(source) => {
                    error!(?source, "case note add failed");
                    ctx.say("Failed to add case note.").await?;
                    return Ok(());
                }
            };

            if !added {
                ctx.say(format!("Case {}{} was not found.", case_code, action_case_number))
                    .await?;
                return Ok(());
            }

            ctx.say(format!("Added note to #{}{}.", case_code, action_case_number))
                .await?;
            return Ok(());
        }

        ctx.say("Supported actions: `reason`, `note`").await?;
        return Ok(());
    }

    let case = get_case_by_label(&ctx.data().db, guild_id.get(), &case_code, action_case_number).await;
    let Some(case) = (match case {
        Ok(case) => case,
        Err(source) => {
            error!(?source, "case load failed");
            ctx.say("Failed to load case.").await?;
            return Ok(());
        }
    }) else {
        ctx.say(format!("Case {}{} was not found.", case_code, action_case_number)).await?;
        return Ok(());
    };

    let events = match get_case_events(&ctx.data().db, guild_id.get(), &case_code, action_case_number).await {
        Ok(events) => events,
        Err(source) => {
            error!(?source, "case events load failed");
            ctx.say("Failed to load case events.").await?;
            return Ok(());
        }
    };
    let target_display = case
        .target_user_id
        .map(|id| format!("<@{}>", id))
        .unwrap_or_else(|| "N/A".to_owned());
    let mut description = format!(
        "**Action :** {}\n**Target :** {}\n**Moderator :** <@{}>\n**Reason :** {}\n**Created :** <t:{}:f>",
        action_display_name(&case.action),
        target_display,
        case.moderator_user_id,
        case.reason.replace('@', "@\u{200B}"),
        case.created_at,
    );

    if let Some(duration_seconds) = case.duration_seconds {
        description.push_str(&format!(
            "\n**Duration :** {}",
            format_compact_duration(duration_seconds)
        ));
    }

    if let Some(note) = events
        .iter()
        .rev()
        .find(|event| event.event_type == "note_added")
        .and_then(|event| event.note.as_deref())
    {
        description.push_str(&format!(
            "\n\n**Note :** {}",
            note.replace('@', "@\u{200B}")
        ));
    }

    if !events.is_empty() {
        description.push_str("\n\n**Event History :**\n");
        for event in events.iter().take(10) {
            let event_name = event_display_name(&event.event_type);
            description.push_str(&format!(
                "• {} by <@{}> • <t:{}:R>\n",
                event_name, event.actor_user_id, event.created_at
            ));
        }
    }

    let embed = serenity::CreateEmbed::new()
        .color(DEFAULT_EMBED_COLOR)
        .title(format!("#{}{}", case.case_code, case.action_case_number))
        .description(description.trim_end().to_owned());

    ctx.send(poise::CreateReply::default().embed(embed)).await?;
    Ok(())
}
