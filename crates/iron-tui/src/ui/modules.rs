//! Module view rendering

use crate::app::App;
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap};

/// Render the modules list view
pub fn render_modules(frame: &mut Frame, area: Rect, app: &App) {
    let items: Vec<ListItem> = app
        .modules
        .iter()
        .enumerate()
        .map(|(i, module)| {
            let is_active = app.is_module_active(&module.id);
            let status = if is_active { "✓" } else { "○" };
            let desc = module.description.as_deref().unwrap_or("");
            let content = format!("{} {} - {}", status, module.id, desc);

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
        .title(" Modules ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let list = List::new(items).block(block);

    let mut state = ListState::default();
    state.select(Some(app.selected_index));

    frame.render_stateful_widget(list, area, &mut state);
}

/// Render module detail view
pub fn render_module_detail(frame: &mut Frame, area: Rect, app: &App) {
    let module = match app.selected_module() {
        Some(m) => m,
        None => {
            let block = Block::default()
                .title(" Module Detail ")
                .borders(Borders::ALL);
            let para = Paragraph::new("No module selected").block(block);
            frame.render_widget(para, area);
            return;
        }
    };

    let is_active = app.is_module_active(&module.id);
    let status = if is_active { "Enabled" } else { "Disabled" };
    let desc = module.description.as_deref().unwrap_or("No description");

    let text = vec![
        Line::from(format!("ID: {}", module.id)),
        Line::from(format!("Description: {}", desc)),
        Line::from(format!("Kind: {:?}", module.kind)),
        Line::from(format!("Status: {}", status)),
        Line::from(""),
        Line::from("Packages:"),
    ];

    let mut lines = text;
    for pkg in &module.packages {
        lines.push(Line::from(format!("  - {}", pkg)));
    }

    lines.push(Line::from(""));
    lines.push(Line::from("Dotfiles:"));
    for mapping in &module.dotfiles {
        lines.push(Line::from(format!(
            "  {} -> {}",
            mapping.source, mapping.target
        )));
    }

    let block = Block::default()
        .title(format!(" Module: {} ", module.id))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let para = Paragraph::new(lines)
        .block(block)
        .wrap(Wrap { trim: true });

    frame.render_widget(para, area);
}
