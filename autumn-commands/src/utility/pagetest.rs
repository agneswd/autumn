use autumn_core::{Context, Error};
use autumn_utils::pagination::paginate_embed_pages;

use crate::CommandMeta;

pub const META: CommandMeta = CommandMeta {
    name: "pagetest",
    desc: "Test embed pagination behavior.",
    category: "utility",
    usage: "!pagetest [page]",
};

const ITEMS_PER_PAGE: usize = 5;

#[poise::command(prefix_command, slash_command, category = "Utility")]
pub async fn pagetest(
    ctx: Context<'_>,
    #[description = "Starting page"] page: Option<usize>,
) -> Result<(), Error> {
    let items = build_test_items();
    let total = total_pages(items.len(), ITEMS_PER_PAGE);
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
            let (start, end) = page_window(items.len(), ITEMS_PER_PAGE, current_page);
            items[start..end]
                .iter()
                .map(|item| format!("• {}", item))
                .collect::<Vec<_>>()
                .join("\n")
        })
        .collect::<Vec<_>>();
    paginate_embed_pages(ctx, "Pagination Test", &pages, requested_page).await?;
    Ok(())
}

fn build_test_items() -> Vec<String> {
    (1..=24)
        .map(|index| format!("Sample pagination item #{index}"))
        .collect()
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
