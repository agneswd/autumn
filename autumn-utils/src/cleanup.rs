use poise::serenity_prelude as serenity;
use tokio::time::{Duration, sleep};
use tracing::error;

use crate::time::now_unix_secs;

const BULK_DELETE_MAX_AGE_SECS: u64 = 14 * 24 * 60 * 60;
const BULK_DELETE_SAFETY_BUFFER_SECS: u64 = 60 * 60;
const HISTORY_PAGE_DELAY_MS: u64 = 1100;

pub async fn purge_user_globally(
    http: &serenity::Http,
    guild_id: serenity::GuildId,
    target_user_id: serenity::UserId,
    cutoff_secs: Option<u64>,
) -> anyhow::Result<u64> {
    let channels = guild_id.channels(http).await?;
    let mut deleted_count = 0_u64;
    let bulk_delete_cutoff = now_unix_secs()
        .saturating_sub(BULK_DELETE_MAX_AGE_SECS.saturating_sub(BULK_DELETE_SAFETY_BUFFER_SECS))
        as i64;

    for channel in channels.values() {
        if !matches!(
            channel.kind,
            serenity::ChannelType::Text
                | serenity::ChannelType::News
                | serenity::ChannelType::PublicThread
                | serenity::ChannelType::PrivateThread
                | serenity::ChannelType::NewsThread
        ) {
            continue;
        }

        let channel_id = channel.id;
        let mut before: Option<serenity::MessageId> = None;

        loop {
            let get_messages = match before {
                Some(before_id) => serenity::GetMessages::new().before(before_id).limit(100),
                None => serenity::GetMessages::new().limit(100),
            };

            let messages = match channel_id.messages(http, get_messages).await {
                Ok(messages) => messages,
                Err(_) => break,
            };

            if messages.is_empty() {
                break;
            }

            before = messages.last().map(|message| message.id);

            let should_break_for_cutoff = cutoff_secs.is_some_and(|cutoff| {
                messages
                    .last()
                    .map(|last| last.timestamp.unix_timestamp() < cutoff as i64)
                    .unwrap_or(false)
            });

            let mut bulk_candidate_ids: Vec<serenity::MessageId> = Vec::new();
            let mut single_delete_ids: Vec<serenity::MessageId> = Vec::new();

            for message in messages {
                if message.author.id != target_user_id {
                    continue;
                }

                if let Some(cutoff) = cutoff_secs
                    && message.timestamp.unix_timestamp() < cutoff as i64
                {
                    continue;
                }

                if message.timestamp.unix_timestamp() >= bulk_delete_cutoff {
                    bulk_candidate_ids.push(message.id);
                } else {
                    single_delete_ids.push(message.id);
                }
            }

            if !bulk_candidate_ids.is_empty() {
                for chunk in bulk_candidate_ids.chunks(100) {
                    if chunk.len() < 2 {
                        single_delete_ids.extend_from_slice(chunk);
                        continue;
                    }

                    match channel_id.delete_messages(http, chunk.to_vec()).await {
                        Ok(_) => {
                            deleted_count = deleted_count.saturating_add(chunk.len() as u64);
                        }
                        Err(source) => {
                            error!(
                                ?source,
                                channel_id = channel_id.get(),
                                count = chunk.len(),
                                "bulk delete failed, falling back to single delete"
                            );
                            single_delete_ids.extend_from_slice(chunk);
                        }
                    }
                }
            }

            for message_id in single_delete_ids {
                if channel_id.delete_message(http, message_id).await.is_ok() {
                    deleted_count = deleted_count.saturating_add(1);
                }
            }

            if should_break_for_cutoff {
                break;
            }

            sleep(Duration::from_millis(HISTORY_PAGE_DELAY_MS)).await;
        }
    }

    Ok(deleted_count)
}
