//! Conversation-title validation and message formatting.

use anyhow::{Result, ensure};

pub const MAX_TITLE_CHARS: usize = 80;

pub fn title(input: &str) -> Result<&str> {
    let title = input.trim();
    ensure!(!title.is_empty(), "conversation_title must not be empty");
    ensure!(
        title.chars().count() <= MAX_TITLE_CHARS,
        "conversation_title must be at most {MAX_TITLE_CHARS} characters"
    );
    ensure!(
        !title.contains(['\n', '\r']),
        "conversation_title must be a single line"
    );
    Ok(title)
}

pub fn titled_text(title: &str, message: &str) -> String {
    format!("{title}\n\n{}", message.trim())
}

pub fn status_text(status: &str, title: Option<&str>, message: &str) -> String {
    match title {
        Some(title) => format!("{status}\n\n{}", titled_text(title, message)),
        None => format!("{status}\n\n{}", message.trim()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_short_single_line_titles() {
        assert_eq!(title("  Finance Page  ").unwrap(), "Finance Page");
        assert!(title("").is_err());
        assert!(title("Finance\nPage").is_err());
        assert!(title(&"x".repeat(MAX_TITLE_CHARS + 1)).is_err());
    }

    #[test]
    fn puts_titles_after_status_markers() {
        assert_eq!(
            status_text("🚨 Action needed", Some("Finance Page"), "Choose one."),
            "🚨 Action needed\n\nFinance Page\n\nChoose one."
        );
    }
}
