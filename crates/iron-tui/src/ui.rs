//! Iron TUI UI Components

use ratatui::prelude::*;

/// Render the dashboard view
pub fn render_dashboard(frame: &mut Frame, area: Rect) {
    // Placeholder - will be implemented
    let block = ratatui::widgets::Block::default()
        .title(" Iron Dashboard ")
        .borders(ratatui::widgets::Borders::ALL);

    frame.render_widget(block, area);
}

/// Render the setup wizard
pub fn render_setup_wizard(frame: &mut Frame, area: Rect) {
    let block = ratatui::widgets::Block::default()
        .title(" Welcome to Iron ")
        .borders(ratatui::widgets::Borders::ALL);

    frame.render_widget(block, area);
}
