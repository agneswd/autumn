use std::time::Duration;

use poise::serenity_prelude as serenity;

pub enum ConfirmationResult {
    Confirmed(serenity::ComponentInteraction),
    Declined(serenity::ComponentInteraction),
    TimedOut(serenity::Message),
}

pub async fn resolve_confirmation_result<U, E>(
    ctx: poise::Context<'_, U, E>,
    confirmation: ConfirmationResult,
    timed_out_text: &str,
    declined_text: &str,
    processing_text: &str,
) -> Result<Option<serenity::ComponentInteraction>, serenity::Error>
where
    U: Send + Sync,
    E: Send + Sync,
{
    match confirmation {
        ConfirmationResult::TimedOut(message) => {
            message
                .channel_id
                .edit_message(
                    ctx.http(),
                    message.id,
                    serenity::EditMessage::new()
                        .content(timed_out_text)
                        .embeds(vec![])
                        .components(vec![]),
                )
                .await?;
            Ok(None)
        }
        ConfirmationResult::Declined(interaction) => {
            interaction
                .create_response(
                    ctx.http(),
                    serenity::CreateInteractionResponse::UpdateMessage(
                        serenity::CreateInteractionResponseMessage::new()
                            .content(declined_text)
                            .embeds(vec![])
                            .components(vec![]),
                    ),
                )
                .await?;
            Ok(None)
        }
        ConfirmationResult::Confirmed(interaction) => {
            interaction
                .create_response(
                    ctx.http(),
                    serenity::CreateInteractionResponse::UpdateMessage(
                        serenity::CreateInteractionResponseMessage::new()
                            .content(processing_text)
                            .embeds(vec![])
                            .components(vec![]),
                    ),
                )
                .await?;
            Ok(Some(interaction))
        }
    }
}

pub async fn prompt_confirm_decline<U, E>(
    ctx: poise::Context<'_, U, E>,
    content: impl Into<String>,
    embed: serenity::CreateEmbed,
    timeout: Duration,
) -> Result<ConfirmationResult, serenity::Error>
where
    U: Send + Sync,
    E: Send + Sync,
{
    let ctx_id = ctx.id();
    let confirm_id = format!("{}_confirm", ctx_id);
    let decline_id = format!("{}_decline", ctx_id);

    let reply = ctx
        .send(
            poise::CreateReply::default()
                .content(content)
                .embed(embed)
                .components(vec![serenity::CreateActionRow::Buttons(vec![
                    serenity::CreateButton::new(&confirm_id)
                        .label("Confirm")
                        .style(serenity::ButtonStyle::Danger),
                    serenity::CreateButton::new(&decline_id)
                        .label("Decline")
                        .style(serenity::ButtonStyle::Secondary),
                ])]),
        )
        .await?;

    let message = reply.message().await?.into_owned();
    let interaction = message
        .await_component_interaction(ctx)
        .author_id(ctx.author().id)
        .timeout(timeout)
        .await;

    let Some(interaction) = interaction else {
        return Ok(ConfirmationResult::TimedOut(message));
    };

    if interaction.data.custom_id == decline_id {
        return Ok(ConfirmationResult::Declined(interaction));
    }

    Ok(ConfirmationResult::Confirmed(interaction))
}
