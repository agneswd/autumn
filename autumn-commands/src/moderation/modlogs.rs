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

    if action
        .as_deref()
        .is_some_and(|value| value.trim().is_empty())
    {
        ctx.say(usage_message(META.usage)).await?;
        return Ok(());
    }

    let rows = list_recent_cases(
        &ctx.data().db,
        guild_id.get(),
        CaseFilters {
            target_user_id: target_user.as_ref().map(|user| user.id.get()),
            moderator_user_id: moderator.as_ref().map(|user| user.id.get()),
            action: action
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty()),
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

            body.push_str(&format!(
                "#{}\n{}\n\n",
                format_case_label(&case.case_code, case.action_case_number),
                fields.join("\n"),
            ));
        }

        pages.push(body.trim_end().to_owned());
    }

    paginate_embed_pages(ctx, "Moderation Logs", &pages, 1).await?;
    Ok(())
}
