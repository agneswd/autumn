/// Parse a compact duration token like `30s`, `10m`, `2h`, `1d`, or plain seconds.
pub fn parse_duration_seconds(raw: &str) -> Option<u64> {
    let value = raw.trim();
    if value.is_empty() {
        return None;
    }

    let mut chars = value.chars();
    let unit = chars.next_back();

    let (number_raw, multiplier) = match unit {
        Some('s') | Some('S') => (chars.as_str(), 1_u64),
        Some('m') | Some('M') => (chars.as_str(), 60_u64),
        Some('h') | Some('H') => (chars.as_str(), 60_u64 * 60),
        Some('d') | Some('D') => (chars.as_str(), 60_u64 * 60 * 24),
        Some(last) if last.is_ascii_digit() => (value, 1_u64),
        _ => return None,
    };

    let number = number_raw.parse::<u64>().ok()?;
    if number == 0 {
        return None;
    }

    number.checked_mul(multiplier)
}
