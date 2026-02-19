//! Secrets view — git-crypt / secrets management

use crate::app::App;
use crate::ui::theme;
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

/// Render the Secrets management screen
pub fn render_secrets(frame: &mut Frame, area: Rect, app: &App) {
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Title bar
            Constraint::Length(8), // Status panel
            Constraint::Min(0),    // Encrypted files
            Constraint::Length(3), // Footer hints
        ])
        .split(area);

    // Title
    let title_block = Block::default()
        .borders(Borders::BOTTOM)
        .border_style(Style::default().fg(theme::OVERLAY));
    let title_para = Paragraph::new(Line::from(vec![
        Span::raw("  "),
        Span::styled("Secrets", Style::default().fg(theme::MAUVE).bold()),
        Span::raw("  "),
        Span::styled("git-crypt encrypted file management", Style::default().fg(theme::SUBTEXT)),
    ]))
    .block(title_block);
    frame.render_widget(title_para, layout[0]);

    // Status panel
    render_secrets_status(frame, layout[1], app);

    // Encrypted files list
    render_encrypted_files(frame, layout[2], app);

    // Footer hints
    let hints = Paragraph::new(Line::from(vec![
        Span::raw("  "),
        Span::styled(" [i] ", Style::default().fg(Color::Black).bg(theme::MAUVE).bold()),
        Span::styled(" Init  ", Style::default().fg(theme::SUBTEXT)),
        Span::styled(" [u] ", Style::default().fg(Color::Black).bg(theme::MAUVE).bold()),
        Span::styled(" Unlock  ", Style::default().fg(theme::SUBTEXT)),
        Span::styled(" [l] ", Style::default().fg(Color::Black).bg(theme::MAUVE).bold()),
        Span::styled(" Lock  ", Style::default().fg(theme::SUBTEXT)),
        Span::styled(" [Esc] ", Style::default().fg(Color::Black).bg(theme::MAUVE).bold()),
        Span::styled(" Back", Style::default().fg(theme::SUBTEXT)),
    ]))
    .block(Block::default().borders(Borders::TOP).border_style(Style::default().fg(theme::OVERLAY)));
    frame.render_widget(hints, layout[3]);
}

fn render_secrets_status(frame: &mut Frame, area: Rect, app: &App) {
    let (status_icon, status_text, status_color) = match app.secrets_status.as_deref() {
        Some("Unlocked") => ("[OK]", "Unlocked — secrets are decrypted and readable", theme::GREEN),
        Some("Locked") => ("[--]", "Locked — secrets are encrypted", theme::YELLOW),
        Some("NotInitialized") => ("[!!]", "Not initialized — press [i] to set up git-crypt", theme::YELLOW),
        Some("NotAvailable") => ("[XX]", "git-crypt not installed — install it to use secrets", theme::RED),
        _ => ("[ ?]", "Unknown — press [r] to refresh", theme::OVERLAY),
    };

    let git_crypt_note = "  git-crypt encrypts specific files in your git repository.";
    let age_note     = "  Use [a] to add an authorized GPG key.";

    let content = vec![
        Line::from(""),
        Line::from(vec![
            Span::raw("  "),
            Span::styled(status_icon, Style::default().fg(status_color).bold()),
            Span::raw("  "),
            Span::styled(status_text, Style::default().fg(status_color)),
        ]),
        Line::from(""),
        Line::from(Span::styled(git_crypt_note, Style::default().fg(theme::OVERLAY))),
        Line::from(Span::styled(age_note, Style::default().fg(theme::OVERLAY))),
    ];

    let block = theme::themed_block("Status", theme::MAUVE);
    frame.render_widget(Paragraph::new(content).block(block).wrap(Wrap { trim: true }), area);
}

fn render_encrypted_files(frame: &mut Frame, area: Rect, app: &App) {
    let block = theme::themed_block("Encrypted Files", theme::MAUVE);

    if app.encrypted_files.is_empty() {
        let empty = Paragraph::new(vec![
            Line::from(""),
            Line::from(Span::styled(
                "No encrypted files tracked.",
                Style::default().fg(theme::SUBTEXT),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "Initialize git-crypt with [i], then add files via .gitattributes.",
                Style::default().fg(theme::OVERLAY),
            )),
        ])
        .block(block)
        .alignment(Alignment::Center);
        frame.render_widget(empty, area);
        return;
    }

    let mut lines = vec![Line::from("")];
    for path in &app.encrypted_files {
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled("[enc]", Style::default().fg(theme::MAUVE)),
            Span::raw("  "),
            Span::styled(
                path.display().to_string(),
                Style::default().fg(theme::TEXT),
            ),
        ]));
    }

    let para = Paragraph::new(lines).block(block).wrap(Wrap { trim: true });
    frame.render_widget(para, area);
}
