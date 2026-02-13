//! Iron TUI - Dashboard interface for Iron
//!
//! Features:
//! - Dashboard home with system health
//! - First-time setup wizard
//! - Bundle/profile selection wizard
//! - Profile builder
//! - Update preview with risk scores

pub mod app;
pub mod event;
pub mod terminal;
pub mod ui;
pub mod widgets;
pub mod wizard;

use app::App;
use event::{Event, EventHandler};
use iron_core::PackageManager;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use terminal::Terminal;

/// Default tick rate for the TUI (250ms)
const TICK_RATE: Duration = Duration::from_millis(250);

/// Run the TUI application with a package manager
pub fn run(package_manager: Arc<dyn PackageManager>) -> anyhow::Result<()> {
    run_with_config(PathBuf::from("."), package_manager)
}

/// Run the TUI application with a specific config directory and package manager
pub fn run_with_config(
    config_dir: PathBuf,
    package_manager: Arc<dyn PackageManager>,
) -> anyhow::Result<()> {
    // Initialize terminal
    let mut terminal = Terminal::new()?;

    // Create application state
    let mut app = App::new(config_dir, package_manager);
    app.init()?;

    // Create event handler
    let events = EventHandler::new(TICK_RATE);

    // Main loop
    while !app.should_quit {
        // Render UI
        terminal.draw(|frame| ui::render(frame, &app))?;

        // Handle events
        match events.next()? {
            Event::Key(key) => app.handle_key(key),
            Event::Mouse(_mouse) => {
                // Mouse handling can be added later
            }
            Event::Resize(_width, _height) => {
                // Terminal handles resize automatically
            }
            Event::Tick => {
                app.tick();
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use app::{ConfirmAction, HealthStatus, View};
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use iron_core::RiskLevel;

    #[test]
    fn test_app_creation() {
        let app = App::default();
        assert_eq!(app.view, View::Dashboard);
        assert!(!app.should_quit);
    }

    #[test]
    fn test_app_navigation() {
        let mut app = App::default();
        app.navigate(View::Bundles);
        assert_eq!(app.view, View::Bundles);
        assert_eq!(app.previous_view, Some(View::Dashboard));
    }

    #[test]
    fn test_app_go_back() {
        let mut app = App::default();
        app.navigate(View::Bundles);
        app.go_back();
        assert_eq!(app.view, View::Dashboard);
    }

    #[test]
    fn test_app_go_back_no_history() {
        let mut app = App::default();
        // Going back with no history should go to Dashboard
        app.view = View::Settings;
        app.go_back();
        assert_eq!(app.view, View::Dashboard);
    }

    #[test]
    fn test_app_navigation_chain() {
        let mut app = App::default();
        app.navigate(View::Bundles);
        app.navigate(View::BundleDetail);
        assert_eq!(app.view, View::BundleDetail);
        assert_eq!(app.previous_view, Some(View::Bundles));
        app.go_back();
        assert_eq!(app.view, View::Bundles);
    }

    #[test]
    fn test_app_messages() {
        let mut app = App::default();

        app.set_status("Test status");
        assert_eq!(app.status_message, Some("Test status".to_string()));

        app.set_error("Test error");
        assert_eq!(app.error_message, Some("Test error".to_string()));

        app.clear_messages();
        assert!(app.status_message.is_none());
        assert!(app.error_message.is_none());
    }

    #[test]
    fn test_app_confirm_dialog() {
        let mut app = App::default();
        assert!(!app.show_confirm);
        assert!(app.confirm_action.is_none());

        app.request_confirm(ConfirmAction::Quit);
        assert!(app.show_confirm);
        assert!(matches!(app.confirm_action, Some(ConfirmAction::Quit)));
    }

    #[test]
    fn test_app_help_overlay() {
        let mut app = App::default();
        assert!(!app.show_help);

        // Help key toggles overlay
        let help_key = KeyEvent::new(KeyCode::Char('?'), KeyModifiers::NONE);
        app.handle_key(help_key);
        assert!(app.show_help);

        // Any key closes help
        app.handle_key(help_key);
        assert!(!app.show_help);
    }

    #[test]
    fn test_app_quit_shortcut() {
        let mut app = App::default();

        // Ctrl+C should quit
        let quit_key = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL);
        app.handle_key(quit_key);
        assert!(app.should_quit);
    }

    #[test]
    fn test_app_health_status_ok() {
        let mut app = App::default();
        app.update_risk = RiskLevel::Low;
        app.pending_updates = vec![];
        assert_eq!(app.system_health(), HealthStatus::Ok);
    }

    #[test]
    fn test_app_health_status_warning() {
        let mut app = App::default();
        app.update_risk = RiskLevel::High;
        assert_eq!(app.system_health(), HealthStatus::Warning);
    }

    #[test]
    fn test_app_health_status_error() {
        let mut app = App::default();
        app.update_risk = RiskLevel::Critical;
        assert_eq!(app.system_health(), HealthStatus::Error);
    }

    #[test]
    fn test_app_module_active() {
        let mut app = App::default();
        app.active_modules = vec!["module1".to_string(), "module2".to_string()];

        assert!(app.is_module_active("module1"));
        assert!(app.is_module_active("module2"));
        assert!(!app.is_module_active("module3"));
    }

    #[test]
    fn test_app_counts() {
        let mut app = App::default();
        app.installed_count = 500;
        app.active_modules = vec!["m1".to_string(), "m2".to_string()];
        app.pending_updates = vec![]; // empty for this test

        assert_eq!(app.package_count(), 500);
        assert_eq!(app.enabled_module_count(), 2);
        assert_eq!(app.pending_update_count(), 0);
    }

    #[test]
    fn test_keyboard_navigation() {
        let mut app = App::default();

        // Test navigation shortcuts
        let nav_bundles = KeyEvent::new(KeyCode::Char('b'), KeyModifiers::NONE);
        app.handle_key(nav_bundles);
        assert_eq!(app.view, View::Bundles);

        let nav_profiles = KeyEvent::new(KeyCode::Char('p'), KeyModifiers::NONE);
        app.handle_key(nav_profiles);
        assert_eq!(app.view, View::Profiles);

        let nav_modules = KeyEvent::new(KeyCode::Char('m'), KeyModifiers::NONE);
        app.handle_key(nav_modules);
        assert_eq!(app.view, View::Modules);

        let nav_dashboard = KeyEvent::new(KeyCode::Char('d'), KeyModifiers::NONE);
        app.handle_key(nav_dashboard);
        assert_eq!(app.view, View::Dashboard);
    }

    #[test]
    fn test_tab_cycle() {
        let mut app = App::default();
        assert_eq!(app.view, View::Dashboard);

        // Tab should cycle through views
        let tab_key = KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE);
        app.handle_key(tab_key);
        assert_eq!(app.view, View::Bundles);

        app.handle_key(tab_key);
        assert_eq!(app.view, View::Profiles);
    }
}
