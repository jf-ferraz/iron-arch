//! Integrated install wizard rendering.

use crate::app::App;
use crate::ui::theme;
use iron_core::InstallStatus;
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph, Wrap};

/// Render the integrated Arch install wizard.
pub fn render_install_wizard(frame: &mut Frame, area: Rect, app: &App) {
    let Some(wizard) = app.install_wizard.as_ref() else {
        let paragraph = Paragraph::new("Install wizard is not initialized.").block(
            Block::default()
                .title("Install Wizard")
                .borders(Borders::ALL),
        );
        frame.render_widget(paragraph, area);
        return;
    };

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(32), Constraint::Percentage(68)])
        .margin(1)
        .split(area);

    let right = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(9),
            Constraint::Length(5),
            Constraint::Min(8),
        ])
        .split(chunks[1]);

    render_phase_list(frame, chunks[0], wizard);
    render_step_detail(frame, right[0], wizard);
    render_status(frame, right[1], wizard);
    render_logs(frame, right[2], wizard);
}

fn render_phase_list(
    frame: &mut Frame,
    area: Rect,
    wizard: &crate::install_wizard::InstallWizardState,
) {
    let items = wizard
        .plan
        .phases
        .iter()
        .enumerate()
        .map(|(index, phase)| {
            let status = wizard
                .phase_statuses
                .get(index)
                .copied()
                .unwrap_or(InstallStatus::Pending);
            let selected = if index == wizard.selected_phase {
                ">"
            } else {
                " "
            };
            let line = Line::from(vec![
                Span::styled(selected, Style::default().fg(theme::MAUVE).bold()),
                Span::raw(" "),
                Span::styled(status_label(status), status_style(status)),
                Span::raw(" "),
                Span::styled(&phase.name, Style::default().fg(theme::TEXT)),
            ]);
            ListItem::new(line)
        })
        .collect::<Vec<_>>();

    let list = List::new(items).block(
        Block::default()
            .title("Phases")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme::OVERLAY)),
    );
    frame.render_widget(list, area);
}

fn render_step_detail(
    frame: &mut Frame,
    area: Rect,
    wizard: &crate::install_wizard::InstallWizardState,
) {
    let Some(phase) = wizard.plan.phases.get(wizard.selected_phase) else {
        return;
    };

    let mut lines = vec![
        Line::from(vec![
            Span::styled("Host: ", Style::default().fg(theme::OVERLAY)),
            Span::styled(
                &wizard.plan.host_id,
                Style::default().fg(theme::TEXT).bold(),
            ),
            Span::raw("   "),
            Span::styled("Target: ", Style::default().fg(theme::OVERLAY)),
            Span::styled(&wizard.plan.target_mount, Style::default().fg(theme::TEXT)),
        ]),
        Line::from(""),
    ];

    for step in &phase.steps {
        let marker = if step.destructive { "[!]" } else { "[ ]" };
        let color = if step.destructive {
            theme::YELLOW
        } else {
            theme::SUBTEXT
        };
        lines.push(Line::from(vec![
            Span::styled(marker, Style::default().fg(color).bold()),
            Span::raw(" "),
            Span::styled(&step.description, Style::default().fg(theme::TEXT)),
        ]));
    }

    let paragraph = Paragraph::new(lines).block(
        Block::default()
            .title(format!("{} [{}]", phase.name, phase.id.as_script_id()))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme::OVERLAY)),
    );
    frame.render_widget(paragraph, area);
}

fn render_status(
    frame: &mut Frame,
    area: Rect,
    wizard: &crate::install_wizard::InstallWizardState,
) {
    let mut lines = vec![Line::from(vec![
        Span::styled("Mode: ", Style::default().fg(theme::OVERLAY)),
        Span::styled(
            format!("{:?}", wizard.mode),
            Style::default().fg(theme::MAUVE).bold(),
        ),
        Span::raw("   "),
        Span::styled("Confirmation: ", Style::default().fg(theme::OVERLAY)),
        Span::styled(
            if wizard.confirmed { "armed" } else { "safe" },
            Style::default().fg(if wizard.confirmed {
                theme::YELLOW
            } else {
                theme::GREEN
            }),
        ),
    ])];

    if wizard.awaiting_confirmation {
        lines.push(Line::from(vec![
            Span::styled("Type INSTALL: ", Style::default().fg(theme::YELLOW).bold()),
            Span::styled(&wizard.confirmation_input, Style::default().fg(theme::TEXT)),
        ]));
    } else if let Some(failure) = &wizard.failure {
        lines.push(Line::from(Span::styled(
            failure,
            Style::default().fg(theme::RED).bold(),
        )));
    } else if wizard.completed {
        lines.push(Line::from(Span::styled(
            "Plan completed. Review logs before rebooting.",
            Style::default().fg(theme::GREEN),
        )));
    } else {
        lines.push(Line::from(
            "Press d for dry-run, r for real run confirmation.",
        ));
    }

    let paragraph = Paragraph::new(lines).block(
        Block::default()
            .title("Run Control")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme::OVERLAY)),
    );
    frame.render_widget(paragraph, area);
}

fn render_logs(frame: &mut Frame, area: Rect, wizard: &crate::install_wizard::InstallWizardState) {
    let height = area.height.saturating_sub(2) as usize;
    let start = wizard.logs.len().saturating_sub(height);
    let lines = wizard.logs[start..]
        .iter()
        .map(|line| Line::from(line.as_str()))
        .collect::<Vec<_>>();

    let paragraph = Paragraph::new(lines).wrap(Wrap { trim: false }).block(
        Block::default()
            .title("Execution Log")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme::OVERLAY)),
    );
    frame.render_widget(paragraph, area);
}

fn status_label(status: InstallStatus) -> &'static str {
    match status {
        InstallStatus::Pending => "[ ]",
        InstallStatus::Running => "[~]",
        InstallStatus::Success => "[OK]",
        InstallStatus::Skipped => "[SKIP]",
        InstallStatus::Failed => "[FAIL]",
        InstallStatus::Blocked => "[BLOCK]",
    }
}

fn status_style(status: InstallStatus) -> Style {
    let color = match status {
        InstallStatus::Pending => theme::OVERLAY,
        InstallStatus::Running => theme::MAUVE,
        InstallStatus::Success => theme::GREEN,
        InstallStatus::Skipped => theme::SUBTEXT,
        InstallStatus::Failed => theme::RED,
        InstallStatus::Blocked => theme::YELLOW,
    };
    Style::default().fg(color).bold()
}
