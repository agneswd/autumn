use crate::CommandMeta;

pub fn unknown_category_message(wanted_category: &str, valid_categories: &[&str]) -> String {
    let valid = valid_categories
        .iter()
        .map(|category| display_category(category))
        .collect::<Vec<_>>()
        .join(", ");

    format!(
        "Unknown category: {}\nValid categories: {}",
        display_category(wanted_category),
        valid
    )
}

pub fn no_commands_message(category: Option<&str>) -> String {
    match category {
        Some(cat) => format!("No commands found in category: {}", display_category(cat)),
        None => "No commands found at all. (This probably means something is broken)".to_owned(),
    }
}

pub fn page_out_of_range_message(requested_page: usize, total_pages: usize) -> String {
    format!(
        "Page {} does not exist. Available pages: 1-{}.",
        requested_page, total_pages
    )
}

pub fn grouped_help_description(commands: &[&CommandMeta]) -> String {
    let mut out = String::new();
    let mut current_category: Option<&str> = None;

    for command in commands {
        if current_category != Some(command.category) {
            if !out.is_empty() {
                out.push('\n');
            }
            out.push_str(&format!("**{}**\n", display_category(command.category)));
            current_category = Some(command.category);
        }

        out.push_str(&format!("`{}`: {}\n", command.name, command.desc));
    }

    if out.is_empty() {
        out.push_str("No commands available.");
    }

    out.trim_end().to_owned()
}

fn display_category(category: &str) -> String {
    let mut chars = category.chars();
    match chars.next() {
        Some(first) => format!("{}{}", first.to_uppercase(), chars.as_str()),
        None => String::new(),
    }
}
