use poise::serenity_prelude as serenity;

use crate::CommandMeta;
use crate::moderation::embeds::{
    fetch_target_profile, guild_only_message, usage_message,
    warnings_window_label_days,
};
use autumn_core::{Context, Error};
use autumn_database::impls::warnings::{now_unix_secs, warnings_since};
use autumn_utils::pagination::paginate_embed_pages_with_icon;
use autumn_utils::permissions::has_user_permission;

pub const META: CommandMeta = CommandMeta {
    name: "warnings",
    desc: "Show warning history for a user in a time window.",
    category: "moderation",
    usage: "!warnings <user> [days|all]",
};

const DEFAULT_DAYS: u64 = 30;
const WARNINGS_PER_PAGE: usize = 5;

enum WarningWindow {
    Days(u64),
    All,
}

#[poise::command(prefix_command, slash_command, category = "Moderation")]
pub async fn warnings(
    ctx: Context<'_>,
    #[description = "The user to check"] user: Option<serenity::User>,
    #[description = "Days or 'all'"] window: Option<String>,
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

    let Some(user) = user else {
        ctx.say(usage_message(META.usage)).await?;
        return Ok(());
    };

    let window = parse_window(window.as_deref());
    let (since, window_label) = match window {
        WarningWindow::Days(days) => (
            now_unix_secs().saturating_sub(days.saturating_mul(86_400)),
            warnings_window_label_days(days),
        ),
        WarningWindow::All => (0, "all time".to_owned()),
    };

    let entries = warnings_since(&ctx.data().db, guild_id.get(), user.id.get(), since).await?;
    let target_profile = fetch_target_profile(ctx.http(), user.id).await;

    if entries.is_empty() {
        let page = format!("Total warnings in {}: **0**\n\nNo warnings in this period.", window_label);
        paginate_embed_pages_with_icon(
            ctx,
            &format!("Warnings for {}", target_profile.display_name),
            &[page],
            1,
            target_profile.avatar_url.as_deref(),
        )
        .await?;
        return Ok(());
    }

    let total = entries.len();
    let total_pages = total_pages(total, WARNINGS_PER_PAGE);
    let pages = (1..=total_pages)
        .map(|current_page| {
            let (start, end) = page_window(total, WARNINGS_PER_PAGE, current_page);
            let mut lines = String::new();

            lines.push_str(&format!(
                "Total warnings in {}: **{}**\n\n",
                window_label, total
            ));

            for display_index in start..end {
                let reverse_index = total - 1 - display_index;
                let entry = &entries[reverse_index];

                lines.push_str(&format!(
                    "#{idx} • by <@{mod_id}>\n**Reason :** {reason}\n**When :** <t:{ts}:R> • <t:{ts}:f>\n\n",
                    idx = reverse_index + 1,
                    mod_id = entry.moderator_id,
                    reason = entry.reason.replace('@', "@\u{200B}"),
                    ts = entry.warned_at,
                ));
            }

            lines.trim_end().to_owned()
        })
        .collect::<Vec<_>>();

    paginate_embed_pages_with_icon(
        ctx,
        &format!("Warnings for {}", target_profile.display_name),
        &pages,
        1,
        target_profile.avatar_url.as_deref(),
    )
    .await?;

    Ok(())
}

fn parse_window(value: Option<&str>) -> WarningWindow {
    let Some(raw) = value.and_then(|entry| entry.split_whitespace().next()) else {
        return WarningWindow::Days(DEFAULT_DAYS);
    };

    if raw.eq_ignore_ascii_case("all") {
        return WarningWindow::All;
    }

    let Some(days) = raw.parse::<u64>().ok().filter(|value| *value > 0) else {
        return WarningWindow::Days(DEFAULT_DAYS);
    };

    WarningWindow::Days(days)
}

fn total_pages(total_items: usize, per_page: usize) -> usize {
    let per_page = per_page.max(1);
    let pages = total_items.div_ceil(per_page);
    pages.max(1)
}

fn page_window(total_items: usize, per_page: usize, page: usize) -> (usize, usize) {
    let per_page = per_page.max(1);
    let page = page.max(1);
    let start = (page - 1).saturating_mul(per_page).min(total_items);
    let end = (start + per_page).min(total_items);
    (start, end)
}
