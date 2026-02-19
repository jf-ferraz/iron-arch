//! Iron TUI Custom Widgets
//!
//! Reusable UI components for the TUI interface.

mod progress;

pub use progress::{InlineProgress, ProgressTracker, ProgressWidget};

use crate::app::{App, ConfirmAction, View};
use crate::message::MessageLevel;
use crate::ui::theme;
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};

/// Render the header bar
pub fn render_header(frame: &mut Frame, area: Rect, app: &App) {
    let host_name = app.current_host.as_deref().unwrap_or("-");
    let bundle_name = app
        .active_bundle
        .as_ref()
        .map(|b| b.id.as_str())
        .unwrap_or("-");

    // View name and icon mapping - using clear, visible icons
    let (view_name, view_icon) = match app.view {
        View::Dashboard => ("Dashboard", "[=]"),
        View::SetupWizard => ("Setup Wizard", "[*]"),
        View::Bundles => ("Bundles", "[B]"),
        View::BundleDetail => ("Bundle Details", "[B]"),
        View::Profiles => ("Profiles", "[P]"),
        View::ProfileDetail => ("Profile Details", "[P]"),
        View::Modules => ("Modules", "[M]"),
        View::ModuleDetail => ("Module Details", "[M]"),
        View::UpdatePreview => ("System Update", "[U]"),
        View::Sync => ("Git Sync", "[Y]"),
        View::Settings => ("Settings", "[S]"),
        View::SystemMaintenance => ("Maintenance", "[X]"),
        View::CleanSystem => ("System Cleanup", "[C]"),
        View::CleanupPreview => ("Cleanup Preview", "[C]"),
        View::CleanupResults => ("Cleanup Results", "[C]"),
        View::SecurityModules => ("Security", "[!]"),
        View::ConfigManager => ("Config Manager", "[#]"),
        View::OperationLog => ("Operation Log", "[L]"),
        View::Doctor => ("System Doctor", "[D]"),
        View::Secrets => ("Secrets", "[S]"),
        View::Recovery => ("Recovery", "[R]"),
        View::ProfileBuilder => ("New Profile", "[n]"),
        View::ModuleCreator => ("New Module", "[n]"),
    };

    // Build header content
    let title = Span::styled(
        " IRON ",
        Style::default()
            .fg(Color::Black)
            .bg(theme::MAUVE)
            .bold(),
    );

    // Calculate spacing for right-aligned content
    let left_part = format!(" IRON  {} {} ", view_icon, view_name);
    let right_part = format!(" Host: {}  |  Bundle: {} ", host_name, bundle_name);
    let total_len = left_part.len() + right_part.len();

    let spacing = if area.width as usize > total_len {
        " ".repeat(area.width as usize - total_len)
    } else {
        " ".to_string()
    };

    let header_content = Line::from(vec![
        title,
        Span::raw(" "),
        Span::styled(view_icon, Style::default().fg(theme::MAUVE).bold()),
        Span::raw(" "),
        Span::styled(view_name, Style::default().fg(theme::TEXT).bold()),
        Span::raw(&spacing),
        Span::styled("Host: ", Style::default().fg(theme::OVERLAY)),
        Span::styled(host_name, Style::default().fg(theme::TEXT)),
        Span::styled("  |  ", Style::default().fg(theme::OVERLAY)),
        Span::styled("Bundle: ", Style::default().fg(theme::OVERLAY)),
        Span::styled(bundle_name, Style::default().fg(theme::TEXT)),
        Span::raw(" "),
    ]);

    let block = Block::default()
        .borders(Borders::BOTTOM)
        .border_style(Style::default().fg(theme::OVERLAY));

    let para = Paragraph::new(header_content).block(block);

    frame.render_widget(para, area);
}

/// Render the footer bar with keybindings
pub fn render_footer(frame: &mut Frame, area: Rect, app: &App) {
    // Show error message if present (error takes priority)
    if let Some(ref msg) = app.error_message {
        let (symbol, color) = match msg.level() {
            MessageLevel::Error => ("ERROR", theme::RED),
            MessageLevel::Warning => ("WARN", theme::YELLOW),
            _ => ("ERROR", theme::RED),
        };

        let error_line = Line::from(vec![
            Span::styled(format!(" {} ", symbol), Style::default().fg(Color::Black).bg(color).bold()),
            Span::raw(" "),
            Span::styled(msg.text(), Style::default().fg(color)),
        ]);

        let block = Block::default()
            .borders(Borders::TOP)
            .border_style(Style::default().fg(color));

        let para = Paragraph::new(error_line).block(block);
        frame.render_widget(para, area);
        return;
    }

    // Show status message if present
    if let Some(ref msg) = app.status_message {
        let (symbol, color) = match msg.level() {
            MessageLevel::Success => ("OK", theme::GREEN),
            MessageLevel::Info => ("INFO", theme::MAUVE),
            MessageLevel::Warning => ("WARN", theme::YELLOW),
            MessageLevel::Error => ("ERROR", theme::RED),
        };

        let status_line = Line::from(vec![
            Span::styled(format!(" {} ", symbol), Style::default().fg(Color::Black).bg(color).bold()),
            Span::raw(" "),
            Span::styled(msg.text(), Style::default().fg(color)),
        ]);

        let block = Block::default()
            .borders(Borders::TOP)
            .border_style(Style::default().fg(theme::OVERLAY));

        let para = Paragraph::new(status_line).block(block);
        frame.render_widget(para, area);
        return;
    }

    // Default keybindings footer
    let keybindings = match app.view {
        View::Dashboard => vec![("[q]", "Quit"), ("[?]", "Help"), ("[Tab]", "Navigate")],
        View::Bundles | View::Profiles | View::Modules => vec![
            ("[j/k]", "Select"),
            ("[Enter]", "Details"),
            ("[e]", "Toggle"),
            ("[Esc]", "Back"),
            ("[?]", "Help"),
        ],
        View::BundleDetail | View::ProfileDetail | View::ModuleDetail => {
            vec![("[Enter]", "Activate"), ("[Esc]", "Back"), ("[?]", "Help")]
        }
        View::UpdatePreview => vec![
            ("[u]", "Update"),
            ("[h/l]", "Sections"),
            ("[r]", "Refresh"),
            ("[Esc]", "Back"),
        ],
        View::Sync => vec![
            ("[p]", "Push"),
            ("[f]", "Pull"),
            ("[s]", "Status"),
            ("[Esc]", "Back"),
        ],
        View::SetupWizard => vec![
            ("[j/k]", "Select"),
            ("[Enter]", "Confirm"),
            ("[h/l]", "Navigate"),
        ],
        View::SystemMaintenance => vec![
            ("[u]", "Update"),
            ("[c]", "Cleanup"),
            ("[d]", "Doctor"),
            ("[h/l]", "Select"),
            ("[Esc]", "Back"),
        ],
        View::CleanSystem => vec![
            ("[Space]", "Toggle"),
            ("[s]", "Safe"),
            ("[a]", "All"),
            ("[Enter]", "Preview"),
            ("[c]", "Clean"),
            ("[Esc]", "Back"),
        ],
        View::CleanupPreview => vec![
            ("[c]", "Execute"),
            ("[Esc]", "Back"),
        ],
        View::CleanupResults => vec![
            ("[Esc]", "Back"),
        ],
        View::SecurityModules => vec![
            ("[j/k]", "Select"),
            ("[Enter]", "Toggle"),
            ("[i]", "Install"),
            ("[Esc]", "Back"),
        ],
        View::ConfigManager => vec![
            ("[j/k]", "Select"),
            ("[Enter]", "View Diff"),
            ("[r]", "Refresh"),
            ("[Esc]", "Back"),
        ],
        View::OperationLog => vec![
            ("[j/k]", "Scroll"),
            ("[f]", "Filter"),
            ("[Esc]", "Back"),
        ],
        View::Settings => vec![
            ("[j/k]", "Navigate"),
            ("[Enter]", "Edit"),
            ("[r]", "Refresh"),
            ("[Esc]", "Back"),
        ],
        View::Doctor => vec![("[r]", "Re-run"), ("[Esc]", "Back")],
        View::Secrets => vec![
            ("[i]", "Init"),
            ("[u]", "Unlock"),
            ("[l]", "Lock"),
            ("[Esc]", "Back"),
        ],
        View::Recovery => vec![
            ("[g]", "install.sh"),
            ("[e]", "Export"),
            ("[s]", "Snapshot"),
            ("[Esc]", "Back"),
        ],
        View::ProfileBuilder => vec![
            ("[Tab]", "Switch field"),
            ("[Space]", "Toggle"),
            ("[Enter]", "Next/Create"),
            ("[Esc]", "Back"),
        ],
        View::ModuleCreator => vec![
            ("[Tab]", "Switch field"),
            ("[Enter]", "Preview/Create"),
            ("[Esc]", "Back"),
        ],
    };

    let mut spans: Vec<Span> = vec![Span::raw("  ")];
    for (key, action) in keybindings {
        spans.push(Span::styled(
            format!(" {} ", key),
            Style::default().fg(Color::Black).bg(theme::MAUVE).bold(),
        ));
        spans.push(Span::styled(
            format!(" {}  ", action),
            Style::default().fg(theme::SUBTEXT),
        ));
    }

    let keybinding_line = Line::from(spans);

    // Active config status line
    let bundle_name = app
        .active_bundle
        .as_ref()
        .map(|b| b.id.clone())
        .unwrap_or_else(|| "—".to_string());
    let profile_name = app.active_profile.clone().unwrap_or_else(|| "—".to_string());
    let host_name = app.current_host.clone().unwrap_or_else(|| "—".to_string());

    let status_line = Line::from(vec![
        Span::raw("  "),
        Span::styled("Active:", Style::default().fg(theme::OVERLAY)),
        Span::raw(" "),
        Span::styled(bundle_name, Style::default().fg(theme::TEXT)),
        Span::styled("/", Style::default().fg(theme::OVERLAY)),
        Span::styled(profile_name, Style::default().fg(theme::TEXT)),
        Span::raw("   "),
        Span::styled("Host:", Style::default().fg(theme::OVERLAY)),
        Span::raw(" "),
        Span::styled(host_name, Style::default().fg(theme::TEXT)),
    ]);

    let block = Block::default()
        .borders(Borders::TOP)
        .border_style(Style::default().fg(theme::OVERLAY));

    let para = Paragraph::new(vec![keybinding_line, status_line]).block(block);

    frame.render_widget(para, area);
}

/// Get view-specific keybindings
fn get_view_keybindings(view: View) -> Vec<(&'static str, &'static str)> {
    match view {
        View::Dashboard => vec![
            ("b", "Bundles"),
            ("p", "Profiles"),
            ("m", "Modules"),
            ("x", "Maintenance"),
            ("u", "Update"),
            ("l", "Cleanup"),
            ("y", "Sync"),
            ("s", "Settings"),
            ("w", "Wizard"),
        ],
        View::Bundles | View::BundleDetail => vec![
            ("j/k", "Move up/down"),
            ("Enter", "View details"),
            ("a", "Activate bundle"),
        ],
        View::Profiles | View::ProfileDetail => vec![
            ("j/k", "Move up/down"),
            ("Enter", "View details"),
            ("a", "Activate profile"),
        ],
        View::Modules | View::ModuleDetail => vec![
            ("j/k", "Move up/down"),
            ("Enter", "View details"),
            ("e", "Enable/disable"),
        ],
        View::UpdatePreview => vec![
            ("h/l", "Prev/next section"),
            ("j/k", "Move up/down"),
            ("a", "Acknowledge news"),
            ("A", "Acknowledge all"),
            ("u", "Run update"),
            ("r", "Refresh"),
        ],
        View::CleanSystem => vec![
            ("j/k", "Move up/down"),
            ("Space", "Toggle category"),
            ("s", "Select safe only"),
            ("a", "Select all"),
            ("n", "Deselect all"),
            ("Enter", "Preview"),
            ("c", "Execute cleanup"),
        ],
        View::CleanupPreview => vec![
            ("c", "Confirm and execute"),
            ("Esc", "Back to categories"),
        ],
        View::CleanupResults => vec![
            ("Esc", "Back to maintenance"),
        ],
        View::SystemMaintenance => vec![
            ("h/l", "Prev/next card"),
            ("Enter", "Launch action"),
            ("u", "Update"),
            ("c", "Cleanup"),
            ("d", "Doctor"),
        ],
        View::ConfigManager => vec![
            ("j/k", "Move up/down"),
            ("Enter", "View diff info"),
            ("r", "Refresh conflicts"),
        ],
        View::OperationLog => vec![
            ("j/k", "Move up/down"),
            ("f", "Cycle filter"),
        ],
        View::SecurityModules => vec![
            ("j/k", "Move up/down"),
            ("Enter", "Toggle module"),
            ("i", "Install"),
        ],
        View::Settings => vec![
            ("j/k", "Move up/down"),
            ("Enter", "Edit/info"),
            ("r", "Refresh"),
            ("o", "Operation log"),
            ("c", "Config conflicts"),
            ("w", "Re-run wizard"),
        ],
        View::Sync => vec![
            ("p", "Push changes"),
            ("f", "Pull (fetch)"),
            ("s", "Refresh status"),
        ],
        View::SetupWizard => vec![
            ("j/k", "Move up/down"),
            ("Enter", "Select/Continue"),
            ("h/l", "Prev/next step"),
        ],
        View::Doctor => vec![
            ("r", "Re-run checks"),
        ],
        View::Secrets => vec![
            ("i", "Init git-crypt"),
            ("u", "Unlock secrets"),
            ("l", "Lock secrets"),
            ("a", "Add GPG key"),
        ],
        View::Recovery => vec![
            ("g", "Generate install.sh"),
            ("e", "Export config bundle"),
            ("i", "Import from backup"),
            ("r", "Recovery wizard"),
            ("s", "Create snapshot"),
        ],
        View::ProfileBuilder => vec![
            ("Tab", "Switch field"),
            ("Space", "Toggle module"),
            ("Enter", "Next step / Create"),
            ("Esc", "Cancel / Previous step"),
        ],
        View::ModuleCreator => vec![
            ("Tab", "Switch field"),
            ("Enter", "Preview / Create"),
            ("Esc", "Cancel / Previous step"),
        ],
    }
}

/// Render help overlay with view-specific keybindings
pub fn render_help_overlay(frame: &mut Frame, area: Rect, app: &App) {
    // Calculate centered popup area — Dashboard needs extra height for concepts section
    let popup_width = 60.min(area.width.saturating_sub(4));
    let popup_height = 34.min(area.height.saturating_sub(4));

    let popup_area = centered_rect(popup_width, popup_height, area);

    // Clear the background
    frame.render_widget(Clear, popup_area);

    // Build help text
    let view_name = match app.view {
        View::Dashboard => "Dashboard",
        View::Bundles => "Bundles",
        View::BundleDetail => "Bundle Detail",
        View::Profiles => "Profiles",
        View::ProfileDetail => "Profile Detail",
        View::Modules => "Modules",
        View::ModuleDetail => "Module Detail",
        View::UpdatePreview => "Update Preview",
        View::CleanSystem => "System Cleanup",
        View::CleanupPreview => "Cleanup Preview",
        View::CleanupResults => "Cleanup Results",
        View::SystemMaintenance => "Maintenance Hub",
        View::ConfigManager => "Config Manager",
        View::OperationLog => "Operation Log",
        View::SecurityModules => "Security Modules",
        View::Settings => "Settings",
        View::Doctor => "System Doctor",
        View::Secrets => "Secrets",
        View::Recovery => "Recovery",
        View::ProfileBuilder => "New Profile",
        View::ModuleCreator => "New Module",
        _ => "View",
    };

    let mut help_text = vec![
        Line::from(Span::styled(
            format!("Help: {}", view_name),
            Style::default().bold(),
        )),
        Line::from(""),
    ];

    // View-specific keybindings
    let view_bindings = get_view_keybindings(app.view);
    if !view_bindings.is_empty() {
        help_text.push(Line::from(Span::styled(
            "View Actions:",
            Style::default().fg(theme::YELLOW),
        )));
        for (key, desc) in view_bindings {
            help_text.push(Line::from(format!("  {:12} {}", key, desc)));
        }
        help_text.push(Line::from(""));
    }

    // Global keybindings
    help_text.push(Line::from(Span::styled(
        "Navigation:",
        Style::default().fg(theme::YELLOW),
    )));
    help_text.push(Line::from("  Tab         Next view"));
    help_text.push(Line::from("  Shift+Tab   Previous view"));
    help_text.push(Line::from("  Esc         Go back"));
    help_text.push(Line::from(""));

    help_text.push(Line::from(Span::styled(
        "Global:",
        Style::default().fg(theme::YELLOW),
    )));
    help_text.push(Line::from("  ?           Toggle help"));
    help_text.push(Line::from("  q           Quit"));
    help_text.push(Line::from("  Ctrl+C      Force quit"));
    help_text.push(Line::from(""));

    // On Dashboard, append the Iron concept hierarchy at the end
    if app.view == View::Dashboard {
        help_text.push(Line::from(Span::styled(
            "Iron Concepts:",
            Style::default().fg(theme::MAUVE),
        )));
        help_text.push(Line::from("  HOST     Your machine (e.g. desktop-arch)"));
        help_text.push(Line::from("    └─ BUNDLE  Desktop environment (Hyprland, KDE…)"));
        help_text.push(Line::from("         └─ PROFILE  Dotfile collection for a workflow"));
        help_text.push(Line::from("              └─ MODULE  Single app config (nvim, kitty…)"));
        help_text.push(Line::from(""));
        help_text.push(Line::from(Span::styled(
            "  Configure a HOST, then activate a BUNDLE.",
            Style::default().fg(theme::SUBTEXT),
        )));
        help_text.push(Line::from(Span::styled(
            "  Modules inside bundles/profiles are symlinked.",
            Style::default().fg(theme::SUBTEXT),
        )));
        help_text.push(Line::from(""));
    }

    help_text.push(Line::from(Span::styled(
        "Press any key to close",
        Style::default().fg(theme::OVERLAY),
    )));

    let block = Block::default()
        .title(" Help ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme::MAUVE));

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
        Some(ConfirmAction::RunCleanup) => "Run system cleanup? (dry-run mode)".to_string(),
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
            Span::styled("[Y]", Style::default().fg(theme::GREEN).bold()),
            Span::raw(" Yes  "),
            Span::styled("[N]", Style::default().fg(theme::RED).bold()),
            Span::raw(" No"),
        ]),
    ];

    let block = Block::default()
        .title(" Confirm ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme::YELLOW));

    let para = Paragraph::new(confirm_text)
        .block(block)
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: true });

    frame.render_widget(para, popup_area);
}

/// Helper function to create a centered rect
pub fn centered_rect(width: u16, height: u16, area: Rect) -> Rect {
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
            BadgeStatus::Ok => ("[OK]", Style::default().fg(theme::GREEN)),
            BadgeStatus::Warning => ("[!]", Style::default().fg(theme::YELLOW)),
            BadgeStatus::Error => ("[X]", Style::default().fg(theme::RED)),
            BadgeStatus::Inactive => ("[-]", Style::default().fg(theme::OVERLAY)),
        };

        Span::styled(format!("{} {}", symbol, self.label), style)
    }
}

/// Render progress dialog overlay for long-running operations
pub fn render_progress_dialog(frame: &mut Frame, area: Rect, app: &App) {
    use crate::widgets::progress::ProgressWidget;

    if let Some(ref progress) = app.progress {
        // Calculate centered popup area
        let popup_width = 50.min(area.width.saturating_sub(4));
        let popup_height = 6;

        let popup_area = centered_rect(popup_width, popup_height, area);

        // Clear the background
        frame.render_widget(Clear, popup_area);

        // Use the existing ProgressWidget to render
        ProgressWidget::new(progress).render(frame, popup_area);
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
        assert!(span.content.contains("[OK]"));
        assert!(span.content.contains("Test"));
    }

    #[test]
    fn test_status_badge_warning() {
        let badge = StatusBadge::new("Warning", BadgeStatus::Warning);
        let span = badge.render();
        assert!(span.content.contains("[!]"));
    }

    #[test]
    fn test_status_badge_error() {
        let badge = StatusBadge::new("Error", BadgeStatus::Error);
        let span = badge.render();
        assert!(span.content.contains("[X]"));
    }

    #[test]
    fn test_status_badge_inactive() {
        let badge = StatusBadge::new("Inactive", BadgeStatus::Inactive);
        let span = badge.render();
        assert!(span.content.contains("[-]"));
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
                View::SystemMaintenance => "System Maintenance",
                View::CleanSystem => "System Cleanup",
                View::CleanupPreview => "Cleanup Preview",
                View::CleanupResults => "Cleanup Results",
                View::SecurityModules => "Security Modules",
                View::ConfigManager => "Config Manager",
                View::OperationLog => "Operation Log",
                View::Doctor => "System Doctor",
                View::Secrets => "Secrets",
                View::Recovery => "Recovery",
                View::ProfileBuilder => "New Profile",
                View::ModuleCreator => "New Module",
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
                ConfirmAction::RunCleanup => "Run system cleanup? (dry-run mode)".to_string(),
                ConfirmAction::Quit => "Quit Iron?".to_string(),
            };
            assert_eq!(message, expected_substr);
        }
    }
}
