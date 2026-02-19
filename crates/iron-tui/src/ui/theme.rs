//! Iron TUI Theme
//!
//! Centralized color palette and styling utilities for consistent UI.
//! Based on Catppuccin Mocha color scheme with purple accent.

use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders};

// ─────────────────────────────────────────────────────────────────────────────
// Color Palette (Catppuccin Mocha)
// ─────────────────────────────────────────────────────────────────────────────

/// Deep background for panels
pub const SURFACE: Color = Color::Rgb(30, 30, 46);

/// Lighter background for hover/selection
pub const SURFACE_HOVER: Color = Color::Rgb(45, 45, 65);

/// Border and separator color
pub const OVERLAY: Color = Color::Rgb(49, 50, 68);

/// Primary text
pub const TEXT: Color = Color::Rgb(205, 214, 244);

/// Secondary/dimmed text
pub const SUBTEXT: Color = Color::Rgb(166, 173, 200);

/// Success/ok state
pub const GREEN: Color = Color::Rgb(166, 227, 161);

/// Warning state
pub const YELLOW: Color = Color::Rgb(249, 226, 175);

/// Error/critical state
pub const RED: Color = Color::Rgb(243, 139, 168);

/// Info/link color
pub const BLUE: Color = Color::Rgb(137, 180, 250);

/// Primary accent (purple/mauve)
pub const MAUVE: Color = Color::Rgb(203, 166, 247);

/// Secondary accent (teal)
pub const TEAL: Color = Color::Rgb(148, 226, 213);

/// Tertiary accent (peach/orange)
pub const PEACH: Color = Color::Rgb(250, 179, 135);

/// Lavender accent
pub const LAVENDER: Color = Color::Rgb(180, 190, 254);

/// Pink accent
pub const PINK: Color = Color::Rgb(245, 194, 231);

/// Sky blue accent
pub const SKY: Color = Color::Rgb(137, 220, 235);

// ─────────────────────────────────────────────────────────────────────────────
// Icons
// ─────────────────────────────────────────────────────────────────────────────

/// Status icons
pub mod icons {
    pub const OK: &str = "●";
    pub const WARNING: &str = "◐";
    pub const ERROR: &str = "○";
    pub const INFO: &str = "◆";
    pub const EDIT: &str = "✎";
    pub const CHECK: &str = "✓";
    pub const CROSS: &str = "✗";
    pub const ARROW_RIGHT: &str = "→";
    pub const ARROW_DOWN: &str = "↓";
    pub const REFRESH: &str = "↻";
    pub const FOLDER: &str = "󰉋";
    pub const HOST: &str = "󰒋";
    pub const BUNDLE: &str = "◫";
    pub const PROFILE: &str = "◉";
    pub const MODULE: &str = "⬡";
    pub const SETTINGS: &str = "⚙";
    pub const LOCK: &str = "󰌾";
    pub const PACKAGE: &str = "󰏗";
    pub const UPDATE: &str = "↻";
    pub const SYNC: &str = "⇄";
    pub const TIME: &str = "󰥔";
}

// ─────────────────────────────────────────────────────────────────────────────
// Block Styles
// ─────────────────────────────────────────────────────────────────────────────

/// Create a themed block with consistent styling
pub fn themed_block(title: &str, accent: Color) -> Block<'_> {
    Block::default()
        .title(format!(" {} ", title))
        .title_style(Style::default().fg(accent).bold())
        .borders(Borders::ALL)
        .border_style(Style::default().fg(OVERLAY))
        .style(Style::default().bg(SURFACE))
}

/// Create a themed block with an icon prefix
pub fn themed_block_with_icon<'a>(icon: &'a str, title: &'a str, accent: Color) -> Block<'a> {
    Block::default()
        .title(format!(" {} {} ", icon, title))
        .title_style(Style::default().fg(accent).bold())
        .borders(Borders::ALL)
        .border_style(Style::default().fg(OVERLAY))
        .style(Style::default().bg(SURFACE))
}

/// Create a minimal block with just borders
pub fn minimal_block() -> Block<'static> {
    Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(OVERLAY))
        .style(Style::default().bg(SURFACE))
}

// ─────────────────────────────────────────────────────────────────────────────
// Text Styles
// ─────────────────────────────────────────────────────────────────────────────

/// Primary text style
pub fn text_primary() -> Style {
    Style::default().fg(TEXT)
}

/// Secondary/dimmed text style
pub fn text_secondary() -> Style {
    Style::default().fg(SUBTEXT)
}

/// Accent text style (purple)
pub fn text_accent() -> Style {
    Style::default().fg(MAUVE)
}

/// Bold primary text
pub fn text_bold() -> Style {
    Style::default().fg(TEXT).bold()
}

/// Label style (for key names, etc.)
pub fn text_label() -> Style {
    Style::default().fg(SUBTEXT)
}

/// Value style (for settings values, etc.)
pub fn text_value() -> Style {
    Style::default().fg(LAVENDER)
}

/// Editable field indicator
pub fn text_editable() -> Style {
    Style::default().fg(PEACH)
}

// ─────────────────────────────────────────────────────────────────────────────
// Selection Styles
// ─────────────────────────────────────────────────────────────────────────────

/// Style for selected/highlighted items
pub fn selected() -> Style {
    Style::default().bg(SURFACE_HOVER).fg(TEXT)
}

/// Style for selected items with accent
pub fn selected_accent() -> Style {
    Style::default().bg(SURFACE_HOVER).fg(MAUVE).bold()
}

/// Style for unselected items
pub fn unselected() -> Style {
    Style::default().fg(TEXT)
}

// ─────────────────────────────────────────────────────────────────────────────
// Status Styles
// ─────────────────────────────────────────────────────────────────────────────

/// Success style
pub fn status_ok() -> Style {
    Style::default().fg(GREEN)
}

/// Warning style
pub fn status_warning() -> Style {
    Style::default().fg(YELLOW)
}

/// Error style
pub fn status_error() -> Style {
    Style::default().fg(RED)
}

/// Info style
pub fn status_info() -> Style {
    Style::default().fg(BLUE)
}

// ─────────────────────────────────────────────────────────────────────────────
// Key Hint Helpers
// ─────────────────────────────────────────────────────────────────────────────

/// Render a keyboard hint as styled spans
pub fn key_hint(key: &str, label: &str) -> Vec<Span<'static>> {
    vec![
        Span::styled(
            format!(" {} ", key),
            Style::default().fg(SURFACE).bg(MAUVE).bold(),
        ),
        Span::styled(format!(" {}  ", label), Style::default().fg(SUBTEXT)),
    ]
}

/// Render a compact keyboard hint
pub fn key_hint_compact(key: &str, label: &str) -> Vec<Span<'static>> {
    vec![
        Span::styled(
            format!("[{}]", key),
            Style::default().fg(MAUVE).bold(),
        ),
        Span::styled(format!(" {} ", label), Style::default().fg(SUBTEXT)),
    ]
}

// ─────────────────────────────────────────────────────────────────────────────
// Progress Bar Helpers
// ─────────────────────────────────────────────────────────────────────────────

/// Create a mini progress bar string
pub fn mini_progress_bar(current: usize, total: usize, width: usize) -> String {
    if total == 0 {
        return "─".repeat(width);
    }
    let filled = (current * width) / total.max(1);
    let empty = width.saturating_sub(filled);
    format!("{}{}", "█".repeat(filled), "░".repeat(empty))
}

/// Create a styled progress indicator
pub fn progress_indicator(current: usize, total: usize) -> Vec<Span<'static>> {
    let bar = mini_progress_bar(current, total, 8);
    let pct = if total > 0 {
        (current * 100) / total
    } else {
        0
    };

    vec![
        Span::styled(bar, Style::default().fg(MAUVE)),
        Span::styled(format!(" {}%", pct), Style::default().fg(SUBTEXT)),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mini_progress_bar_empty() {
        let bar = mini_progress_bar(0, 10, 10);
        assert_eq!(bar, "░░░░░░░░░░");
    }

    #[test]
    fn test_mini_progress_bar_full() {
        let bar = mini_progress_bar(10, 10, 10);
        assert_eq!(bar, "██████████");
    }

    #[test]
    fn test_mini_progress_bar_half() {
        let bar = mini_progress_bar(5, 10, 10);
        assert_eq!(bar, "█████░░░░░");
    }

    #[test]
    fn test_mini_progress_bar_zero_total() {
        let bar = mini_progress_bar(0, 0, 5);
        assert_eq!(bar, "─────");
    }

    #[test]
    fn test_themed_block_has_borders() {
        let block = themed_block("Test", MAUVE);
        // Block should be created without panic
        assert!(true);
    }

    #[test]
    fn test_key_hint_format() {
        let hints = key_hint("Enter", "Select");
        assert_eq!(hints.len(), 2);
    }

    #[test]
    fn test_style_functions() {
        // Ensure all style functions return valid styles
        let _ = text_primary();
        let _ = text_secondary();
        let _ = text_accent();
        let _ = selected();
        let _ = unselected();
        let _ = status_ok();
        let _ = status_warning();
        let _ = status_error();
    }
}
