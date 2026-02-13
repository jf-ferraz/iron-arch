//! Setup wizard rendering

use crate::app::App;
use crate::wizard::{InputMode, WizardStep};
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

/// Render the setup wizard
pub fn render_setup_wizard(frame: &mut Frame, area: Rect, app: &App) {
    // Create layout: progress bar, content, navigation hints
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Progress bar
            Constraint::Min(0),    // Content
            Constraint::Length(3), // Navigation hints
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
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
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
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
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

    let para = Paragraph::new(text).block(block).wrap(Wrap { trim: true });

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
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
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
            let prefix = if i == selected_idx {
                "  ● "
            } else {
                "  ○ "
            };
            let style = if i == selected_idx {
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            text.push(Line::from(Span::styled(
                format!("{}{}", prefix, bundle),
                style,
            )));
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

    let para = Paragraph::new(text).block(block).wrap(Wrap { trim: true });

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
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from("A profile is a collection of modules for a specific workflow."),
        Line::from("Select a profile to start with (you can change this later):"),
        Line::from(""),
    ];

    if profiles.is_empty() {
        text.push(Line::from(Span::styled(
            "No profiles found. Create profiles in your config directory.",
            Style::default().fg(Color::Yellow),
        )));
    } else {
        for (i, profile) in profiles.iter().enumerate() {
            let prefix = if i == selected_idx {
                "  ● "
            } else {
                "  ○ "
            };
            let style = if i == selected_idx {
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            text.push(Line::from(Span::styled(
                format!("{}{}", prefix, profile),
                style,
            )));
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

    let para = Paragraph::new(text).block(block).wrap(Wrap { trim: true });

    frame.render_widget(para, area);
}

/// Render confirmation step
fn render_wizard_confirmation(frame: &mut Frame, area: Rect, app: &App) {
    let wizard = &app.wizard;

    let bundle_name = wizard.selected_bundle().unwrap_or("None");
    let profile_name = wizard.selected_profile().unwrap_or("None");

    let mut final_text = vec![
        Line::from(""),
        Line::from(Span::styled(
            "Confirmation",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from("Please review your configuration:"),
        Line::from(""),
        Line::from(format!("  Host ID: {}", wizard.host_id)),
        Line::from(format!("  Bundle:  {}", bundle_name)),
        Line::from(format!("  Profile: {}", profile_name)),
        Line::from(""),
        Line::from(Span::styled(
            "Press Enter to apply, Backspace to go back",
            Style::default().fg(Color::Gray),
        )),
    ];

    // Show any validation errors
    if let Some(ref error) = wizard.error {
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
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
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
        spans.push(Span::styled(
            "[Backspace]",
            Style::default().fg(Color::Gray),
        ));
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
