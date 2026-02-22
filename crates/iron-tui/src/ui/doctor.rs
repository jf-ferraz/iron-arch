//! Doctor view — system health checks
//!
//! Uses the shared `DoctorService` from iron-core for system-level health
//! checks, supplemented with TUI-specific state checks (updates, news, profile).

use crate::app::App;
use crate::ui::theme;
use iron_core::detect_snapshot_backend;
use iron_core::services::doctor::{
    CheckStatus, DefaultDoctorService, DoctorConfig, DoctorService, HealthCheck,
};
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
        Span::styled(
            "Health checks for your Iron installation",
            Style::default().fg(theme::SUBTEXT),
        ),
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
    let checks_para = Paragraph::new(lines)
        .block(checks_block)
        .wrap(Wrap { trim: true });
    frame.render_widget(checks_para, layout[1]);

    // Footer hints
    let hints = Paragraph::new(Line::from(vec![
        Span::raw("  "),
        Span::styled(
            " [r] ",
            Style::default().fg(Color::Black).bg(theme::MAUVE).bold(),
        ),
        Span::styled(" Re-run  ", Style::default().fg(theme::SUBTEXT)),
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
    frame.render_widget(hints, layout[2]);
}

/// Map `CheckStatus` to a TUI colour.
fn color_for(status: CheckStatus) -> Color {
    match status {
        CheckStatus::Pass => theme::GREEN,
        CheckStatus::Warn => theme::YELLOW,
        CheckStatus::Fail => theme::PINK,
    }
}

/// Map `CheckStatus` to an icon string.
fn icon_for(status: CheckStatus) -> &'static str {
    match status {
        CheckStatus::Pass => "[OK]",
        CheckStatus::Warn | CheckStatus::Fail => "[!!]",
    }
}

/// Human-readable label for a HealthCheck name.
fn label_for(name: &str) -> &'static str {
    match name {
        "state_file" => "State file",
        "directories" => "Directory structure",
        "current_host" => "Host configured",
        "git" => "Git repository",
        "tools" => "External tools",
        "packages" => "Package installation",
        "snapshot" => "Snapshot backend",
        "secrets" => "Secrets",
        "symlinks" => "Symlink integrity",
        "services" => "Service availability",
        _ => "Check",
    }
}

/// Convert a shared `HealthCheck` into (icon, label, detail, color).
fn map_check(check: &HealthCheck) -> (&'static str, &'static str, String, Color) {
    (
        icon_for(check.status),
        label_for(&check.name),
        check.message.clone(),
        color_for(check.status),
    )
}

/// Build list of health check results from current app state.
/// Returns (icon, label, detail, color).
///
/// Runs the shared `DoctorService` for system-level checks, then appends
/// TUI-specific items (pending updates, arch news, active profile).
fn build_health_checks(app: &App) -> Vec<(&'static str, &'static str, String, Color)> {
    let mut out: Vec<(&str, &str, String, Color)> = Vec::new();

    // --- System-level checks from DoctorService ---
    let config = DoctorConfig {
        root: app.config_dir.clone(),
        current_host: app.current_host.clone(),
        active_bundle: app.active_bundle.as_ref().map(|b| b.id.clone()),
        snapshot_backend: detect_snapshot_backend(),
    };

    if let Ok(report) = DefaultDoctorService::new(config).check_all() {
        for check in &report.checks {
            let (icon, label, detail, color) = map_check(check);
            out.push((icon, label, detail, color));
        }
    }

    // --- TUI-specific checks ---

    // Pending updates
    let updates = app.pending_update_count();
    out.push((
        if updates == 0 { "[OK]" } else { "[!!]" },
        "System updates",
        if updates == 0 {
            "Up to date".to_string()
        } else {
            format!("{} update(s) available — press [u] to review", updates)
        },
        if updates == 0 {
            theme::GREEN
        } else {
            theme::YELLOW
        },
    ));

    // Arch news
    let news = app.arch_news.iter().filter(|n| n.requires_manual).count();
    out.push((
        if news == 0 { "[OK]" } else { "[!!]" },
        "Arch news",
        if news == 0 {
            "No manual intervention required".to_string()
        } else {
            format!("{} item(s) require manual attention", news)
        },
        if news == 0 { theme::GREEN } else { theme::PINK },
    ));

    // Active profile
    let profile_ok = app.active_profile.is_some();
    out.push((
        if profile_ok { "[OK]" } else { "[ ]" },
        "Active profile",
        app.active_profile
            .clone()
            .unwrap_or_else(|| "None (optional)".to_string()),
        if profile_ok {
            theme::GREEN
        } else {
            theme::OVERLAY
        },
    ));

    out
}
