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
#[allow(clippy::field_reassign_with_default)]
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

    // ==========================================================================
    // TICK_RATE Constant Tests
    // ==========================================================================

    #[test]
    fn test_tick_rate_constant() {
        assert_eq!(TICK_RATE, Duration::from_millis(250));
        assert!(TICK_RATE.as_millis() > 0);
        assert!(TICK_RATE.as_millis() < 1000);
    }

    // ==========================================================================
    // View Enum Tests
    // ==========================================================================

    #[test]
    fn test_view_enum_equality() {
        assert_eq!(View::Dashboard, View::Dashboard);
        assert_ne!(View::Dashboard, View::Bundles);
    }

    #[test]
    fn test_view_enum_clone() {
        let view = View::SetupWizard;
        let cloned = view.clone();
        assert_eq!(view, cloned);
    }

    #[test]
    fn test_view_enum_copy() {
        let view = View::Modules;
        let copied = view;
        assert_eq!(view, copied);
    }

    #[test]
    fn test_view_enum_debug() {
        let view = View::UpdatePreview;
        let debug_str = format!("{:?}", view);
        assert!(debug_str.contains("UpdatePreview"));
    }

    #[test]
    fn test_all_view_variants() {
        let views = [
            View::Dashboard,
            View::SetupWizard,
            View::Bundles,
            View::BundleDetail,
            View::Profiles,
            View::ProfileDetail,
            View::Modules,
            View::ModuleDetail,
            View::UpdatePreview,
            View::Sync,
            View::Settings,
        ];

        // Verify all views are unique
        for (i, view1) in views.iter().enumerate() {
            for (j, view2) in views.iter().enumerate() {
                if i == j {
                    assert_eq!(view1, view2);
                } else {
                    assert_ne!(view1, view2);
                }
            }
        }
    }

    // ==========================================================================
    // HealthStatus Enum Tests
    // ==========================================================================

    #[test]
    fn test_health_status_equality() {
        assert_eq!(HealthStatus::Ok, HealthStatus::Ok);
        assert_ne!(HealthStatus::Ok, HealthStatus::Warning);
        assert_ne!(HealthStatus::Warning, HealthStatus::Error);
    }

    #[test]
    fn test_health_status_clone() {
        let status = HealthStatus::Warning;
        let cloned = status.clone();
        assert_eq!(status, cloned);
    }

    #[test]
    fn test_health_status_copy() {
        let status = HealthStatus::Error;
        let copied = status;
        assert_eq!(status, copied);
    }

    #[test]
    fn test_health_status_debug() {
        assert!(format!("{:?}", HealthStatus::Ok).contains("Ok"));
        assert!(format!("{:?}", HealthStatus::Warning).contains("Warning"));
        assert!(format!("{:?}", HealthStatus::Error).contains("Error"));
    }

    // ==========================================================================
    // ConfirmAction Enum Tests
    // ==========================================================================

    #[test]
    fn test_confirm_action_clone() {
        let action = ConfirmAction::SwitchBundle("test".to_string());
        let cloned = action.clone();
        match cloned {
            ConfirmAction::SwitchBundle(id) => assert_eq!(id, "test"),
            _ => panic!("Clone failed"),
        }
    }

    #[test]
    fn test_confirm_action_debug() {
        let action = ConfirmAction::EnableModule("nvim".to_string());
        let debug_str = format!("{:?}", action);
        assert!(debug_str.contains("EnableModule"));
        assert!(debug_str.contains("nvim"));
    }

    #[test]
    fn test_confirm_action_all_variants() {
        let actions = vec![
            ConfirmAction::SwitchBundle("bundle".to_string()),
            ConfirmAction::RemoveBundle("bundle".to_string()),
            ConfirmAction::EnableModule("module".to_string()),
            ConfirmAction::DisableModule("module".to_string()),
            ConfirmAction::RunUpdate,
            ConfirmAction::Quit,
        ];

        for action in actions {
            let debug_str = format!("{:?}", action);
            assert!(!debug_str.is_empty());
        }
    }

    // ==========================================================================
    // App State Accessors Tests
    // ==========================================================================

    #[test]
    fn test_app_selected_bundle_none() {
        let app = App::default();
        assert!(app.selected_bundle().is_none());
    }

    #[test]
    fn test_app_selected_bundle_with_bundles() {
        use iron_core::{Bundle, BundleType};

        let mut app = App::default();
        app.bundles = vec![Bundle {
            id: "hyprland".to_string(),
            name: "Hyprland".to_string(),
            description: Some("Compositor".to_string()),
            bundle_type: BundleType::WaylandCompositor,
            packages: vec![],
            aur_packages: vec![],
            profiles: vec![],
            default_profile: None,
            conflicts: vec![],
            services: vec![],
            post_install: None,
        }];
        app.selected_index = 0;

        let selected = app.selected_bundle();
        assert!(selected.is_some());
        assert_eq!(selected.unwrap().id, "hyprland");
    }

    #[test]
    fn test_app_selected_profile_none() {
        let app = App::default();
        assert!(app.selected_profile().is_none());
    }

    #[test]
    fn test_app_selected_profile_with_profiles() {
        use iron_core::Profile;

        let mut app = App::default();
        app.profiles = vec![Profile {
            id: "developer".to_string(),
            name: "Developer".to_string(),
            description: Some("Dev profile".to_string()),
            modules: vec![],
            theme: None,
            shell: None,
            extends: None,
            for_bundle: None,
        }];
        app.selected_index = 0;

        let selected = app.selected_profile();
        assert!(selected.is_some());
        assert_eq!(selected.unwrap().id, "developer");
    }

    #[test]
    fn test_app_selected_module_none() {
        let app = App::default();
        assert!(app.selected_module().is_none());
    }

    #[test]
    fn test_app_selected_module_with_modules() {
        use iron_core::{Module, ModuleKind};

        let mut app = App::default();
        app.modules = vec![Module {
            id: "nvim-ide".to_string(),
            name: "Neovim IDE".to_string(),
            description: Some("IDE setup".to_string()),
            kind: ModuleKind::AppConfig,
            packages: vec![],
            aur_packages: vec![],
            dotfiles: vec![],
            conflicts: vec![],
            depends: vec![],
            pre_install: None,
            post_install: None,
        }];
        app.selected_index = 0;

        let selected = app.selected_module();
        assert!(selected.is_some());
        assert_eq!(selected.unwrap().id, "nvim-ide");
    }

    // ==========================================================================
    // App Health Calculation Tests
    // ==========================================================================

    #[test]
    fn test_app_health_with_many_updates() {
        use iron_core::PackageUpdate;

        let mut app = App::default();
        app.update_risk = RiskLevel::Low;
        // More than 50 updates triggers warning
        app.pending_updates = (0..51).map(|i| PackageUpdate {
            name: format!("pkg-{}", i),
            current_version: "1.0.0".to_string(),
            new_version: "1.1.0".to_string(),
            is_aur: false,
            is_flagged: false,
            repository: "extra".to_string(),
        }).collect();

        assert_eq!(app.system_health(), HealthStatus::Warning);
    }

    #[test]
    fn test_app_health_medium_risk() {
        let mut app = App::default();
        app.update_risk = RiskLevel::Medium;
        assert_eq!(app.system_health(), HealthStatus::Ok);
    }

    // ==========================================================================
    // App Update Risk Tests
    // ==========================================================================

    #[test]
    fn test_app_update_risk_level() {
        let mut app = App::default();
        app.update_risk = RiskLevel::Critical;
        assert_eq!(app.update_risk_level(), RiskLevel::Critical);
    }

    #[test]
    fn test_app_pending_updates_list() {
        use iron_core::PackageUpdate;

        let mut app = App::default();
        app.pending_updates = vec![
            PackageUpdate {
                name: "test-pkg".to_string(),
                current_version: "1.0.0".to_string(),
                new_version: "2.0.0".to_string(),
                is_aur: false,
                is_flagged: false,
                repository: "core".to_string(),
            }
        ];

        let updates = app.pending_updates_list();
        assert_eq!(updates.len(), 1);
        assert_eq!(updates[0].name, "test-pkg");
    }

    // ==========================================================================
    // App Quit Tests
    // ==========================================================================

    #[test]
    fn test_app_quit_method() {
        let mut app = App::default();
        assert!(!app.should_quit);

        app.quit();

        assert!(app.should_quit);
    }

    // ==========================================================================
    // App Tick Tests
    // ==========================================================================

    #[test]
    fn test_app_tick_no_crash() {
        let mut app = App::default();

        // tick() should not crash
        app.tick();
        app.tick();
        app.tick();

        // App state should be unchanged (tick is placeholder)
        assert!(!app.should_quit);
    }

    // ==========================================================================
    // Navigation Reset Tests
    // ==========================================================================

    #[test]
    fn test_navigate_resets_selected_index() {
        let mut app = App::default();
        app.selected_index = 5;

        app.navigate(View::Bundles);

        assert_eq!(app.selected_index, 0);
    }

    #[test]
    fn test_navigate_clears_messages() {
        let mut app = App::default();
        app.set_status("Test status");
        app.set_error("Test error");

        app.navigate(View::Modules);

        assert!(app.status_message.is_none());
        assert!(app.error_message.is_none());
    }

    #[test]
    fn test_go_back_resets_selected_index() {
        let mut app = App::default();
        app.navigate(View::Bundles);
        app.selected_index = 10;

        app.go_back();

        assert_eq!(app.selected_index, 0);
    }

    #[test]
    fn test_go_back_clears_messages() {
        let mut app = App::default();
        app.navigate(View::Bundles);
        app.set_status("Test status");
        app.set_error("Test error");

        app.go_back();

        assert!(app.status_message.is_none());
        assert!(app.error_message.is_none());
    }
}
