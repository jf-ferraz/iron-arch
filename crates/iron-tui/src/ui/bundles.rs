//! Bundle view rendering

use crate::app::App;
use crate::ui::theme;
use ratatui::prelude::*;
use ratatui::widgets::{List, ListItem, ListState, Paragraph, Wrap};

/// Render the bundles list view
pub fn render_bundles(frame: &mut Frame, area: Rect, app: &App) {
    let block = theme::themed_block("Bundles", theme::MAUVE);

    if app.bundles.is_empty() {
        let empty = Paragraph::new(vec![
            Line::from(""),
            Line::from(Span::styled(
                "No bundles found.",
                Style::default()
                    .fg(theme::SUBTEXT)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "Create `bundles/hyprland/bundle.toml` in your config directory,",
                Style::default().fg(theme::OVERLAY),
            )),
            Line::from(Span::styled(
                "or run the setup wizard with [w] to get started.",
                Style::default().fg(theme::OVERLAY),
            )),
        ])
        .block(block)
        .alignment(Alignment::Center);
        frame.render_widget(empty, area);
        return;
    }

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
                theme::selected()
            } else if is_active {
                Style::default().fg(theme::GREEN)
            } else {
                theme::unselected()
            };

            ListItem::new(content).style(style)
        })
        .collect();

    let list = List::new(items).block(block).highlight_symbol("▸ ");

    let mut state = ListState::default();
    if !app.bundles.is_empty() {
        state.select(Some(app.selected_index));
    }

    frame.render_stateful_widget(list, area, &mut state);
}

/// Render bundle detail view
pub fn render_bundle_detail(frame: &mut Frame, area: Rect, app: &App) {
    let bundle = match app.selected_bundle() {
        Some(b) => b,
        None => {
            let block = theme::themed_block("Bundle Detail", theme::MAUVE);
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
        Line::from(""),
        Line::from(vec![
            Span::styled("ID          ", Style::default().fg(theme::SUBTEXT)),
            Span::styled(bundle.id.as_str(), Style::default().fg(theme::TEXT).bold()),
        ]),
        Line::from(vec![
            Span::styled("Description ", Style::default().fg(theme::SUBTEXT)),
            Span::styled(desc, Style::default().fg(theme::LAVENDER)),
        ]),
        Line::from(vec![
            Span::styled("Type        ", Style::default().fg(theme::SUBTEXT)),
            Span::styled(
                format!("{:?}", bundle.bundle_type),
                Style::default().fg(theme::LAVENDER),
            ),
        ]),
        Line::from(vec![
            Span::styled("Status      ", Style::default().fg(theme::SUBTEXT)),
            Span::styled(
                status,
                Style::default().fg(if is_active {
                    theme::GREEN
                } else {
                    theme::OVERLAY
                }),
            ),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "Profiles:",
            Style::default().fg(theme::YELLOW).bold(),
        )),
    ];

    let mut lines = text;

    // D-007: Packages section
    lines.push(Line::from(Span::styled(
        "Packages:",
        Style::default().fg(theme::YELLOW).bold(),
    )));
    if bundle.packages.is_empty() && bundle.aur_packages.is_empty() {
        lines.push(Line::from(Span::styled(
            "  No packages declared",
            Style::default().fg(theme::OVERLAY).italic(),
        )));
    } else {
        for pkg in &bundle.packages {
            lines.push(Line::from(format!("  - {}", pkg)));
        }
        for pkg in &bundle.aur_packages {
            lines.push(Line::from(vec![
                Span::raw("  - "),
                Span::styled(pkg.as_str(), Style::default().fg(theme::TEXT)),
                Span::styled(" (AUR)", Style::default().fg(theme::PEACH)),
            ]));
        }
    }
    lines.push(Line::from(""));

    // D-007: Services section
    lines.push(Line::from(Span::styled(
        "Services:",
        Style::default().fg(theme::YELLOW).bold(),
    )));
    if bundle.services.is_empty() {
        lines.push(Line::from(Span::styled(
            "  No services declared",
            Style::default().fg(theme::OVERLAY).italic(),
        )));
    } else {
        for svc in &bundle.services {
            lines.push(Line::from(format!("  - {}", svc)));
        }
    }
    lines.push(Line::from(""));

    // Profiles section
    lines.push(Line::from(Span::styled(
        "Profiles:",
        Style::default().fg(theme::YELLOW).bold(),
    )));
    if bundle.profiles.is_empty() {
        lines.push(Line::from(Span::styled(
            "  No profiles configured",
            Style::default().fg(theme::OVERLAY).italic(),
        )));
    } else {
        for profile_id in &bundle.profiles {
            lines.push(Line::from(format!("  - {}", profile_id)));
        }
    }
    lines.push(Line::from(""));

    // D-007: Conflicts section
    if !bundle.conflicts.is_empty() {
        lines.push(Line::from(Span::styled(
            "Conflicts:",
            Style::default().fg(theme::RED).bold(),
        )));
        for conflict in &bundle.conflicts {
            lines.push(Line::from(vec![
                Span::styled("  ⚠ ", Style::default().fg(theme::RED)),
                Span::raw(conflict.as_str()),
            ]));
        }
        lines.push(Line::from(""));
    }

    lines.push(Line::from(Span::styled(
        "[Esc] Back  [Enter] Activate  [d] Deactivate",
        Style::default().fg(theme::SUBTEXT),
    )));

    let title = format!("Bundle: {}", bundle.id);
    let block = theme::themed_block(&title, theme::MAUVE);

    let para = Paragraph::new(lines).block(block).wrap(Wrap { trim: true });

    frame.render_widget(para, area);
}
