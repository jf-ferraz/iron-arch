//! Configuration Manager View
//!
//! Displays and manages .pacnew/.pacsave configuration file conflicts
//! detected after system updates.

use crate::app::App;
use crate::ui::theme;
use ratatui::prelude::*;
use ratatui::widgets::{Cell, Paragraph, Row, Table};

/// Config conflict type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum ConflictType {
    /// New config file from package (.pacnew)
    Pacnew,
    /// Saved user config (.pacsave)
    Pacsave,
}

#[allow(dead_code)]
impl ConflictType {
    /// Get the display name
    pub fn name(&self) -> &'static str {
        match self {
            Self::Pacnew => ".pacnew",
            Self::Pacsave => ".pacsave",
        }
    }

    /// Get the icon
    pub fn icon(&self) -> &'static str {
        match self {
            Self::Pacnew => "N",
            Self::Pacsave => "S",
        }
    }

    /// Get the description
    pub fn description(&self) -> &'static str {
        match self {
            Self::Pacnew => "New config from package update",
            Self::Pacsave => "User config saved during removal",
        }
    }
}

/// Render the configuration manager view
pub fn render_config_manager(frame: &mut Frame, area: Rect, app: &App) {
    // Main layout: Header + Conflicts List (footer handles keybindings)
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // Header
            Constraint::Min(10),    // Conflicts list
        ])
        .split(area);

    render_header(frame, layout[0], app);
    render_conflicts_list(frame, layout[1], app);
}

/// Render header section
fn render_header(frame: &mut Frame, area: Rect, app: &App) {
    let conflict_count = app
        .post_update_result
        .as_ref()
        .map(|r| r.config_conflicts.len())
        .unwrap_or(0);

    let status = if conflict_count == 0 {
        Span::styled("No conflicts", Style::default().fg(theme::GREEN))
    } else {
        Span::styled(
            format!("{} conflict{}", conflict_count, if conflict_count == 1 { "" } else { "s" }),
            Style::default().fg(theme::YELLOW),
        )
    };

    let header_text = Line::from(vec![
        Span::styled("Configuration Manager", Style::default().fg(theme::TEXT).bold()),
        Span::raw("  │  "),
        status,
    ]);

    let block = theme::themed_block("Config", theme::BLUE);

    let paragraph = Paragraph::new(header_text)
        .block(block)
        .alignment(Alignment::Center);

    frame.render_widget(paragraph, area);
}

/// Render the conflicts list
fn render_conflicts_list(frame: &mut Frame, area: Rect, app: &App) {
    let block = theme::themed_block("Configuration Conflicts", theme::BLUE);

    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Get conflicts from post-update results
    let conflicts = app
        .post_update_result
        .as_ref()
        .map(|r| &r.config_conflicts)
        .cloned()
        .unwrap_or_default();

    if conflicts.is_empty() {
        let no_conflicts = Paragraph::new(vec![
            Line::from(""),
            Line::from(Span::styled(
                "✓ No configuration conflicts detected",
                Style::default().fg(theme::GREEN),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "Configuration conflicts appear after package updates.",
                Style::default().fg(theme::SUBTEXT),
            )),
        ])
        .alignment(Alignment::Center);

        frame.render_widget(no_conflicts, inner);
        return;
    }

    // Create table rows
    let rows: Vec<Row> = conflicts
        .iter()
        .enumerate()
        .map(|(i, conflict)| {
            let is_selected = i == app.selected_index;

            // Determine conflict type from conflict_type field
            use iron_core::services::update::ConfigConflictType;
            let (icon, type_str, color) = match conflict.conflict_type {
                ConfigConflictType::Pacnew => ("N", ".pacnew", theme::YELLOW),
                ConfigConflictType::Pacsave => ("S", ".pacsave", theme::TEAL),
            };

            let style = if is_selected {
                theme::selected()
            } else {
                theme::unselected()
            };

            Row::new(vec![
                Cell::from(icon),
                Cell::from(type_str).style(Style::default().fg(color)),
                Cell::from(conflict.original.as_str()),
            ])
            .style(style)
        })
        .collect();

    let widths = [
        Constraint::Length(3),   // Icon
        Constraint::Length(10),  // Type
        Constraint::Min(30),     // Path
    ];

    let table = Table::new(rows, widths)
        .header(
            Row::new(vec!["", "Type", "File Path"])
                .style(Style::default().fg(theme::YELLOW).bold())
                .bottom_margin(1),
        )
        .column_spacing(2);

    frame.render_widget(table, inner);
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    fn create_test_terminal() -> Terminal<TestBackend> {
        let backend = TestBackend::new(100, 25);
        Terminal::new(backend).unwrap()
    }

    #[test]
    fn test_render_config_manager_no_panic() {
        let mut terminal = create_test_terminal();
        let app = App::default();

        terminal
            .draw(|f| {
                render_config_manager(f, f.area(), &app);
            })
            .unwrap();
    }

    #[test]
    fn test_conflict_type_names() {
        assert_eq!(ConflictType::Pacnew.name(), ".pacnew");
        assert_eq!(ConflictType::Pacsave.name(), ".pacsave");
    }

    #[test]
    fn test_conflict_type_icons() {
        assert_eq!(ConflictType::Pacnew.icon(), "N");
        assert_eq!(ConflictType::Pacsave.icon(), "S");
    }
}
