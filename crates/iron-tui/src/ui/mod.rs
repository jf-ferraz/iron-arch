//! Iron TUI UI Rendering
//!
//! Main rendering functions for all views.

mod bundles;
mod dashboard;
mod modules;
mod profiles;
mod settings;
mod update;
mod wizard;

use crate::app::{App, View};
use crate::widgets::{render_confirm_dialog, render_footer, render_header, render_help_overlay};
use ratatui::prelude::*;

// Re-export for external use
pub use bundles::{render_bundle_detail, render_bundles};
pub use dashboard::render_dashboard;
pub use modules::{render_module_detail, render_modules};
pub use profiles::{render_profile_detail, render_profiles};
pub use settings::render_settings;
pub use update::{render_sync, render_update_preview};
pub use wizard::render_setup_wizard;

/// Main render function - dispatches to view-specific renderers
pub fn render(frame: &mut Frame, app: &App) {
    let area = frame.area();

    // Create main layout: header, content, footer
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header
            Constraint::Min(0),    // Content
            Constraint::Length(3), // Footer
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
#[cfg(test)]
mod tests;
