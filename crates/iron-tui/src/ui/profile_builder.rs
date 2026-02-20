//! Profile Builder wizard — 3-step UI for creating profiles

use crate::app::App;
use crate::ui::theme;
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap};

/// Render the Profile Builder wizard
pub fn render_profile_builder(frame: &mut Frame, area: Rect, app: &App) {
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Title bar
            Constraint::Min(0),    // Content
            Constraint::Length(3), // Footer hints
        ])
        .split(area);

    // Title bar
    let step_label = match app.profile_builder_step {
        0 => "Step 1/3 — Name & Description",
        1 => "Step 2/3 — Select Modules",
        2 => "Step 3/3 — Preview & Create",
        _ => "Done",
    };
    let title_block = Block::default()
        .borders(Borders::BOTTOM)
        .border_style(Style::default().fg(theme::OVERLAY));
    let title_para = Paragraph::new(Line::from(vec![
        Span::raw("  "),
        Span::styled("New Profile", Style::default().fg(theme::MAUVE).bold()),
        Span::raw("  "),
        Span::styled(step_label, Style::default().fg(theme::SUBTEXT)),
    ]))
    .block(title_block);
    frame.render_widget(title_para, layout[0]);

    match app.profile_builder_step {
        0 => render_step_name(frame, layout[1], app),
        1 => render_step_modules(frame, layout[1], app),
        _ => render_step_preview(frame, layout[1], app),
    }

    // Footer hints
    let hints = match app.profile_builder_step {
        0 => vec![
            Span::styled(
                " [Tab] ",
                Style::default().fg(Color::Black).bg(theme::MAUVE).bold(),
            ),
            Span::styled(" Switch field  ", Style::default().fg(theme::SUBTEXT)),
            Span::styled(
                " [Enter] ",
                Style::default().fg(Color::Black).bg(theme::MAUVE).bold(),
            ),
            Span::styled(" Next  ", Style::default().fg(theme::SUBTEXT)),
            Span::styled(
                " [Esc] ",
                Style::default().fg(Color::Black).bg(theme::MAUVE).bold(),
            ),
            Span::styled(" Cancel", Style::default().fg(theme::SUBTEXT)),
        ],
        1 => vec![
            Span::styled(
                " [Space] ",
                Style::default().fg(Color::Black).bg(theme::MAUVE).bold(),
            ),
            Span::styled(" Toggle  ", Style::default().fg(theme::SUBTEXT)),
            Span::styled(
                " [Enter] ",
                Style::default().fg(Color::Black).bg(theme::MAUVE).bold(),
            ),
            Span::styled(" Next  ", Style::default().fg(theme::SUBTEXT)),
            Span::styled(
                " [Esc] ",
                Style::default().fg(Color::Black).bg(theme::MAUVE).bold(),
            ),
            Span::styled(" Back", Style::default().fg(theme::SUBTEXT)),
        ],
        _ => vec![
            Span::styled(
                " [Enter] ",
                Style::default().fg(Color::Black).bg(theme::GREEN).bold(),
            ),
            Span::styled(" Create Profile  ", Style::default().fg(theme::SUBTEXT)),
            Span::styled(
                " [Esc] ",
                Style::default().fg(Color::Black).bg(theme::MAUVE).bold(),
            ),
            Span::styled(" Back", Style::default().fg(theme::SUBTEXT)),
        ],
    };
    let footer_block = Block::default()
        .borders(Borders::TOP)
        .border_style(Style::default().fg(theme::OVERLAY));
    frame.render_widget(
        Paragraph::new(Line::from(hints)).block(footer_block),
        layout[2],
    );
}

/// Step 1: Name and description input
fn render_step_name(frame: &mut Frame, area: Rect, app: &App) {
    let inner = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(3), // Name field
            Constraint::Length(1), // Spacer
            Constraint::Length(3), // Description field
            Constraint::Min(0),    // Hints
        ])
        .split(area);

    let outer = theme::themed_block("Profile Details", theme::MAUVE);
    frame.render_widget(outer, area);

    // Name field
    let name_color = if !app.profile_builder_editing_desc {
        theme::MAUVE
    } else {
        theme::OVERLAY
    };
    let name_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(name_color))
        .title(Span::styled(
            " Profile Name ",
            Style::default().fg(name_color).bold(),
        ));
    let name_display = if app.profile_builder_name.is_empty() {
        Span::styled(
            "e.g. developer",
            Style::default().fg(theme::OVERLAY).italic(),
        )
    } else {
        Span::styled(
            app.profile_builder_name.as_str(),
            Style::default().fg(theme::TEXT),
        )
    };
    frame.render_widget(
        Paragraph::new(Line::from(name_display)).block(name_block),
        inner[0],
    );

    // Description field
    let desc_color = if app.profile_builder_editing_desc {
        theme::MAUVE
    } else {
        theme::OVERLAY
    };
    let desc_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(desc_color))
        .title(Span::styled(
            " Description (optional) ",
            Style::default().fg(desc_color).bold(),
        ));
    let desc_display = if app.profile_builder_description.is_empty() {
        Span::styled(
            "e.g. Development environment",
            Style::default().fg(theme::OVERLAY).italic(),
        )
    } else {
        Span::styled(
            app.profile_builder_description.as_str(),
            Style::default().fg(theme::TEXT),
        )
    };
    frame.render_widget(
        Paragraph::new(Line::from(desc_display)).block(desc_block),
        inner[2],
    );

    // Hint
    frame.render_widget(
        Paragraph::new(Span::styled(
            "  Type to enter text, Tab to switch fields, Enter to continue",
            Style::default().fg(theme::SUBTEXT),
        )),
        inner[3],
    );
}

/// Step 2: Module selection checklist
fn render_step_modules(frame: &mut Frame, area: Rect, app: &App) {
    let block = theme::themed_block("Select Modules", theme::MAUVE);

    if app.modules.is_empty() {
        // F-011: Show clear guidance when no modules exist
        let para = Paragraph::new(vec![
            Line::from(""),
            Line::from(Span::styled(
                "  No modules found.",
                Style::default().fg(theme::PEACH).bold(),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "  Create modules first using [n] from the Modules view,",
                Style::default().fg(theme::SUBTEXT),
            )),
            Line::from(Span::styled(
                "  or use `iron module create <name>` from the CLI.",
                Style::default().fg(theme::SUBTEXT),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "  Press [Enter] to create the profile without modules,",
                Style::default().fg(theme::OVERLAY),
            )),
            Line::from(Span::styled(
                "  or [Esc] to go back.",
                Style::default().fg(theme::OVERLAY),
            )),
        ])
        .block(block)
        .wrap(Wrap { trim: false });
        frame.render_widget(para, area);
        return;
    }

    let items: Vec<ListItem> = app
        .modules
        .iter()
        .enumerate()
        .map(|(i, module)| {
            let checked = app.profile_builder_selected_modules.contains(&module.id);
            let cursor = i == app.profile_builder_module_cursor;
            let check_sym = if checked { "[✓]" } else { "[ ]" };
            let desc = module.description.as_deref().unwrap_or("");
            let content = format!("{} {} — {}", check_sym, module.id, desc);
            let style = if cursor {
                theme::selected()
            } else if checked {
                Style::default().fg(theme::GREEN)
            } else {
                theme::unselected()
            };
            ListItem::new(content).style(style)
        })
        .collect();

    let list = List::new(items).block(block).highlight_symbol("▸ ");
    let mut state = ListState::default();
    state.select(Some(app.profile_builder_module_cursor));
    frame.render_stateful_widget(list, area, &mut state);
}

/// Step 3: Preview before creating
fn render_step_preview(frame: &mut Frame, area: Rect, app: &App) {
    let block = theme::themed_block("Preview", theme::MAUVE);

    let name = if app.profile_builder_name.is_empty() {
        "<unnamed>"
    } else {
        &app.profile_builder_name
    };
    let desc = if app.profile_builder_description.is_empty() {
        "No description"
    } else {
        &app.profile_builder_description
    };

    let mut lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("Name        ", Style::default().fg(theme::SUBTEXT)),
            Span::styled(name, Style::default().fg(theme::TEXT).bold()),
        ]),
        Line::from(vec![
            Span::styled("Description ", Style::default().fg(theme::SUBTEXT)),
            Span::styled(desc, Style::default().fg(theme::LAVENDER)),
        ]),
        Line::from(vec![
            Span::styled("Path        ", Style::default().fg(theme::SUBTEXT)),
            Span::styled(
                format!("profiles/{}/profile.toml", name),
                Style::default().fg(theme::OVERLAY),
            ),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "Modules:",
            Style::default().fg(theme::YELLOW).bold(),
        )),
    ];

    if app.profile_builder_selected_modules.is_empty() {
        lines.push(Line::from(Span::styled(
            "  (none selected — profile will have no modules)",
            Style::default().fg(theme::OVERLAY).italic(),
        )));
    } else {
        for module_id in &app.profile_builder_selected_modules {
            lines.push(Line::from(format!("  - {}", module_id)));
        }
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "Press [Enter] to create this profile, or [Esc] to go back.",
        Style::default().fg(theme::SUBTEXT),
    )));

    let para = Paragraph::new(lines).block(block).wrap(Wrap { trim: true });
    frame.render_widget(para, area);
}
