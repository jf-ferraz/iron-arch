//! Dashboard view rendering
//!
//! A clean, professional dashboard for the Iron TUI application.

use crate::app::{App, HealthStatus};
use chrono::{DateTime, Utc};
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph};

// ─────────────────────────────────────────────────────────────────────────────
// Helper Functions
// ─────────────────────────────────────────────────────────────────────────────

/// Format a DateTime as a relative time string (e.g., "3 days ago", "never")
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

/// Create a simple bordered block with title
fn simple_block(title: &str) -> Block<'_> {
    Block::default()
        .title(format!(" {} ", title))
        .title_style(Style::default().fg(Color::Cyan).bold())
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray))
}

/// Create a mini progress bar string
fn progress_bar(current: usize, total: usize, width: usize) -> String {
    if total == 0 {
        return "-".repeat(width);
    }
    let filled = (current * width) / total.max(1);
    let empty = width.saturating_sub(filled);
    format!("{}{}", "#".repeat(filled), "-".repeat(empty))
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
        HealthStatus::Ok => ("[OK]", "Healthy", Color::Green, "All systems operational"),
        HealthStatus::Warning => ("[!!]", "Attention", Color::Yellow, "Updates or issues pending"),
        HealthStatus::Error => ("[XX]", "Critical", Color::Red, "Action required"),
    };

    let packages = app.package_count();
    let updates = app.pending_update_count();

    // Build status display
    let status_line = Line::from(vec![
        Span::styled(format!(" {} ", icon), Style::default().fg(status_color).bold()),
        Span::styled(status_text, Style::default().fg(status_color).bold()),
        Span::styled(format!("  {}", desc), Style::default().fg(Color::Gray)),
    ]);

    let packages_line = Line::from(vec![
        Span::styled("   Packages   ", Style::default().fg(Color::Gray)),
        Span::styled(format!("{}", packages), Style::default().fg(Color::White).bold()),
        Span::styled(" installed", Style::default().fg(Color::Gray)),
    ]);

    let updates_line = if updates > 0 {
        Line::from(vec![
            Span::styled("   Updates    ", Style::default().fg(Color::Gray)),
            Span::styled(format!("{}", updates), Style::default().fg(Color::Yellow).bold()),
            Span::styled(" available", Style::default().fg(Color::Gray)),
        ])
    } else {
        Line::from(vec![
            Span::styled("   Updates    ", Style::default().fg(Color::Gray)),
            Span::styled("[OK] up to date", Style::default().fg(Color::Green)),
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
        Color::DarkGray
    } else {
        let days = last_update
            .map(|t| Utc::now().signed_duration_since(t).num_days())
            .unwrap_or(999);
        if days <= 1 {
            Color::Green
        } else if days <= 7 {
            Color::Yellow
        } else {
            Color::Red
        }
    };

    let clean_color = if last_clean.is_none() {
        Color::DarkGray
    } else {
        let days = last_clean
            .map(|t| Utc::now().signed_duration_since(t).num_days())
            .unwrap_or(999);
        if days <= 7 {
            Color::Green
        } else if days <= 30 {
            Color::Yellow
        } else {
            Color::Red
        }
    };

    let content = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("   Last Update   ", Style::default().fg(Color::Gray)),
            Span::styled(update_str, Style::default().fg(update_color)),
        ]),
        Line::from(vec![
            Span::styled("   Last Cleanup  ", Style::default().fg(Color::Gray)),
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
        Span::styled("[b]", Style::default().fg(Color::Cyan).bold()),
        Span::styled(" Bundles  ", Style::default().fg(Color::Gray)),
        Span::styled("[p]", Style::default().fg(Color::Cyan).bold()),
        Span::styled(" Profiles  ", Style::default().fg(Color::Gray)),
        Span::styled("[m]", Style::default().fg(Color::Cyan).bold()),
        Span::styled(" Modules", Style::default().fg(Color::Gray)),
    ]);

    // Row 2: Actions
    let row2 = Line::from(vec![
        Span::raw("  "),
        Span::styled("[u]", Style::default().fg(Color::Cyan).bold()),
        Span::styled(" Update   ", Style::default().fg(Color::Gray)),
        Span::styled("[x]", Style::default().fg(Color::Cyan).bold()),
        Span::styled(" Maintain  ", Style::default().fg(Color::Gray)),
        Span::styled("[l]", Style::default().fg(Color::Cyan).bold()),
        Span::styled(" Cleanup", Style::default().fg(Color::Gray)),
    ]);

    // Row 3: Tools
    let row3 = Line::from(vec![
        Span::raw("  "),
        Span::styled("[y]", Style::default().fg(Color::Cyan).bold()),
        Span::styled(" Sync     ", Style::default().fg(Color::Gray)),
        Span::styled("[s]", Style::default().fg(Color::Cyan).bold()),
        Span::styled(" Settings  ", Style::default().fg(Color::Gray)),
        Span::styled("[?]", Style::default().fg(Color::Cyan).bold()),
        Span::styled(" Help", Style::default().fg(Color::Gray)),
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
        .unwrap_or("-");

    let profile = app.active_profile.as_deref().unwrap_or("-");
    let modules = app.enabled_module_count();
    let total_modules = app.modules.len();

    // Visual progress for modules
    let module_bar = progress_bar(modules, total_modules, 10);

    let content = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("  Bundle    ", Style::default().fg(Color::Gray)),
            Span::styled(
                bundle,
                Style::default()
                    .fg(if bundle == "-" {
                        Color::DarkGray
                    } else {
                        Color::White
                    })
                    .bold(),
            ),
        ]),
        Line::from(vec![
            Span::styled("  Profile   ", Style::default().fg(Color::Gray)),
            Span::styled(
                profile,
                Style::default()
                    .fg(if profile == "-" {
                        Color::DarkGray
                    } else {
                        Color::White
                    })
                    .bold(),
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Modules   ", Style::default().fg(Color::Gray)),
            Span::styled(module_bar, Style::default().fg(Color::Cyan)),
            Span::styled(
                format!(" {}/{}", modules, total_modules),
                Style::default().fg(Color::White),
            ),
        ]),
        Line::from(vec![
            Span::styled("  Pending   ", Style::default().fg(Color::Gray)),
            Span::styled(
                format!("{} updates", app.pending_update_count()),
                Style::default().fg(if app.pending_update_count() > 0 {
                    Color::Yellow
                } else {
                    Color::Green
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
            Span::styled("  [!] ", Style::default().fg(Color::Yellow)),
            Span::styled(
                format!("{} package updates available", updates),
                Style::default().fg(Color::Yellow),
            ),
        ]));
        content.push(Line::from(vec![
            Span::styled("      Press ", Style::default().fg(Color::Gray)),
            Span::styled("[u]", Style::default().fg(Color::Cyan).bold()),
            Span::styled(" to review updates", Style::default().fg(Color::Gray)),
        ]));
    }

    // Check for news requiring attention
    let news_count = app.arch_news.iter().filter(|n| n.requires_manual).count();
    if news_count > 0 {
        if updates > 0 {
            content.push(Line::from(""));
        }
        content.push(Line::from(vec![
            Span::styled("  [i] ", Style::default().fg(Color::Magenta)),
            Span::styled(
                format!("{} Arch news requiring attention", news_count),
                Style::default().fg(Color::Magenta),
            ),
        ]));
    }

    // If no alerts
    if !has_alerts && news_count == 0 {
        content.push(Line::from(vec![
            Span::styled("  [OK] ", Style::default().fg(Color::Green)),
            Span::styled(
                "All clear - no pending notifications",
                Style::default().fg(Color::Green),
            ),
        ]));
    }

    frame.render_widget(Paragraph::new(content), inner);
}
