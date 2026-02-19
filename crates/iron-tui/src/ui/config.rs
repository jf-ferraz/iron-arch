//! Configuration Manager View
//!
//! Displays and manages .pacnew/.pacsave configuration file conflicts
//! detected after system updates.

use crate::app::App;
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph, Row, Table};

/// Config conflict type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConflictType {
    /// New config file from package (.pacnew)
    Pacnew,
    /// Saved user config (.pacsave)
    Pacsave,
}

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
            Self::Pacnew => "📄",
            Self::Pacsave => "💾",
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
    // Main layout: Header, Conflicts List, Help
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // Header
            Constraint::Min(10),    // Conflicts list
            Constraint::Length(3),  // Help bar
        ])
        .split(area);

    render_header(frame, layout[0], app);
    render_conflicts_list(frame, layout[1], app);
    render_help(frame, layout[2]);
}

/// Render header section
fn render_header(frame: &mut Frame, area: Rect, app: &App) {
    let conflict_count = app
        .post_update_result
        .as_ref()
        .map(|r| r.config_conflicts.len())
        .unwrap_or(0);

    let status = if conflict_count == 0 {
        Span::styled("No conflicts", Style::default().fg(Color::Green))
    } else {
        Span::styled(
            format!("{} conflict{}", conflict_count, if conflict_count == 1 { "" } else { "s" }),
            Style::default().fg(Color::Yellow),
        )
    };

    let header_text = Line::from(vec![
        Span::styled("Configuration Manager", Style::default().fg(Color::White).bold()),
        Span::raw("  │  "),
        status,
    ]);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Blue));

    let paragraph = Paragraph::new(header_text)
        .block(block)
        .alignment(Alignment::Center);

    frame.render_widget(paragraph, area);
}

/// Render the conflicts list
fn render_conflicts_list(frame: &mut Frame, area: Rect, app: &App) {
    let block = Block::default()
        .title(" Configuration Conflicts ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Blue));

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
                Style::default().fg(Color::Green),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "Run a system update to check for new conflicts.",
                Style::default().fg(Color::Gray),
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
                ConfigConflictType::Pacnew => ("📄", ".pacnew", Color::Yellow),
                ConfigConflictType::Pacsave => ("💾", ".pacsave", Color::Cyan),
            };

            let style = if is_selected {
                Style::default().bg(Color::DarkGray).fg(Color::White)
            } else {
                Style::default().fg(Color::White)
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
                .style(Style::default().fg(Color::Yellow).bold())
                .bottom_margin(1),
        )
        .column_spacing(2);

    frame.render_widget(table, inner);
}

/// Render help bar
fn render_help(frame: &mut Frame, area: Rect) {
    let help_items = vec![
        ("↑↓", "Navigate"),
        ("Enter", "View Diff"),
        ("r", "Mark Resolved"),
        ("Esc", "Back"),
    ];

    let help_spans: Vec<Span> = help_items
        .iter()
        .flat_map(|(key, action)| {
            vec![
                Span::styled(
                    format!("[{}]", key),
                    Style::default().fg(Color::Yellow),
                ),
                Span::raw(format!(" {}  ", action)),
            ]
        })
        .collect();

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray));

    let paragraph = Paragraph::new(Line::from(help_spans))
        .block(block)
        .alignment(Alignment::Center);

    frame.render_widget(paragraph, area);
}

use ratatui::widgets::Cell;

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
        assert_eq!(ConflictType::Pacnew.icon(), "📄");
        assert_eq!(ConflictType::Pacsave.icon(), "💾");
    }
}
