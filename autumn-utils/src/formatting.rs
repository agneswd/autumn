/// Format a case ID label (e.g., "WARN", 5 -> "WARN5").
pub fn format_case_label(case_code: &str, action_case_number: u64) -> String {
    format!("{}{}", case_code.to_ascii_uppercase(), action_case_number)
}

/// Convert internal action identifiers to user-facing names.
pub fn action_display_name(action: &str) -> String {
    match action {
        "warn" => "Warn".to_owned(),
        "ban" => "Ban".to_owned(),
        "kick" => "Kick".to_owned(),
        "timeout" => "Timeout".to_owned(),
        "unban" => "Unban".to_owned(),
        "untimeout" => "Untimeout".to_owned(),
        "unwarn" => "Unwarn".to_owned(),
        "unwarn_all" => "Unwarn All".to_owned(),
        "purge" => "Purge".to_owned(),
        "terminate" => "Terminate".to_owned(),
        other => {
            let normalized = other.trim();
            if normalized.is_empty() {
                return "Unknown".to_owned();
            }

            normalized
                .split('_')
                .filter(|part| !part.is_empty())
                .map(|part| {
                    let mut chars = part.chars();
                    match chars.next() {
                        Some(first) => {
                            format!(
                                "{}{}",
                                first.to_uppercase(),
                                chars.as_str().to_ascii_lowercase()
                            )
                        }
                        None => String::new(),
                    }
                })
                .filter(|part| !part.is_empty())
                .collect::<Vec<_>>()
                .join(" ")
        }
    }
}

/// Parse a case label like "WARN5" into ("WARN", 5).
pub fn parse_case_label(raw: &str) -> Option<(String, u64)> {
    let input = raw.trim();
    if input.is_empty() {
        return None;
    }

    let mut split_idx = 0;
    for (idx, ch) in input.char_indices() {
        if ch.is_ascii_alphabetic() {
            split_idx = idx + ch.len_utf8();
            continue;
        }
        break;
    }

    if split_idx == 0 || split_idx >= input.len() {
        return None;
    }

    let (code, number_part) = input.split_at(split_idx);
    let number = number_part.parse::<u64>().ok().filter(|value| *value > 0)?;
    Some((code.to_ascii_uppercase(), number))
}

/// Format seconds into a compact human-readable duration (e.g. 59s, 1m, 1h, 1d, 1h 30m).
pub fn format_compact_duration(total_seconds: u64) -> String {
    let days = total_seconds / 86_400;
    let hours = (total_seconds % 86_400) / 3_600;
    let minutes = (total_seconds % 3_600) / 60;
    let seconds = total_seconds % 60;

    if days > 0 {
        return if hours > 0 {
            format!("{}d {}h", days, hours)
        } else {
            format!("{}d", days)
        };
    }

    if hours > 0 {
        let mut parts = vec![format!("{}h", hours)];
        if minutes > 0 {
            parts.push(format!("{}m", minutes));
        }
        if seconds > 0 {
            parts.push(format!("{}s", seconds));
        }
        return parts.join(" ");
    }

    if minutes > 0 {
        return if seconds > 0 {
            format!("{}m {}s", minutes, seconds)
        } else {
            format!("{}m", minutes)
        };
    }

    format!("{}s", seconds)
}

/// Map internal case event keys to user-facing labels.
pub fn event_display_name(event_type: &str) -> &'static str {
    match event_type {
        "created" => "Created",
        "reason_updated" => "Reason Updated",
        "note_added" => "Note Added",
        _ => "Updated",
    }
}

#[cfg(test)]
mod tests {
    use super::{
        action_display_name, event_display_name, format_case_label, format_compact_duration,
        parse_case_label,
    };

    #[test]
    fn formats_case_labels_uppercase() {
        assert_eq!(format_case_label("w", 12), "W12");
        assert_eq!(format_case_label("uwa", 3), "UWA3");
    }

    #[test]
    fn parses_case_labels() {
        assert_eq!(parse_case_label("w12"), Some(("W".to_owned(), 12)));
        assert_eq!(parse_case_label("UWA3"), Some(("UWA".to_owned(), 3)));
        assert_eq!(parse_case_label("  t5  "), Some(("T".to_owned(), 5)));
        assert_eq!(parse_case_label("12"), None);
        assert_eq!(parse_case_label("W0"), None);
        assert_eq!(parse_case_label(""), None);
    }

    #[test]
    fn action_names_are_user_friendly() {
        assert_eq!(action_display_name("warn"), "Warn");
        assert_eq!(action_display_name("unwarn_all"), "Unwarn All");
        assert_eq!(action_display_name("custom_action"), "Custom Action");
    }

    #[test]
    fn event_names_are_user_friendly() {
        assert_eq!(event_display_name("created"), "Created");
        assert_eq!(event_display_name("reason_updated"), "Reason Updated");
        assert_eq!(event_display_name("note_added"), "Note Added");
        assert_eq!(event_display_name("other"), "Updated");
    }

    #[test]
    fn compact_duration_formatting() {
        assert_eq!(format_compact_duration(59), "59s");
        assert_eq!(format_compact_duration(60), "1m");
        assert_eq!(format_compact_duration(61), "1m 1s");
        assert_eq!(format_compact_duration(3600), "1h");
        assert_eq!(format_compact_duration(3660), "1h 1m");
        assert_eq!(format_compact_duration(3670), "1h 1m 10s");
        assert_eq!(format_compact_duration(3605), "1h 5s");
        assert_eq!(format_compact_duration(86400), "1d");
        assert_eq!(format_compact_duration(90000), "1d 1h");
    }
}
