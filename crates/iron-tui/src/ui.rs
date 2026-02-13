//! Iron TUI UI Rendering
//!
//! Main rendering functions for all views.

use crate::app::{App, HealthStatus, View};
use crate::widgets::{render_confirm_dialog, render_footer, render_header, render_help_overlay};
use crate::wizard::{InputMode, WizardStep};
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap};

/// Main render function - dispatches to view-specific renderers
pub fn render(frame: &mut Frame, app: &App) {
    let area = frame.area();

    // Create main layout: header, content, footer
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // Header
            Constraint::Min(0),     // Content
            Constraint::Length(3),  // Footer
        ])
        .split(area);

    // Render header
    render_header(frame, layout[0], app);

    // Render view content
    match app.view {
        View::Dashboard => render_dashboard(frame, layout[1], app),
        View::SetupWizard => render_setup_wizard(frame, layout[1], app),
        View::Bundles => render_bundles(frame, layout[1], app),
        View::BundleDetail => render_bundle_detail(frame, layout[1], app),
        View::Profiles => render_profiles(frame, layout[1], app),
        View::ProfileDetail => render_profile_detail(frame, layout[1], app),
        View::Modules => render_modules(frame, layout[1], app),
        View::ModuleDetail => render_module_detail(frame, layout[1], app),
        View::UpdatePreview => render_update_preview(frame, layout[1], app),
        View::Sync => render_sync(frame, layout[1], app),
        View::Settings => render_settings(frame, layout[1], app),
    }

    // Render footer
    render_footer(frame, layout[2], app);

    // Render overlays
    if app.show_help {
        render_help_overlay(frame, area);
    }

    if app.show_confirm {
        render_confirm_dialog(frame, area, app);
    }
}

/// Render the dashboard view
fn render_dashboard(frame: &mut Frame, area: Rect, app: &App) {
    // Split into two columns
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    // Left column: System Health + Maintenance
    let left_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(6),  // System Health
            Constraint::Length(5),  // Maintenance
            Constraint::Min(0),     // Quick Actions
        ])
        .split(columns[0]);

    // System Health panel
    let health_status = match app.system_health() {
        HealthStatus::Ok => ("● System OK", Style::default().fg(Color::Green)),
        HealthStatus::Warning => ("⚠ Warning", Style::default().fg(Color::Yellow)),
        HealthStatus::Error => ("✗ Error", Style::default().fg(Color::Red)),
    };

    let health_text = vec![
        Line::from(Span::styled(health_status.0, health_status.1)),
        Line::from(format!("● {} packages installed", app.package_count())),
        Line::from("● No conflicts detected"),
    ];

    let health_block = Block::default()
        .title(" System Health ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let health_para = Paragraph::new(health_text)
        .block(health_block)
        .wrap(Wrap { trim: true });

    frame.render_widget(health_para, left_layout[0]);

    // Maintenance panel
    let maintenance_text = vec![
        Line::from("Last Update: 3 days ago"),
        Line::from("Last Clean:  1 week ago"),
    ];

    let maintenance_block = Block::default()
        .title(" Maintenance ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let maintenance_para = Paragraph::new(maintenance_text)
        .block(maintenance_block)
        .wrap(Wrap { trim: true });

    frame.render_widget(maintenance_para, left_layout[1]);

    // Quick Actions panel
    let actions_text = vec![
        Line::from(vec![
            Span::styled("[U]", Style::default().fg(Color::Yellow)),
            Span::raw(" Update  "),
            Span::styled("[B]", Style::default().fg(Color::Yellow)),
            Span::raw(" Bundles  "),
            Span::styled("[P]", Style::default().fg(Color::Yellow)),
            Span::raw(" Profiles"),
        ]),
        Line::from(vec![
            Span::styled("[M]", Style::default().fg(Color::Yellow)),
            Span::raw(" Modules "),
            Span::styled("[S]", Style::default().fg(Color::Yellow)),
            Span::raw(" Settings "),
            Span::styled("[?]", Style::default().fg(Color::Yellow)),
            Span::raw(" Help"),
        ]),
    ];

    let actions_block = Block::default()
        .title(" Quick Actions ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let actions_para = Paragraph::new(actions_text)
        .block(actions_block)
        .wrap(Wrap { trim: true });

    frame.render_widget(actions_para, left_layout[2]);

    // Right column: Active Configuration + Alerts
    let right_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(8),  // Active Configuration
            Constraint::Min(0),     // Alerts
        ])
        .split(columns[1]);

    // Active Configuration panel
    let host_name = app.current_host.as_deref().unwrap_or("unknown");
    let bundle_name = app.active_bundle.as_ref()
        .map(|b| b.id.as_str())
        .unwrap_or("none");
    let profile_name = app.active_profile.as_deref().unwrap_or("none");

    let config_text = vec![
        Line::from(format!("Host:    {}", host_name)),
        Line::from(format!("Bundle:  {} (active)", bundle_name)),
        Line::from(format!("Profile: {}", profile_name)),
        Line::from(format!("Modules: {} enabled", app.enabled_module_count())),
    ];

    let config_block = Block::default()
        .title(" Active Configuration ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Green));

    let config_para = Paragraph::new(config_text)
        .block(config_block)
        .wrap(Wrap { trim: true });

    frame.render_widget(config_para, right_layout[0]);

    // Alerts panel
    let update_count = app.pending_update_count();
    let alert_style = if update_count > 0 {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default().fg(Color::Green)
    };

    let alerts_text = vec![
        Line::from(Span::styled(
            format!("⚠ {} updates available", update_count),
            alert_style,
        )),
        Line::from(Span::styled(
            "● No conflicts detected",
            Style::default().fg(Color::Green),
        )),
    ];

    let alerts_block = Block::default()
        .title(" Alerts ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let alerts_para = Paragraph::new(alerts_text)
        .block(alerts_block)
        .wrap(Wrap { trim: true });

    frame.render_widget(alerts_para, right_layout[1]);
}

/// Render the setup wizard
fn render_setup_wizard(frame: &mut Frame, area: Rect, app: &App) {
    // Create layout: progress bar, content, navigation hints
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // Progress bar
            Constraint::Min(0),     // Content
            Constraint::Length(3),  // Navigation hints
        ])
        .split(area);

    // Render progress indicator
    render_wizard_progress(frame, layout[0], &app.wizard);

    // Render step content
    match app.wizard.step {
        WizardStep::Welcome => render_wizard_welcome(frame, layout[1]),
        WizardStep::HostSetup => render_wizard_host_setup(frame, layout[1], app),
        WizardStep::BundleSelection => render_wizard_bundle_selection(frame, layout[1], app),
        WizardStep::ProfileSelection => render_wizard_profile_selection(frame, layout[1], app),
        WizardStep::Confirmation => render_wizard_confirmation(frame, layout[1], app),
        WizardStep::Complete => render_wizard_complete(frame, layout[1]),
    }

    // Render navigation hints
    render_wizard_navigation(frame, layout[2], &app.wizard);
}

/// Render wizard progress indicator
fn render_wizard_progress(frame: &mut Frame, area: Rect, wizard: &crate::wizard::WizardState) {
    let step_num = wizard.step_number();
    let total = wizard.total_steps();

    let progress_text = format!("Step {} of {}", step_num.min(total), total);

    // Visual progress bar
    let filled = "━".repeat(step_num.min(total) * 4);
    let empty = "─".repeat((total - step_num.min(total)) * 4);
    let progress_bar = format!("[{}{}]", filled, empty);

    let text = vec![Line::from(vec![
        Span::raw("  "),
        Span::styled(&progress_bar, Style::default().fg(Color::Cyan)),
        Span::raw("  "),
        Span::styled(progress_text, Style::default().fg(Color::Gray)),
    ])];

    let para = Paragraph::new(text);
    frame.render_widget(para, area);
}

/// Render welcome step
fn render_wizard_welcome(frame: &mut Frame, area: Rect) {
    let text = vec![
        Line::from(""),
        Line::from(""),
        Line::from(Span::styled(
            "╔═══════════════════════════════════╗",
            Style::default().fg(Color::Cyan),
        )),
        Line::from(Span::styled(
            "║       Welcome to Iron!            ║",
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            "╚═══════════════════════════════════╝",
            Style::default().fg(Color::Cyan),
        )),
        Line::from(""),
        Line::from(""),
        Line::from("Iron helps you manage your Arch Linux configuration"),
        Line::from("with elegance, stability, and ease."),
        Line::from(""),
        Line::from("This wizard will guide you through:"),
        Line::from("  • Setting up your host identifier"),
        Line::from("  • Selecting a desktop environment bundle"),
        Line::from("  • Choosing a development profile"),
        Line::from(""),
        Line::from(Span::styled(
            "Press Enter to begin...",
            Style::default().fg(Color::Green),
        )),
    ];

    let block = Block::default()
        .title(" First-Time Setup ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let para = Paragraph::new(text)
        .block(block)
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: true });

    frame.render_widget(para, area);
}

/// Render host setup step
fn render_wizard_host_setup(frame: &mut Frame, area: Rect, app: &App) {
    let is_editing = app.host_input.mode == InputMode::Editing;

    let input_style = if is_editing {
        Style::default().fg(Color::Yellow).bg(Color::DarkGray)
    } else {
        Style::default().fg(Color::White)
    };

    let cursor_hint = if is_editing { "│" } else { "" };
    let input_value = format!("{}{}", app.host_input.value, cursor_hint);

    let text = vec![
        Line::from(""),
        Line::from(Span::styled(
            "Host Setup",
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from("Each machine you use Iron on has a unique host identifier."),
        Line::from("This allows you to maintain separate configurations per machine."),
        Line::from(""),
        Line::from(format!("Detected hostname: {}", app.wizard.host_id)),
        Line::from(""),
        Line::from("Host ID:"),
        Line::from(Span::styled(format!("  > {} ", input_value), input_style)),
        Line::from(""),
        if is_editing {
            Line::from(Span::styled(
                "Press Enter to confirm, Esc to cancel",
                Style::default().fg(Color::Gray),
            ))
        } else {
            Line::from(Span::styled(
                "Press [e] to edit, Enter to continue",
                Style::default().fg(Color::Gray),
            ))
        },
    ];

    let block = Block::default()
        .title(" Host Setup ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let para = Paragraph::new(text)
        .block(block)
        .wrap(Wrap { trim: true });

    frame.render_widget(para, area);
}

/// Render bundle selection step
fn render_wizard_bundle_selection(frame: &mut Frame, area: Rect, app: &App) {
    let bundles = &app.wizard.available_bundles;
    let selected_idx = app.wizard.selected_bundle_index;

    let mut text = vec![
        Line::from(""),
        Line::from(Span::styled(
            "Bundle Selection",
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from("A bundle is a complete desktop environment configuration."),
        Line::from("Select the environment that best fits your workflow:"),
        Line::from(""),
    ];

    if bundles.is_empty() {
        text.push(Line::from(Span::styled(
            "No bundles found. Create bundles in your config directory.",
            Style::default().fg(Color::Yellow),
        )));
    } else {
        for (i, bundle) in bundles.iter().enumerate() {
            let prefix = if i == selected_idx { "  ● " } else { "  ○ " };
            let style = if i == selected_idx {
                Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            text.push(Line::from(Span::styled(format!("{}{}", prefix, bundle), style)));
        }
    }

    text.push(Line::from(""));
    text.push(Line::from(Span::styled(
        "Use ↑/↓ to select, Enter to continue",
        Style::default().fg(Color::Gray),
    )));

    let block = Block::default()
        .title(" Bundle Selection ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let para = Paragraph::new(text)
        .block(block)
        .wrap(Wrap { trim: true });

    frame.render_widget(para, area);
}

/// Render profile selection step
fn render_wizard_profile_selection(frame: &mut Frame, area: Rect, app: &App) {
    let profiles = &app.wizard.available_profiles;
    let selected_idx = app.wizard.selected_profile_index;

    let mut text = vec![
        Line::from(""),
        Line::from(Span::styled(
            "Profile Selection",
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from("A profile configures your development tools and workflows."),
        Line::from("Select a profile (optional - you can skip this):"),
        Line::from(""),
    ];

    if profiles.is_empty() {
        text.push(Line::from(Span::styled(
            "No profiles found. You can add profiles later.",
            Style::default().fg(Color::Gray),
        )));
    } else {
        for (i, profile) in profiles.iter().enumerate() {
            let prefix = if i == selected_idx { "  ● " } else { "  ○ " };
            let style = if i == selected_idx {
                Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            text.push(Line::from(Span::styled(format!("{}{}", prefix, profile), style)));
        }
    }

    text.push(Line::from(""));
    text.push(Line::from(Span::styled(
        "Use ↑/↓ to select, Enter to continue",
        Style::default().fg(Color::Gray),
    )));

    let block = Block::default()
        .title(" Profile Selection ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let para = Paragraph::new(text)
        .block(block)
        .wrap(Wrap { trim: true });

    frame.render_widget(para, area);
}

/// Render confirmation step
fn render_wizard_confirmation(frame: &mut Frame, area: Rect, app: &App) {
    let bundle = app.wizard.selected_bundle().unwrap_or("(none)");
    let profile = app.wizard.selected_profile().unwrap_or("(none)");

    let text = vec![
        Line::from(""),
        Line::from(Span::styled(
            "Confirm Configuration",
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from("Please review your selections:"),
        Line::from(""),
        Line::from(format!("  Host:    {}", app.wizard.host_id)),
        Line::from(format!("  Bundle:  {}", bundle)),
        Line::from(format!("  Profile: {}", profile)),
        Line::from(""),
        Line::from(""),
        Line::from(Span::styled(
            "Press Enter or [y] to apply these settings",
            Style::default().fg(Color::Green),
        )),
        Line::from(Span::styled(
            "Press Backspace or Esc to go back",
            Style::default().fg(Color::Gray),
        )),
    ];

    let mut final_text = text;

    if let Some(ref error) = app.wizard.error {
        final_text.push(Line::from(""));
        final_text.push(Line::from(Span::styled(
            format!("Error: {}", error),
            Style::default().fg(Color::Red),
        )));
    }

    let block = Block::default()
        .title(" Confirmation ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow));

    let para = Paragraph::new(final_text)
        .block(block)
        .wrap(Wrap { trim: true });

    frame.render_widget(para, area);
}

/// Render completion step
fn render_wizard_complete(frame: &mut Frame, area: Rect) {
    let text = vec![
        Line::from(""),
        Line::from(""),
        Line::from(Span::styled(
            "╔═══════════════════════════════════╗",
            Style::default().fg(Color::Green),
        )),
        Line::from(Span::styled(
            "║        Setup Complete!            ║",
            Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            "╚═══════════════════════════════════╝",
            Style::default().fg(Color::Green),
        )),
        Line::from(""),
        Line::from(""),
        Line::from("Your Iron configuration is ready."),
        Line::from(""),
        Line::from("You can now:"),
        Line::from("  • Browse and enable modules"),
        Line::from("  • Customize your bundle"),
        Line::from("  • Check for system updates"),
        Line::from(""),
        Line::from(Span::styled(
            "Press Enter to go to the Dashboard...",
            Style::default().fg(Color::Cyan),
        )),
    ];

    let block = Block::default()
        .title(" Complete ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Green));

    let para = Paragraph::new(text)
        .block(block)
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: true });

    frame.render_widget(para, area);
}

/// Render wizard navigation hints
fn render_wizard_navigation(frame: &mut Frame, area: Rect, wizard: &crate::wizard::WizardState) {
    let mut spans = vec![Span::raw("  ")];

    if wizard.can_go_back() {
        spans.push(Span::styled("[Backspace]", Style::default().fg(Color::Gray)));
        spans.push(Span::raw(" Back  "));
    }

    if wizard.can_proceed() {
        spans.push(Span::styled("[Enter]", Style::default().fg(Color::Green)));
        spans.push(Span::raw(" Continue  "));
    }

    spans.push(Span::styled("[q]", Style::default().fg(Color::Red)));
    spans.push(Span::raw(" Quit"));

    let text = vec![Line::from(spans)];

    let block = Block::default()
        .borders(Borders::TOP)
        .border_style(Style::default().fg(Color::DarkGray));

    let para = Paragraph::new(text).block(block);
    frame.render_widget(para, area);
}

/// Render the bundles list view
fn render_bundles(frame: &mut Frame, area: Rect, app: &App) {
    let items: Vec<ListItem> = app.bundles
        .iter()
        .enumerate()
        .map(|(i, bundle)| {
            let is_active = app.active_bundle.as_ref()
                .map(|b| b.id == bundle.id)
                .unwrap_or(false);

            let status = if is_active { "●" } else { "○" };
            let desc = bundle.description.as_deref().unwrap_or("");
            let content = format!("{} {} - {}", status, bundle.id, desc);

            let style = if i == app.selected_index {
                Style::default().bg(Color::DarkGray).fg(Color::White)
            } else if is_active {
                Style::default().fg(Color::Green)
            } else {
                Style::default()
            };

            ListItem::new(content).style(style)
        })
        .collect();

    let block = Block::default()
        .title(" Bundles ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let list = List::new(items).block(block);

    let mut state = ListState::default();
    state.select(Some(app.selected_index));

    frame.render_stateful_widget(list, area, &mut state);
}

/// Render bundle detail view
fn render_bundle_detail(frame: &mut Frame, area: Rect, app: &App) {
    let bundle = match app.selected_bundle() {
        Some(b) => b,
        None => {
            let block = Block::default()
                .title(" Bundle Detail ")
                .borders(Borders::ALL);
            let para = Paragraph::new("No bundle selected").block(block);
            frame.render_widget(para, area);
            return;
        }
    };

    let is_active = app.active_bundle.as_ref()
        .map(|b| b.id == bundle.id)
        .unwrap_or(false);

    let status = if is_active { "Active" } else { "Inactive" };
    let desc = bundle.description.as_deref().unwrap_or("No description");

    let text = vec![
        Line::from(format!("ID: {}", bundle.id)),
        Line::from(format!("Description: {}", desc)),
        Line::from(format!("Type: {:?}", bundle.bundle_type)),
        Line::from(format!("Status: {}", status)),
        Line::from(""),
        Line::from("Profiles:"),
    ];

    let mut lines = text;
    for profile_id in &bundle.profiles {
        lines.push(Line::from(format!("  - {}", profile_id)));
    }

    let block = Block::default()
        .title(format!(" Bundle: {} ", bundle.id))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let para = Paragraph::new(lines)
        .block(block)
        .wrap(Wrap { trim: true });

    frame.render_widget(para, area);
}

/// Render the profiles list view
fn render_profiles(frame: &mut Frame, area: Rect, app: &App) {
    let items: Vec<ListItem> = app.profiles
        .iter()
        .enumerate()
        .map(|(i, profile)| {
            let is_active = app.active_profile.as_ref()
                .map(|p| *p == profile.id)
                .unwrap_or(false);

            let status = if is_active { "●" } else { "○" };
            let desc = profile.description.as_deref().unwrap_or("");
            let content = format!("{} {} - {}", status, profile.id, desc);

            let style = if i == app.selected_index {
                Style::default().bg(Color::DarkGray).fg(Color::White)
            } else if is_active {
                Style::default().fg(Color::Green)
            } else {
                Style::default()
            };

            ListItem::new(content).style(style)
        })
        .collect();

    let block = Block::default()
        .title(" Profiles ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let list = List::new(items).block(block);

    let mut state = ListState::default();
    state.select(Some(app.selected_index));

    frame.render_stateful_widget(list, area, &mut state);
}

/// Render profile detail view
fn render_profile_detail(frame: &mut Frame, area: Rect, app: &App) {
    let profile = match app.selected_profile() {
        Some(p) => p,
        None => {
            let block = Block::default()
                .title(" Profile Detail ")
                .borders(Borders::ALL);
            let para = Paragraph::new("No profile selected").block(block);
            frame.render_widget(para, area);
            return;
        }
    };

    let desc = profile.description.as_deref().unwrap_or("No description");

    let text = vec![
        Line::from(format!("ID: {}", profile.id)),
        Line::from(format!("Description: {}", desc)),
        Line::from(""),
        Line::from("Modules:"),
    ];

    let mut lines = text;
    for module_id in &profile.modules {
        lines.push(Line::from(format!("  - {}", module_id)));
    }

    let block = Block::default()
        .title(format!(" Profile: {} ", profile.id))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let para = Paragraph::new(lines)
        .block(block)
        .wrap(Wrap { trim: true });

    frame.render_widget(para, area);
}

/// Render the modules list view
fn render_modules(frame: &mut Frame, area: Rect, app: &App) {
    let items: Vec<ListItem> = app.modules
        .iter()
        .enumerate()
        .map(|(i, module)| {
            let is_active = app.is_module_active(&module.id);
            let status = if is_active { "✓" } else { "○" };
            let desc = module.description.as_deref().unwrap_or("");
            let content = format!("{} {} - {}", status, module.id, desc);

            let style = if i == app.selected_index {
                Style::default().bg(Color::DarkGray).fg(Color::White)
            } else if is_active {
                Style::default().fg(Color::Green)
            } else {
                Style::default()
            };

            ListItem::new(content).style(style)
        })
        .collect();

    let block = Block::default()
        .title(" Modules ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let list = List::new(items).block(block);

    let mut state = ListState::default();
    state.select(Some(app.selected_index));

    frame.render_stateful_widget(list, area, &mut state);
}

/// Render module detail view
fn render_module_detail(frame: &mut Frame, area: Rect, app: &App) {
    let module = match app.selected_module() {
        Some(m) => m,
        None => {
            let block = Block::default()
                .title(" Module Detail ")
                .borders(Borders::ALL);
            let para = Paragraph::new("No module selected").block(block);
            frame.render_widget(para, area);
            return;
        }
    };

    let is_active = app.is_module_active(&module.id);
    let status = if is_active { "Enabled" } else { "Disabled" };
    let desc = module.description.as_deref().unwrap_or("No description");

    let text = vec![
        Line::from(format!("ID: {}", module.id)),
        Line::from(format!("Description: {}", desc)),
        Line::from(format!("Kind: {:?}", module.kind)),
        Line::from(format!("Status: {}", status)),
        Line::from(""),
        Line::from("Packages:"),
    ];

    let mut lines = text;
    for pkg in &module.packages {
        lines.push(Line::from(format!("  - {}", pkg)));
    }

    lines.push(Line::from(""));
    lines.push(Line::from("Dotfiles:"));
    for mapping in &module.dotfiles {
        lines.push(Line::from(format!("  {} -> {}", mapping.source, mapping.target)));
    }

    let block = Block::default()
        .title(format!(" Module: {} ", module.id))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let para = Paragraph::new(lines)
        .block(block)
        .wrap(Wrap { trim: true });

    frame.render_widget(para, area);
}

/// Render the update preview view
fn render_update_preview(frame: &mut Frame, area: Rect, app: &App) {
    use iron_core::RiskLevel;

    let update_count = app.pending_update_count();
    let risk_level = app.update_risk_level();
    let updates = app.pending_updates_list();

    // Risk level styling
    let (risk_symbol, risk_color, risk_text) = match risk_level {
        RiskLevel::Low => ("●", Color::Green, "Safe to update"),
        RiskLevel::Medium => ("⚠", Color::Yellow, "Review recommended"),
        RiskLevel::High => ("⚠", Color::Red, "Attention required"),
        RiskLevel::Critical => ("✗", Color::Red, "Create snapshot first!"),
    };

    // Split into header and list
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(6),  // Summary
            Constraint::Min(0),     // Package list
        ])
        .split(area);

    // Summary section
    let summary_text = vec![
        Line::from(vec![
            Span::raw("Status: "),
            Span::styled(
                format!("{} updates available", update_count),
                Style::default().fg(if update_count > 0 { Color::Yellow } else { Color::Green }),
            ),
        ]),
        Line::from(vec![
            Span::raw("Risk:   "),
            Span::styled(format!("{} {}", risk_symbol, risk_text), Style::default().fg(risk_color)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("[r]", Style::default().fg(Color::Yellow)),
            Span::raw(" Refresh  "),
            Span::styled("[u]", Style::default().fg(Color::Yellow)),
            Span::raw(" Update  "),
            Span::styled("[Esc]", Style::default().fg(Color::Gray)),
            Span::raw(" Back"),
        ]),
    ];

    let summary_block = Block::default()
        .title(" System Update ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let summary_para = Paragraph::new(summary_text)
        .block(summary_block)
        .wrap(Wrap { trim: true });

    frame.render_widget(summary_para, layout[0]);

    // Package list section
    let items: Vec<ListItem> = updates
        .iter()
        .take(50)  // Limit displayed items
        .map(|pkg| {
            let aur_marker = if pkg.is_aur { "[AUR] " } else { "" };
            let content = format!(
                "{}{}: {} -> {}",
                aur_marker, pkg.name, pkg.current_version, pkg.new_version
            );
            let style = if pkg.is_aur {
                Style::default().fg(Color::Magenta)
            } else if pkg.name.starts_with("linux") {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default()
            };
            ListItem::new(content).style(style)
        })
        .collect();

    let list_title = if updates.len() > 50 {
        format!(" Packages (showing 50 of {}) ", updates.len())
    } else {
        " Packages ".to_string()
    };

    let list_block = Block::default()
        .title(list_title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray));

    let list = List::new(items).block(list_block);

    frame.render_widget(list, layout[1]);
}

/// Render sync status view
fn render_sync(frame: &mut Frame, area: Rect, _app: &App) {
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

    let para = Paragraph::new(text)
        .block(block)
        .wrap(Wrap { trim: true });

    frame.render_widget(para, area);
}

/// Render settings view
fn render_settings(frame: &mut Frame, area: Rect, _app: &App) {
    let text = vec![
        Line::from("Settings"),
        Line::from(""),
        Line::from("• Config directory: ."),
        Line::from("• State file: state.json"),
        Line::from(""),
        Line::from("Press [Enter] to edit settings"),
    ];

    let block = Block::default()
        .title(" Settings ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let para = Paragraph::new(text)
        .block(block)
        .wrap(Wrap { trim: true });

    frame.render_widget(para, area);
}
