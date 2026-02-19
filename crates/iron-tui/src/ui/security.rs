//! Security Modules View
//!
//! Displays and manages security-related modules such as:
//! - Firewall (ufw, firewalld)
//! - Intrusion detection (fail2ban)
//! - Audit logging (auditd)
//! - SELinux/AppArmor

use crate::app::App;
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph, Row, Table};

/// Security module category
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecurityCategory {
    Firewall,
    IntrusionDetection,
    AuditLogging,
    AccessControl,
}

impl SecurityCategory {
    /// Get category name
    pub fn name(&self) -> &'static str {
        match self {
            Self::Firewall => "Firewall",
            Self::IntrusionDetection => "Intrusion Detection",
            Self::AuditLogging => "Audit Logging",
            Self::AccessControl => "Access Control",
        }
    }
}

/// Known security modules
const SECURITY_MODULE_IDS: &[&str] = &[
    "ufw",
    "firewalld",
    "fail2ban",
    "auditd",
    "apparmor",
    "selinux",
    "clamav",
];

/// Render the security modules view
pub fn render_security_modules(frame: &mut Frame, area: Rect, app: &App) {
    // Main layout: Header, Module List, Help
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // Header
            Constraint::Min(10),    // Module list
            Constraint::Length(3),  // Help bar
        ])
        .split(area);

    render_header(frame, layout[0], app);
    render_module_list(frame, layout[1], app);
    render_help(frame, layout[2]);
}

/// Render header section
fn render_header(frame: &mut Frame, area: Rect, app: &App) {
    // Count enabled security modules
    let enabled_count = app
        .modules
        .iter()
        .filter(|m| {
            SECURITY_MODULE_IDS.contains(&m.id.as_str())
                && app.active_modules.contains(&m.id)
        })
        .count();

    let total_count = app
        .modules
        .iter()
        .filter(|m| SECURITY_MODULE_IDS.contains(&m.id.as_str()))
        .count();

    let header_text = Line::from(vec![
        Span::styled("Security Modules", Style::default().fg(Color::White).bold()),
        Span::raw("  │  "),
        Span::styled(
            format!("{}/{} enabled", enabled_count, total_count),
            Style::default().fg(if enabled_count > 0 { Color::Green } else { Color::Yellow }),
        ),
    ]);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Blue));

    let paragraph = Paragraph::new(header_text)
        .block(block)
        .alignment(Alignment::Center);

    frame.render_widget(paragraph, area);
}

/// Render the module list
fn render_module_list(frame: &mut Frame, area: Rect, app: &App) {
    let block = Block::default()
        .title(" Security Modules ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Blue));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Filter modules to only show security-related ones
    let security_modules: Vec<_> = app
        .modules
        .iter()
        .filter(|m| {
            SECURITY_MODULE_IDS.contains(&m.id.as_str())
                || m.id.contains("security")
                || m.id.contains("firewall")
                || m.id.contains("audit")
        })
        .collect();

    if security_modules.is_empty() {
        let no_modules = Paragraph::new(vec![
            Line::from(""),
            Line::from(Span::styled(
                "No security modules available",
                Style::default().fg(Color::Gray),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "Security modules can be added to your configuration.",
                Style::default().fg(Color::DarkGray),
            )),
            Line::from(""),
            Line::from(vec![
                Span::raw("Press "),
                Span::styled("[m]", Style::default().fg(Color::Yellow)),
                Span::raw(" to view all available modules."),
            ]),
        ])
        .alignment(Alignment::Center);

        frame.render_widget(no_modules, inner);
        return;
    }

    // Create table rows
    let rows: Vec<Row> = security_modules
        .iter()
        .enumerate()
        .map(|(i, module)| {
            let is_selected = i == app.selected_index;
            let is_enabled = app.active_modules.contains(&module.id);

            // Status indicator (inline, not using StatusBadge due to lifetime issues)
            let (status_icon, status_text, status_color) = if is_enabled {
                ("●", "Enabled", Color::Green)
            } else {
                ("○", "Disabled", Color::Gray)
            };

            let style = if is_selected {
                Style::default().bg(Color::DarkGray).fg(Color::White)
            } else {
                Style::default().fg(Color::White)
            };

            let description = module
                .description
                .as_deref()
                .unwrap_or("No description");

            Row::new(vec![
                Cell::from(format!("{} {}", status_icon, status_text))
                    .style(Style::default().fg(status_color)),
                Cell::from(module.name.as_str()),
                Cell::from(description).style(Style::default().fg(Color::Gray)),
            ])
            .style(style)
        })
        .collect();

    let widths = [
        Constraint::Length(12),  // Status
        Constraint::Length(20),  // Name
        Constraint::Min(30),     // Description
    ];

    let table = Table::new(rows, widths)
        .header(
            Row::new(vec!["Status", "Module", "Description"])
                .style(Style::default().fg(Color::Yellow).bold())
                .bottom_margin(1),
        )
        .column_spacing(2);

    frame.render_widget(table, inner);
}

/// Render help bar
fn render_help(frame: &mut Frame, area: Rect) {
    let help_items = vec![
        ("↑↓", "Navigate"),
        ("Enter", "Toggle"),
        ("i", "Install"),
        ("Esc", "Back"),
    ];

    let help_spans: Vec<Span> = help_items
        .iter()
        .flat_map(|(key, action)| {
            vec![
                Span::styled(
                    format!("[{}]", key),
                    Style::default().fg(Color::Yellow),
                ),
                Span::raw(format!(" {}  ", action)),
            ]
        })
        .collect();

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray));

    let paragraph = Paragraph::new(Line::from(help_spans))
        .block(block)
        .alignment(Alignment::Center);

    frame.render_widget(paragraph, area);
}

use ratatui::widgets::Cell;

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    fn create_test_terminal() -> Terminal<TestBackend> {
        let backend = TestBackend::new(100, 25);
        Terminal::new(backend).unwrap()
    }

    #[test]
    fn test_render_security_modules_no_panic() {
        let mut terminal = create_test_terminal();
        let app = App::default();

        terminal
            .draw(|f| {
                render_security_modules(f, f.area(), &app);
            })
            .unwrap();
    }

    #[test]
    fn test_security_category_names() {
        assert_eq!(SecurityCategory::Firewall.name(), "Firewall");
        assert_eq!(SecurityCategory::IntrusionDetection.name(), "Intrusion Detection");
        assert_eq!(SecurityCategory::AuditLogging.name(), "Audit Logging");
        assert_eq!(SecurityCategory::AccessControl.name(), "Access Control");
    }

    #[test]
    fn test_security_module_ids() {
        assert!(SECURITY_MODULE_IDS.contains(&"ufw"));
        assert!(SECURITY_MODULE_IDS.contains(&"fail2ban"));
        assert!(SECURITY_MODULE_IDS.contains(&"apparmor"));
    }
}
