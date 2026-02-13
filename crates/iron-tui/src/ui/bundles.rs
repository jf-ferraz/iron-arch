//! Bundle view rendering

use crate::app::App;
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap};

/// Render the bundles list view
pub fn render_bundles(frame: &mut Frame, area: Rect, app: &App) {
    let items: Vec<ListItem> = app
        .bundles
        .iter()
        .enumerate()
        .map(|(i, bundle)| {
            let is_active = app
                .active_bundle
                .as_ref()
                .map(|b| b.id == bundle.id)
                .unwrap_or(false);

            let status = if is_active { "●" } else { "○" };
            let desc = bundle.description.as_deref().unwrap_or("");
            let content = format!("{} {} - {}", status, bundle.id, desc);

            let style = if i == app.selected_index {
                Style::default().bg(Color::DarkGray).fg(Color::White)
            } else if is_active {
                Style::default().fg(Color::Green)
            } else {
                Style::default()
            };

            ListItem::new(content).style(style)
        })
        .collect();

    let block = Block::default()
        .title(" Bundles ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let list = List::new(items).block(block);

    let mut state = ListState::default();
    state.select(Some(app.selected_index));

    frame.render_stateful_widget(list, area, &mut state);
}

/// Render bundle detail view
pub fn render_bundle_detail(frame: &mut Frame, area: Rect, app: &App) {
    let bundle = match app.selected_bundle() {
        Some(b) => b,
        None => {
            let block = Block::default()
                .title(" Bundle Detail ")
                .borders(Borders::ALL);
            let para = Paragraph::new("No bundle selected").block(block);
            frame.render_widget(para, area);
            return;
        }
    };

    let is_active = app
        .active_bundle
        .as_ref()
        .map(|b| b.id == bundle.id)
        .unwrap_or(false);

    let status = if is_active { "Active" } else { "Inactive" };
    let desc = bundle.description.as_deref().unwrap_or("No description");

    let text = vec![
        Line::from(format!("ID: {}", bundle.id)),
        Line::from(format!("Description: {}", desc)),
        Line::from(format!("Type: {:?}", bundle.bundle_type)),
        Line::from(format!("Status: {}", status)),
        Line::from(""),
        Line::from("Profiles:"),
    ];

    let mut lines = text;
    for profile_id in &bundle.profiles {
        lines.push(Line::from(format!("  - {}", profile_id)));
    }

    let block = Block::default()
        .title(format!(" Bundle: {} ", bundle.id))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let para = Paragraph::new(lines)
        .block(block)
        .wrap(Wrap { trim: true });

    frame.render_widget(para, area);
}
