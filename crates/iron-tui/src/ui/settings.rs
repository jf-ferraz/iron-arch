//! Settings view rendering
//!
//! Displays configuration values and system state.

use crate::app::App;
use crate::ui::theme;
use crate::ui::utils::format_relative_time;
use ratatui::prelude::*;
use ratatui::widgets::{Cell, Paragraph, Row, Table};

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
    let block = theme::themed_block("Configuration", theme::MAUVE);

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
        .and_then(|sm| sm.maintenance().last_sync);

    let settings = [SettingItem {
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
        }];

    // Create table rows
    let rows: Vec<Row> = settings
        .iter()
        .enumerate()
        .map(|(i, item)| {
            let is_selected = i == app.selected_index;

            let row_style = if is_selected {
                theme::selected()
            } else {
                theme::unselected()
            };

            // Selection indicator
            let selector = if is_selected {
                Cell::from(Span::styled(">", Style::default().fg(theme::MAUVE).bold()))
            } else {
                Cell::from(" ")
            };

            // Key name
            let key_cell = Cell::from(Span::styled(
                item.key,
                if is_selected {
                    Style::default().fg(theme::TEXT).bold()
                } else {
                    Style::default().fg(theme::SUBTEXT)
                },
            ));

            // Value with special styling
            let value_style = if item.editable {
                Style::default().fg(theme::PEACH)
            } else if item.value == "none" || item.value == "not set" || item.value == "never" {
                Style::default().fg(theme::OVERLAY).italic()
            } else {
                Style::default().fg(theme::LAVENDER)
            };
            let value_cell = Cell::from(Span::styled(&item.value, value_style));

            // Edit indicator
            let edit_cell = if item.editable {
                Cell::from(Span::styled("[E]", Style::default().fg(theme::PEACH)))
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
    let block = theme::minimal_block();

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
        Span::styled("  Hint: ", Style::default().fg(theme::MAUVE)),
        Span::styled(hint, Style::default().fg(theme::SUBTEXT)),
    ]);

    frame.render_widget(Paragraph::new(content), inner);
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

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
