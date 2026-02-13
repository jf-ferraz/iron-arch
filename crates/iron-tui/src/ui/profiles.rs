//! Profile view rendering

use crate::app::App;
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap};

/// Render the profiles list view
pub fn render_profiles(frame: &mut Frame, area: Rect, app: &App) {
    let items: Vec<ListItem> = app
        .profiles
        .iter()
        .enumerate()
        .map(|(i, profile)| {
            let is_active = app
                .active_profile
                .as_ref()
                .map(|p| *p == profile.id)
                .unwrap_or(false);

            let status = if is_active { "●" } else { "○" };
            let desc = profile.description.as_deref().unwrap_or("");
            let content = format!("{} {} - {}", status, profile.id, desc);

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
        .title(" Profiles ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let list = List::new(items).block(block);

    let mut state = ListState::default();
    state.select(Some(app.selected_index));

    frame.render_stateful_widget(list, area, &mut state);
}

/// Render profile detail view
pub fn render_profile_detail(frame: &mut Frame, area: Rect, app: &App) {
    let profile = match app.selected_profile() {
        Some(p) => p,
        None => {
            let block = Block::default()
                .title(" Profile Detail ")
                .borders(Borders::ALL);
            let para = Paragraph::new("No profile selected").block(block);
            frame.render_widget(para, area);
            return;
        }
    };

    let desc = profile.description.as_deref().unwrap_or("No description");

    let text = vec![
        Line::from(format!("ID: {}", profile.id)),
        Line::from(format!("Description: {}", desc)),
        Line::from(""),
        Line::from("Modules:"),
    ];

    let mut lines = text;
    for module_id in &profile.modules {
        lines.push(Line::from(format!("  - {}", module_id)));
    }

    let block = Block::default()
        .title(format!(" Profile: {} ", profile.id))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let para = Paragraph::new(lines).block(block).wrap(Wrap { trim: true });

    frame.render_widget(para, area);
}
