//! System Maintenance Hub View
//!
//! Central hub for system maintenance operations:
//! - Update: System package updates
//! - Cleanup: Cache and orphan cleanup
//! - Doctor: System health diagnostics

use crate::app::App;
use chrono::{DateTime, Utc};
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph};

/// Maintenance action types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MaintenanceAction {
    Update,
    Cleanup,
    Doctor,
}

impl MaintenanceAction {
    /// Get all actions in display order
    pub fn all() -> [Self; 3] {
        [Self::Update, Self::Cleanup, Self::Doctor]
    }

    /// Get the action name
    pub fn name(&self) -> &'static str {
        match self {
            Self::Update => "System Update",
            Self::Cleanup => "System Cleanup",
            Self::Doctor => "System Doctor",
        }
    }

    /// Get the action description
    pub fn description(&self) -> &'static str {
        match self {
            Self::Update => "Check for and install package updates",
            Self::Cleanup => "Remove cached packages and orphans",
            Self::Doctor => "Run system health diagnostics",
        }
    }

    /// Get the keyboard shortcut
    pub fn shortcut(&self) -> &'static str {
        match self {
            Self::Update => "u",
            Self::Cleanup => "c",
            Self::Doctor => "d",
        }
    }

    /// Get the icon for this action
    pub fn icon(&self) -> &'static str {
        match self {
            Self::Update => "↻",
            Self::Cleanup => "🧹",
            Self::Doctor => "🩺",
        }
    }
}

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
            } else if duration.num_days() < 7 {
                let days = duration.num_days();
                format!("{} day{} ago", days, if days == 1 { "" } else { "s" })
            } else if duration.num_weeks() < 4 {
                let weeks = duration.num_weeks();
                format!("{} week{} ago", weeks, if weeks == 1 { "" } else { "s" })
            } else {
                let months = duration.num_days() / 30;
                if months < 12 {
                    format!("{} month{} ago", months, if months == 1 { "" } else { "s" })
                } else {
                    "over a year ago".to_string()
                }
            }
        }
        None => "never".to_string(),
    }
}

/// Render the system maintenance hub view
pub fn render_system_maintenance(frame: &mut Frame, area: Rect, app: &App) {
    // Main layout: Header, Action Cards, Help
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // Header
            Constraint::Min(12),    // Action cards
            Constraint::Length(3),  // Help bar
        ])
        .split(area);

    render_header(frame, layout[0]);
    render_action_cards(frame, layout[1], app);
    render_help(frame, layout[2]);
}

/// Render header section
fn render_header(frame: &mut Frame, area: Rect) {
    let header_text = Line::from(vec![
        Span::styled("System Maintenance", Style::default().fg(Color::White).bold()),
        Span::raw("  │  "),
        Span::styled("Hub", Style::default().fg(Color::Cyan)),
    ]);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Blue));

    let paragraph = Paragraph::new(header_text)
        .block(block)
        .alignment(Alignment::Center);

    frame.render_widget(paragraph, area);
}

/// Render the three action cards
fn render_action_cards(frame: &mut Frame, area: Rect, app: &App) {
    let block = Block::default()
        .title(" Maintenance Actions ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Blue));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Split into three columns for the cards
    let cards = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Ratio(1, 3),
            Constraint::Ratio(1, 3),
            Constraint::Ratio(1, 3),
        ])
        .split(inner);

    // Get maintenance timestamps
    let maintenance = app
        .state_manager
        .as_ref()
        .map(|sm| sm.maintenance());

    let actions = MaintenanceAction::all();
    for (i, action) in actions.iter().enumerate() {
        let is_selected = app.selected_index == i;
        render_action_card(frame, cards[i], *action, is_selected, &maintenance);
    }
}

/// Render a single action card
fn render_action_card(
    frame: &mut Frame,
    area: Rect,
    action: MaintenanceAction,
    is_selected: bool,
    maintenance: &Option<iron_core::state::MaintenanceState>,
) {
    let border_color = if is_selected {
        Color::Yellow
    } else {
        Color::DarkGray
    };

    let bg_color = if is_selected {
        Color::DarkGray
    } else {
        Color::Reset
    };

    let block = Block::default()
        .title(format!(" {} {} ", action.icon(), action.name()))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color))
        .style(Style::default().bg(bg_color));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Get last run timestamp for this action
    let last_run = maintenance.as_ref().and_then(|m| match action {
        MaintenanceAction::Update => m.last_update,
        MaintenanceAction::Cleanup => m.last_clean,
        MaintenanceAction::Doctor => m.last_doctor,
    });

    let content = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled(action.description(), Style::default().fg(Color::White)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Last run: ", Style::default().fg(Color::Gray)),
            Span::styled(
                format_relative_time(last_run),
                Style::default().fg(Color::Cyan),
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Press ", Style::default().fg(Color::Gray)),
            Span::styled(
                format!("[{}]", action.shortcut()),
                Style::default().fg(Color::Yellow),
            ),
            Span::styled(" to launch", Style::default().fg(Color::Gray)),
        ]),
    ];

    let paragraph = Paragraph::new(content).alignment(Alignment::Center);
    frame.render_widget(paragraph, inner);
}

/// Render help bar
fn render_help(frame: &mut Frame, area: Rect) {
    let help_items = vec![
        ("u", "Update"),
        ("c", "Cleanup"),
        ("d", "Doctor"),
        ("←→", "Select"),
        ("Enter", "Launch"),
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

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    fn create_test_terminal() -> Terminal<TestBackend> {
        let backend = TestBackend::new(120, 30);
        Terminal::new(backend).unwrap()
    }

    #[test]
    fn test_render_system_maintenance_no_panic() {
        let mut terminal = create_test_terminal();
        let app = App::default();

        terminal
            .draw(|f| {
                render_system_maintenance(f, f.area(), &app);
            })
            .unwrap();
    }

    #[test]
    fn test_maintenance_action_all() {
        let actions = MaintenanceAction::all();
        assert_eq!(actions.len(), 3);
    }

    #[test]
    fn test_maintenance_action_names() {
        assert_eq!(MaintenanceAction::Update.name(), "System Update");
        assert_eq!(MaintenanceAction::Cleanup.name(), "System Cleanup");
        assert_eq!(MaintenanceAction::Doctor.name(), "System Doctor");
    }

    #[test]
    fn test_maintenance_action_shortcuts() {
        assert_eq!(MaintenanceAction::Update.shortcut(), "u");
        assert_eq!(MaintenanceAction::Cleanup.shortcut(), "c");
        assert_eq!(MaintenanceAction::Doctor.shortcut(), "d");
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
    fn test_format_relative_time_minutes() {
        let time = Utc::now() - chrono::Duration::minutes(5);
        let result = format_relative_time(Some(time));
        assert!(result.contains("mins ago") || result.contains("min ago"));
    }

    #[test]
    fn test_format_relative_time_hours() {
        let time = Utc::now() - chrono::Duration::hours(3);
        let result = format_relative_time(Some(time));
        assert!(result.contains("hours ago"));
    }

    #[test]
    fn test_format_relative_time_days() {
        let time = Utc::now() - chrono::Duration::days(2);
        let result = format_relative_time(Some(time));
        assert!(result.contains("days ago"));
    }

    #[test]
    fn test_format_relative_time_weeks() {
        let time = Utc::now() - chrono::Duration::weeks(2);
        let result = format_relative_time(Some(time));
        assert!(result.contains("weeks ago"));
    }
}
