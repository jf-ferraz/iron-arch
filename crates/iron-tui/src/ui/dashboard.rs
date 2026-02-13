//! Dashboard view rendering

use crate::app::{App, HealthStatus};
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

/// Render the dashboard view
pub fn render_dashboard(frame: &mut Frame, area: Rect, app: &App) {
    // Split into two columns
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    // Left column: System Health + Maintenance
    let left_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(6), // System Health
            Constraint::Length(5), // Maintenance
            Constraint::Min(0),    // Quick Actions
        ])
        .split(columns[0]);

    // System Health panel
    let health_status = match app.system_health() {
        HealthStatus::Ok => ("● System OK", Style::default().fg(Color::Green)),
        HealthStatus::Warning => ("⚠ Warning", Style::default().fg(Color::Yellow)),
        HealthStatus::Error => ("✗ Error", Style::default().fg(Color::Red)),
    };

    let health_text = vec![
        Line::from(Span::styled(health_status.0, health_status.1)),
        Line::from(format!("● {} packages installed", app.package_count())),
        Line::from("● No conflicts detected"),
    ];

    let health_block = Block::default()
        .title(" System Health ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let health_para = Paragraph::new(health_text)
        .block(health_block)
        .wrap(Wrap { trim: true });

    frame.render_widget(health_para, left_layout[0]);

    // Maintenance panel
    let maintenance_text = vec![
        Line::from("Last Update: 3 days ago"),
        Line::from("Last Clean: 7 days ago"),
    ];

    let maintenance_block = Block::default()
        .title(" Maintenance ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let maintenance_para = Paragraph::new(maintenance_text)
        .block(maintenance_block)
        .wrap(Wrap { trim: true });

    frame.render_widget(maintenance_para, left_layout[1]);

    // Quick Actions panel
    let actions_text = vec![
        Line::from(vec![
            Span::styled("[b]", Style::default().fg(Color::Yellow)),
            Span::raw(" Bundles  "),
            Span::styled("[p]", Style::default().fg(Color::Yellow)),
            Span::raw(" Profiles  "),
            Span::styled("[m]", Style::default().fg(Color::Yellow)),
            Span::raw(" Modules"),
        ]),
        Line::from(vec![
            Span::styled("[u]", Style::default().fg(Color::Yellow)),
            Span::raw(" Updates  "),
            Span::styled("[s]", Style::default().fg(Color::Yellow)),
            Span::raw(" Settings  "),
            Span::styled("[?]", Style::default().fg(Color::Yellow)),
            Span::raw(" Help"),
        ]),
    ];

    let actions_block = Block::default()
        .title(" Quick Actions ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let actions_para = Paragraph::new(actions_text)
        .block(actions_block)
        .wrap(Wrap { trim: true });

    frame.render_widget(actions_para, left_layout[2]);

    // Right column: Active Config + Alerts
    let right_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(8), // Active Config
            Constraint::Min(0),    // Alerts
        ])
        .split(columns[1]);

    // Active Config panel
    let bundle_name = app
        .active_bundle
        .as_ref()
        .map(|b| b.id.as_str())
        .unwrap_or("None");

    let profile_name = app.active_profile.as_deref().unwrap_or("None");

    let config_text = vec![
        Line::from(format!("Bundle: {}", bundle_name)),
        Line::from(format!("Profile: {}", profile_name)),
        Line::from(format!("Modules: {} enabled", app.enabled_module_count())),
        Line::from(format!("Pending: {} updates", app.pending_update_count())),
    ];

    let config_block = Block::default()
        .title(" Active Configuration ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let config_para = Paragraph::new(config_text)
        .block(config_block)
        .wrap(Wrap { trim: true });

    frame.render_widget(config_para, right_layout[0]);

    // Alerts panel
    let mut alerts_text = vec![];

    if app.pending_update_count() > 0 {
        alerts_text.push(Line::from(Span::styled(
            format!("⚠ {} updates available", app.pending_update_count()),
            Style::default().fg(Color::Yellow),
        )));
    }

    if alerts_text.is_empty() {
        alerts_text.push(Line::from(Span::styled(
            "✓ No alerts",
            Style::default().fg(Color::Green),
        )));
    }

    let alerts_block = Block::default()
        .title(" Alerts ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let alerts_para = Paragraph::new(alerts_text)
        .block(alerts_block)
        .wrap(Wrap { trim: true });

    frame.render_widget(alerts_para, right_layout[1]);
}
