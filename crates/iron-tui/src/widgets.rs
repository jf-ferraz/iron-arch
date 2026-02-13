//! Iron TUI Custom Widgets
//!
//! Reusable UI components for the TUI interface.

use crate::app::{App, ConfirmAction, View};
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};

/// Render the header bar
pub fn render_header(frame: &mut Frame, area: Rect, app: &App) {
    let host_name = app.current_host.as_deref().unwrap_or("unknown");
    let bundle_name = app
        .active_bundle
        .as_ref()
        .map(|b| b.id.as_str())
        .unwrap_or("none");

    // Create header with title and status info
    let title = Span::styled(
        " IRON ",
        Style::default()
            .fg(Color::White)
            .bg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    );

    let view_name = match app.view {
        View::Dashboard => "Dashboard",
        View::SetupWizard => "Setup",
        View::Bundles => "Bundles",
        View::BundleDetail => "Bundle Detail",
        View::Profiles => "Profiles",
        View::ProfileDetail => "Profile Detail",
        View::Modules => "Modules",
        View::ModuleDetail => "Module Detail",
        View::UpdatePreview => "Update",
        View::Sync => "Sync",
        View::Settings => "Settings",
    };

    let status_info = format!("{} │ {}", host_name, bundle_name);

    // Calculate spacing
    let title_len = 6; // " IRON "
    let view_len = view_name.len();
    let status_len = status_info.len();
    let total_content = title_len + view_len + status_len + 4; // padding

    let spacing = if area.width as usize > total_content {
        " ".repeat(area.width as usize - total_content)
    } else {
        " ".to_string()
    };

    let header_content = Line::from(vec![
        title,
        Span::raw(" "),
        Span::styled(view_name, Style::default().fg(Color::Yellow)),
        Span::raw(&spacing),
        Span::styled(&status_info, Style::default().fg(Color::Gray)),
        Span::raw(" "),
    ]);

    let block = Block::default()
        .borders(Borders::BOTTOM)
        .border_style(Style::default().fg(Color::DarkGray));

    let para = Paragraph::new(header_content).block(block);

    frame.render_widget(para, area);
}

/// Render the footer bar with keybindings
pub fn render_footer(frame: &mut Frame, area: Rect, app: &App) {
    // Show error message if present
    if let Some(ref error) = app.error_message {
        let error_line = Line::from(vec![
            Span::styled(" ✗ ", Style::default().fg(Color::Red)),
            Span::styled(error, Style::default().fg(Color::Red)),
        ]);

        let block = Block::default()
            .borders(Borders::TOP)
            .border_style(Style::default().fg(Color::Red));

        let para = Paragraph::new(error_line).block(block);
        frame.render_widget(para, area);
        return;
    }

    // Show status message if present
    if let Some(ref status) = app.status_message {
        let status_line = Line::from(vec![
            Span::styled(" ● ", Style::default().fg(Color::Green)),
            Span::styled(status, Style::default().fg(Color::Green)),
        ]);

        let block = Block::default()
            .borders(Borders::TOP)
            .border_style(Style::default().fg(Color::DarkGray));

        let para = Paragraph::new(status_line).block(block);
        frame.render_widget(para, area);
        return;
    }

    // Default keybindings footer
    let keybindings = match app.view {
        View::Dashboard => vec![("[q]", "Quit"), ("[?]", "Help"), ("[Tab]", "Navigate")],
        View::Bundles | View::Profiles | View::Modules => vec![
            ("[↑↓]", "Select"),
            ("[Enter]", "Details"),
            ("[Esc]", "Back"),
            ("[?]", "Help"),
        ],
        View::BundleDetail | View::ProfileDetail | View::ModuleDetail => {
            vec![("[Enter]", "Activate"), ("[Esc]", "Back"), ("[?]", "Help")]
        }
        View::UpdatePreview => vec![("[u]", "Update"), ("[r]", "Refresh"), ("[Esc]", "Back")],
        _ => vec![("[q]", "Quit"), ("[?]", "Help"), ("[Esc]", "Back")],
    };

    let mut spans: Vec<Span> = vec![Span::raw(" ")];
    for (key, action) in keybindings {
        spans.push(Span::styled(key, Style::default().fg(Color::Yellow)));
        spans.push(Span::raw(" "));
        spans.push(Span::raw(action));
        spans.push(Span::raw("  "));
    }

    let footer_line = Line::from(spans);

    let block = Block::default()
        .borders(Borders::TOP)
        .border_style(Style::default().fg(Color::DarkGray));

    let para = Paragraph::new(footer_line).block(block);

    frame.render_widget(para, area);
}

/// Render help overlay
pub fn render_help_overlay(frame: &mut Frame, area: Rect) {
    // Calculate centered popup area
    let popup_width = 50.min(area.width.saturating_sub(4));
    let popup_height = 18.min(area.height.saturating_sub(4));

    let popup_area = centered_rect(popup_width, popup_height, area);

    // Clear the background
    frame.render_widget(Clear, popup_area);

    let help_text = vec![
        Line::from(Span::styled(
            "Keyboard Shortcuts",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from("Navigation:"),
        Line::from("  d         Go to Dashboard"),
        Line::from("  b         Go to Bundles"),
        Line::from("  p         Go to Profiles"),
        Line::from("  m         Go to Modules"),
        Line::from("  u         Go to Update"),
        Line::from("  s         Go to Settings"),
        Line::from("  Tab       Next section"),
        Line::from("  Shift+Tab Previous section"),
        Line::from(""),
        Line::from("Lists:"),
        Line::from("  ↑/k       Move up"),
        Line::from("  ↓/j       Move down"),
        Line::from("  Enter     Select/Activate"),
        Line::from("  Esc       Go back"),
        Line::from(""),
        Line::from("General:"),
        Line::from("  ?         Toggle help"),
        Line::from("  q         Quit"),
        Line::from("  Ctrl+C    Force quit"),
        Line::from(""),
        Line::from(Span::styled(
            "Press any key to close",
            Style::default().fg(Color::Gray),
        )),
    ];

    let block = Block::default()
        .title(" Help ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .style(Style::default().bg(Color::Black));

    let para = Paragraph::new(help_text)
        .block(block)
        .wrap(Wrap { trim: true });

    frame.render_widget(para, popup_area);
}

/// Render confirm dialog
pub fn render_confirm_dialog(frame: &mut Frame, area: Rect, app: &App) {
    let message = match &app.confirm_action {
        Some(ConfirmAction::SwitchBundle(id)) => format!("Switch to bundle '{}'?", id),
        Some(ConfirmAction::RemoveBundle(id)) => format!("Remove bundle '{}'?", id),
        Some(ConfirmAction::EnableModule(id)) => format!("Enable module '{}'?", id),
        Some(ConfirmAction::DisableModule(id)) => format!("Disable module '{}'?", id),
        Some(ConfirmAction::RunUpdate) => "Run system update?".to_string(),
        Some(ConfirmAction::Quit) => "Quit Iron?".to_string(),
        None => "Confirm action?".to_string(),
    };

    // Calculate centered popup area
    let popup_width = 40.min(area.width.saturating_sub(4));
    let popup_height = 7;

    let popup_area = centered_rect(popup_width, popup_height, area);

    // Clear the background
    frame.render_widget(Clear, popup_area);

    let confirm_text = vec![
        Line::from(""),
        Line::from(message),
        Line::from(""),
        Line::from(vec![
            Span::styled("[Y]", Style::default().fg(Color::Green)),
            Span::raw(" Yes  "),
            Span::styled("[N]", Style::default().fg(Color::Red)),
            Span::raw(" No"),
        ]),
    ];

    let block = Block::default()
        .title(" Confirm ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow))
        .style(Style::default().bg(Color::Black));

    let para = Paragraph::new(confirm_text)
        .block(block)
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: true });

    frame.render_widget(para, popup_area);
}

/// Helper function to create a centered rect
fn centered_rect(width: u16, height: u16, area: Rect) -> Rect {
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 2;

    Rect::new(x, y, width.min(area.width), height.min(area.height))
}

/// Status badge widget for health indicators
#[derive(Debug, Clone)]
pub struct StatusBadge {
    pub label: String,
    pub status: BadgeStatus,
}

#[derive(Debug, Clone, Copy)]
pub enum BadgeStatus {
    Ok,
    Warning,
    Error,
    Inactive,
}

impl StatusBadge {
    pub fn new(label: impl Into<String>, status: BadgeStatus) -> Self {
        Self {
            label: label.into(),
            status,
        }
    }

    pub fn render(&self) -> Span<'_> {
        let (symbol, style) = match self.status {
            BadgeStatus::Ok => ("●", Style::default().fg(Color::Green)),
            BadgeStatus::Warning => ("⚠", Style::default().fg(Color::Yellow)),
            BadgeStatus::Error => ("✗", Style::default().fg(Color::Red)),
            BadgeStatus::Inactive => ("○", Style::default().fg(Color::Gray)),
        };

        Span::styled(format!("{} {}", symbol, self.label), style)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ==========================================================================
    // StatusBadge tests
    // ==========================================================================

    #[test]
    fn test_status_badge_ok() {
        let badge = StatusBadge::new("Test", BadgeStatus::Ok);
        let span = badge.render();
        assert!(span.content.contains("●"));
        assert!(span.content.contains("Test"));
    }

    #[test]
    fn test_status_badge_warning() {
        let badge = StatusBadge::new("Warning", BadgeStatus::Warning);
        let span = badge.render();
        assert!(span.content.contains("⚠"));
    }

    #[test]
    fn test_status_badge_error() {
        let badge = StatusBadge::new("Error", BadgeStatus::Error);
        let span = badge.render();
        assert!(span.content.contains("✗"));
    }

    #[test]
    fn test_status_badge_inactive() {
        let badge = StatusBadge::new("Inactive", BadgeStatus::Inactive);
        let span = badge.render();
        assert!(span.content.contains("○"));
    }

    #[test]
    fn test_status_badge_label() {
        let badge = StatusBadge::new("Custom Label", BadgeStatus::Ok);
        assert_eq!(badge.label, "Custom Label");
    }

    #[test]
    fn test_status_badge_clone() {
        let badge = StatusBadge::new("Test", BadgeStatus::Warning);
        let cloned = badge.clone();
        assert_eq!(badge.label, cloned.label);
    }

    // ==========================================================================
    // BadgeStatus tests
    // ==========================================================================

    #[test]
    fn test_badge_status_copy() {
        let status = BadgeStatus::Ok;
        let copied = status;
        assert!(matches!(copied, BadgeStatus::Ok));
    }

    #[test]
    fn test_badge_status_debug() {
        let status = BadgeStatus::Warning;
        let debug_str = format!("{:?}", status);
        assert!(debug_str.contains("Warning"));
    }

    // ==========================================================================
    // centered_rect tests
    // ==========================================================================

    #[test]
    fn test_centered_rect() {
        let area = Rect::new(0, 0, 100, 50);
        let popup = centered_rect(40, 10, area);
        assert_eq!(popup.x, 30);
        assert_eq!(popup.y, 20);
        assert_eq!(popup.width, 40);
        assert_eq!(popup.height, 10);
    }

    #[test]
    fn test_centered_rect_larger_than_area() {
        let area = Rect::new(0, 0, 30, 20);
        let popup = centered_rect(50, 30, area);
        // Should be clamped to area size
        assert!(popup.width <= area.width);
        assert!(popup.height <= area.height);
    }

    #[test]
    fn test_centered_rect_exact_size() {
        let area = Rect::new(10, 10, 50, 30);
        let popup = centered_rect(50, 30, area);
        // Should fill the entire area
        assert_eq!(popup.x, 10);
        assert_eq!(popup.y, 10);
    }

    #[test]
    fn test_centered_rect_small_popup() {
        let area = Rect::new(0, 0, 80, 40);
        let popup = centered_rect(10, 5, area);
        assert_eq!(popup.x, 35); // (80-10)/2
        assert_eq!(popup.y, 17); // (40-5)/2 = 17 (integer division)
    }

    #[test]
    fn test_centered_rect_with_offset() {
        let area = Rect::new(20, 10, 100, 50);
        let popup = centered_rect(40, 10, area);
        // Should be centered relative to area position
        assert_eq!(popup.x, 20 + 30); // area.x + (100-40)/2
        assert_eq!(popup.y, 10 + 20); // area.y + (50-10)/2
    }

    #[test]
    fn test_centered_rect_zero_size() {
        let area = Rect::new(0, 0, 100, 50);
        let popup = centered_rect(0, 0, area);
        assert_eq!(popup.width, 0);
        assert_eq!(popup.height, 0);
    }

    // ==========================================================================
    // View keybindings tests (content verification)
    // ==========================================================================

    #[test]
    fn test_view_names() {
        // Test that View enum has correct string representations
        let views = vec![
            (View::Dashboard, "Dashboard"),
            (View::SetupWizard, "Setup"),
            (View::Bundles, "Bundles"),
            (View::Profiles, "Profiles"),
            (View::Modules, "Modules"),
            (View::Settings, "Settings"),
        ];

        for (view, expected_name) in views {
            let name = match view {
                View::Dashboard => "Dashboard",
                View::SetupWizard => "Setup",
                View::Bundles => "Bundles",
                View::BundleDetail => "Bundle Detail",
                View::Profiles => "Profiles",
                View::ProfileDetail => "Profile Detail",
                View::Modules => "Modules",
                View::ModuleDetail => "Module Detail",
                View::UpdatePreview => "Update",
                View::Sync => "Sync",
                View::Settings => "Settings",
            };
            assert_eq!(name, expected_name);
        }
    }

    // ==========================================================================
    // ConfirmAction tests
    // ==========================================================================

    #[test]
    fn test_confirm_action_messages() {
        let actions = vec![
            (
                ConfirmAction::SwitchBundle("hyprland".to_string()),
                "Switch to bundle 'hyprland'?",
            ),
            (
                ConfirmAction::EnableModule("nvim".to_string()),
                "Enable module 'nvim'?",
            ),
            (
                ConfirmAction::DisableModule("kitty".to_string()),
                "Disable module 'kitty'?",
            ),
            (ConfirmAction::Quit, "Quit Iron?"),
        ];

        for (action, expected_substr) in actions {
            let message = match &action {
                ConfirmAction::SwitchBundle(id) => format!("Switch to bundle '{}'?", id),
                ConfirmAction::RemoveBundle(id) => format!("Remove bundle '{}'?", id),
                ConfirmAction::EnableModule(id) => format!("Enable module '{}'?", id),
                ConfirmAction::DisableModule(id) => format!("Disable module '{}'?", id),
                ConfirmAction::RunUpdate => "Run system update?".to_string(),
                ConfirmAction::Quit => "Quit Iron?".to_string(),
            };
            assert_eq!(message, expected_substr);
        }
    }
}
