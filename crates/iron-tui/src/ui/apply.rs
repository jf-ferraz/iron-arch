//! F1-010: Apply View — Shows apply plan and execution progress
//! F1-018: Drift Detail View — Shows drift report

use crate::app::App;
use crate::ui::theme;
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};

/// Render the Apply view (F1-010)
pub fn render_apply(frame: &mut Frame, area: Rect, app: &App) {
    let block = Block::default()
        .title(" Apply System State ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme::MAUVE));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let plan_count = app.apply_plan_count.unwrap_or(0);

    if plan_count == 0 {
        let msg = Paragraph::new(vec![
            Line::from(""),
            Line::from(Span::styled(
                "  System is already in desired state — nothing to do ✓",
                Style::default().fg(theme::GREEN),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "  Your host.toml declaration matches the current system.",
                Style::default().fg(theme::SUBTEXT),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "  [Esc] Back to Dashboard",
                Style::default().fg(theme::SUBTEXT),
            )),
        ]);
        frame.render_widget(msg, inner);
        return;
    }

    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Summary
            Constraint::Min(5),    // Plan list
            Constraint::Length(3), // Controls
        ])
        .split(inner);

    // Summary
    let summary = Paragraph::new(Line::from(vec![
        Span::styled("  Plan: ", Style::default().fg(theme::TEXT).bold()),
        Span::styled(
            format!("{} action(s) to converge system", plan_count),
            Style::default().fg(theme::YELLOW),
        ),
    ]));
    frame.render_widget(summary, layout[0]);

    // Plan actions placeholder
    let items: Vec<ListItem> = vec![ListItem::new(Line::from(Span::styled(
        format!(
            "  {} pending action(s) — run 'iron apply --dry-run' for details",
            plan_count
        ),
        Style::default().fg(theme::TEXT),
    )))];

    let list = List::new(items).block(Block::default().title(" Actions ").borders(Borders::ALL));
    frame.render_widget(list, layout[1]);

    // Controls
    let controls = Paragraph::new(Line::from(vec![
        Span::styled("  [Enter] ", Style::default().fg(theme::MAUVE)),
        Span::raw("Apply  "),
        Span::styled("[Esc] ", Style::default().fg(theme::MAUVE)),
        Span::raw("Cancel  "),
    ]));
    frame.render_widget(controls, layout[2]);
}

/// Render the Drift Detail view (F1-018)
pub fn render_drift_detail(frame: &mut Frame, area: Rect, app: &App) {
    let block = Block::default()
        .title(" Drift Detection ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme::YELLOW));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let drift_count = app.drift_count.unwrap_or(0);

    if drift_count == 0 {
        let msg = Paragraph::new(vec![
            Line::from(""),
            Line::from(Span::styled(
                "  System is clean ✓ — no drift detected",
                Style::default().fg(theme::GREEN),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "  All packages, configs, and services match declared state.",
                Style::default().fg(theme::SUBTEXT),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "  [Esc] Back to Dashboard",
                Style::default().fg(theme::SUBTEXT),
            )),
        ]);
        frame.render_widget(msg, inner);
        return;
    }

    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Summary
            Constraint::Min(5),    // Drift list
            Constraint::Length(3), // Controls
        ])
        .split(inner);

    // Summary
    let summary = Paragraph::new(Line::from(vec![Span::styled(
        format!("  ⚠ {} drift(s) detected", drift_count),
        Style::default().fg(theme::YELLOW).bold(),
    )]));
    frame.render_widget(summary, layout[0]);

    // Drift details placeholder
    let info = Paragraph::new(vec![
        Line::from(Span::styled(
            "  Run 'iron diff' for detailed drift report",
            Style::default().fg(theme::TEXT),
        )),
        Line::from(Span::styled(
            "  Run 'iron diff --correct' to fix drift",
            Style::default().fg(theme::SUBTEXT),
        )),
    ]);
    frame.render_widget(info, layout[1]);

    // Controls
    let controls = Paragraph::new(Line::from(vec![
        Span::styled("  [Esc] ", Style::default().fg(theme::MAUVE)),
        Span::raw("Back  "),
    ]));
    frame.render_widget(controls, layout[2]);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::App;

    #[test]
    fn test_render_apply_no_panic() {
        let app = App::default();
        let backend = ratatui::backend::TestBackend::new(80, 24);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        terminal.draw(|f| render_apply(f, f.area(), &app)).unwrap();
    }

    #[test]
    fn test_render_drift_detail_no_panic() {
        let app = App::default();
        let backend = ratatui::backend::TestBackend::new(80, 24);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        terminal
            .draw(|f| render_drift_detail(f, f.area(), &app))
            .unwrap();
    }

    #[test]
    fn test_render_apply_with_plan() {
        let mut app = App::default();
        app.apply_plan_count = Some(5);
        let backend = ratatui::backend::TestBackend::new(80, 24);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        terminal.draw(|f| render_apply(f, f.area(), &app)).unwrap();
    }

    #[test]
    fn test_render_drift_with_count() {
        let mut app = App::default();
        app.drift_count = Some(3);
        let backend = ratatui::backend::TestBackend::new(80, 24);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        terminal
            .draw(|f| render_drift_detail(f, f.area(), &app))
            .unwrap();
    }
}
