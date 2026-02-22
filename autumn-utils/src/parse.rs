/// Parse a compact duration token like `30s`, `10m`, `2h`, `1d`, or plain seconds.
pub fn parse_duration_seconds(raw: &str) -> Option<u64> {
    let value = raw.trim();
    if value.is_empty() {
        return None;
    }

    let compact: String = value.chars().filter(|ch| !ch.is_whitespace()).collect();
    if compact.is_empty() {
        return None;
    }

    let bytes = compact.as_bytes();
    let mut cursor = 0;
    let mut total_seconds = 0_u64;
    let mut saw_unit_segment = false;

    while cursor < bytes.len() {
        let number_start = cursor;
        while cursor < bytes.len() && bytes[cursor].is_ascii_digit() {
            cursor += 1;
        }

        if number_start == cursor {
            return None;
        }

        let number = compact[number_start..cursor].parse::<u64>().ok()?;
        if number == 0 {
            return None;
        }

        let saw_unit = cursor < bytes.len();
        let multiplier = if saw_unit {
            let unit = bytes[cursor] as char;
            cursor += 1;

            match unit {
                's' | 'S' => 1_u64,
                'm' | 'M' => 60_u64,
                'h' | 'H' => 60_u64 * 60,
                'd' | 'D' => 60_u64 * 60 * 24,
                _ => return None,
            }
        } else {
            1_u64
        };

        if !saw_unit && saw_unit_segment {
            return None;
        }

        saw_unit_segment = saw_unit_segment || saw_unit;

        let part_seconds = number.checked_mul(multiplier)?;
        total_seconds = total_seconds.checked_add(part_seconds)?;
    }

    if total_seconds == 0 {
        None
    } else {
        Some(total_seconds)
    }
}

pub fn has_duration_unit(raw: &str) -> bool {
    let value = raw.trim();
    let Some(last) = value.chars().last() else {
        return false;
    };

    matches!(last, 's' | 'S' | 'm' | 'M' | 'h' | 'H' | 'd' | 'D')
}
