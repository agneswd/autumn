use poise::serenity_prelude as serenity;

use crate::CommandMeta;
use crate::moderation::embeds::{guild_only_message, usage_message};
use autumn_core::{Context, Error};
use autumn_database::impls::cases::{CaseFilters, list_recent_cases};
use autumn_utils::formatting::{action_display_name, format_case_label, format_compact_duration};
use autumn_utils::pagination::paginate_embed_pages;
use autumn_utils::permissions::has_user_permission;

pub const META: CommandMeta = CommandMeta {
    name: "modlogs",
    desc: "View recent moderation actions.",
    category: "moderation",
    usage: "!modlogs [target_user] [moderator] [action]",
};

const CASES_PER_PAGE: usize = 5;

#[poise::command(prefix_command, slash_command, category = "Moderation")]
pub async fn modlogs(
    ctx: Context<'_>,
    #[description = "Filter by target user"] target_user: Option<serenity::User>,
    #[description = "Filter by moderator"] moderator: Option<serenity::User>,
    #[description = "Filter by action (ban, warn, etc.)"] action: Option<String>,
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
        return Ok(());
    }

    if action.as_deref().is_some_and(|value| value.trim().is_empty()) {
        ctx.say(usage_message(META.usage)).await?;
        return Ok(());
    }

    let rows = list_recent_cases(
        &ctx.data().db,
        guild_id.get(),
        CaseFilters {
            target_user_id: target_user.as_ref().map(|user| user.id.get()),
            moderator_user_id: moderator.as_ref().map(|user| user.id.get()),
            action: action.as_deref().map(str::trim).filter(|value| !value.is_empty()),
            limit: 200,
        },
    )
    .await?;

    if rows.is_empty() {
        ctx.say("No matching moderation cases found.").await?;
        return Ok(());
    }

    let total = rows.len();
    let total_pages = total.div_ceil(CASES_PER_PAGE);
    let mut pages = Vec::with_capacity(total_pages);

    for page in 0..total_pages {
        let start = page * CASES_PER_PAGE;
        let end = (start + CASES_PER_PAGE).min(total);

        let mut body = String::new();
        body.push_str(&format!("Total cases: **{}**\n\n", total));
        for case in &rows[start..end] {
            let target_display = case
                .target_user_id
                .map(|id| format!("<@{}>", id))
                .unwrap_or_else(|| "N/A".to_owned());
            let action_name = action_display_name(&case.action);
            let duration_line = case
                .duration_seconds
                .map(|seconds| format!("\n**Duration :** {}", format_compact_duration(seconds)))
                .unwrap_or_default();

            body.push_str(&format!(
                "#{}\n**Action :** {}\n**Target :** {}\n**Moderator :** <@{}>\n**Reason :** {}{}\n**When :** <t:{}:R> • <t:{}:f>\n\n",
                format_case_label(&case.case_code, case.action_case_number),
                action_name,
                target_display,
                case.moderator_user_id,
                case.reason.replace('@', "@\u{200B}"),
                duration_line,
                case.created_at,
                case.created_at,
            ));
        }

        pages.push(body.trim_end().to_owned());
    }

    paginate_embed_pages(ctx, "Moderation Logs", &pages, 1).await?;
    Ok(())
}
