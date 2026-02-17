use crate::CommandMeta;
use autumn_core::{Context, Error};
use autumn_utils::pagination::paginate_embed_pages;
use autumn_utils::permissions::{has_user_permission, permission_names, resolve_user_permissions};

pub const META: CommandMeta = CommandMeta {
    name: "permissions",
    desc: "Display your server permissions.",
    category: "moderation",
    usage: "!permissions [page]",
};

const PERMISSIONS_PER_PAGE: usize = 10;

#[poise::command(prefix_command, slash_command, category = "Moderation")]
pub async fn permissions(
    ctx: Context<'_>,
    #[description = "Starting page"] page: Option<usize>,
) -> Result<(), Error> {
    let Some(guild_id) = ctx.guild_id() else {
        ctx.say("This command only works in servers.").await?;
        return Ok(());
    };

    if !has_user_permission(
        ctx.http(),
        guild_id,
        ctx.author().id,
        poise::serenity_prelude::Permissions::MANAGE_MESSAGES,
    )
    .await?
    {
        ctx.say("You are not permitted to use this command.").await?;
        return Ok(());
    }

    let perms = resolve_user_permissions(ctx.http(), guild_id, ctx.author().id).await?;
    let names = permission_names(perms);

    if names.is_empty() {
        ctx.say("You have no permissions set.").await?;
        return Ok(());
    }

    let total = total_pages(names.len(), PERMISSIONS_PER_PAGE);
    let requested_page = page.unwrap_or(1);

    if requested_page == 0 || requested_page > total {
        ctx.say(format!(
            "Page {} does not exist. Available pages: 1-{}.",
            requested_page, total
        ))
        .await?;
        return Ok(());
    }

    let pages = (1..=total)
        .map(|current_page| {
            let (start, end) = page_window(names.len(), PERMISSIONS_PER_PAGE, current_page);
            names[start..end]
                .iter()
                .enumerate()
                .map(|(index, item)| format!("{}. {}", start + index + 1, item))
                .collect::<Vec<_>>()
                .join("\n")
        })
        .collect::<Vec<_>>();
    paginate_embed_pages(ctx, "Your Permissions", &pages, requested_page).await?;
    Ok(())
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
