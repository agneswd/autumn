use poise::serenity_prelude as serenity;
use std::time::Duration;

use crate::CommandMeta;
use crate::moderation::embeds::{guild_only_message, permission_denied_message};
use autumn_core::{Context, Error};
use autumn_database::impls::notes::{add_user_note, clear_user_notes, list_user_notes};
use autumn_utils::confirmation::{prompt_confirm_decline, resolve_confirmation_result};
use autumn_utils::pagination::paginate_embed_pages;
use autumn_utils::permissions::has_user_permission;

pub const META: CommandMeta = CommandMeta {
    name: "notes",
    desc: "Add or view a moderator note for a user.",
    category: "moderation",
    usage: "!notes <user> [note|clear]",
};

const NOTES_PER_PAGE: usize = 5;
const NOTES_CLEAR_CONFIRM_TIMEOUT_SECS: u64 = 30;

#[poise::command(prefix_command, slash_command, category = "Moderation")]
pub async fn notes(
    ctx: Context<'_>,
    #[description = "Target user"] user: Option<serenity::User>,
    #[description = "Optional note to add"] #[rest] note: Option<String>,
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

    let Some(user) = user else {
        ctx.say(format!("Usage: `{}`", META.usage)).await?;
        return Ok(());
    };

    if let Some(note) = note.map(|entry| entry.trim().to_owned()) {
        if note.eq_ignore_ascii_case("clear") {
            let confirm_embed = serenity::CreateEmbed::new().description(format!(
                "This will clear all notes for <@{}>.",
                user.id.get()
            ));

            let confirmation = prompt_confirm_decline(
                ctx,
                "Confirm note clear",
                confirm_embed,
                Duration::from_secs(NOTES_CLEAR_CONFIRM_TIMEOUT_SECS),
            )
            .await?;

            let Some(interaction) = resolve_confirmation_result(
                ctx,
                confirmation,
                "Timed out",
                "Note clear cancelled.",
                "Clearing notes...",
            )
            .await?
            else {
                return Ok(());
            };

            let removed = clear_user_notes(&ctx.data().db, guild_id.get(), user.id.get()).await?;
            interaction
                .edit_response(
                    ctx.http(),
                    serenity::EditInteractionResponse::new()
                        .content(format!("Cleared {} note(s) for <@{}>.", removed, user.id.get()))
                        .embeds(vec![]),
                )
                .await?;
            return Ok(());
        }

        if note.is_empty() {
            ctx.say("Note content cannot be empty.").await?;
            return Ok(());
        }

        if note.len() > 2000 {
            ctx.say("Note content is too long (max 2000 characters).").await?;
            return Ok(());
        }

        let saved_note = add_user_note(
            &ctx.data().db,
            guild_id.get(),
            user.id.get(),
            ctx.author().id.get(),
            &note,
        )
        .await?;

        ctx.say(format!("Added note #{} for <@{}>.", saved_note.id, saved_note.target_user_id))
            .await?;
        return Ok(());
    }

    let notes = list_user_notes(&ctx.data().db, guild_id.get(), user.id.get()).await?;
    if notes.is_empty() {
        ctx.say(format!("No notes found for <@{}>.", user.id.get())).await?;
        return Ok(());
    }

    let total = notes.len();
    let total_pages = total.div_ceil(NOTES_PER_PAGE);
    let mut pages = Vec::with_capacity(total_pages);

    for page in 0..total_pages {
        let start = page * NOTES_PER_PAGE;
        let end = (start + NOTES_PER_PAGE).min(total);

        let mut body = String::new();
        for note in &notes[start..end] {
            body.push_str(&format!(
                "#{} • by <@{}> • <t:{}:R>\n{}\n\n",
                note.id,
                note.author_user_id,
                note.created_at,
                note.content.replace('@', "@\u{200B}"),
            ));
        }

        pages.push(body.trim_end().to_owned());
    }

    paginate_embed_pages(ctx, &format!("Notes for {}", user.name), &pages, 1).await?;
    Ok(())
}
