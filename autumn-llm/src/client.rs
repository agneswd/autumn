use std::env;

use anyhow::Context as _;
use autumn_database::{Database, impls::llm_chat::list_recent_llm_chat_messages};
use ollama_rs::{
    Ollama,
    generation::chat::{ChatMessage, request::ChatMessageRequest},
    models::ModelOptions,
};

#[derive(Clone, Debug)]
pub struct LlmService {
    client: Ollama,
    model: String,
}

impl LlmService {
    pub fn from_env_optional() -> anyhow::Result<Option<Self>> {
        let enabled = env::var("OLLAMA_ENABLED")
            .ok()
            .map(|value| {
                matches!(
                    value.trim().to_ascii_lowercase().as_str(),
                    "1" | "true" | "yes" | "on"
                )
            })
            .unwrap_or(true);

        if !enabled {
            return Ok(None);
        }

        let host_raw = env::var("OLLAMA_HOST").ok();
        let port_raw = env::var("OLLAMA_PORT").ok();
        let model_raw = env::var("OLLAMA_MODEL").ok();

        let host = host_raw.as_deref().map(str::trim).unwrap_or_default();
        let port = port_raw.as_deref().map(str::trim).unwrap_or_default();
        let model = model_raw.as_deref().map(str::trim).unwrap_or_default();

        if host.is_empty() && port.is_empty() && model.is_empty() {
            return Ok(None);
        }

        Ok(Some(Self::from_env()?))
    }

    pub fn from_env() -> anyhow::Result<Self> {
        let host = env::var("OLLAMA_HOST")
            .ok()
            .map(|value| value.trim().to_owned())
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| "http://127.0.0.1".to_owned());
        let port = env::var("OLLAMA_PORT")
            .ok()
            .and_then(|value| value.parse::<u16>().ok())
            .unwrap_or(11434);
        let model = env::var("OLLAMA_MODEL")
            .ok()
            .map(|value| value.trim().to_owned())
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| "gpt-oss:20b-cloud".to_owned());

        let client = Ollama::new(host, port);
        Ok(Self { client, model })
    }

    pub async fn generate_channel_reply(
        &self,
        db: &Database,
        guild_id: u64,
        channel_id: u64,
        user_prompt: &str,
        author_display_name: &str,
    ) -> anyhow::Result<String> {
        let history = list_recent_llm_chat_messages(db, guild_id, channel_id, 20).await?;

        let mut messages = Vec::with_capacity(history.len() + 2);
        messages.push(ChatMessage::system(crate::prompt::system_prompt()));

        for item in history.into_iter().rev() {
            let mapped = match item.role.as_str() {
                "user" => ChatMessage::user(format_history_content(
                    "user",
                    item.display_name.as_deref(),
                    &item.content,
                )),
                "assistant" => ChatMessage::assistant(item.content.clone()),
                _ => continue,
            };
            messages.push(mapped);
        }

        let priority_prompt = format!(
            "--- LATEST MESSAGE TO REPLY TO ---\n{}: {}",
            author_display_name, user_prompt
        );
        messages.push(ChatMessage::user(priority_prompt));

        let request = ChatMessageRequest::new(self.model.clone(), messages).options(
            ModelOptions::default()
                .temperature(0.75)
                .repeat_penalty(1.2),
        );
        let response = self
            .client
            .send_chat_messages(request)
            .await
            .context("failed to get ollama chat response")?;

        Ok(response.message.content.trim().to_owned())
    }
}

fn format_history_content(role: &str, display_name: Option<&str>, content: &str) -> String {
    let normalized_name = display_name
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .map(|name| name.replace('\n', " "))
        .unwrap_or_else(|| "unknown".to_owned());

    if role == "user" {
        format!("{}: {}", normalized_name, content)
    } else {
        content.to_owned()
    }
}
