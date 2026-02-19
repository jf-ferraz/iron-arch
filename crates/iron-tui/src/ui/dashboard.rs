//! Dashboard view rendering
//!
//! A clean, professional dashboard for the Iron TUI application.

use crate::app::{App, HealthStatus};
use crate::ui::utils::format_relative_time;
use chrono::Utc;
use ratatui::prelude::*;
use ratatui::widgets::{Block, Paragraph};
use crate::ui::theme;

// ─────────────────────────────────────────────────────────────────────────────
// Helper Functions
// ─────────────────────────────────────────────────────────────────────────────

/// Create a simple bordered block with title (uses theme)
fn simple_block(title: &str) -> Block<'_> {
    theme::themed_block(title, theme::MAUVE)
}

/// Create a mini progress bar string (uses theme)
fn progress_bar(current: usize, total: usize, width: usize) -> String {
    theme::mini_progress_bar(current, total, width)
}

// ─────────────────────────────────────────────────────────────────────────────
// Main Render Function
// ─────────────────────────────────────────────────────────────────────────────

/// Render the dashboard view
pub fn render_dashboard(frame: &mut Frame, area: Rect, app: &App) {
    // Main layout: left panel (58%) + right panel (42%)
    let main_columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(58), Constraint::Percentage(42)])
        .margin(1)
        .split(area);

    // Left column layout
    let left_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(7), // System Status
            Constraint::Length(6), // Quick Stats
            Constraint::Min(7),    // Quick Actions
        ])
        .split(main_columns[0]);

    // Right column layout
    let right_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(9), // Active Configuration
            Constraint::Min(5),    // Alerts & Notifications
        ])
        .split(main_columns[1]);

    // Render all panels
    render_system_status(frame, left_layout[0], app);
    render_quick_stats(frame, left_layout[1], app);
    render_quick_actions(frame, left_layout[2]);
    render_active_config(frame, right_layout[0], app);
    render_alerts(frame, right_layout[1], app);
}

// ─────────────────────────────────────────────────────────────────────────────
// Panel Renderers
// ─────────────────────────────────────────────────────────────────────────────

/// System Status panel - health overview with visual indicators
fn render_system_status(frame: &mut Frame, area: Rect, app: &App) {
    let block = simple_block("System Status");
    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Determine health status and styling
    let (icon, status_text, status_color, desc) = match app.system_health() {
        HealthStatus::Ok => ("[OK]", "Healthy", theme::GREEN, "All systems operational"),
        HealthStatus::Warning => ("[!!]", "Attention", theme::YELLOW, "Updates or issues pending"),
        HealthStatus::Error => ("[XX]", "Critical", theme::RED, "Action required"),
    };

    let packages = app.package_count();
    let updates = app.pending_update_count();

    // Build status display
    let status_line = Line::from(vec![
        Span::styled(format!(" {} ", icon), Style::default().fg(status_color).bold()),
        Span::styled(status_text, Style::default().fg(status_color).bold()),
        Span::styled(format!("  {}", desc), Style::default().fg(theme::SUBTEXT)),
    ]);

    let packages_line = Line::from(vec![
        Span::styled("   Packages   ", Style::default().fg(theme::SUBTEXT)),
        Span::styled(format!("{}", packages), Style::default().fg(theme::TEXT).bold()),
        Span::styled(" installed", Style::default().fg(theme::SUBTEXT)),
    ]);

    let updates_line = if updates > 0 {
        Line::from(vec![
            Span::styled("   Updates    ", Style::default().fg(theme::SUBTEXT)),
            Span::styled(format!("{}", updates), Style::default().fg(theme::YELLOW).bold()),
            Span::styled(" available", Style::default().fg(theme::SUBTEXT)),
        ])
    } else {
        Line::from(vec![
            Span::styled("   Updates    ", Style::default().fg(theme::SUBTEXT)),
            Span::styled("[OK] up to date", Style::default().fg(theme::GREEN)),
        ])
    };

    let content = vec![
        Line::from(""),
        status_line,
        Line::from(""),
        packages_line,
        updates_line,
    ];

    frame.render_widget(Paragraph::new(content), inner);
}

/// Quick Stats panel - maintenance timestamps
fn render_quick_stats(frame: &mut Frame, area: Rect, app: &App) {
    let block = simple_block("Maintenance");
    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Get maintenance times from state manager
    let (last_update, last_clean) = app
        .state_manager
        .as_ref()
        .map(|sm| {
            let m = sm.maintenance();
            (m.last_update, m.last_clean)
        })
        .unwrap_or((None, None));

    let update_str = format_relative_time(last_update);
    let clean_str = format_relative_time(last_clean);

    // Color code based on age
    let update_color = if last_update.is_none() {
        theme::OVERLAY
    } else {
        let days = last_update
            .map(|t| Utc::now().signed_duration_since(t).num_days())
            .unwrap_or(999);
        if days <= 1 {
            theme::GREEN
        } else if days <= 7 {
            theme::YELLOW
        } else {
            theme::RED
        }
    };

    let clean_color = if last_clean.is_none() {
        theme::OVERLAY
    } else {
        let days = last_clean
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

    let content = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("   Last Update   ", Style::default().fg(theme::SUBTEXT)),
            Span::styled(update_str, Style::default().fg(update_color)),
        ]),
        Line::from(vec![
            Span::styled("   Last Cleanup  ", Style::default().fg(theme::SUBTEXT)),
            Span::styled(clean_str, Style::default().fg(clean_color)),
        ]),
    ];

    frame.render_widget(Paragraph::new(content), inner);
}

/// Quick Actions panel - keyboard shortcuts in a clean grid
fn render_quick_actions(frame: &mut Frame, area: Rect) {
    let block = simple_block("Quick Actions");
    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Row 1: Navigation
    let row1 = Line::from(vec![
        Span::raw("  "),
        Span::styled("[b]", Style::default().fg(theme::MAUVE).bold()),
        Span::styled(" Bundles  ", Style::default().fg(theme::SUBTEXT)),
        Span::styled("[p]", Style::default().fg(theme::MAUVE).bold()),
        Span::styled(" Profiles  ", Style::default().fg(theme::SUBTEXT)),
        Span::styled("[m]", Style::default().fg(theme::MAUVE).bold()),
        Span::styled(" Modules", Style::default().fg(theme::SUBTEXT)),
    ]);

    // Row 2: Actions
    let row2 = Line::from(vec![
        Span::raw("  "),
        Span::styled("[u]", Style::default().fg(theme::MAUVE).bold()),
        Span::styled(" Update   ", Style::default().fg(theme::SUBTEXT)),
        Span::styled("[x]", Style::default().fg(theme::MAUVE).bold()),
        Span::styled(" Maintain  ", Style::default().fg(theme::SUBTEXT)),
        Span::styled("[l]", Style::default().fg(theme::MAUVE).bold()),
        Span::styled(" Cleanup", Style::default().fg(theme::SUBTEXT)),
    ]);

    // Row 3: Tools
    let row3 = Line::from(vec![
        Span::raw("  "),
        Span::styled("[y]", Style::default().fg(theme::MAUVE).bold()),
        Span::styled(" Sync     ", Style::default().fg(theme::SUBTEXT)),
        Span::styled("[s]", Style::default().fg(theme::MAUVE).bold()),
        Span::styled(" Settings  ", Style::default().fg(theme::SUBTEXT)),
        Span::styled("[?]", Style::default().fg(theme::MAUVE).bold()),
        Span::styled(" Help", Style::default().fg(theme::SUBTEXT)),
    ]);

    let content = vec![Line::from(""), row1, row2, row3];

    frame.render_widget(Paragraph::new(content), inner);
}

/// Active Configuration panel - current system config
fn render_active_config(frame: &mut Frame, area: Rect, app: &App) {
    let block = simple_block("Active Configuration");
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let bundle = app
        .active_bundle
        .as_ref()
        .map(|b| b.id.as_str())
        .unwrap_or("not set");

    let profile = app.active_profile.as_deref().unwrap_or("not set");
    let modules = app.enabled_module_count();
    let total_modules = app.modules.len();

    // Visual progress for modules
    let module_bar = progress_bar(modules, total_modules, 10);

    let content = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("  Bundle    ", Style::default().fg(theme::SUBTEXT)),
            Span::styled(
                bundle,
                Style::default()
                    .fg(if bundle == "not set" {
                        theme::OVERLAY
                    } else {
                        theme::TEXT
                    })
                    .bold(),
            ),
        ]),
        Line::from(vec![
            Span::styled("  Profile   ", Style::default().fg(theme::SUBTEXT)),
            Span::styled(
                profile,
                Style::default()
                    .fg(if profile == "not set" {
                        theme::OVERLAY
                    } else {
                        theme::TEXT
                    })
                    .bold(),
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Modules   ", Style::default().fg(theme::SUBTEXT)),
            Span::styled(module_bar, Style::default().fg(theme::MAUVE)),
            Span::styled(
                format!(" {}/{}", modules, total_modules),
                Style::default().fg(theme::TEXT),
            ),
        ]),
        Line::from(vec![
            Span::styled("  Pending   ", Style::default().fg(theme::SUBTEXT)),
            Span::styled(
                format!("{} updates", app.pending_update_count()),
                Style::default().fg(if app.pending_update_count() > 0 {
                    theme::YELLOW
                } else {
                    theme::GREEN
                }),
            ),
        ]),
    ];

    frame.render_widget(Paragraph::new(content), inner);
}

/// Alerts panel - notifications and warnings
fn render_alerts(frame: &mut Frame, area: Rect, app: &App) {
    let updates = app.pending_update_count();
    let has_alerts = updates > 0;

    let block = simple_block("Notifications");
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let mut content = vec![Line::from("")];

    if updates > 0 {
        content.push(Line::from(vec![
            Span::styled("  [!] ", Style::default().fg(theme::YELLOW)),
            Span::styled(
                format!("{} package updates available", updates),
                Style::default().fg(theme::YELLOW),
            ),
        ]));
        content.push(Line::from(vec![
            Span::styled("      Press ", Style::default().fg(theme::SUBTEXT)),
            Span::styled("[u]", Style::default().fg(theme::MAUVE).bold()),
            Span::styled(" to review updates", Style::default().fg(theme::SUBTEXT)),
        ]));
    }

    // Check for news requiring attention
    let news_count = app.arch_news.iter().filter(|n| n.requires_manual).count();
    if news_count > 0 {
        if updates > 0 {
            content.push(Line::from(""));
        }
        content.push(Line::from(vec![
            Span::styled("  [i] ", Style::default().fg(theme::PINK)),
            Span::styled(
                format!("{} Arch news requiring attention", news_count),
                Style::default().fg(theme::PINK),
            ),
        ]));
    }

    // If no alerts, show all-clear or onboarding nudge
    if !has_alerts && news_count == 0 {
        if app.active_bundle.is_none() && app.modules.is_empty() {
            content.push(Line::from(vec![
                Span::styled("  ", Style::default()),
                Span::styled("Press ", Style::default().fg(theme::SUBTEXT)),
                Span::styled("[w]", Style::default().fg(theme::MAUVE).bold()),
                Span::styled(" to get started", Style::default().fg(theme::SUBTEXT)),
            ]));
        } else {
            content.push(Line::from(vec![
                Span::styled("  [OK] ", Style::default().fg(theme::GREEN)),
                Span::styled(
                    "All clear - no pending notifications",
                    Style::default().fg(theme::GREEN),
                ),
            ]));
        }
    }

    frame.render_widget(Paragraph::new(content), inner);
}
