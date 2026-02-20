//! System Scan view — displays results from the ScanService
//!
//! Shows discovered configs, package overlaps, conflicts, and
//! recommendations from the most recent system scan.

use crate::app::App;
use crate::ui::theme;
use iron_core::services::scan::ScanReport;
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

/// Render the System Scan results screen.
pub fn render_system_scan(frame: &mut Frame, area: Rect, app: &App) {
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Title bar
            Constraint::Min(0),    // Content
            Constraint::Length(3), // Footer hints
        ])
        .split(area);

    // Title
    let title_block = Block::default()
        .borders(Borders::BOTTOM)
        .border_style(Style::default().fg(theme::OVERLAY));
    let title_para = Paragraph::new(Line::from(vec![
        Span::raw("  "),
        Span::styled("System Scan", Style::default().fg(theme::PEACH).bold()),
        Span::raw("  "),
        Span::styled(
            "Discovered configs, packages & conflicts",
            Style::default().fg(theme::SUBTEXT),
        ),
    ]))
    .block(title_block);
    frame.render_widget(title_para, layout[0]);

    // Content
    let content_lines = match &app.scan_report {
        Some(report) => build_report_lines(report),
        None => vec![
            Line::from(""),
            Line::from(Span::styled(
                "  No scan results yet. Press [s] from the dashboard to run a system scan.",
                Style::default().fg(theme::SUBTEXT),
            )),
        ],
    };

    let scroll = app.scan_scroll;
    let content_block = theme::themed_block("Scan Results", theme::PEACH);
    let content_para = Paragraph::new(content_lines)
        .block(content_block)
        .wrap(Wrap { trim: true })
        .scroll((scroll as u16, 0));
    frame.render_widget(content_para, layout[1]);

    // Footer
    let hints = Paragraph::new(Line::from(vec![
        Span::raw("  "),
        Span::styled(
            " [r] ",
            Style::default().fg(Color::Black).bg(theme::PEACH).bold(),
        ),
        Span::styled(" Re-scan  ", Style::default().fg(theme::SUBTEXT)),
        Span::styled(
            " [↑↓] ",
            Style::default().fg(Color::Black).bg(theme::PEACH).bold(),
        ),
        Span::styled(" Scroll  ", Style::default().fg(theme::SUBTEXT)),
        Span::styled(
            " [Esc] ",
            Style::default().fg(Color::Black).bg(theme::PEACH).bold(),
        ),
        Span::styled(" Back", Style::default().fg(theme::SUBTEXT)),
    ]))
    .block(
        Block::default()
            .borders(Borders::TOP)
            .border_style(Style::default().fg(theme::OVERLAY)),
    );
    frame.render_widget(hints, layout[2]);
}

/// Build display lines from a `ScanReport`.
fn build_report_lines(report: &ScanReport) -> Vec<Line<'_>> {
    let mut lines: Vec<Line> = Vec::new();
    lines.push(Line::from(""));

    // --- Summary ---
    let summary = &report.summary;
    lines.push(Line::from(Span::styled(
        "  Summary",
        Style::default().fg(theme::MAUVE).bold(),
    )));
    lines.push(Line::from(vec![
        Span::raw("    Configs scanned:     "),
        Span::styled(
            format!("{}", summary.configs_scanned),
            Style::default().fg(theme::TEXT),
        ),
    ]));
    lines.push(Line::from(vec![
        Span::raw("    Pkgs already inst:   "),
        Span::styled(
            format!("{}", summary.packages_already_installed),
            Style::default().fg(theme::TEXT),
        ),
    ]));
    lines.push(Line::from(vec![
        Span::raw("    Conflicts:           "),
        Span::styled(
            format!("{}", summary.conflicts_found),
            Style::default()
                .fg(if summary.conflicts_found > 0 {
                    theme::PINK
                } else {
                    theme::GREEN
                }),
        ),
    ]));
    lines.push(Line::from(vec![
        Span::raw("    Recommendations:     "),
        Span::styled(
            format!("{}", summary.recommendations_count),
            Style::default().fg(theme::TEXT),
        ),
    ]));
    lines.push(Line::from(""));

    // --- Discovered Configs ---
    if !report.existing_configs.is_empty() {
        lines.push(Line::from(Span::styled(
            "  Discovered Configs",
            Style::default().fg(theme::MAUVE).bold(),
        )));
        for cfg in &report.existing_configs {
            let managed_tag = if cfg.is_symlink {
                Span::styled(" [symlink]", Style::default().fg(theme::GREEN))
            } else {
                Span::styled(" [file]", Style::default().fg(theme::YELLOW))
            };
            lines.push(Line::from(vec![
                Span::raw("    "),
                Span::styled(&cfg.app_name, Style::default().fg(theme::TEXT).bold()),
                managed_tag,
                Span::styled(format!("  {}", cfg.path.display()), Style::default().fg(theme::SUBTEXT)),
            ]));
        }
        lines.push(Line::from(""));
    }

    // --- Conflicts ---
    if !report.potential_conflicts.is_empty() {
        lines.push(Line::from(Span::styled(
            "  Conflicts",
            Style::default().fg(theme::PINK).bold(),
        )));
        for conflict in &report.potential_conflicts {
            lines.push(Line::from(vec![
                Span::raw("    "),
                Span::styled(
                    format!("{}", conflict.path.display()),
                    Style::default().fg(theme::YELLOW).bold(),
                ),
                Span::raw("  "),
                Span::styled(&conflict.description, Style::default().fg(theme::TEXT)),
            ]));
        }
        lines.push(Line::from(""));
    }

    // --- Recommendations ---
    if !report.recommendations.is_empty() {
        lines.push(Line::from(Span::styled(
            "  Recommendations",
            Style::default().fg(theme::TEAL).bold(),
        )));
        for (i, rec) in report.recommendations.iter().enumerate() {
            lines.push(Line::from(vec![
                Span::raw("    "),
                Span::styled(
                    format!("{}. ", i + 1),
                    Style::default().fg(theme::TEAL),
                ),
                Span::styled(rec.as_str(), Style::default().fg(theme::TEXT)),
            ]));
        }
        lines.push(Line::from(""));
    }

    // --- Installed Package Overlap ---
    if !report.installed_packages.is_empty() {
        lines.push(Line::from(Span::styled(
            "  Installed Package Overlap",
            Style::default().fg(theme::MAUVE).bold(),
        )));
        for pkg in &report.installed_packages {
            lines.push(Line::from(vec![
                Span::raw("    "),
                Span::styled(pkg.as_str(), Style::default().fg(theme::TEXT)),
            ]));
        }
        lines.push(Line::from(""));
    }

    lines
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    fn create_test_terminal() -> Terminal<TestBackend> {
        let backend = TestBackend::new(120, 40);
        Terminal::new(backend).unwrap()
    }

    #[test]
    fn render_system_scan_no_report_no_panic() {
        let mut terminal = create_test_terminal();
        let app = App::default();
        terminal
            .draw(|f| render_system_scan(f, f.area(), &app))
            .unwrap();
    }

    #[test]
    fn render_system_scan_with_report_no_panic() {
        let mut terminal = create_test_terminal();
        let mut app = App::default();
        app.scan_report = Some(ScanReport {
            existing_configs: vec![],
            potential_conflicts: vec![],
            installed_packages: vec![],
            recommendations: vec!["Back up existing configs".to_string()],
            summary: iron_core::services::scan::ScanSummary {
                configs_scanned: 3,
                packages_already_installed: 2,
                conflicts_found: 0,
                recommendations_count: 1,
            },
        });
        terminal
            .draw(|f| render_system_scan(f, f.area(), &app))
            .unwrap();
    }

    #[test]
    fn build_report_lines_empty_report() {
        let report = ScanReport {
            existing_configs: vec![],
            potential_conflicts: vec![],
            installed_packages: vec![],
            recommendations: vec![],
            summary: iron_core::services::scan::ScanSummary {
                configs_scanned: 0,
                packages_already_installed: 0,
                conflicts_found: 0,
                recommendations_count: 0,
            },
        };
        let lines = build_report_lines(&report);
        // Should have summary section lines at minimum
        assert!(!lines.is_empty());
    }

    #[test]
    fn build_report_lines_with_recommendations() {
        let report = ScanReport {
            existing_configs: vec![],
            potential_conflicts: vec![],
            installed_packages: vec![],
            recommendations: vec![
                "Back up ~/.config/kitty".to_string(),
                "Review existing nvim config".to_string(),
            ],
            summary: iron_core::services::scan::ScanSummary {
                configs_scanned: 5,
                packages_already_installed: 3,
                conflicts_found: 0,
                recommendations_count: 2,
            },
        };
        let lines = build_report_lines(&report);
        // More lines because recommendations section is present
        let text = format!("{:?}", lines);
        assert!(text.contains("Recommendations"));
    }
}
