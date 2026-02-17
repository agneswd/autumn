use poise::serenity_prelude as serenity;

/// Default embed color used across the bot UI.
pub const DEFAULT_EMBED_COLOR: u32 = 0x90_55_30;

/// Build a standard paginated embed with consistent styling.
pub fn build_paginated_embed(
    title: &str,
    description: impl Into<String>,
    page: usize,
    total_pages: usize,
) -> serenity::CreateEmbed {
    build_paginated_embed_with_footer_note(title, description, page, total_pages, None)
}

/// Build a standard paginated embed with an optional footer suffix.
pub fn build_paginated_embed_with_footer_note(
    title: &str,
    description: impl Into<String>,
    page: usize,
    total_pages: usize,
    footer_note: Option<&str>,
) -> serenity::CreateEmbed {
    let page = page.max(1);
    let total_pages = total_pages.max(1);

    let footer_text = if total_pages > 1 {
        match footer_note {
            Some(note) if !note.is_empty() => format!("Page {}/{} • {}", page, total_pages, note),
            _ => format!("Page {}/{}", page, total_pages),
        }
    } else {
        match footer_note {
            Some(note) if !note.is_empty() => note.to_owned(),
            _ => String::new(),
        }
    };

    let builder = serenity::CreateEmbed::new()
        .title(title.to_owned())
        .color(DEFAULT_EMBED_COLOR)
        .description(description);

    if footer_text.is_empty() {
        builder
    } else {
        builder.footer(serenity::CreateEmbedFooter::new(footer_text))
    }
}
