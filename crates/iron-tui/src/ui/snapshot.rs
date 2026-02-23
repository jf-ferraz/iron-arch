//! Snapshot Timeline View
//!
//! F2-007: Visual timeline of snapshots with restore action.

use crate::app::App;
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
};

/// Render the snapshot timeline view.
pub fn render_snapshots(frame: &mut Frame, area: Rect, app: &App) {
    let chunks = Layout::vertical([Constraint::Min(5), Constraint::Length(3)]).split(area);

    render_snapshot_list(frame, chunks[0], app);
    render_snapshot_status(frame, chunks[1], app);
}

fn render_snapshot_list(frame: &mut Frame, area: Rect, app: &App) {
    let block = Block::default()
        .title(" Snapshots ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    if app.snapshot_list.is_empty() {
        let msg = Paragraph::new(vec![
            Line::from(""),
            Line::from(Span::styled(
                "  No snapshots found.",
                Style::default().fg(Color::DarkGray),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "  Create one: iron snapshot create <name>",
                Style::default().fg(Color::DarkGray),
            )),
            Line::from(Span::styled(
                "  Or press [c] to create from here.",
                Style::default().fg(Color::DarkGray),
            )),
        ])
        .block(block);
        frame.render_widget(msg, area);
        return;
    }

    let items: Vec<ListItem> = app
        .snapshot_list
        .iter()
        .enumerate()
        .map(|(i, snap)| {
            let auto_badge = if snap.auto {
                Span::styled(" [auto]", Style::default().fg(Color::DarkGray))
            } else {
                Span::styled(" [manual]", Style::default().fg(Color::Green))
            };

            let date = snap.timestamp.format("%Y-%m-%d %H:%M").to_string();
            let modules = format!("{} modules", snap.active_modules.len());
            let pkgs = format!("{} pkgs", snap.explicit_packages.len());

            let name_style = if i == app.selected_index {
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };

            let line = Line::from(vec![
                Span::styled(format!("  {:<24}", truncate(&snap.name, 23)), name_style),
                Span::styled(format!("{:<18}", date), Style::default().fg(Color::Yellow)),
                Span::styled(format!("{:<14}", modules), Style::default().fg(Color::Blue)),
                Span::styled(pkgs, Style::default().fg(Color::Magenta)),
                auto_badge,
            ]);

            ListItem::new(line)
        })
        .collect();

    let list = List::new(items).block(block).highlight_style(
        Style::default()
            .bg(Color::DarkGray)
            .add_modifier(Modifier::BOLD),
    );

    frame.render_widget(list, area);
}

fn render_snapshot_status(frame: &mut Frame, area: Rect, app: &App) {
    let total = app.snapshot_list.len();
    let auto_count = app.snapshot_list.iter().filter(|s| s.auto).count();
    let manual_count = total - auto_count;

    let status = Line::from(vec![
        Span::styled(
            format!(" {} total", total),
            Style::default().fg(Color::White),
        ),
        Span::styled(" | ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            format!("{} manual", manual_count),
            Style::default().fg(Color::Green),
        ),
        Span::styled(" | ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            format!("{} auto", auto_count),
            Style::default().fg(Color::DarkGray),
        ),
    ]);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray));

    let paragraph = Paragraph::new(status).block(block);
    frame.render_widget(paragraph, area);
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let truncated: String = s.chars().take(max.saturating_sub(3)).collect();
        format!("{}...", truncated)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::{Terminal, backend::TestBackend};

    fn test_app() -> App {
        App::default()
    }

    #[test]
    fn test_render_snapshots_empty_no_panic() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let app = test_app();
        terminal
            .draw(|f| render_snapshots(f, f.area(), &app))
            .unwrap();
    }

    #[test]
    fn test_render_snapshots_with_data_no_panic() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut app = test_app();

        app.snapshot_list = vec![
            iron_core::services::snapshot_service::SnapshotRecord {
                id: "abc".to_string(),
                name: "pre-kde".to_string(),
                timestamp: chrono::Utc::now(),
                active_bundle: Some("hyprland".to_string()),
                active_modules: vec!["nvim".into(), "fish".into()],
                explicit_packages: vec!["neovim".into()],
                ..Default::default()
            },
            iron_core::services::snapshot_service::SnapshotRecord {
                id: "def".to_string(),
                name: "auto-pre-apply-20260222".to_string(),
                timestamp: chrono::Utc::now(),
                auto: true,
                ..Default::default()
            },
        ];

        terminal
            .draw(|f| render_snapshots(f, f.area(), &app))
            .unwrap();
    }

    #[test]
    fn test_truncate_snapshot_view() {
        assert_eq!(truncate("short", 10), "short");
        assert_eq!(truncate("a-really-long-name-here", 10), "a-reall...");
    }
}
