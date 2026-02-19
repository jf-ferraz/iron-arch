//! Module Creator wizard — 2-step UI for creating modules

use crate::app::App;
use crate::ui::theme;
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

/// Render the Module Creator wizard
pub fn render_module_creator(frame: &mut Frame, area: Rect, app: &App) {
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Title bar
            Constraint::Min(0),    // Content
            Constraint::Length(3), // Footer hints
        ])
        .split(area);

    // Title bar
    let step_label = match app.module_creator_step {
        0 => "Step 1/2 — Name, Description & Packages",
        _ => "Step 2/2 — Preview & Create",
    };
    let title_block = Block::default()
        .borders(Borders::BOTTOM)
        .border_style(Style::default().fg(theme::OVERLAY));
    let title_para = Paragraph::new(Line::from(vec![
        Span::raw("  "),
        Span::styled("New Module", Style::default().fg(theme::MAUVE).bold()),
        Span::raw("  "),
        Span::styled(step_label, Style::default().fg(theme::SUBTEXT)),
    ]))
    .block(title_block);
    frame.render_widget(title_para, layout[0]);

    match app.module_creator_step {
        0 => render_step_details(frame, layout[1], app),
        _ => render_step_preview(frame, layout[1], app),
    }

    // Footer hints
    let hints: Vec<Span> = match app.module_creator_step {
        0 => vec![
            Span::styled(" [Tab] ", Style::default().fg(Color::Black).bg(theme::MAUVE).bold()),
            Span::styled(" Next field  ", Style::default().fg(theme::SUBTEXT)),
            Span::styled(" [Enter] ", Style::default().fg(Color::Black).bg(theme::MAUVE).bold()),
            Span::styled(" Preview  ", Style::default().fg(theme::SUBTEXT)),
            Span::styled(" [Esc] ", Style::default().fg(Color::Black).bg(theme::MAUVE).bold()),
            Span::styled(" Cancel", Style::default().fg(theme::SUBTEXT)),
        ],
        _ => vec![
            Span::styled(" [Enter] ", Style::default().fg(Color::Black).bg(theme::GREEN).bold()),
            Span::styled(" Create Module  ", Style::default().fg(theme::SUBTEXT)),
            Span::styled(" [Esc] ", Style::default().fg(Color::Black).bg(theme::MAUVE).bold()),
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

/// Step 1: Name, description, packages
fn render_step_details(frame: &mut Frame, area: Rect, app: &App) {
    let inner = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(3), // Name
            Constraint::Length(1), // Spacer
            Constraint::Length(3), // Description
            Constraint::Length(1), // Spacer
            Constraint::Length(3), // Packages
            Constraint::Min(0),    // Hint
        ])
        .split(area);

    let outer = theme::themed_block("Module Details", theme::MAUVE);
    frame.render_widget(outer, area);

    // Helper to build a field block with highlight on active field
    let field_color = |idx: usize| {
        if app.module_creator_active_field == idx {
            theme::MAUVE
        } else {
            theme::OVERLAY
        }
    };

    // Name field
    let name_color = field_color(0);
    let name_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(name_color))
        .title(Span::styled(" Module ID ", Style::default().fg(name_color).bold()));
    let name_display = if app.module_creator_name.is_empty() {
        Span::styled("e.g. nvim", Style::default().fg(theme::OVERLAY).italic())
    } else {
        Span::styled(app.module_creator_name.as_str(), Style::default().fg(theme::TEXT))
    };
    frame.render_widget(
        Paragraph::new(Line::from(name_display)).block(name_block),
        inner[0],
    );

    // Description field
    let desc_color = field_color(1);
    let desc_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(desc_color))
        .title(Span::styled(
            " Description (optional) ",
            Style::default().fg(desc_color).bold(),
        ));
    let desc_display = if app.module_creator_description.is_empty() {
        Span::styled(
            "e.g. Neovim editor configuration",
            Style::default().fg(theme::OVERLAY).italic(),
        )
    } else {
        Span::styled(app.module_creator_description.as_str(), Style::default().fg(theme::TEXT))
    };
    frame.render_widget(
        Paragraph::new(Line::from(desc_display)).block(desc_block),
        inner[2],
    );

    // Packages field
    let pkg_color = field_color(2);
    let pkg_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(pkg_color))
        .title(Span::styled(
            " Packages (comma-separated) ",
            Style::default().fg(pkg_color).bold(),
        ));
    let pkg_display = if app.module_creator_packages.is_empty() {
        Span::styled(
            "e.g. neovim, tree-sitter, ripgrep",
            Style::default().fg(theme::OVERLAY).italic(),
        )
    } else {
        Span::styled(app.module_creator_packages.as_str(), Style::default().fg(theme::TEXT))
    };
    frame.render_widget(
        Paragraph::new(Line::from(pkg_display)).block(pkg_block),
        inner[4],
    );

    // Hint
    frame.render_widget(
        Paragraph::new(Span::styled(
            "  Tab to switch fields, Enter to preview",
            Style::default().fg(theme::SUBTEXT),
        )),
        inner[5],
    );
}

/// Step 2: Preview the module before creating
fn render_step_preview(frame: &mut Frame, area: Rect, app: &App) {
    let block = theme::themed_block("Preview", theme::MAUVE);

    let id = if app.module_creator_name.is_empty() {
        "<unnamed>"
    } else {
        &app.module_creator_name
    };
    let desc = if app.module_creator_description.is_empty() {
        "No description"
    } else {
        &app.module_creator_description
    };

    let packages: Vec<&str> = if app.module_creator_packages.is_empty() {
        vec![]
    } else {
        app.module_creator_packages
            .split(',')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .collect()
    };

    let mut lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("ID          ", Style::default().fg(theme::SUBTEXT)),
            Span::styled(id, Style::default().fg(theme::TEXT).bold()),
        ]),
        Line::from(vec![
            Span::styled("Description ", Style::default().fg(theme::SUBTEXT)),
            Span::styled(desc, Style::default().fg(theme::LAVENDER)),
        ]),
        Line::from(vec![
            Span::styled("Path        ", Style::default().fg(theme::SUBTEXT)),
            Span::styled(
                format!("modules/{}/module.toml", id),
                Style::default().fg(theme::OVERLAY),
            ),
        ]),
        Line::from(""),
        Line::from(Span::styled("Packages:", Style::default().fg(theme::YELLOW).bold())),
    ];

    if packages.is_empty() {
        lines.push(Line::from(Span::styled(
            "  (none — you can add packages later by editing module.toml)",
            Style::default().fg(theme::OVERLAY).italic(),
        )));
    } else {
        for pkg in &packages {
            lines.push(Line::from(format!("  - {}", pkg)));
        }
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "Press [Enter] to create this module, or [Esc] to go back.",
        Style::default().fg(theme::SUBTEXT),
    )));

    let para = Paragraph::new(lines).block(block).wrap(Wrap { trim: true });
    frame.render_widget(para, area);
}
