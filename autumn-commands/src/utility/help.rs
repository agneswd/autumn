use crate::utility::embeds::{
    grouped_help_description, no_commands_message, page_out_of_range_message,
    unknown_category_message,
};
use crate::{COMMANDS, CommandMeta};
use autumn_core::{Context, Error};
use autumn_utils::pagination::paginate_embed_pages;

pub const META: CommandMeta = CommandMeta {
    name: "help",
    desc: "Lists out all available commands.",
    category: "utility",
    usage: "!help [page|category]",
};

const HELP_COMMANDS_PER_PAGE: usize = 20;

#[poise::command(prefix_command, slash_command, category = "Utility")]
pub async fn help(
    ctx: Context<'_>,
    #[description = "Page number or category"] query: Option<String>,
) -> Result<(), Error> {
    let query = query.as_deref();
    let parsed_page = query.and_then(|raw| raw.parse::<usize>().ok().filter(|page| *page >= 1));
    let category = match (query, parsed_page) {
        (Some(raw), None) => Some(raw),
        _ => None,
    };

    let mut categories: Vec<&str> = COMMANDS.iter().map(|c| c.category).collect();
    categories.sort_unstable();
    categories.dedup();

    if let Some(wanted_category) = category
        && !categories.contains(&wanted_category)
    {
        ctx.say(unknown_category_message(wanted_category, &categories))
            .await?;
        return Ok(());
    }

    let commands = sorted_commands(category);
    if commands.is_empty() {
        ctx.say(no_commands_message(category)).await?;
        return Ok(());
    }

    let requested_page = parsed_page.unwrap_or(1);
    let total = total_pages(commands.len(), HELP_COMMANDS_PER_PAGE);

    if requested_page > total {
        ctx.say(page_out_of_range_message(requested_page, total)).await?;
        return Ok(());
    }

    let pages = (1..=total)
        .map(|page| {
            let (start, end) = page_window(commands.len(), HELP_COMMANDS_PER_PAGE, page);
            grouped_help_description(&commands[start..end])
        })
        .collect::<Vec<_>>();
    paginate_embed_pages(ctx, "Available Commands", &pages, requested_page).await?;
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

fn sorted_commands(category: Option<&str>) -> Vec<&'static CommandMeta> {
    let mut filtered: Vec<&'static CommandMeta> = COMMANDS
        .iter()
        .filter(|cmd| match category {
            Some(wanted) => cmd.category == wanted,
            None => true,
        })
        .collect();

    filtered.sort_unstable_by(|left, right| {
        left.category
            .cmp(right.category)
            .then_with(|| left.name.cmp(right.name))
    });

    filtered
}
