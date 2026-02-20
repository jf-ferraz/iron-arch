//! UI Rendering Tests
//!
//! Tests for TUI rendering functions using ratatui TestBackend.

use super::*;
use crate::app::{App, View};
use iron_core::{
    Bundle, BundleType, DotfileMapping, Module, ModuleKind, PackageUpdate, Profile, RiskLevel,
};
use ratatui::{Terminal, backend::TestBackend};

// =============================================================================
// Test Helpers
// =============================================================================

/// Create a test terminal with the given dimensions
fn create_test_terminal(width: u16, height: u16) -> Terminal<TestBackend> {
    let backend = TestBackend::new(width, height);
    Terminal::new(backend).unwrap()
}

/// Create a test bundle
fn create_test_bundle(id: &str, description: &str, profiles: Vec<&str>) -> Bundle {
    Bundle {
        id: id.to_string(),
        name: id.to_string(),
        description: Some(description.to_string()),
        bundle_type: BundleType::WaylandCompositor,
        packages: vec!["wayland".to_string()],
        aur_packages: vec![],
        profiles: profiles.iter().map(|s| s.to_string()).collect(),
        default_profile: profiles.first().map(|s| s.to_string()),
        conflicts: vec![],
        services: vec![],
        post_install: None,
    }
}

/// Create a test module
fn create_test_module(id: &str, description: &str, packages: Vec<&str>) -> Module {
    Module {
        id: id.to_string(),
        name: id.to_string(),
        description: Some(description.to_string()),
        kind: ModuleKind::AppConfig,
        packages: packages.iter().map(|s| s.to_string()).collect(),
        aur_packages: vec![],
        dotfiles: vec![DotfileMapping {
            source: "config".to_string(),
            target: format!("~/.config/{}", id),
            link: true,
        }],
        conflicts: vec![],
        depends: vec![],
        pre_install: None,
        post_install: None,
    }
}

/// Create a test profile
fn create_test_profile(id: &str, description: &str, modules: Vec<&str>) -> Profile {
    Profile {
        id: id.to_string(),
        name: id.to_string(),
        description: Some(description.to_string()),
        modules: modules.iter().map(|s| s.to_string()).collect(),
        theme: None,
        shell: None,
        extends: None,
        for_bundle: None,
    }
}

/// Create a test package update
fn create_test_update(name: &str, is_aur: bool) -> PackageUpdate {
    PackageUpdate {
        name: name.to_string(),
        current_version: "1.0.0".to_string(),
        new_version: "1.1.0".to_string(),
        is_aur,
        is_flagged: false,
        repository: if is_aur {
            "aur".to_string()
        } else {
            "extra".to_string()
        },
        ..Default::default()
    }
}

/// Create an App with test data for bundle views
fn create_app_with_bundles() -> App {
    let mut app = App::default();
    let bundle1 = create_test_bundle("hyprland", "Hyprland compositor", vec!["developer"]);
    let bundle2 = create_test_bundle("niri", "Niri compositor", vec!["minimal"]);
    app.active_bundle = Some(bundle1.clone());
    app.bundles = vec![bundle1, bundle2];
    app
}

/// Create an App with test data for module views
fn create_app_with_modules() -> App {
    let mut app = App::default();
    app.modules = vec![
        create_test_module(
            "nvim-ide",
            "Neovim IDE configuration",
            vec!["neovim", "ripgrep"],
        ),
        create_test_module("kitty-dev", "Kitty terminal config", vec!["kitty"]),
    ];
    app.active_modules = vec!["nvim-ide".to_string()];
    app
}

/// Create an App with test data for profile views
fn create_app_with_profiles() -> App {
    let mut app = App::default();
    app.profiles = vec![
        create_test_profile(
            "developer",
            "Developer workstation",
            vec!["nvim-ide", "kitty-dev"],
        ),
        create_test_profile("minimal", "Minimal setup", vec!["kitty-dev"]),
    ];
    app.active_profile = Some("developer".to_string());
    app
}

/// Create an App with pending updates
fn create_app_with_updates(count: usize, risk: RiskLevel) -> App {
    let mut app = App::default();
    app.pending_updates = (0..count)
        .map(|i| create_test_update(&format!("package-{}", i), i % 3 == 0))
        .collect();
    app.update_risk = risk;
    app
}

/// Get buffer content as a string for assertions
fn buffer_to_string(terminal: &Terminal<TestBackend>) -> String {
    terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .map(|c| c.symbol())
        .collect()
}

/// Check if buffer contains text (ignores whitespace padding)
fn buffer_contains(terminal: &Terminal<TestBackend>, text: &str) -> bool {
    buffer_to_string(terminal).contains(text)
}

// =============================================================================
// Dashboard Tests
// =============================================================================

#[test]
fn test_dashboard_renders_health_ok() {
    let mut terminal = create_test_terminal(80, 24);
    let app = App::default();

    terminal
        .draw(|f| {
            render_dashboard(f, f.area(), &app);
        })
        .unwrap();

    assert!(buffer_contains(&terminal, "System Status"));
    assert!(buffer_contains(&terminal, "Healthy"));
}

#[test]
fn test_dashboard_renders_health_warning() {
    let mut terminal = create_test_terminal(80, 24);
    let mut app = App::default();
    app.update_risk = RiskLevel::High;

    terminal
        .draw(|f| {
            render_dashboard(f, f.area(), &app);
        })
        .unwrap();

    assert!(buffer_contains(&terminal, "Attention"));
}

#[test]
fn test_dashboard_renders_health_error() {
    let mut terminal = create_test_terminal(80, 24);
    let mut app = App::default();
    app.update_risk = RiskLevel::Critical;

    terminal
        .draw(|f| {
            render_dashboard(f, f.area(), &app);
        })
        .unwrap();

    assert!(buffer_contains(&terminal, "Critical"));
}

#[test]
fn test_dashboard_shows_package_count() {
    let mut terminal = create_test_terminal(80, 24);
    let mut app = App::default();
    app.installed_count = 150;

    terminal
        .draw(|f| {
            render_dashboard(f, f.area(), &app);
        })
        .unwrap();

    assert!(buffer_contains(&terminal, "150"));
    assert!(buffer_contains(&terminal, "installed"));
}

#[test]
fn test_dashboard_shows_active_configuration() {
    let mut terminal = create_test_terminal(80, 24);
    let mut app = create_app_with_bundles();
    app.active_profile = Some("developer".to_string());

    terminal
        .draw(|f| {
            render_dashboard(f, f.area(), &app);
        })
        .unwrap();

    assert!(buffer_contains(&terminal, "Active Configuration"));
    assert!(buffer_contains(&terminal, "hyprland"));
    assert!(buffer_contains(&terminal, "developer"));
}

#[test]
fn test_dashboard_shows_quick_actions() {
    let mut terminal = create_test_terminal(80, 24);
    let app = App::default();

    terminal
        .draw(|f| {
            render_dashboard(f, f.area(), &app);
        })
        .unwrap();

    assert!(buffer_contains(&terminal, "Quick Actions"));
    // Key hints are now styled as " b ", " p ", " m "
    assert!(buffer_contains(&terminal, "Bundles"));
    assert!(buffer_contains(&terminal, "Profiles"));
    assert!(buffer_contains(&terminal, "Modules"));
}

#[test]
fn test_dashboard_shows_pending_updates_alert() {
    let mut terminal = create_test_terminal(80, 24);
    let app = create_app_with_updates(5, RiskLevel::Low);

    terminal
        .draw(|f| {
            render_dashboard(f, f.area(), &app);
        })
        .unwrap();

    assert!(buffer_contains(&terminal, "5 package updates"));
}

#[test]
fn test_dashboard_shows_no_alerts_when_empty() {
    let mut terminal = create_test_terminal(80, 24);
    let app = App::default();

    terminal
        .draw(|f| {
            render_dashboard(f, f.area(), &app);
        })
        .unwrap();

    // Default app has no bundles/modules, so it shows onboarding nudge
    assert!(buffer_contains(&terminal, "get started"));
}

// =============================================================================
// Bundles View Tests
// =============================================================================

#[test]
fn test_bundles_renders_list() {
    let mut terminal = create_test_terminal(80, 24);
    let app = create_app_with_bundles();

    terminal
        .draw(|f| {
            render_bundles(f, f.area(), &app);
        })
        .unwrap();

    assert!(buffer_contains(&terminal, "Bundles"));
    assert!(buffer_contains(&terminal, "hyprland"));
    assert!(buffer_contains(&terminal, "niri"));
}

#[test]
fn test_bundles_shows_active_indicator() {
    let mut terminal = create_test_terminal(80, 24);
    let app = create_app_with_bundles();

    terminal
        .draw(|f| {
            render_bundles(f, f.area(), &app);
        })
        .unwrap();

    // Active bundle has filled circle
    assert!(buffer_contains(&terminal, "●"));
    // Inactive bundle has empty circle
    assert!(buffer_contains(&terminal, "○"));
}

#[test]
fn test_bundles_shows_descriptions() {
    let mut terminal = create_test_terminal(80, 24);
    let app = create_app_with_bundles();

    terminal
        .draw(|f| {
            render_bundles(f, f.area(), &app);
        })
        .unwrap();

    assert!(buffer_contains(&terminal, "Hyprland compositor"));
    assert!(buffer_contains(&terminal, "Niri compositor"));
}

#[test]
fn test_bundles_empty_list() {
    let mut terminal = create_test_terminal(80, 24);
    let app = App::default();

    terminal
        .draw(|f| {
            render_bundles(f, f.area(), &app);
        })
        .unwrap();

    // Should still render the container
    assert!(buffer_contains(&terminal, "Bundles"));
}

// =============================================================================
// Bundle Detail Tests
// =============================================================================

#[test]
fn test_bundle_detail_renders_info() {
    let mut terminal = create_test_terminal(80, 24);
    let app = create_app_with_bundles();

    terminal
        .draw(|f| {
            render_bundle_detail(f, f.area(), &app);
        })
        .unwrap();

    assert!(buffer_contains(&terminal, "Bundle: hyprland"));
    assert!(buffer_contains(&terminal, "Hyprland compositor"));
    assert!(buffer_contains(&terminal, "Active"));
}

#[test]
fn test_bundle_detail_shows_profiles() {
    let mut terminal = create_test_terminal(80, 24);
    let app = create_app_with_bundles();

    terminal
        .draw(|f| {
            render_bundle_detail(f, f.area(), &app);
        })
        .unwrap();

    assert!(buffer_contains(&terminal, "Profiles"));
    assert!(buffer_contains(&terminal, "developer"));
}

#[test]
fn test_bundle_detail_no_selection() {
    let mut terminal = create_test_terminal(80, 24);
    let app = App::default();

    terminal
        .draw(|f| {
            render_bundle_detail(f, f.area(), &app);
        })
        .unwrap();

    assert!(buffer_contains(&terminal, "No bundle selected"));
}

// =============================================================================
// Modules View Tests
// =============================================================================

#[test]
fn test_modules_renders_list() {
    let mut terminal = create_test_terminal(80, 24);
    let app = create_app_with_modules();

    terminal
        .draw(|f| {
            render_modules(f, f.area(), &app);
        })
        .unwrap();

    assert!(buffer_contains(&terminal, "Modules"));
    assert!(buffer_contains(&terminal, "nvim-ide"));
    assert!(buffer_contains(&terminal, "kitty-dev"));
}

#[test]
fn test_modules_shows_enabled_indicator() {
    let mut terminal = create_test_terminal(80, 24);
    let app = create_app_with_modules();

    terminal
        .draw(|f| {
            render_modules(f, f.area(), &app);
        })
        .unwrap();

    // Enabled module has checkmark
    assert!(buffer_contains(&terminal, "✓"));
    // Disabled module has empty circle
    assert!(buffer_contains(&terminal, "○"));
}

#[test]
fn test_modules_shows_descriptions() {
    let mut terminal = create_test_terminal(80, 24);
    let app = create_app_with_modules();

    terminal
        .draw(|f| {
            render_modules(f, f.area(), &app);
        })
        .unwrap();

    assert!(buffer_contains(&terminal, "Neovim IDE configuration"));
    assert!(buffer_contains(&terminal, "Kitty terminal config"));
}

// =============================================================================
// Module Detail Tests
// =============================================================================

#[test]
fn test_module_detail_renders_info() {
    let mut terminal = create_test_terminal(80, 24);
    let app = create_app_with_modules();

    terminal
        .draw(|f| {
            render_module_detail(f, f.area(), &app);
        })
        .unwrap();

    assert!(buffer_contains(&terminal, "Module: nvim-ide"));
    assert!(buffer_contains(&terminal, "Enabled"));
}

#[test]
fn test_module_detail_shows_packages() {
    let mut terminal = create_test_terminal(80, 24);
    let app = create_app_with_modules();

    terminal
        .draw(|f| {
            render_module_detail(f, f.area(), &app);
        })
        .unwrap();

    assert!(buffer_contains(&terminal, "Packages"));
    assert!(buffer_contains(&terminal, "neovim"));
    assert!(buffer_contains(&terminal, "ripgrep"));
}

#[test]
fn test_module_detail_shows_dotfiles() {
    let mut terminal = create_test_terminal(80, 24);
    let app = create_app_with_modules();

    terminal
        .draw(|f| {
            render_module_detail(f, f.area(), &app);
        })
        .unwrap();

    assert!(buffer_contains(&terminal, "Dotfiles"));
    assert!(buffer_contains(&terminal, "config"));
    assert!(buffer_contains(&terminal, ".config/nvim"));
}

#[test]
fn test_module_detail_no_selection() {
    let mut terminal = create_test_terminal(80, 24);
    let app = App::default();

    terminal
        .draw(|f| {
            render_module_detail(f, f.area(), &app);
        })
        .unwrap();

    assert!(buffer_contains(&terminal, "No module selected"));
}

// =============================================================================
// Profiles View Tests
// =============================================================================

#[test]
fn test_profiles_renders_list() {
    let mut terminal = create_test_terminal(80, 24);
    let app = create_app_with_profiles();

    terminal
        .draw(|f| {
            render_profiles(f, f.area(), &app);
        })
        .unwrap();

    assert!(buffer_contains(&terminal, "Profiles"));
    assert!(buffer_contains(&terminal, "developer"));
    assert!(buffer_contains(&terminal, "minimal"));
}

#[test]
fn test_profiles_shows_active_indicator() {
    let mut terminal = create_test_terminal(80, 24);
    let app = create_app_with_profiles();

    terminal
        .draw(|f| {
            render_profiles(f, f.area(), &app);
        })
        .unwrap();

    // Active profile has filled circle, inactive has empty
    assert!(buffer_contains(&terminal, "●"));
    assert!(buffer_contains(&terminal, "○"));
}

#[test]
fn test_profiles_shows_descriptions() {
    let mut terminal = create_test_terminal(80, 24);
    let app = create_app_with_profiles();

    terminal
        .draw(|f| {
            render_profiles(f, f.area(), &app);
        })
        .unwrap();

    assert!(buffer_contains(&terminal, "Developer workstation"));
    assert!(buffer_contains(&terminal, "Minimal setup"));
}

// =============================================================================
// Profile Detail Tests
// =============================================================================

#[test]
fn test_profile_detail_renders_info() {
    let mut terminal = create_test_terminal(80, 24);
    let app = create_app_with_profiles();

    terminal
        .draw(|f| {
            render_profile_detail(f, f.area(), &app);
        })
        .unwrap();

    assert!(buffer_contains(&terminal, "Profile: developer"));
    assert!(buffer_contains(&terminal, "Developer workstation"));
}

#[test]
fn test_profile_detail_shows_modules() {
    let mut terminal = create_test_terminal(80, 24);
    let app = create_app_with_profiles();

    terminal
        .draw(|f| {
            render_profile_detail(f, f.area(), &app);
        })
        .unwrap();

    assert!(buffer_contains(&terminal, "Modules"));
    assert!(buffer_contains(&terminal, "nvim-ide"));
    assert!(buffer_contains(&terminal, "kitty-dev"));
}

#[test]
fn test_profile_detail_no_selection() {
    let mut terminal = create_test_terminal(80, 24);
    let app = App::default();

    terminal
        .draw(|f| {
            render_profile_detail(f, f.area(), &app);
        })
        .unwrap();

    assert!(buffer_contains(&terminal, "No profile selected"));
}

// =============================================================================
// Update Preview Tests
// =============================================================================

#[test]
fn test_update_preview_shows_count() {
    let mut terminal = create_test_terminal(80, 24);
    let app = create_app_with_updates(10, RiskLevel::Low);

    terminal
        .draw(|f| {
            render_update_preview(f, f.area(), &app);
        })
        .unwrap();

    // New UI format shows "N package(s)" in header section
    assert!(buffer_contains(&terminal, "10 package(s)"));
}

#[test]
fn test_update_preview_risk_low() {
    let mut terminal = create_test_terminal(80, 24);
    let app = create_app_with_updates(5, RiskLevel::Low);

    terminal
        .draw(|f| {
            render_update_preview(f, f.area(), &app);
        })
        .unwrap();

    assert!(buffer_contains(&terminal, "Safe to update"));
}

#[test]
fn test_update_preview_risk_medium() {
    let mut terminal = create_test_terminal(80, 24);
    let app = create_app_with_updates(5, RiskLevel::Medium);

    terminal
        .draw(|f| {
            render_update_preview(f, f.area(), &app);
        })
        .unwrap();

    assert!(buffer_contains(&terminal, "Review recommended"));
}

#[test]
fn test_update_preview_risk_high() {
    let mut terminal = create_test_terminal(80, 24);
    let app = create_app_with_updates(5, RiskLevel::High);

    terminal
        .draw(|f| {
            render_update_preview(f, f.area(), &app);
        })
        .unwrap();

    assert!(buffer_contains(&terminal, "Attention required"));
}

#[test]
fn test_update_preview_risk_critical() {
    let mut terminal = create_test_terminal(80, 24);
    let app = create_app_with_updates(5, RiskLevel::Critical);

    terminal
        .draw(|f| {
            render_update_preview(f, f.area(), &app);
        })
        .unwrap();

    assert!(buffer_contains(&terminal, "Create snapshot first"));
}

#[test]
fn test_update_preview_shows_packages() {
    let mut terminal = create_test_terminal(80, 24);
    let app = create_app_with_updates(3, RiskLevel::Low);

    terminal
        .draw(|f| {
            render_update_preview(f, f.area(), &app);
        })
        .unwrap();

    assert!(buffer_contains(&terminal, "Packages"));
    assert!(buffer_contains(&terminal, "package-0"));
    assert!(buffer_contains(&terminal, "1.0.0"));
    assert!(buffer_contains(&terminal, "1.1.0"));
}

#[test]
fn test_update_preview_aur_marker() {
    let mut terminal = create_test_terminal(80, 24);
    let app = create_app_with_updates(3, RiskLevel::Low);

    terminal
        .draw(|f| {
            render_update_preview(f, f.area(), &app);
        })
        .unwrap();

    // package-0 is AUR (i % 3 == 0)
    assert!(buffer_contains(&terminal, "[AUR]"));
}

#[test]
fn test_update_preview_keybindings() {
    let mut terminal = create_test_terminal(80, 24);
    let app = App::default();

    terminal
        .draw(|f| {
            render_update_preview(f, f.area(), &app);
        })
        .unwrap();

    assert!(buffer_contains(&terminal, "[r]"));
    assert!(buffer_contains(&terminal, "Refresh"));
    assert!(buffer_contains(&terminal, "[u]"));
    assert!(buffer_contains(&terminal, "Update"));
}

// =============================================================================
// Settings View Tests
// =============================================================================

#[test]
fn test_settings_renders() {
    let mut terminal = create_test_terminal(80, 24);
    let app = App::default();

    terminal
        .draw(|f| {
            render_settings(f, f.area(), &app);
        })
        .unwrap();

    // Settings view now uses just the Configuration block (no internal header)
    assert!(buffer_contains(&terminal, "Configuration"));
    assert!(buffer_contains(&terminal, "Current Host"));
}

// =============================================================================
// Sync View Tests
// =============================================================================

#[test]
fn test_sync_renders() {
    let mut terminal = create_test_terminal(80, 24);
    let app = App::default();

    terminal
        .draw(|f| {
            render_sync(f, f.area(), &app);
        })
        .unwrap();

    assert!(buffer_contains(&terminal, "Sync"));
    assert!(buffer_contains(&terminal, "Git Sync"));
    assert!(buffer_contains(&terminal, "[p]"));
    assert!(buffer_contains(&terminal, "Push"));
    assert!(buffer_contains(&terminal, "[f]"));
    assert!(buffer_contains(&terminal, "Pull"));
}

// =============================================================================
// Main Render Function Tests
// =============================================================================

#[test]
fn test_render_dispatches_to_dashboard() {
    let mut terminal = create_test_terminal(80, 24);
    let mut app = App::default();
    app.view = View::Dashboard;

    terminal
        .draw(|f| {
            render(f, &app);
        })
        .unwrap();

    assert!(buffer_contains(&terminal, "System Status"));
}

#[test]
fn test_render_dispatches_to_bundles() {
    let mut terminal = create_test_terminal(80, 24);
    let mut app = create_app_with_bundles();
    app.view = View::Bundles;

    terminal
        .draw(|f| {
            render(f, &app);
        })
        .unwrap();

    assert!(buffer_contains(&terminal, "hyprland"));
}

#[test]
fn test_render_dispatches_to_modules() {
    let mut terminal = create_test_terminal(80, 24);
    let mut app = create_app_with_modules();
    app.view = View::Modules;

    terminal
        .draw(|f| {
            render(f, &app);
        })
        .unwrap();

    assert!(buffer_contains(&terminal, "nvim-ide"));
}

#[test]
fn test_render_dispatches_to_profiles() {
    let mut terminal = create_test_terminal(80, 24);
    let mut app = create_app_with_profiles();
    app.view = View::Profiles;

    terminal
        .draw(|f| {
            render(f, &app);
        })
        .unwrap();

    assert!(buffer_contains(&terminal, "developer"));
}

#[test]
fn test_render_dispatches_to_settings() {
    let mut terminal = create_test_terminal(80, 24);
    let mut app = App::default();
    app.view = View::Settings;

    terminal
        .draw(|f| {
            render(f, &app);
        })
        .unwrap();

    assert!(buffer_contains(&terminal, "Settings"));
}

#[test]
fn test_render_dispatches_to_update_preview() {
    let mut terminal = create_test_terminal(80, 24);
    let mut app = App::default();
    app.view = View::UpdatePreview;

    terminal
        .draw(|f| {
            render(f, &app);
        })
        .unwrap();

    assert!(buffer_contains(&terminal, "System Update"));
}

#[test]
fn test_render_shows_header() {
    let mut terminal = create_test_terminal(80, 24);
    let app = App::default();

    terminal
        .draw(|f| {
            render(f, &app);
        })
        .unwrap();

    assert!(buffer_contains(&terminal, "IRON"));
}

#[test]
fn test_render_shows_help_overlay() {
    let mut terminal = create_test_terminal(80, 24);
    let mut app = App::default();
    app.show_help = true;

    terminal
        .draw(|f| {
            render(f, &app);
        })
        .unwrap();

    assert!(buffer_contains(&terminal, "Help"));
    assert!(buffer_contains(&terminal, "Navigation"));
}

#[test]
fn test_render_shows_confirm_dialog() {
    let mut terminal = create_test_terminal(80, 24);
    let mut app = App::default();
    app.show_confirm = true;
    app.confirm_action = Some(crate::app::ConfirmAction::Quit);

    terminal
        .draw(|f| {
            render(f, &app);
        })
        .unwrap();

    assert!(buffer_contains(&terminal, "Confirm"));
    assert!(buffer_contains(&terminal, "Quit Iron"));
}

// =============================================================================
// Terminal Size Tests
// =============================================================================

#[test]
fn test_render_minimum_size() {
    let mut terminal = create_test_terminal(40, 12);
    let app = App::default();

    // Should not panic at minimum size
    terminal
        .draw(|f| {
            render(f, &app);
        })
        .unwrap();
}

#[test]
fn test_render_large_size() {
    let mut terminal = create_test_terminal(200, 60);
    let app = App::default();

    // Should handle large terminal gracefully
    terminal
        .draw(|f| {
            render(f, &app);
        })
        .unwrap();

    assert!(buffer_contains(&terminal, "IRON"));
}

#[test]
fn test_bundles_renders_at_various_sizes() {
    for (width, height) in [(40, 12), (80, 24), (120, 40), (200, 60)] {
        let mut terminal = create_test_terminal(width, height);
        let app = create_app_with_bundles();

        terminal
            .draw(|f| {
                render_bundles(f, f.area(), &app);
            })
            .unwrap();

        // Should always show the title
        assert!(buffer_contains(&terminal, "Bundles"));
    }
}

// =============================================================================
// Wizard UI Rendering Tests
// =============================================================================

/// Create an App in wizard mode
fn create_app_with_wizard() -> App {
    let mut app = App::default();
    app.view = View::SetupWizard;
    app.wizard = crate::wizard::WizardState::new();
    app.wizard.available_bundles = vec![
        "hyprland".to_string(),
        "niri".to_string(),
        "sway".to_string(),
    ];
    app.wizard.available_profiles = vec!["developer".to_string(), "minimal".to_string()];
    app.wizard.host_id = "desktop".to_string();
    app.host_input = crate::wizard::TextInput::new("desktop");
    app
}

#[test]
fn test_wizard_renders_welcome_step() {
    let mut terminal = create_test_terminal(80, 24);
    let app = create_app_with_wizard();

    terminal
        .draw(|f| {
            super::wizard::render_setup_wizard(f, f.area(), &app);
        })
        .unwrap();

    assert!(buffer_contains(&terminal, "Welcome to Iron"));
    assert!(buffer_contains(&terminal, "First-Time Setup"));
    assert!(buffer_contains(&terminal, "Press Enter to begin"));
}

#[test]
fn test_wizard_renders_progress_indicator() {
    let mut terminal = create_test_terminal(80, 24);
    let app = create_app_with_wizard();

    terminal
        .draw(|f| {
            super::wizard::render_setup_wizard(f, f.area(), &app);
        })
        .unwrap();

    assert!(buffer_contains(&terminal, "Step 1 of 5"));
}

#[test]
fn test_wizard_renders_host_setup_step() {
    let mut terminal = create_test_terminal(80, 24);
    let mut app = create_app_with_wizard();
    app.wizard.step = crate::wizard::WizardStep::HostSetup;

    terminal
        .draw(|f| {
            super::wizard::render_setup_wizard(f, f.area(), &app);
        })
        .unwrap();

    assert!(buffer_contains(&terminal, "Host Setup"));
    assert!(buffer_contains(&terminal, "desktop"));
    assert!(buffer_contains(&terminal, "Host ID"));
}

#[test]
fn test_wizard_renders_host_setup_editing_mode() {
    let mut terminal = create_test_terminal(80, 24);
    let mut app = create_app_with_wizard();
    app.wizard.step = crate::wizard::WizardStep::HostSetup;
    app.host_input.enter_edit_mode();

    terminal
        .draw(|f| {
            super::wizard::render_setup_wizard(f, f.area(), &app);
        })
        .unwrap();

    assert!(buffer_contains(&terminal, "Press Enter to confirm"));
}

#[test]
fn test_wizard_renders_bundle_selection_step() {
    let mut terminal = create_test_terminal(80, 24);
    let mut app = create_app_with_wizard();
    app.wizard.step = crate::wizard::WizardStep::BundleSelection;

    terminal
        .draw(|f| {
            super::wizard::render_setup_wizard(f, f.area(), &app);
        })
        .unwrap();

    assert!(buffer_contains(&terminal, "Bundle Selection"));
    assert!(buffer_contains(&terminal, "hyprland"));
    assert!(buffer_contains(&terminal, "niri"));
    assert!(buffer_contains(&terminal, "sway"));
}

#[test]
fn test_wizard_renders_bundle_selection_indicator() {
    let mut terminal = create_test_terminal(80, 24);
    let mut app = create_app_with_wizard();
    app.wizard.step = crate::wizard::WizardStep::BundleSelection;
    app.wizard.selected_bundle_index = 0;

    terminal
        .draw(|f| {
            super::wizard::render_setup_wizard(f, f.area(), &app);
        })
        .unwrap();

    // Selected bundle should have filled circle
    assert!(buffer_contains(&terminal, "●"));
    // Other bundles have empty circles
    assert!(buffer_contains(&terminal, "○"));
}

#[test]
fn test_wizard_renders_empty_bundles() {
    let mut terminal = create_test_terminal(80, 24);
    let mut app = create_app_with_wizard();
    app.wizard.step = crate::wizard::WizardStep::BundleSelection;
    app.wizard.available_bundles.clear();

    terminal
        .draw(|f| {
            super::wizard::render_setup_wizard(f, f.area(), &app);
        })
        .unwrap();

    assert!(buffer_contains(&terminal, "No bundles found"));
}

#[test]
fn test_wizard_renders_profile_selection_step() {
    let mut terminal = create_test_terminal(80, 24);
    let mut app = create_app_with_wizard();
    app.wizard.step = crate::wizard::WizardStep::ProfileSelection;

    terminal
        .draw(|f| {
            super::wizard::render_setup_wizard(f, f.area(), &app);
        })
        .unwrap();

    assert!(buffer_contains(&terminal, "Profile Selection"));
    assert!(buffer_contains(&terminal, "developer"));
    assert!(buffer_contains(&terminal, "minimal"));
}

#[test]
fn test_wizard_renders_empty_profiles() {
    let mut terminal = create_test_terminal(80, 24);
    let mut app = create_app_with_wizard();
    app.wizard.step = crate::wizard::WizardStep::ProfileSelection;
    app.wizard.available_profiles.clear();

    terminal
        .draw(|f| {
            super::wizard::render_setup_wizard(f, f.area(), &app);
        })
        .unwrap();

    assert!(buffer_contains(&terminal, "No profiles found"));
}

#[test]
fn test_wizard_renders_confirmation_step() {
    let mut terminal = create_test_terminal(80, 24);
    let mut app = create_app_with_wizard();
    app.wizard.step = crate::wizard::WizardStep::Confirmation;
    app.wizard.host_id = "myhost".to_string();

    terminal
        .draw(|f| {
            super::wizard::render_setup_wizard(f, f.area(), &app);
        })
        .unwrap();

    assert!(buffer_contains(&terminal, "Confirmation"));
    assert!(buffer_contains(&terminal, "Host ID"));
    assert!(buffer_contains(&terminal, "myhost"));
    assert!(buffer_contains(&terminal, "Bundle"));
    assert!(buffer_contains(&terminal, "Profile"));
}

#[test]
fn test_wizard_renders_confirmation_with_error() {
    let mut terminal = create_test_terminal(80, 24);
    let mut app = create_app_with_wizard();
    app.wizard.step = crate::wizard::WizardStep::Confirmation;
    app.wizard.error = Some("Test error message".to_string());

    terminal
        .draw(|f| {
            super::wizard::render_setup_wizard(f, f.area(), &app);
        })
        .unwrap();

    assert!(buffer_contains(&terminal, "Error"));
    assert!(buffer_contains(&terminal, "Test error message"));
}

#[test]
fn test_wizard_renders_complete_step() {
    let mut terminal = create_test_terminal(80, 24);
    let mut app = create_app_with_wizard();
    app.wizard.step = crate::wizard::WizardStep::Complete;

    terminal
        .draw(|f| {
            super::wizard::render_setup_wizard(f, f.area(), &app);
        })
        .unwrap();

    assert!(buffer_contains(&terminal, "Setup Complete"));
    assert!(buffer_contains(
        &terminal,
        "Press Enter to go to the Dashboard"
    ));
}

#[test]
fn test_wizard_renders_navigation_hints() {
    let mut terminal = create_test_terminal(80, 24);
    let mut app = create_app_with_wizard();
    app.wizard.step = crate::wizard::WizardStep::BundleSelection;

    terminal
        .draw(|f| {
            super::wizard::render_setup_wizard(f, f.area(), &app);
        })
        .unwrap();

    // Should show back and continue hints
    assert!(buffer_contains(&terminal, "Backspace"));
    assert!(buffer_contains(&terminal, "Back"));
    assert!(buffer_contains(&terminal, "Enter"));
    assert!(buffer_contains(&terminal, "Continue"));
    assert!(buffer_contains(&terminal, "[q]"));
    assert!(buffer_contains(&terminal, "Quit"));
}

#[test]
fn test_wizard_progress_updates_per_step() {
    for (step, expected_num) in [
        (crate::wizard::WizardStep::Welcome, 1),
        (crate::wizard::WizardStep::HostSetup, 2),
        (crate::wizard::WizardStep::BundleSelection, 3),
        (crate::wizard::WizardStep::ProfileSelection, 4),
        (crate::wizard::WizardStep::Confirmation, 5),
    ] {
        let mut terminal = create_test_terminal(80, 24);
        let mut app = create_app_with_wizard();
        app.wizard.step = step.clone();

        terminal
            .draw(|f| {
                super::wizard::render_setup_wizard(f, f.area(), &app);
            })
            .unwrap();

        assert!(
            buffer_contains(&terminal, &format!("Step {} of 5", expected_num)),
            "Expected Step {} of 5 for {:?}",
            expected_num,
            step.clone()
        );
    }
}

#[test]
fn test_wizard_renders_at_various_sizes() {
    for (width, height) in [(40, 12), (80, 24), (120, 40)] {
        let mut terminal = create_test_terminal(width, height);
        let app = create_app_with_wizard();

        // Should not panic at any size
        terminal
            .draw(|f| {
                super::wizard::render_setup_wizard(f, f.area(), &app);
            })
            .unwrap();
    }
}

#[test]
fn test_render_dispatches_to_wizard() {
    let mut terminal = create_test_terminal(80, 24);
    let app = create_app_with_wizard();

    terminal
        .draw(|f| {
            render(f, &app);
        })
        .unwrap();

    assert!(buffer_contains(&terminal, "Welcome to Iron"));
}

#[test]
fn test_render_host_selection_empty() {
    let mut terminal = create_test_terminal(80, 24);
    let mut app = App::default();
    app.view = View::HostSelection;

    terminal
        .draw(|f| {
            render(f, &app);
        })
        .unwrap();

    assert!(buffer_contains(&terminal, "No hosts configured"));
}

#[test]
fn test_render_host_selection_with_hosts() {
    use iron_core::host::{HardwareSpec, Host};

    let mut terminal = create_test_terminal(100, 30);
    let mut app = App::default();
    app.view = View::HostSelection;
    app.discovered_hosts = vec![
        Host {
            id: "desktop".to_string(),
            name: "Desktop Workstation".to_string(),
            description: None,
            hardware: HardwareSpec {
                cpu: Some("AMD Ryzen 7 9800X3D".to_string()),
                gpu: Some("RX 9060 XT".to_string()),
                ram_mb: Some(30720),
                monitors: vec![],
                chassis: None,
            },
            install_params: None,
            installed_bundles: vec![],
            active_bundle: None,
        },
        Host {
            id: "laptop".to_string(),
            name: "Laptop".to_string(),
            description: None,
            hardware: HardwareSpec::default(),
            install_params: None,
            installed_bundles: vec![],
            active_bundle: None,
        },
    ];
    app.current_host = Some("desktop".to_string());

    terminal
        .draw(|f| {
            render(f, &app);
        })
        .unwrap();

    assert!(buffer_contains(&terminal, "Host Selection"));
    assert!(buffer_contains(&terminal, "desktop"));
    assert!(buffer_contains(&terminal, "laptop"));
}
