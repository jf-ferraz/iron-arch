//! Iron TUI UI Rendering
//!
//! Main rendering functions for all views.

mod bundles;
mod clean;
mod config;
mod dashboard;
mod maintenance;
mod modules;
pub mod operation_log;
mod profiles;
mod security;
mod settings;
pub mod theme;
mod update;
pub mod utils;
mod wizard;

use crate::app::{App, View};
use crate::widgets::{render_confirm_dialog, render_footer, render_header, render_help_overlay, render_progress_dialog};
use ratatui::prelude::*;

// Re-export for external use
pub use bundles::{render_bundle_detail, render_bundles};
pub use clean::{render_clean_system, render_cleanup_preview, render_cleanup_results};
pub use config::render_config_manager;
pub use dashboard::render_dashboard;
pub use maintenance::render_system_maintenance;
pub use modules::{render_module_detail, render_modules};
pub use operation_log::render_operation_log;
pub use profiles::{render_profile_detail, render_profiles};
pub use security::render_security_modules;
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
        // Phase 3: System Cleanup
        View::CleanSystem => render_clean_system(frame, layout[1], app),
        View::CleanupPreview => clean::render_cleanup_preview(frame, layout[1], app),
        View::CleanupResults => clean::render_cleanup_results(frame, layout[1], app),
        // Phase 4-5 views
        View::SystemMaintenance => render_system_maintenance(frame, layout[1], app),
        View::SecurityModules => render_security_modules(frame, layout[1], app),
        View::ConfigManager => render_config_manager(frame, layout[1], app),
        View::OperationLog => render_operation_log(frame, layout[1], app),
    }

    // Render footer
    render_footer(frame, layout[2], app);

    // Render overlays
    if app.show_help {
        render_help_overlay(frame, area, app);
    }

    if app.show_confirm {
        render_confirm_dialog(frame, area, app);
    }

    // Render progress dialog for long-running operations
    if app.progress.is_some() {
        render_progress_dialog(frame, area, app);
    }
}

#[cfg(test)]
mod tests;
