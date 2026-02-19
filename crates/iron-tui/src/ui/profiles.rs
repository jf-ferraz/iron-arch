//! Profile view rendering

use crate::app::App;
use crate::ui::theme;
use ratatui::prelude::*;
use ratatui::widgets::{List, ListItem, ListState, Paragraph, Wrap};

/// Render the profiles list view
pub fn render_profiles(frame: &mut Frame, area: Rect, app: &App) {
    let block = theme::themed_block("Profiles", theme::MAUVE);

    if app.profiles.is_empty() {
        let empty = Paragraph::new(vec![
            Line::from(""),
            Line::from(Span::styled(
                "No profiles found.",
                Style::default().fg(theme::SUBTEXT).add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "Press [n] to create your first profile,",
                Style::default().fg(theme::GREEN),
            )),
            Line::from(Span::styled(
                "or create `profiles/<name>/profile.toml` in your config directory.",
                Style::default().fg(theme::OVERLAY),
            )),
        ])
        .block(block)
        .alignment(Alignment::Center);
        frame.render_widget(empty, area);
        return;
    }

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
                theme::selected()
            } else if is_active {
                Style::default().fg(theme::GREEN)
            } else {
                theme::unselected()
            };

            ListItem::new(content).style(style)
        })
        .collect();

    let list = List::new(items)
        .block(block)
        .highlight_symbol("▸ ");

    let mut state = ListState::default();
    if !app.profiles.is_empty() {
        state.select(Some(app.selected_index));
    }

    frame.render_stateful_widget(list, area, &mut state);
}

/// Render profile detail view
pub fn render_profile_detail(frame: &mut Frame, area: Rect, app: &App) {
    let profile = match app.selected_profile() {
        Some(p) => p,
        None => {
            let block = theme::themed_block("Profile Detail", theme::MAUVE);
            let para = Paragraph::new("No profile selected").block(block);
            frame.render_widget(para, area);
            return;
        }
    };

    let desc = profile.description.as_deref().unwrap_or("No description");

    let text = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("ID          ", Style::default().fg(theme::SUBTEXT)),
            Span::styled(profile.id.as_str(), Style::default().fg(theme::TEXT).bold()),
        ]),
        Line::from(vec![
            Span::styled("Description ", Style::default().fg(theme::SUBTEXT)),
            Span::styled(desc, Style::default().fg(theme::LAVENDER)),
        ]),
        Line::from(""),
        Line::from(Span::styled("Modules:", Style::default().fg(theme::YELLOW).bold())),
    ];

    let mut lines = text;
    if profile.modules.is_empty() {
        lines.push(Line::from(Span::styled(
            "  No modules in this profile",
            Style::default().fg(theme::OVERLAY).italic(),
        )));
    } else {
        for module_id in &profile.modules {
            lines.push(Line::from(format!("  - {}", module_id)));
        }
    }
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "[Esc] Back  [Enter] Activate",
        Style::default().fg(theme::SUBTEXT),
    )));

    let title = format!("Profile: {}", profile.id);
    let block = theme::themed_block(&title, theme::MAUVE);

    let para = Paragraph::new(lines).block(block).wrap(Wrap { trim: true });

    frame.render_widget(para, area);
}
