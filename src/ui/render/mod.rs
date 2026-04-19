pub mod file_ops;
pub mod port;
pub mod top;

pub use file_ops::*;
pub use port::*;
pub use top::*;

use crate::ui::Theme;

/// Truncate string to specified length
pub fn truncate_string(s: &str, max_len: usize) -> String {
    if s.chars().count() <= max_len {
        s.to_string()
    } else {
        let end = s
            .char_indices()
            .map(|(i, _)| i)
            .nth(max_len.saturating_sub(3))
            .unwrap_or(s.len());
        format!("{}...", &s[..end])
    }
}

/// Return tree drawing characters for the given position
pub fn tree_branches(total: usize, index: usize) -> (&'static str, &'static str) {
    let is_last = index == total - 1;
    if is_last {
        ("└─", "   ")
    } else {
        ("├─", "│  ")
    }
}

/// Format byte size to human-readable string
pub fn format_size(size: u64) -> String {
    super::format_size(size)
}

/// Display error message
pub fn display_error(error: &anyhow::Error) {
    let theme = Theme::new();
    eprintln!("{} {}", theme.error_bold("Error:"), error);
}
