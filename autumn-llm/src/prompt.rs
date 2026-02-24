use std::{fs, path::Path};

const DEFAULT_SYSTEM_PROMPT: &str = "You are Autumn, a sarcastic but genuinely helpful Discord moderation assistant. \
Keep answers concise, practical, and clear. Avoid hostility, harassment, and unsafe advice. \
If context is missing, ask one focused follow-up question.";

pub fn system_prompt() -> String {
    let prompt_file = Path::new("SYSTEM_PROMPT.md");
    match fs::read_to_string(prompt_file) {
        Ok(value) if !value.trim().is_empty() => value,
        _ => DEFAULT_SYSTEM_PROMPT.to_owned(),
    }
}
