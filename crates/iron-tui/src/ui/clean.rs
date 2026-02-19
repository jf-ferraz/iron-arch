//! System Cleanup View (Phase 3)
//!
//! UI for selecting and executing cleanup operations across 8 categories:
//! - Package cache management
//! - Orphan package removal
//! - Journal log vacuum
//! - User cache cleanup
//! - Thumbnail cache
//! - Application logs
//! - Browser cache (aggressive)
//! - Developer cache (aggressive)

use crate::app::App;
use crate::ui::theme;
use iron_core::services::clean::{format_bytes, CleanupCategory};
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Cell, List, ListItem, Paragraph, Row, Table};

/// Render the system cleanup view
pub fn render_clean_system(frame: &mut Frame, area: Rect, app: &App) {
    // Main layout: Header, Categories, Summary, Help
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // Header
            Constraint::Min(12),    // Category list
            Constraint::Length(5),  // Summary
            Constraint::Length(3),  // Help bar
        ])
        .split(area);

    render_header(frame, layout[0], app);
    render_categories(frame, layout[1], app);
    render_summary(frame, layout[2], app);
    render_help(frame, layout[3], app);
}

/// Render header section
fn render_header(frame: &mut Frame, area: Rect, app: &App) {
    let mode_text = if app.cleanup_preview_mode {
        "Preview Mode"
    } else {
        "Execution Mode"
    };

    let mode_color = if app.cleanup_preview_mode {
        theme::BLUE
    } else {
        theme::GREEN
    };

    let header_text = Line::from(vec![
        Span::styled("System Cleanup", Style::default().fg(theme::TEXT).bold()),
        Span::raw("  │  "),
        Span::styled(mode_text, Style::default().fg(mode_color)),
    ]);

    let block = theme::themed_block("Cleanup", theme::PEACH);

    let paragraph = Paragraph::new(header_text)
        .block(block)
        .alignment(Alignment::Center);

    frame.render_widget(paragraph, area);
}

/// Render category selection list
fn render_categories(frame: &mut Frame, area: Rect, app: &App) {
    let block = theme::themed_block("Select Categories", theme::PEACH);

    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Create table rows for each category
    let all_categories = CleanupCategory::all();
    let rows: Vec<Row> = all_categories
        .iter()
        .enumerate()
        .map(|(i, cat)| {
            let is_selected = app.is_cleanup_category_selected(cat);
            let is_highlighted = i == app.selected_index;

            // Checkbox
            let checkbox = if is_selected { "[x]" } else { "[ ]" };

            // Get preview data if available
            let preview = app.cleanup_preview_for(cat);
            let space = preview
                .map(|p| p.space_formatted())
                .unwrap_or_else(|| "...".to_string());
            let details = preview
                .map(|p| p.details.clone())
                .unwrap_or_else(|| "Scanning...".to_string());

            // Aggressive indicator
            let name = if cat.is_aggressive() {
                format!("{} ⚠", cat.name())
            } else {
                cat.name().to_string()
            };

            // Style based on selection state
            let style = if is_highlighted {
                theme::selected()
            } else if cat.is_aggressive() {
                Style::default().fg(theme::YELLOW)
            } else {
                theme::unselected()
            };

            let checkbox_style = if is_selected {
                Style::default().fg(theme::GREEN)
            } else {
                Style::default().fg(theme::OVERLAY)
            };

            Row::new(vec![
                Cell::from(checkbox).style(checkbox_style),
                Cell::from(name),
                Cell::from(space).style(Style::default().fg(theme::LAVENDER)),
                Cell::from(details).style(Style::default().fg(theme::SUBTEXT)),
            ])
            .style(style)
        })
        .collect();

    let widths = [
        Constraint::Length(5),   // Checkbox
        Constraint::Length(18),  // Name
        Constraint::Length(12),  // Space
        Constraint::Min(20),     // Details
    ];

    let table = Table::new(rows, widths)
        .header(
            Row::new(vec!["", "Category", "Space", "Details"])
                .style(Style::default().fg(theme::YELLOW).bold())
                .bottom_margin(1),
        )
        .column_spacing(2);

    frame.render_widget(table, inner);
}

/// Render summary section
fn render_summary(frame: &mut Frame, area: Rect, app: &App) {
    let block = theme::themed_block("Summary", theme::PEACH);

    let selected_count = app.cleanup_categories.len();
    let total_space = app.cleanup_total_space();
    let has_aggressive = app
        .cleanup_categories
        .iter()
        .any(|c| c.is_aggressive());

    let mut lines = vec![
        Line::from(vec![
            Span::raw("Selected: "),
            Span::styled(
                format!("{} categories", selected_count),
                Style::default().fg(if selected_count > 0 {
                    theme::GREEN
                } else {
                    theme::SUBTEXT
                }),
            ),
            Span::raw("  │  "),
            Span::raw("Total reclaimable: "),
            Span::styled(
                format_bytes(total_space),
                Style::default().fg(theme::LAVENDER).bold(),
            ),
        ]),
    ];

    if has_aggressive {
        lines.push(Line::from(vec![
            Span::styled("⚠ ", Style::default().fg(theme::YELLOW)),
            Span::styled(
                "Aggressive categories selected - may affect application data",
                Style::default().fg(theme::YELLOW),
            ),
        ]));
    }

    // Show execution results if available
    if let Some(ref summary) = app.cleanup_summary {
        lines.push(Line::from(vec![
            Span::styled("✓ ", Style::default().fg(theme::GREEN)),
            Span::raw("Last run: "),
            Span::styled(
                format!("{} cleaned", summary.space_formatted()),
                Style::default().fg(theme::GREEN),
            ),
            Span::raw(format!(
                " ({} succeeded, {} failed)",
                summary.successful, summary.failed
            )),
        ]));
    }

    let paragraph = Paragraph::new(lines).block(block);
    frame.render_widget(paragraph, area);
}

/// Render help bar
fn render_help(frame: &mut Frame, area: Rect, app: &App) {
    let help_items = if app.cleanup_preview_mode {
        vec![
            ("Space", "Toggle"),
            ("a", "All"),
            ("s", "Safe only"),
            ("n", "None"),
            ("Enter", "Preview"),
            ("c", "Clean"),
            ("Esc", "Back"),
        ]
    } else {
        vec![
            ("Enter", "Confirm"),
            ("Esc", "Cancel"),
        ]
    };

    let help_spans: Vec<Span> = help_items
        .iter()
        .flat_map(|(key, action)| {
            vec![
                Span::styled(
                    format!("[{}]", key),
                    Style::default().fg(theme::YELLOW),
                ),
                Span::raw(format!(" {}  ", action)),
            ]
        })
        .collect();

    let block = theme::minimal_block();

    let paragraph = Paragraph::new(Line::from(help_spans))
        .block(block)
        .alignment(Alignment::Center);

    frame.render_widget(paragraph, area);
}

/// Render cleanup preview results (detailed view before execution)
pub fn render_cleanup_preview(frame: &mut Frame, area: Rect, app: &App) {
    let block = theme::themed_block("Cleanup Preview - Review Before Execution", theme::YELLOW);

    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Layout: Preview list + warnings
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(0),     // Preview items
            Constraint::Length(5),  // Warnings
        ])
        .split(inner);

    // Preview items
    let items: Vec<ListItem> = app
        .cleanup_previews
        .iter()
        .filter(|p| app.cleanup_categories.contains(&p.category))
        .map(|preview| {
            let text = format!(
                "• {} - {} ({} items)\n  {}",
                preview.category.name(),
                preview.space_formatted(),
                preview.items_count,
                preview.details
            );
            ListItem::new(text).style(Style::default().fg(theme::TEXT))
        })
        .collect();

    let list = List::new(items);
    frame.render_widget(list, layout[0]);

    // Warnings section
    let warnings: Vec<String> = app
        .cleanup_previews
        .iter()
        .filter(|p| app.cleanup_categories.contains(&p.category))
        .flat_map(|p| p.warnings.clone())
        .collect();

    if !warnings.is_empty() {
        let warning_block = Block::default()
            .title(" Warnings ")
            .borders(Borders::TOP)
            .border_style(Style::default().fg(theme::YELLOW));

        let warning_text = warnings
            .iter()
            .map(|w| Line::from(vec![
                Span::styled("⚠ ", Style::default().fg(theme::YELLOW)),
                Span::raw(w.as_str()),
            ]))
            .collect::<Vec<_>>();

        let paragraph = Paragraph::new(warning_text).block(warning_block);
        frame.render_widget(paragraph, layout[1]);
    }
}

/// Render cleanup execution results
pub fn render_cleanup_results(frame: &mut Frame, area: Rect, app: &App) {
    let summary = match &app.cleanup_summary {
        Some(s) => s,
        None => return,
    };

    let title_color = if summary.failed > 0 {
        theme::YELLOW
    } else {
        theme::GREEN
    };

    let block = theme::themed_block("Cleanup Results", title_color);

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let items: Vec<ListItem> = summary
        .results
        .iter()
        .map(|result| {
            let (icon, color) = if result.success {
                ("✓", theme::GREEN)
            } else {
                ("✗", theme::RED)
            };

            let text = if result.success {
                format!(
                    "{} {} - {} ({} items)",
                    icon,
                    result.category.name(),
                    result.space_formatted(),
                    result.items_cleaned
                )
            } else {
                format!(
                    "{} {} - {}",
                    icon,
                    result.category.name(),
                    result.error.as_deref().unwrap_or("Unknown error")
                )
            };

            ListItem::new(text).style(Style::default().fg(color))
        })
        .collect();

    let list = List::new(items);
    frame.render_widget(list, inner);
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    fn create_test_terminal() -> Terminal<TestBackend> {
        let backend = TestBackend::new(80, 24);
        Terminal::new(backend).unwrap()
    }

    #[test]
    fn test_render_clean_system_no_panic() {
        let mut terminal = create_test_terminal();
        let app = App::default();

        terminal
            .draw(|f| {
                render_clean_system(f, f.area(), &app);
            })
            .unwrap();
    }

    #[test]
    fn test_render_with_selected_categories() {
        let mut terminal = create_test_terminal();
        let mut app = App::default();

        // Select some categories
        app.cleanup_categories = vec![
            CleanupCategory::PackageCache,
            CleanupCategory::Thumbnails,
        ];

        terminal
            .draw(|f| {
                render_clean_system(f, f.area(), &app);
            })
            .unwrap();
    }

    #[test]
    fn test_render_cleanup_preview_no_panic() {
        let mut terminal = create_test_terminal();
        let app = App::default();

        terminal
            .draw(|f| {
                render_cleanup_preview(f, f.area(), &app);
            })
            .unwrap();
    }
}
