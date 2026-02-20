//! Module Creator wizard — 3-step UI for creating modules (D-012: dotfiles step)

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
        0 => "Step 1/3 — Name, Description & Packages",
        1 => "Step 2/3 — Dotfile Mappings",
        _ => "Step 3/3 — Preview & Create",
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
        1 => render_step_dotfiles(frame, layout[1], app),
        _ => render_step_preview(frame, layout[1], app),
    }

    // Footer hints
    let hints: Vec<Span> = match app.module_creator_step {
        0 => vec![
            Span::styled(
                " [Tab] ",
                Style::default().fg(Color::Black).bg(theme::MAUVE).bold(),
            ),
            Span::styled(" Next field  ", Style::default().fg(theme::SUBTEXT)),
            Span::styled(
                " [Enter] ",
                Style::default().fg(Color::Black).bg(theme::MAUVE).bold(),
            ),
            Span::styled(" Next step  ", Style::default().fg(theme::SUBTEXT)),
            Span::styled(
                " [Esc] ",
                Style::default().fg(Color::Black).bg(theme::MAUVE).bold(),
            ),
            Span::styled(" Cancel", Style::default().fg(theme::SUBTEXT)),
        ],
        1 => vec![
            Span::styled(
                " [Tab] ",
                Style::default().fg(Color::Black).bg(theme::MAUVE).bold(),
            ),
            Span::styled(" Source/Target  ", Style::default().fg(theme::SUBTEXT)),
            Span::styled(
                " [Enter] ",
                Style::default().fg(Color::Black).bg(theme::MAUVE).bold(),
            ),
            Span::styled(" Add / Preview  ", Style::default().fg(theme::SUBTEXT)),
            Span::styled(
                " [Backspace] ",
                Style::default().fg(Color::Black).bg(theme::MAUVE).bold(),
            ),
            Span::styled(" Delete entry  ", Style::default().fg(theme::SUBTEXT)),
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
            Span::styled(" Create Module  ", Style::default().fg(theme::SUBTEXT)),
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
        .title(Span::styled(
            " Module ID ",
            Style::default().fg(name_color).bold(),
        ));
    let name_display = if app.module_creator_name.is_empty() {
        Span::styled("e.g. nvim", Style::default().fg(theme::OVERLAY).italic())
    } else {
        Span::styled(
            app.module_creator_name.as_str(),
            Style::default().fg(theme::TEXT),
        )
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
        Span::styled(
            app.module_creator_description.as_str(),
            Style::default().fg(theme::TEXT),
        )
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
        Span::styled(
            app.module_creator_packages.as_str(),
            Style::default().fg(theme::TEXT),
        )
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

/// D-012: Step 2 — Dotfile mapping configuration
fn render_step_dotfiles(frame: &mut Frame, area: Rect, app: &App) {
    let block = theme::themed_block("Dotfile Mappings (optional)", theme::MAUVE);
    frame.render_widget(block, area);

    let inner = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(2), // Help text
            Constraint::Length(1), // Spacer
            Constraint::Min(0),    // Entries + input
        ])
        .split(area);

    frame.render_widget(
        Paragraph::new(vec![
            Line::from(Span::styled(
                "  Add source → target mappings for dotfiles managed by this module.",
                Style::default().fg(theme::SUBTEXT),
            )),
            Line::from(Span::styled(
                "  Source is relative to the module dir; target supports ~ for $HOME.",
                Style::default().fg(theme::OVERLAY),
            )),
        ]),
        inner[0],
    );

    let mut lines: Vec<Line> = Vec::new();

    // Show existing entries
    for (i, (src, tgt)) in app.module_creator_dotfiles.iter().enumerate() {
        let prefix = format!("  {}. ", i + 1);
        lines.push(Line::from(vec![
            Span::styled(prefix, Style::default().fg(theme::SUBTEXT)),
            Span::styled(src.as_str(), Style::default().fg(theme::LAVENDER)),
            Span::styled(" → ", Style::default().fg(theme::OVERLAY)),
            Span::styled(tgt.as_str(), Style::default().fg(theme::GREEN)),
        ]));
    }

    if !app.module_creator_dotfiles.is_empty() {
        lines.push(Line::from(""));
    }

    // Current input row
    let entry_num = app.module_creator_dotfiles.len() + 1;
    let src_color = if app.module_creator_dotfile_field == 0 {
        theme::MAUVE
    } else {
        theme::OVERLAY
    };
    let tgt_color = if app.module_creator_dotfile_field == 1 {
        theme::MAUVE
    } else {
        theme::OVERLAY
    };

    // Get the "in-progress" values from the last empty entry or defaults
    lines.push(Line::from(vec![
        Span::styled(
            format!("  {}. Source: ", entry_num),
            Style::default().fg(theme::SUBTEXT),
        ),
        Span::styled("▏ ", Style::default().fg(src_color)),
        Span::styled(
            format!("(e.g. config/{}/)", app.module_creator_name),
            Style::default().fg(theme::OVERLAY).italic(),
        ),
    ]));
    lines.push(Line::from(vec![
        Span::styled(
            format!("     Target: "),
            Style::default().fg(theme::SUBTEXT),
        ),
        Span::styled("▏ ", Style::default().fg(tgt_color)),
        Span::styled(
            format!("(e.g. ~/.config/{}/)", app.module_creator_name),
            Style::default().fg(theme::OVERLAY).italic(),
        ),
    ]));

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "  Enter to add mapping & continue, Esc to go back, Tab to toggle source/target",
        Style::default().fg(theme::SUBTEXT),
    )));

    frame.render_widget(Paragraph::new(lines), inner[2]);
}

/// Step 3: Preview the module before creating
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
        Line::from(Span::styled(
            "Packages:",
            Style::default().fg(theme::YELLOW).bold(),
        )),
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

    // D-012: Show dotfile mappings in preview
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "Dotfiles:",
        Style::default().fg(theme::YELLOW).bold(),
    )));
    if app.module_creator_dotfiles.is_empty() {
        lines.push(Line::from(Span::styled(
            "  (none — you can add dotfiles later by editing module.toml)",
            Style::default().fg(theme::OVERLAY).italic(),
        )));
    } else {
        for (src, tgt) in &app.module_creator_dotfiles {
            lines.push(Line::from(vec![
                Span::raw(format!("  {} ", src)),
                Span::styled("→ ", Style::default().fg(theme::OVERLAY)),
                Span::raw(tgt.as_str()),
            ]));
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
