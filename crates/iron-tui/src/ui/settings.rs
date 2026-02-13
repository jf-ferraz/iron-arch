//! Settings view rendering

use crate::app::App;
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

/// Render settings view
pub fn render_settings(frame: &mut Frame, area: Rect, _app: &App) {
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

    let para = Paragraph::new(text).block(block).wrap(Wrap { trim: true });

    frame.render_widget(para, area);
}
