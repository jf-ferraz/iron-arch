//! Recovery view — backup, export, and restoration

use crate::app::App;
use crate::ui::theme;
use crate::ui::utils::format_relative_time;
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

/// Render the Recovery screen
pub fn render_recovery(frame: &mut Frame, area: Rect, app: &App) {
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Title bar
            Constraint::Length(7), // Status panel
            Constraint::Min(0),    // Actions panel
            Constraint::Length(3), // Footer hints
        ])
        .split(area);

    // Title
    let title_block = Block::default()
        .borders(Borders::BOTTOM)
        .border_style(Style::default().fg(theme::OVERLAY));
    let title_para = Paragraph::new(Line::from(vec![
        Span::raw("  "),
        Span::styled("Recovery", Style::default().fg(theme::MAUVE).bold()),
        Span::raw("  "),
        Span::styled(
            "Backup, export, and restore your configuration",
            Style::default().fg(theme::SUBTEXT),
        ),
    ]))
    .block(title_block);
    frame.render_widget(title_para, layout[0]);

    // Status panel
    render_recovery_status(frame, layout[1], app);

    // Actions panel
    render_recovery_actions(frame, layout[2]);

    // Footer hints
    let hints = Paragraph::new(Line::from(vec![
        Span::raw("  "),
        Span::styled(
            " [g] ",
            Style::default().fg(Color::Black).bg(theme::MAUVE).bold(),
        ),
        Span::styled(" install.sh  ", Style::default().fg(theme::SUBTEXT)),
        Span::styled(
            " [e] ",
            Style::default().fg(Color::Black).bg(theme::MAUVE).bold(),
        ),
        Span::styled(" Export  ", Style::default().fg(theme::SUBTEXT)),
        Span::styled(
            " [s] ",
            Style::default().fg(Color::Black).bg(theme::MAUVE).bold(),
        ),
        Span::styled(" Snapshot  ", Style::default().fg(theme::SUBTEXT)),
        Span::styled(
            " [Esc] ",
            Style::default().fg(Color::Black).bg(theme::MAUVE).bold(),
        ),
        Span::styled(" Back", Style::default().fg(theme::SUBTEXT)),
    ]))
    .block(
        Block::default()
            .borders(Borders::TOP)
            .border_style(Style::default().fg(theme::OVERLAY)),
    );
    frame.render_widget(hints, layout[3]);
}

fn render_recovery_status(frame: &mut Frame, area: Rect, app: &App) {
    let backup_str = format_relative_time(app.last_backup);
    let backup_color = if app.last_backup.is_none() {
        theme::OVERLAY
    } else {
        use chrono::Utc;
        let days = app
            .last_backup
            .map(|t| Utc::now().signed_duration_since(t).num_days())
            .unwrap_or(999);
        if days <= 7 {
            theme::GREEN
        } else if days <= 30 {
            theme::YELLOW
        } else {
            theme::RED
        }
    };

    let bundle_id = app
        .active_bundle
        .as_ref()
        .map(|b| b.id.as_str())
        .unwrap_or("none");
    let profile_id = app.active_profile.as_deref().unwrap_or("none");

    let content = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("  Last Backup  ", Style::default().fg(theme::SUBTEXT)),
            Span::styled(backup_str, Style::default().fg(backup_color)),
        ]),
        Line::from(vec![
            Span::styled("  Bundle       ", Style::default().fg(theme::SUBTEXT)),
            Span::styled(bundle_id, Style::default().fg(theme::TEXT)),
        ]),
        Line::from(vec![
            Span::styled("  Profile      ", Style::default().fg(theme::SUBTEXT)),
            Span::styled(profile_id, Style::default().fg(theme::TEXT)),
        ]),
        Line::from(vec![
            Span::styled("  Modules      ", Style::default().fg(theme::SUBTEXT)),
            Span::styled(
                format!("{} available", app.modules.len()),
                Style::default().fg(theme::TEXT),
            ),
        ]),
    ];

    let block = theme::themed_block("Current State", theme::MAUVE);
    frame.render_widget(
        Paragraph::new(content)
            .block(block)
            .wrap(Wrap { trim: true }),
        area,
    );
}

fn render_recovery_actions(frame: &mut Frame, area: Rect) {
    let lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::raw("  "),
            Span::styled("[g]", Style::default().fg(theme::MAUVE).bold()),
            Span::styled(
                " Generate install.sh   — Bootstraps a fresh machine from your config",
                Style::default().fg(theme::TEXT),
            ),
        ]),
        Line::from(vec![
            Span::raw("  "),
            Span::styled("[e]", Style::default().fg(theme::MAUVE).bold()),
            Span::styled(
                " Export config bundle  — Creates a portable archive of your configuration",
                Style::default().fg(theme::TEXT),
            ),
        ]),
        Line::from(vec![
            Span::raw("  "),
            Span::styled("[i]", Style::default().fg(theme::MAUVE).bold()),
            Span::styled(
                " Import from backup    — Restores configuration from an archive",
                Style::default().fg(theme::TEXT),
            ),
        ]),
        Line::from(vec![
            Span::raw("  "),
            Span::styled("[r]", Style::default().fg(theme::MAUVE).bold()),
            Span::styled(
                " Recovery wizard       — Step-by-step system restoration guide",
                Style::default().fg(theme::TEXT),
            ),
        ]),
        Line::from(vec![
            Span::raw("  "),
            Span::styled("[s]", Style::default().fg(theme::MAUVE).bold()),
            Span::styled(
                " Create snapshot now   — Take a timeshift/snapper snapshot immediately",
                Style::default().fg(theme::TEXT),
            ),
        ]),
    ];

    let block = theme::themed_block("Recovery Actions", theme::MAUVE);
    frame.render_widget(
        Paragraph::new(lines).block(block).wrap(Wrap { trim: true }),
        area,
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    fn create_test_terminal() -> Terminal<TestBackend> {
        let backend = TestBackend::new(80, 24);
        Terminal::new(backend).unwrap()
    }

    #[test]
    fn test_render_recovery_no_panic() {
        let mut terminal = create_test_terminal();
        let app = App::default();

        terminal
            .draw(|f| {
                render_recovery(f, f.area(), &app);
            })
            .unwrap();
    }

    #[test]
    fn test_render_recovery_with_backup() {
        let mut terminal = create_test_terminal();
        let mut app = App::default();
        app.last_backup = Some(chrono::Utc::now());

        terminal
            .draw(|f| {
                render_recovery(f, f.area(), &app);
            })
            .unwrap();
    }

    #[test]
    fn test_render_recovery_old_backup() {
        let mut terminal = create_test_terminal();
        let mut app = App::default();
        app.last_backup = Some(chrono::Utc::now() - chrono::Duration::days(45));

        terminal
            .draw(|f| {
                render_recovery(f, f.area(), &app);
            })
            .unwrap();
    }

    #[test]
    fn test_render_recovery_no_backup() {
        let mut terminal = create_test_terminal();
        let mut app = App::default();
        app.last_backup = None;

        terminal
            .draw(|f| {
                render_recovery(f, f.area(), &app);
            })
            .unwrap();
    }

    #[test]
    fn test_render_recovery_with_active_bundle() {
        let mut terminal = create_test_terminal();
        let mut app = App::default();
        app.active_bundle = Some(iron_core::Bundle {
            id: "hyprland".to_string(),
            name: "Hyprland".to_string(),
            description: None,
            bundle_type: iron_core::BundleType::WaylandCompositor,
            packages: vec![],
            aur_packages: vec![],
            profiles: vec![],
            default_profile: None,
            conflicts: vec![],
            services: vec![],
            post_install: None,
        });

        terminal
            .draw(|f| {
                render_recovery(f, f.area(), &app);
            })
            .unwrap();
    }

    #[test]
    fn test_render_recovery_footer_hints() {
        let mut terminal = create_test_terminal();
        let app = App::default();

        // Render and verify no panic (footer rendered within render_recovery)
        terminal
            .draw(|f| {
                render_recovery(f, f.area(), &app);
            })
            .unwrap();
    }
}
