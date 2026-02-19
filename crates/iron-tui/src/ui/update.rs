//! Update and sync view rendering
//!
//! Enhanced update UI (Phase 2.3) with:
//! - Pre-flight check results with status indicators
//! - Arch News section with acknowledgment
//! - Package list with risk assessment
//! - Reboot requirement warnings

use crate::app::{App, UpdateSection};
use iron_core::services::update::PreflightStatus;
use iron_core::RiskLevel;
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, Wrap};

/// Status indicator symbols and colors
fn status_indicator(status: PreflightStatus) -> (char, Color) {
    match status {
        PreflightStatus::Pass => ('✓', Color::Green),
        PreflightStatus::Warning => ('⚠', Color::Yellow),
        PreflightStatus::Fail => ('✗', Color::Red),
        PreflightStatus::Skipped => ('○', Color::DarkGray),
    }
}

/// Risk level indicator
fn risk_indicator(risk: RiskLevel) -> (&'static str, Color, &'static str) {
    match risk {
        RiskLevel::Low => ("●", Color::Green, "Safe to update"),
        RiskLevel::Medium => ("⚠", Color::Yellow, "Review recommended"),
        RiskLevel::High => ("⚠", Color::Red, "Attention required"),
        RiskLevel::Critical => ("✗", Color::Red, "Create snapshot first!"),
    }
}

/// Compute risk level for a package based on its name
fn package_risk(name: &str) -> (char, Color) {
    let name_lower = name.to_lowercase();

    // Critical: kernel packages
    if name_lower.starts_with("linux") && !name_lower.contains("-headers") {
        return ('!', Color::Red);
    }

    // High: nvidia, systemd, glibc
    if name_lower.starts_with("nvidia")
        || name_lower == "systemd"
        || name_lower.starts_with("systemd-")
        || name_lower == "glibc"
        || name_lower == "gcc-libs"
    {
        return ('!', Color::LightRed);
    }

    // Medium: mesa, pipewire, etc.
    if name_lower.starts_with("mesa")
        || name_lower.starts_with("vulkan")
        || name_lower.starts_with("pipewire")
        || name_lower.starts_with("wireplumber")
    {
        return ('~', Color::Yellow);
    }

    // Low: everything else
    (' ', Color::White)
}

/// Render the enhanced update preview view
pub fn render_update_preview(frame: &mut Frame, area: Rect, app: &App) {
    let update_count = app.pending_update_count();
    let risk_level = app.update_risk_level();
    let (risk_symbol, risk_color, risk_text) = risk_indicator(risk_level);

    // Main layout: Header, Pre-flight, News, Packages
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(5),  // Header with summary
            Constraint::Length(9),  // Pre-flight checks
            Constraint::Length(7),  // News section (if any)
            Constraint::Min(0),     // Package list
        ])
        .split(area);

    // ==========================================================================
    // Header Section
    // ==========================================================================
    render_header_section(frame, layout[0], app, update_count, risk_symbol, risk_color, risk_text);

    // ==========================================================================
    // Pre-flight Checks Section
    // ==========================================================================
    render_preflight_section(frame, layout[1], app);

    // ==========================================================================
    // News Section
    // ==========================================================================
    render_news_section(frame, layout[2], app);

    // ==========================================================================
    // Package List Section
    // ==========================================================================
    render_packages_section(frame, layout[3], app);
}

/// Render header section with summary
fn render_header_section(
    frame: &mut Frame,
    area: Rect,
    app: &App,
    update_count: usize,
    risk_symbol: &str,
    risk_color: Color,
    risk_text: &str,
) {
    let can_proceed = app.can_proceed_with_update();
    let reboot_required = app.reboot_required;

    let mut lines = vec![
        Line::from(vec![
            Span::raw("Updates: "),
            Span::styled(
                format!("{} package(s)", update_count),
                Style::default().fg(if update_count > 0 {
                    Color::Yellow
                } else {
                    Color::Green
                }),
            ),
            Span::raw("  │  Risk: "),
            Span::styled(
                format!("{} {}", risk_symbol, risk_text),
                Style::default().fg(risk_color),
            ),
        ]),
    ];

    // Reboot warning
    if reboot_required {
        lines.push(Line::from(vec![
            Span::styled("⚡ ", Style::default().fg(Color::Yellow)),
            Span::styled(
                "Reboot required after update (kernel/systemd/glibc)",
                Style::default().fg(Color::Yellow),
            ),
        ]));
    }

    // Status line
    if !can_proceed && app.has_preflight_results() {
        lines.push(Line::from(vec![
            Span::styled("✗ ", Style::default().fg(Color::Red)),
            Span::styled(
                "Cannot proceed - resolve issues below",
                Style::default().fg(Color::Red),
            ),
        ]));
    }

    // Key hints
    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled("[r]", Style::default().fg(Color::Yellow)),
        Span::raw(" Refresh  "),
        Span::styled("[u]", Style::default().fg(if can_proceed { Color::Green } else { Color::DarkGray })),
        Span::raw(" Update  "),
        Span::styled("[Tab]", Style::default().fg(Color::Yellow)),
        Span::raw(" Section  "),
        Span::styled("[Esc]", Style::default().fg(Color::Gray)),
        Span::raw(" Back"),
    ]));

    let block = Block::default()
        .title(" System Update ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let para = Paragraph::new(lines).block(block);
    frame.render_widget(para, area);
}

/// Render pre-flight checks section
fn render_preflight_section(frame: &mut Frame, area: Rect, app: &App) {
    let is_selected = app.update_section == UpdateSection::PreflightChecks;

    let border_color = if is_selected { Color::Cyan } else { Color::DarkGray };

    let block = Block::default()
        .title(" Pre-flight Checks ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    if let Some(result) = app.preflight_checks() {
        let items: Vec<ListItem> = result
            .checks
            .iter()
            .enumerate()
            .map(|(i, check)| {
                let (symbol, color) = status_indicator(check.status);
                let is_item_selected = is_selected && i == app.update_section_index;

                let style = if is_item_selected {
                    Style::default().fg(color).bg(Color::DarkGray)
                } else {
                    Style::default().fg(color)
                };

                let content = format!("{} {} - {}", symbol, check.name, check.message);
                ListItem::new(content).style(style)
            })
            .collect();

        let list = List::new(items);
        frame.render_widget(list, inner);
    } else {
        let para = Paragraph::new("Press [r] to run pre-flight checks")
            .style(Style::default().fg(Color::DarkGray))
            .alignment(Alignment::Center);
        frame.render_widget(para, inner);
    }
}

/// Render news section
fn render_news_section(frame: &mut Frame, area: Rect, app: &App) {
    let is_selected = app.update_section == UpdateSection::News;
    let news = app.unacknowledged_news();
    let has_critical = app.has_critical_news();

    let border_color = if is_selected {
        Color::Cyan
    } else if has_critical {
        Color::Red
    } else if !news.is_empty() {
        Color::Yellow
    } else {
        Color::DarkGray
    };

    let title = if news.is_empty() {
        " Arch News (all acknowledged) ".to_string()
    } else if has_critical {
        format!(" ⚠ Arch News ({} unread, blocks update) ", news.len())
    } else {
        format!(" Arch News ({} unread) ", news.len())
    };

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    if news.is_empty() {
        let para = Paragraph::new("✓ All news acknowledged")
            .style(Style::default().fg(Color::Green))
            .alignment(Alignment::Center);
        frame.render_widget(para, inner);
    } else {
        let items: Vec<ListItem> = news
            .iter()
            .enumerate()
            .map(|(i, item)| {
                let is_item_selected = is_selected && i == app.update_section_index;
                let critical_marker = if item.requires_manual { "⚠ " } else { "" };

                let style = if is_item_selected {
                    Style::default()
                        .fg(if item.requires_manual { Color::Red } else { Color::Yellow })
                        .bg(Color::DarkGray)
                } else if item.requires_manual {
                    Style::default().fg(Color::Red)
                } else {
                    Style::default().fg(Color::Yellow)
                };

                let content = format!(
                    "{}[{}] {}",
                    critical_marker,
                    &item.date[..10.min(item.date.len())],
                    item.title
                );
                ListItem::new(content).style(style)
            })
            .collect();

        // Key hints for news section
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(0), Constraint::Length(1)])
            .split(inner);

        let list = List::new(items);
        frame.render_widget(list, layout[0]);

        if is_selected {
            let hints = Line::from(vec![
                Span::styled("[a]", Style::default().fg(Color::Yellow)),
                Span::raw(" Ack selected  "),
                Span::styled("[A]", Style::default().fg(Color::Yellow)),
                Span::raw(" Ack all"),
            ]);
            frame.render_widget(Paragraph::new(hints), layout[1]);
        }
    }
}

/// Render packages section
fn render_packages_section(frame: &mut Frame, area: Rect, app: &App) {
    let is_selected = app.update_section == UpdateSection::Packages;
    let updates = app.pending_updates_list();

    let border_color = if is_selected { Color::Cyan } else { Color::DarkGray };

    let title = if updates.len() > 50 {
        format!(" Packages (showing 50 of {}) ", updates.len())
    } else {
        format!(" Packages ({}) ", updates.len())
    };

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    if updates.is_empty() {
        let para = Paragraph::new("✓ System is up to date")
            .style(Style::default().fg(Color::Green))
            .alignment(Alignment::Center);
        frame.render_widget(para, inner);
        return;
    }

    let items: Vec<ListItem> = updates
        .iter()
        .take(50)
        .enumerate()
        .map(|(i, pkg)| {
            let is_item_selected = is_selected && i == app.update_section_index;

            // Risk-based styling computed from package name
            let (risk_char, base_color) = package_risk(&pkg.name);

            let aur_marker = if pkg.is_aur { "[AUR] " } else { "" };
            let content = format!(
                "{} {}{}: {} → {}",
                risk_char, aur_marker, pkg.name, pkg.current_version, pkg.new_version
            );

            let style = if is_item_selected {
                Style::default().fg(base_color).bg(Color::DarkGray)
            } else {
                Style::default().fg(base_color)
            };

            ListItem::new(content).style(style)
        })
        .collect();

    // Add scrollbar if needed
    let list_area = if updates.len() > inner.height as usize {
        let layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Min(0), Constraint::Length(1)])
            .split(inner);

        // Render scrollbar
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("▲"))
            .end_symbol(Some("▼"));

        let mut scrollbar_state = ScrollbarState::new(updates.len().min(50))
            .position(app.update_section_index);

        frame.render_stateful_widget(scrollbar, layout[1], &mut scrollbar_state);
        layout[0]
    } else {
        inner
    };

    let list = List::new(items);
    frame.render_widget(list, list_area);
}

/// Render sync status view
pub fn render_sync(frame: &mut Frame, area: Rect, _app: &App) {
    let text = vec![
        Line::from("Git Sync Status"),
        Line::from(""),
        Line::from("Press [p] to push changes"),
        Line::from("Press [l] to pull changes"),
    ];

    let block = Block::default()
        .title(" Sync ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let para = Paragraph::new(text).block(block).wrap(Wrap { trim: true });

    frame.render_widget(para, area);
}
