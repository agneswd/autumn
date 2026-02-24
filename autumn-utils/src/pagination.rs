use std::time::Duration;

use poise::serenity_prelude as serenity;

use crate::embed::DEFAULT_EMBED_COLOR;

pub const PAGINATION_TIMEOUT_SECS: u64 = 60 * 3;

fn build_page_embed(
    title: &str,
    description: &str,
    page: usize,
    total_pages: usize,
    author_icon_url: Option<&str>,
    show_footer: bool,
) -> serenity::CreateEmbed {
    let mut embed = serenity::CreateEmbed::new()
        .color(DEFAULT_EMBED_COLOR)
        .description(description.to_owned());

    if show_footer {
        embed = embed.footer(serenity::CreateEmbedFooter::new(format!(
            "Page {}/{}",
            page.max(1),
            total_pages.max(1)
        )));
    }

    if let Some(url) = author_icon_url {
        embed = embed.author(serenity::CreateEmbedAuthor::new(title).icon_url(url));
    } else {
        embed = embed.title(title.to_owned());
    }

    embed
}

fn pagination_components(
    prev_id: &str,
    jump_id: &str,
    next_id: &str,
    current_page: usize,
    total_pages: usize,
) -> Vec<serenity::CreateActionRow> {
    let is_first_page = current_page == 0;
    let is_last_page = current_page + 1 >= total_pages;

    vec![serenity::CreateActionRow::Buttons(vec![
        serenity::CreateButton::new(prev_id)
            .label("Prev")
            .disabled(is_first_page)
            .style(serenity::ButtonStyle::Secondary),
        serenity::CreateButton::new(jump_id)
            .label("Jump")
            .style(serenity::ButtonStyle::Secondary),
        serenity::CreateButton::new(next_id)
            .label("Next")
            .disabled(is_last_page)
            .style(serenity::ButtonStyle::Secondary),
    ])]
}

pub async fn paginate_embed_pages<U, E>(
    ctx: poise::Context<'_, U, E>,
    title: &str,
    pages: &[String],
    start_page: usize,
) -> Result<(), serenity::Error>
where
    U: Send + Sync,
    E: Send + Sync,
{
    paginate_embed_pages_with_icon(ctx, title, pages, start_page, None).await
}

pub async fn paginate_embed_pages_with_icon<U, E>(
    ctx: poise::Context<'_, U, E>,
    title: &str,
    pages: &[String],
    start_page: usize,
    author_icon_url: Option<&str>,
) -> Result<(), serenity::Error>
where
    U: Send + Sync,
    E: Send + Sync,
{
    if pages.is_empty() {
        return Ok(());
    }

    let total_pages = pages.len();
    let mut current_page = start_page.clamp(1, total_pages) - 1;

    if total_pages <= 1 {
        ctx.send(poise::CreateReply::default().embed(build_page_embed(
            title,
            &pages[current_page],
            current_page + 1,
            total_pages,
            author_icon_url,
            false,
        )))
        .await?;

        return Ok(());
    }

    let ctx_id = ctx.id();
    let prev_button_id = format!("{}_prev", ctx_id);
    let jump_button_id = format!("{}_jump", ctx_id);
    let next_button_id = format!("{}_next", ctx_id);
    let jump_modal_id = format!("{}_jump_modal", ctx_id);
    let jump_input_id = format!("{}_jump_input", ctx_id);

    let reply = ctx
        .send(
            poise::CreateReply::default()
                .embed(build_page_embed(
                    title,
                    &pages[current_page],
                    current_page + 1,
                    total_pages,
                    author_icon_url,
                    true,
                ))
                .components(pagination_components(
                    &prev_button_id,
                    &jump_button_id,
                    &next_button_id,
                    current_page,
                    total_pages,
                )),
        )
        .await?;

    let message = reply.message().await?;
    let message_id = message.id;
    let channel_id = message.channel_id;

    while let Some(press) = serenity::collector::ComponentInteractionCollector::new(ctx)
        .filter({
            let prefix = format!("{}", ctx_id);
            let author_id = ctx.author().id;
            move |interaction| {
                interaction.data.custom_id.starts_with(&prefix)
                    && interaction.user.id == author_id
                    && interaction.message.id == message_id
            }
        })
        .timeout(Duration::from_secs(PAGINATION_TIMEOUT_SECS))
        .await
    {
        if press.data.custom_id == next_button_id {
            if current_page + 1 < total_pages {
                current_page += 1;
            }

            press
                .create_response(
                    ctx.http(),
                    serenity::CreateInteractionResponse::UpdateMessage(
                        serenity::CreateInteractionResponseMessage::new()
                            .embed(build_page_embed(
                                title,
                                &pages[current_page],
                                current_page + 1,
                                total_pages,
                                author_icon_url,
                                true,
                            ))
                            .components(pagination_components(
                                &prev_button_id,
                                &jump_button_id,
                                &next_button_id,
                                current_page,
                                total_pages,
                            )),
                    ),
                )
                .await?;
            continue;
        }

        if press.data.custom_id == prev_button_id {
            current_page = current_page.saturating_sub(1);

            press
                .create_response(
                    ctx.http(),
                    serenity::CreateInteractionResponse::UpdateMessage(
                        serenity::CreateInteractionResponseMessage::new()
                            .embed(build_page_embed(
                                title,
                                &pages[current_page],
                                current_page + 1,
                                total_pages,
                                author_icon_url,
                                true,
                            ))
                            .components(pagination_components(
                                &prev_button_id,
                                &jump_button_id,
                                &next_button_id,
                                current_page,
                                total_pages,
                            )),
                    ),
                )
                .await?;
            continue;
        }

        if press.data.custom_id != jump_button_id {
            continue;
        }

        press
            .create_response(
                ctx.http(),
                serenity::CreateInteractionResponse::Modal(
                    serenity::CreateModal::new(&jump_modal_id, "Jump to Page").components(vec![
                        serenity::CreateActionRow::InputText(
                            serenity::CreateInputText::new(
                                serenity::InputTextStyle::Short,
                                "Page Number",
                                &jump_input_id,
                            )
                            .placeholder(format!("1-{}", total_pages))
                            .required(true),
                        ),
                    ]),
                ),
            )
            .await?;

        let maybe_modal = serenity::collector::ModalInteractionCollector::new(ctx)
            .author_id(ctx.author().id)
            .channel_id(ctx.channel_id())
            .custom_ids(vec![jump_modal_id.clone()])
            .timeout(Duration::from_secs(PAGINATION_TIMEOUT_SECS))
            .await;

        if let Some(modal) = maybe_modal {
            modal
                .create_response(ctx.http(), serenity::CreateInteractionResponse::Acknowledge)
                .await?;

            let submitted_page = modal
                .data
                .components
                .iter()
                .flat_map(|row| row.components.iter())
                .find_map(|component| {
                    if let serenity::ActionRowComponent::InputText(input) = component
                        && input.custom_id == jump_input_id
                    {
                        return input.value.clone();
                    }

                    None
                });

            if let Some(submitted_page) = submitted_page
                && let Ok(target_page) = submitted_page.trim().parse::<usize>()
                && (1..=total_pages).contains(&target_page)
            {
                current_page = target_page - 1;

                channel_id
                    .edit_message(
                        ctx.http(),
                        message_id,
                        serenity::EditMessage::new()
                            .embed(build_page_embed(
                                title,
                                &pages[current_page],
                                current_page + 1,
                                total_pages,
                                author_icon_url,
                                true,
                            ))
                            .components(pagination_components(
                                &prev_button_id,
                                &jump_button_id,
                                &next_button_id,
                                current_page,
                                total_pages,
                            )),
                    )
                    .await?;
            }
        }
    }

    let _ = channel_id
        .edit_message(
            ctx.http(),
            message_id,
            serenity::EditMessage::new().embed(build_page_embed(
                title,
                &pages[current_page],
                current_page + 1,
                total_pages,
                author_icon_url,
                true,
            )),
        )
        .await;

    Ok(())
}
