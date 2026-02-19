//! Settings view rendering
//!
//! Displays configuration values and system state.

use crate::app::App;
use chrono::{DateTime, Utc};
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Cell, Paragraph, Row, Table};

// ─────────────────────────────────────────────────────────────────────────────
// Settings Item Definition
// ─────────────────────────────────────────────────────────────────────────────

/// Settings item for display
#[derive(Debug, Clone)]
struct SettingItem {
    key: &'static str,
    value: String,
    editable: bool,
}

// ─────────────────────────────────────────────────────────────────────────────
// Helper Functions
// ─────────────────────────────────────────────────────────────────────────────

/// Format a DateTime as a relative time string
fn format_relative_time(time: Option<DateTime<Utc>>) -> String {
    match time {
        Some(dt) => {
            let now = Utc::now();
            let duration = now.signed_duration_since(dt);

            if duration.num_minutes() < 1 {
                "just now".to_string()
            } else if duration.num_minutes() < 60 {
                let mins = duration.num_minutes();
                format!("{} min{} ago", mins, if mins == 1 { "" } else { "s" })
            } else if duration.num_hours() < 24 {
                let hours = duration.num_hours();
                format!("{} hour{} ago", hours, if hours == 1 { "" } else { "s" })
            } else {
                let days = duration.num_days();
                format!("{} day{} ago", days, if days == 1 { "" } else { "s" })
            }
        }
        None => "never".to_string(),
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Main Render Function
// ─────────────────────────────────────────────────────────────────────────────

/// Render settings view
pub fn render_settings(frame: &mut Frame, area: Rect, app: &App) {
    // Main layout: settings panel with hint bar
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Min(10),   // Settings table
            Constraint::Length(3), // Hint bar
        ])
        .split(area);

    render_settings_panel(frame, layout[0], app);
    render_hint_bar(frame, layout[1], app);
}

/// Render the main settings panel
fn render_settings_panel(frame: &mut Frame, area: Rect, app: &App) {
    let block = Block::default()
        .title(" Configuration ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Build settings items from app state
    let config_dir = app.config_dir.display().to_string();
    let current_host = app.current_host.as_deref().unwrap_or("not set");
    let active_bundle = app
        .active_bundle
        .as_ref()
        .map(|b| b.id.as_str())
        .unwrap_or("none");
    let active_profile = app.active_profile.as_deref().unwrap_or("none");
    let module_count = app.active_modules.len();
    let total_modules = app.modules.len();

    // Get sync info from state manager
    let last_sync = app
        .state_manager
        .as_ref()
        .map(|sm| sm.maintenance().last_sync)
        .flatten();

    let settings = vec![
        SettingItem {
            key: "Config Directory",
            value: config_dir,
            editable: false,
        },
        SettingItem {
            key: "Current Host",
            value: current_host.to_string(),
            editable: true,
        },
        SettingItem {
            key: "Active Bundle",
            value: active_bundle.to_string(),
            editable: false,
        },
        SettingItem {
            key: "Active Profile",
            value: active_profile.to_string(),
            editable: false,
        },
        SettingItem {
            key: "Enabled Modules",
            value: format!("{}/{}", module_count, total_modules),
            editable: false,
        },
        SettingItem {
            key: "Last Sync",
            value: format_relative_time(last_sync),
            editable: false,
        },
        SettingItem {
            key: "Installed Packages",
            value: format!("{}", app.installed_count),
            editable: false,
        },
        SettingItem {
            key: "Pending Updates",
            value: format!("{}", app.pending_updates.len()),
            editable: false,
        },
    ];

    // Create table rows
    let rows: Vec<Row> = settings
        .iter()
        .enumerate()
        .map(|(i, item)| {
            let is_selected = i == app.selected_index;

            let row_style = if is_selected {
                Style::default().bg(Color::DarkGray).fg(Color::White)
            } else {
                Style::default()
            };

            // Selection indicator
            let selector = if is_selected {
                Cell::from(Span::styled(">", Style::default().fg(Color::Cyan).bold()))
            } else {
                Cell::from(" ")
            };

            // Key name
            let key_cell = Cell::from(Span::styled(
                item.key,
                if is_selected {
                    Style::default().fg(Color::White).bold()
                } else {
                    Style::default().fg(Color::Gray)
                },
            ));

            // Value with special styling
            let value_style = if item.editable {
                Style::default().fg(Color::Yellow)
            } else if item.value == "none" || item.value == "not set" || item.value == "never" {
                Style::default().fg(Color::DarkGray).italic()
            } else {
                Style::default().fg(Color::Cyan)
            };
            let value_cell = Cell::from(Span::styled(&item.value, value_style));

            // Edit indicator
            let edit_cell = if item.editable {
                Cell::from(Span::styled("[E]", Style::default().fg(Color::Yellow)))
            } else {
                Cell::from("")
            };

            Row::new(vec![selector, key_cell, value_cell, edit_cell]).style(row_style)
        })
        .collect();

    let widths = [
        Constraint::Length(2),  // Selector
        Constraint::Length(20), // Key
        Constraint::Min(20),    // Value
        Constraint::Length(4),  // Edit indicator
    ];

    let table = Table::new(rows, widths).column_spacing(1);

    frame.render_widget(table, inner);
}

/// Render the hint bar showing contextual help
fn render_hint_bar(frame: &mut Frame, area: Rect, app: &App) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Show hint for selected item
    let hint = if app.selected_index < 8 {
        let hints = [
            "Configuration directory (read-only)",
            "Use Setup Wizard [w] to change host",
            "Use Bundles view [b] to change bundle",
            "Use Profiles view [p] to change profile",
            "Use Modules view [m] to manage modules",
            "Last git sync time",
            "Total installed packages",
            "Use Update view [u] to review updates",
        ];
        hints.get(app.selected_index).copied().unwrap_or("")
    } else {
        ""
    };

    let content = Line::from(vec![
        Span::styled("  Hint: ", Style::default().fg(Color::Cyan)),
        Span::styled(hint, Style::default().fg(Color::Gray)),
    ]);

    frame.render_widget(Paragraph::new(content), inner);
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    fn create_test_terminal() -> Terminal<TestBackend> {
        let backend = TestBackend::new(100, 30);
        Terminal::new(backend).unwrap()
    }

    #[test]
    fn test_render_settings_no_panic() {
        let mut terminal = create_test_terminal();
        let app = App::default();

        terminal
            .draw(|f| {
                render_settings(f, f.area(), &app);
            })
            .unwrap();
    }

    #[test]
    fn test_format_relative_time_none() {
        assert_eq!(format_relative_time(None), "never");
    }

    #[test]
    fn test_format_relative_time_just_now() {
        let now = Utc::now();
        let result = format_relative_time(Some(now));
        assert_eq!(result, "just now");
    }

    #[test]
    fn test_setting_item_creation() {
        let item = SettingItem {
            key: "Test",
            value: "Value".to_string(),
            editable: true,
        };
        assert_eq!(item.key, "Test");
        assert!(item.editable);
    }
}
