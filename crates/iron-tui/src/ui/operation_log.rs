//! Operation Log View
//!
//! Displays operation history from JSONL log files.
//! Supports filtering by operation type and searching.

use crate::app::App;
use iron_core::state::OperationStatus;
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph, Row, Table};

/// Operation filter options
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OperationFilter {
    #[default]
    All,
    Update,
    Clean,
    Sync,
    Switch,
    Module,
}

impl OperationFilter {
    /// Get all filter options
    pub fn all() -> [Self; 6] {
        [
            Self::All,
            Self::Update,
            Self::Clean,
            Self::Sync,
            Self::Switch,
            Self::Module,
        ]
    }

    /// Get display name
    pub fn name(&self) -> &'static str {
        match self {
            Self::All => "All",
            Self::Update => "Update",
            Self::Clean => "Clean",
            Self::Sync => "Sync",
            Self::Switch => "Switch",
            Self::Module => "Module",
        }
    }
}

/// Render the operation log view
pub fn render_operation_log(frame: &mut Frame, area: Rect, app: &App) {
    // Main layout: Header, Log List, Help
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // Header
            Constraint::Min(10),    // Log list
            Constraint::Length(3),  // Help bar
        ])
        .split(area);

    render_header(frame, layout[0], app);
    render_log_list(frame, layout[1], app);
    render_help(frame, layout[2]);
}

/// Render header section
fn render_header(frame: &mut Frame, area: Rect, app: &App) {
    // Get operation count from state manager
    let op_count = app
        .state_manager
        .as_ref()
        .map(|sm| sm.state().last_operations.len())
        .unwrap_or(0);

    let header_text = Line::from(vec![
        Span::styled("Operation Log", Style::default().fg(Color::White).bold()),
        Span::raw("  │  "),
        Span::styled(
            format!("{} operations", op_count),
            Style::default().fg(Color::Cyan),
        ),
        Span::raw("  │  "),
        Span::styled("Filter: All", Style::default().fg(Color::Gray)),
    ]);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Blue));

    let paragraph = Paragraph::new(header_text)
        .block(block)
        .alignment(Alignment::Center);

    frame.render_widget(paragraph, area);
}

/// Render the log list
fn render_log_list(frame: &mut Frame, area: Rect, app: &App) {
    let block = Block::default()
        .title(" Recent Operations ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Blue));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Get operations from state manager
    let operations = app
        .state_manager
        .as_ref()
        .map(|sm| sm.state().last_operations.clone())
        .unwrap_or_default();

    if operations.is_empty() {
        let no_ops = Paragraph::new(vec![
            Line::from(""),
            Line::from(Span::styled(
                "No operations recorded yet",
                Style::default().fg(Color::Gray),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "Operations will appear here after running updates, cleanups, etc.",
                Style::default().fg(Color::DarkGray),
            )),
        ])
        .alignment(Alignment::Center);

        frame.render_widget(no_ops, inner);
        return;
    }

    // Create table rows (most recent first)
    let rows: Vec<Row> = operations
        .iter()
        .rev()
        .enumerate()
        .map(|(i, op)| {
            let is_selected = i == app.selected_index;

            // Status icon and color
            let (icon, status_color) = match op.status {
                OperationStatus::Success => ("✓", Color::Green),
                OperationStatus::Failed => ("✗", Color::Red),
                OperationStatus::Partial => ("◐", Color::Yellow),
                OperationStatus::Skipped => ("○", Color::Gray),
            };

            // Format timestamp
            let timestamp = op.timestamp.format("%Y-%m-%d %H:%M").to_string();

            let style = if is_selected {
                Style::default().bg(Color::DarkGray).fg(Color::White)
            } else {
                Style::default().fg(Color::White)
            };

            let details = op.details.as_deref().unwrap_or("-");

            Row::new(vec![
                Cell::from(icon).style(Style::default().fg(status_color)),
                Cell::from(timestamp).style(Style::default().fg(Color::Cyan)),
                Cell::from(op.operation.as_str()),
                Cell::from(details).style(Style::default().fg(Color::Gray)),
            ])
            .style(style)
        })
        .collect();

    let widths = [
        Constraint::Length(3),   // Status icon
        Constraint::Length(17),  // Timestamp
        Constraint::Length(20),  // Operation
        Constraint::Min(20),     // Details
    ];

    let table = Table::new(rows, widths)
        .header(
            Row::new(vec!["", "Timestamp", "Operation", "Details"])
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
        ("f", "Filter"),
        ("/", "Search"),
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
    fn test_render_operation_log_no_panic() {
        let mut terminal = create_test_terminal();
        let app = App::default();

        terminal
            .draw(|f| {
                render_operation_log(f, f.area(), &app);
            })
            .unwrap();
    }

    #[test]
    fn test_operation_filter_all() {
        let filters = OperationFilter::all();
        assert_eq!(filters.len(), 6);
    }

    #[test]
    fn test_operation_filter_names() {
        assert_eq!(OperationFilter::All.name(), "All");
        assert_eq!(OperationFilter::Update.name(), "Update");
        assert_eq!(OperationFilter::Clean.name(), "Clean");
    }

    #[test]
    fn test_operation_filter_default() {
        let filter = OperationFilter::default();
        assert_eq!(filter, OperationFilter::All);
    }
}
