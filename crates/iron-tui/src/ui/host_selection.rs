//! Host Selection view rendering (S1-P2-001)
//!
//! Lists discovered host configs for multi-machine setups.

use crate::app::App;
use crate::ui::theme;
use ratatui::prelude::*;
use ratatui::widgets::{List, ListItem, ListState, Paragraph};

/// Render the host selection view
pub fn render_host_selection(frame: &mut Frame, area: Rect, app: &App) {
    let block = theme::themed_block("Host Selection", theme::MAUVE);

    if app.discovered_hosts.is_empty() {
        let empty = Paragraph::new(vec![
            Line::from(""),
            Line::from(Span::styled(
                "No hosts configured.",
                Style::default()
                    .fg(theme::SUBTEXT)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "Press [c] to create a new host, or [w] for setup wizard.",
                Style::default().fg(theme::OVERLAY),
            )),
        ])
        .block(block)
        .alignment(Alignment::Center);
        frame.render_widget(empty, area);
        return;
    }

    let items: Vec<ListItem> = app
        .discovered_hosts
        .iter()
        .enumerate()
        .map(|(i, host)| {
            let is_current = app.current_host.as_ref() == Some(&host.id);
            let marker = if is_current { "●" } else { "○" };

            // Build hardware summary: "CPU · GPU · RAM · Chassis"
            let hw = &host.hardware;
            let cpu_short = hw
                .cpu
                .as_deref()
                .unwrap_or("Unknown CPU")
                .split_whitespace()
                .take(4)
                .collect::<Vec<_>>()
                .join(" ");
            let gpu_short = hw.gpu.as_deref().unwrap_or("No GPU");
            let ram = hw
                .ram_mb
                .map(|r| format!("{} GB", r / 1024))
                .unwrap_or_default();
            let chassis = hw
                .chassis
                .as_ref()
                .map(|c| format!("{:?}", c))
                .unwrap_or_default();

            let name = &host.name;
            let line1 = format!("{} {} — {}", marker, host.id, name);
            let line2 = format!("  {} · {} · {} · {}", cpu_short, gpu_short, ram, chassis);

            let style = if i == app.selected_index {
                theme::selected()
            } else if is_current {
                Style::default().fg(theme::GREEN)
            } else {
                theme::unselected()
            };

            ListItem::new(vec![
                Line::styled(line1, style),
                Line::styled(line2, Style::default().fg(theme::OVERLAY)),
            ])
        })
        .collect();

    let list = List::new(items)
        .block(block)
        .highlight_symbol("▸ ");
    let mut state = ListState::default();
    state.select(Some(app.selected_index));
    frame.render_stateful_widget(list, area, &mut state);
}
