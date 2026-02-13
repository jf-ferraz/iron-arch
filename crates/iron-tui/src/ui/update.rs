//! Update and sync view rendering

use crate::app::App;
use iron_core::RiskLevel;
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph, Wrap};

/// Render the update preview view
pub fn render_update_preview(frame: &mut Frame, area: Rect, app: &App) {
    let update_count = app.pending_update_count();
    let risk_level = app.update_risk_level();
    let updates = app.pending_updates_list();

    // Risk level styling
    let (risk_symbol, risk_color, risk_text) = match risk_level {
        RiskLevel::Low => ("●", Color::Green, "Safe to update"),
        RiskLevel::Medium => ("⚠", Color::Yellow, "Review recommended"),
        RiskLevel::High => ("⚠", Color::Red, "Attention required"),
        RiskLevel::Critical => ("✗", Color::Red, "Create snapshot first!"),
    };

    // Split into header and list
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(6), // Summary
            Constraint::Min(0),    // Package list
        ])
        .split(area);

    // Summary section
    let summary_text = vec![
        Line::from(vec![
            Span::raw("Status: "),
            Span::styled(
                format!("{} updates available", update_count),
                Style::default().fg(if update_count > 0 {
                    Color::Yellow
                } else {
                    Color::Green
                }),
            ),
        ]),
        Line::from(vec![
            Span::raw("Risk:   "),
            Span::styled(
                format!("{} {}", risk_symbol, risk_text),
                Style::default().fg(risk_color),
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("[r]", Style::default().fg(Color::Yellow)),
            Span::raw(" Refresh  "),
            Span::styled("[u]", Style::default().fg(Color::Yellow)),
            Span::raw(" Update  "),
            Span::styled("[Esc]", Style::default().fg(Color::Gray)),
            Span::raw(" Back"),
        ]),
    ];

    let summary_block = Block::default()
        .title(" System Update ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let summary_para = Paragraph::new(summary_text)
        .block(summary_block)
        .wrap(Wrap { trim: true });

    frame.render_widget(summary_para, layout[0]);

    // Package list section
    let items: Vec<ListItem> = updates
        .iter()
        .take(50) // Limit displayed items
        .map(|pkg| {
            let aur_marker = if pkg.is_aur { "[AUR] " } else { "" };
            let content = format!(
                "{}{}: {} -> {}",
                aur_marker, pkg.name, pkg.current_version, pkg.new_version
            );
            let style = if pkg.is_aur {
                Style::default().fg(Color::Magenta)
            } else if pkg.name.starts_with("linux") {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default()
            };
            ListItem::new(content).style(style)
        })
        .collect();

    let list_title = if updates.len() > 50 {
        format!(" Packages (showing 50 of {}) ", updates.len())
    } else {
        " Packages ".to_string()
    };

    let list_block = Block::default()
        .title(list_title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray));

    let list = List::new(items).block(list_block);

    frame.render_widget(list, layout[1]);
}

/// Render sync status view
pub fn render_sync(frame: &mut Frame, area: Rect, _app: &App) {
    let text = vec![
        Line::from("Git Sync Status"),
        Line::from(""),
        Line::from("Press [p] to push changes"),
        Line::from("Press [l] to pull changes"),
    ];

    let block = Block::default()
        .title(" Sync ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let para = Paragraph::new(text).block(block).wrap(Wrap { trim: true });

    frame.render_widget(para, area);
}
