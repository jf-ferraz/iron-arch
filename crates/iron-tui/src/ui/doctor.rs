//! Doctor view — system health checks

use crate::app::App;
use crate::ui::theme;
use iron_core::snapshot::SnapshotBackend;
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

/// Render the Doctor health-check screen
pub fn render_doctor(frame: &mut Frame, area: Rect, app: &App) {
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Title bar
            Constraint::Min(0),    // Checks
            Constraint::Length(3), // Footer hints
        ])
        .split(area);

    // Title
    let title_block = Block::default()
        .borders(Borders::BOTTOM)
        .border_style(Style::default().fg(theme::OVERLAY));
    let title_para = Paragraph::new(Line::from(vec![
        Span::raw("  "),
        Span::styled("System Doctor", Style::default().fg(theme::MAUVE).bold()),
        Span::raw("  "),
        Span::styled("Health checks for your Iron installation", Style::default().fg(theme::SUBTEXT)),
    ]))
    .block(title_block);
    frame.render_widget(title_para, layout[0]);

    // Health checks
    let checks = build_health_checks(app);
    let mut lines: Vec<Line> = vec![Line::from("")];
    for (icon, label, detail, color) in checks {
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled(icon, Style::default().fg(color).bold()),
            Span::raw("  "),
            Span::styled(label, Style::default().fg(theme::TEXT).bold()),
            Span::styled(format!("  {}", detail), Style::default().fg(theme::SUBTEXT)),
        ]));
    }

    let checks_block = theme::themed_block("Health Checks", theme::MAUVE);
    let checks_para = Paragraph::new(lines).block(checks_block).wrap(Wrap { trim: true });
    frame.render_widget(checks_para, layout[1]);

    // Footer hints
    let hints = Paragraph::new(Line::from(vec![
        Span::raw("  "),
        Span::styled(" [r] ", Style::default().fg(Color::Black).bg(theme::MAUVE).bold()),
        Span::styled(" Re-run  ", Style::default().fg(theme::SUBTEXT)),
        Span::styled(" [Esc] ", Style::default().fg(Color::Black).bg(theme::MAUVE).bold()),
        Span::styled(" Back", Style::default().fg(theme::SUBTEXT)),
    ]))
    .block(Block::default().borders(Borders::TOP).border_style(Style::default().fg(theme::OVERLAY)));
    frame.render_widget(hints, layout[2]);
}

/// Build list of health check results from current app state.
/// Returns (icon, label, detail, color).
fn build_health_checks(app: &App) -> Vec<(&'static str, &'static str, String, Color)> {
    let mut checks = Vec::new();

    // 1. Host configured
    let host_ok = app.current_host.is_some();
    checks.push((
        if host_ok { "[OK]" } else { "[!!]" },
        "Host configured",
        app.current_host
            .clone()
            .unwrap_or_else(|| "No host set — run setup wizard".to_string()),
        if host_ok { theme::GREEN } else { theme::YELLOW },
    ));

    // 2. Active bundle
    let bundle_ok = app.active_bundle.is_some();
    checks.push((
        if bundle_ok { "[OK]" } else { "[!!]" },
        "Active bundle",
        app.active_bundle
            .as_ref()
            .map(|b| b.id.clone())
            .unwrap_or_else(|| "No bundle active — press [b] to select one".to_string()),
        if bundle_ok { theme::GREEN } else { theme::YELLOW },
    ));

    // 3. Modules available
    let modules_ok = !app.modules.is_empty();
    checks.push((
        if modules_ok { "[OK]" } else { "[!!]" },
        "Modules discovered",
        format!(
            "{} module(s) found",
            app.modules.len()
        ),
        if modules_ok { theme::GREEN } else { theme::YELLOW },
    ));

    // 4. Pending updates
    let updates = app.pending_update_count();
    checks.push((
        if updates == 0 { "[OK]" } else { "[!!]" },
        "System updates",
        if updates == 0 {
            "Up to date".to_string()
        } else {
            format!("{} update(s) available — press [u] to review", updates)
        },
        if updates == 0 { theme::GREEN } else { theme::YELLOW },
    ));

    // 5. Arch news
    let news = app.arch_news.iter().filter(|n| n.requires_manual).count();
    checks.push((
        if news == 0 { "[OK]" } else { "[!!]" },
        "Arch news",
        if news == 0 {
            "No manual intervention required".to_string()
        } else {
            format!("{} item(s) require manual attention", news)
        },
        if news == 0 { theme::GREEN } else { theme::PINK },
    ));

    // 6. Active profile
    let profile_ok = app.active_profile.is_some();
    checks.push((
        if profile_ok { "[OK]" } else { "[ ]" },
        "Active profile",
        app.active_profile
            .clone()
            .unwrap_or_else(|| "None (optional)".to_string()),
        if profile_ok { theme::GREEN } else { theme::OVERLAY },
    ));

    // 7. Snapshot backend
    let (snap_icon, snap_detail, snap_color) = match app.snapshot_backend {
        SnapshotBackend::Timeshift => (
            "[OK]",
            "Timeshift detected — snapshots available".to_string(),
            theme::GREEN,
        ),
        SnapshotBackend::Snapper => (
            "[OK]",
            "Snapper detected — snapshots available".to_string(),
            theme::GREEN,
        ),
        SnapshotBackend::None => (
            "[!!]",
            "No snapshot tool detected — install timeshift or snapper for rollback support".to_string(),
            theme::YELLOW,
        ),
    };
    checks.push((snap_icon, "Snapshot backend", snap_detail, snap_color));

    checks
}
