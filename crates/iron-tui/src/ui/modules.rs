//! Module view rendering

use crate::app::App;
use crate::ui::theme;
use ratatui::prelude::*;
use ratatui::widgets::{List, ListItem, ListState, Paragraph, Wrap};

/// Render the modules list view
pub fn render_modules(frame: &mut Frame, area: Rect, app: &App) {
    let block = theme::themed_block("Modules", theme::MAUVE);

    if app.modules.is_empty() {
        let empty = Paragraph::new(vec![
            Line::from(""),
            Line::from(Span::styled("No modules found", Style::default().fg(theme::SUBTEXT))),
            Line::from(""),
            Line::from(Span::styled(
                "Modules are loaded from your active bundle and profiles.",
                Style::default().fg(theme::OVERLAY),
            )),
        ])
        .block(block)
        .alignment(Alignment::Center);
        frame.render_widget(empty, area);
        return;
    }

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
    if !app.modules.is_empty() {
        state.select(Some(app.selected_index));
    }

    frame.render_stateful_widget(list, area, &mut state);
}

/// Render module detail view
pub fn render_module_detail(frame: &mut Frame, area: Rect, app: &App) {
    let module = match app.selected_module() {
        Some(m) => m,
        None => {
            let block = theme::themed_block("Module Detail", theme::MAUVE);
            let para = Paragraph::new("No module selected").block(block);
            frame.render_widget(para, area);
            return;
        }
    };

    let is_active = app.is_module_active(&module.id);
    let status = if is_active { "Enabled" } else { "Disabled" };
    let desc = module.description.as_deref().unwrap_or("No description");

    let text = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("ID          ", Style::default().fg(theme::SUBTEXT)),
            Span::styled(module.id.as_str(), Style::default().fg(theme::TEXT).bold()),
        ]),
        Line::from(vec![
            Span::styled("Description ", Style::default().fg(theme::SUBTEXT)),
            Span::styled(desc, Style::default().fg(theme::LAVENDER)),
        ]),
        Line::from(vec![
            Span::styled("Kind        ", Style::default().fg(theme::SUBTEXT)),
            Span::styled(format!("{:?}", module.kind), Style::default().fg(theme::LAVENDER)),
        ]),
        Line::from(vec![
            Span::styled("Status      ", Style::default().fg(theme::SUBTEXT)),
            Span::styled(
                status,
                Style::default().fg(if is_active { theme::GREEN } else { theme::OVERLAY }),
            ),
        ]),
        Line::from(""),
        Line::from(Span::styled("Packages:", Style::default().fg(theme::YELLOW).bold())),
    ];

    let mut lines = text;
    if module.packages.is_empty() {
        lines.push(Line::from(Span::styled(
            "  No packages",
            Style::default().fg(theme::OVERLAY).italic(),
        )));
    } else {
        for pkg in &module.packages {
            lines.push(Line::from(format!("  - {}", pkg)));
        }
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled("Dotfiles:", Style::default().fg(theme::YELLOW).bold())));
    if module.dotfiles.is_empty() {
        lines.push(Line::from(Span::styled(
            "  No dotfile mappings",
            Style::default().fg(theme::OVERLAY).italic(),
        )));
    } else {
        for mapping in &module.dotfiles {
            lines.push(Line::from(format!(
                "  {} -> {}",
                mapping.source, mapping.target
            )));
        }
    }
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "[Esc] Back  [e] Toggle",
        Style::default().fg(theme::SUBTEXT),
    )));

    let title = format!("Module: {}", module.id);
    let block = theme::themed_block(&title, theme::MAUVE);

    let para = Paragraph::new(lines).block(block).wrap(Wrap { trim: true });

    frame.render_widget(para, area);
}
