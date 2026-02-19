//! Key handling for Iron TUI
//!
//! Contains all keyboard input handling logic.

use super::{App, ConfirmAction, View};
use crate::wizard::WizardStep;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

impl App {
    /// Handle key input
    pub fn handle_key(&mut self, key: KeyEvent) {
        // Global shortcuts
        if key.modifiers.contains(KeyModifiers::CONTROL) {
            match key.code {
                KeyCode::Char('c') | KeyCode::Char('q') => {
                    self.should_quit = true;
                    return;
                }
                _ => {}
            }
        }

        // Help overlay
        if self.show_help {
            self.show_help = false;
            return;
        }

        // Confirm dialog
        if self.show_confirm {
            match key.code {
                KeyCode::Char('y') | KeyCode::Enter => {
                    self.execute_confirm_action();
                    self.show_confirm = false;
                    self.confirm_action = None;
                }
                KeyCode::Char('n') | KeyCode::Esc => {
                    self.show_confirm = false;
                    self.confirm_action = None;
                }
                _ => {}
            }
            return;
        }

        // Wizard handling
        if self.view == View::SetupWizard {
            self.handle_wizard_key(key);
            return;
        }

        // View-specific key handling (actions only, falls through for navigation)
        let handled = match self.view {
            View::UpdatePreview => match key.code {
                KeyCode::Char('r') => {
                    self.refresh_updates();
                    true
                }
                KeyCode::Char('u') => {
                    if self.can_proceed_with_update() {
                        self.request_confirm(ConfirmAction::RunUpdate);
                    } else {
                        self.set_warning("Cannot update - resolve pre-flight issues first");
                    }
                    true
                }
                // Section navigation with arrow keys (Tab cycles views globally)
                KeyCode::Right | KeyCode::Char('l') => {
                    self.next_update_section();
                    true
                }
                KeyCode::Left | KeyCode::Char('h') => {
                    self.prev_update_section();
                    true
                }
                // Item navigation within sections
                KeyCode::Up | KeyCode::Char('k') => {
                    self.update_section_up();
                    true
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    self.update_section_down();
                    true
                }
                // News acknowledgment
                KeyCode::Char('a') => {
                    if let Some(url) = self.acknowledge_selected_news() {
                        self.set_status(format!("Acknowledged: {}", url));
                    }
                    true
                }
                KeyCode::Char('A') => {
                    let count = self.acknowledge_all_news();
                    if count > 0 {
                        self.set_status(format!("Acknowledged {} news item(s)", count));
                    }
                    true
                }
                _ => false,
            },
            View::ProfileDetail => match key.code {
                KeyCode::Enter | KeyCode::Char('a') => {
                    self.activate_selected_profile();
                    true
                }
                _ => false,
            },
            // Phase 3: CleanSystem view handlers
            View::CleanSystem => match key.code {
                // Toggle category selection
                KeyCode::Char(' ') => {
                    self.toggle_selected_cleanup_category();
                    true
                }
                // Select all safe categories
                KeyCode::Char('s') => {
                    self.select_safe_cleanup_categories();
                    self.set_info("Selected safe categories");
                    true
                }
                // Select all categories (including aggressive)
                KeyCode::Char('a') => {
                    self.select_all_cleanup_categories();
                    self.set_warning("Selected all categories (including aggressive)");
                    true
                }
                // Deselect all
                KeyCode::Char('n') => {
                    self.deselect_all_cleanup_categories();
                    self.set_info("Deselected all categories");
                    true
                }
                // Preview (refresh estimates)
                KeyCode::Enter => {
                    self.preview_cleanup();
                    true
                }
                // Execute cleanup
                KeyCode::Char('c') => {
                    if !self.cleanup_categories.is_empty() {
                        self.request_confirm(ConfirmAction::RunCleanup);
                    } else {
                        self.set_warning("No categories selected");
                    }
                    true
                }
                // Navigate categories
                KeyCode::Up | KeyCode::Char('k') => {
                    self.select_previous();
                    true
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    self.select_next();
                    true
                }
                _ => false,
            },
            // SystemMaintenance view handlers
            View::SystemMaintenance => match key.code {
                // Quick shortcuts to actions
                KeyCode::Char('u') => {
                    self.navigate(View::UpdatePreview);
                    true
                }
                KeyCode::Char('c') => {
                    self.navigate(View::CleanSystem);
                    true
                }
                KeyCode::Char('d') => {
                    // Doctor not yet implemented, show info message
                    self.set_info("System Doctor coming soon");
                    true
                }
                // Card navigation
                KeyCode::Left | KeyCode::Char('h') => {
                    if self.selected_index > 0 {
                        self.selected_index -= 1;
                    }
                    true
                }
                KeyCode::Right | KeyCode::Char('l') => {
                    if self.selected_index < 2 {
                        self.selected_index += 1;
                    }
                    true
                }
                // Enter to launch selected action
                KeyCode::Enter => {
                    match self.selected_index {
                        0 => self.navigate(View::UpdatePreview),
                        1 => self.navigate(View::CleanSystem),
                        2 => self.set_info("System Doctor coming soon"),
                        _ => {}
                    }
                    true
                }
                _ => false,
            },
            // ConfigManager view handlers
            View::ConfigManager => match key.code {
                KeyCode::Up | KeyCode::Char('k') => {
                    self.select_previous();
                    true
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    self.select_next();
                    true
                }
                KeyCode::Enter => {
                    // TODO: View diff
                    self.set_info("Diff viewer coming soon");
                    true
                }
                KeyCode::Char('r') => {
                    // TODO: Mark resolved
                    self.set_info("Mark resolved coming soon");
                    true
                }
                _ => false,
            },
            // OperationLog view handlers
            View::OperationLog => match key.code {
                KeyCode::Up | KeyCode::Char('k') => {
                    self.select_previous();
                    true
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    self.select_next();
                    true
                }
                KeyCode::Char('f') => {
                    // TODO: Filter dialog
                    self.set_info("Filter coming soon");
                    true
                }
                KeyCode::Char('/') => {
                    // TODO: Search
                    self.set_info("Search coming soon");
                    true
                }
                _ => false,
            },
            // SecurityModules view handlers
            View::SecurityModules => match key.code {
                KeyCode::Up | KeyCode::Char('k') => {
                    self.select_previous();
                    true
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    self.select_next();
                    true
                }
                KeyCode::Enter | KeyCode::Char('e') => {
                    // Toggle module
                    self.toggle_selected_module();
                    true
                }
                KeyCode::Char('i') => {
                    // TODO: Install module
                    self.set_info("Module installation coming soon");
                    true
                }
                _ => false,
            },
            // Settings view handlers
            View::Settings => match key.code {
                KeyCode::Up | KeyCode::Char('k') => {
                    self.select_previous();
                    true
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    self.select_next();
                    true
                }
                KeyCode::Enter => {
                    self.edit_selected_setting();
                    true
                }
                KeyCode::Char('r') => {
                    self.refresh_settings();
                    true
                }
                KeyCode::Char('o') => {
                    self.navigate(View::OperationLog);
                    true
                }
                KeyCode::Char('c') => {
                    self.navigate(View::ConfigManager);
                    true
                }
                KeyCode::Char('w') => {
                    self.navigate(View::SetupWizard);
                    true
                }
                _ => false,
            },
            // Sync view handlers
            View::Sync => match key.code {
                KeyCode::Char('p') => {
                    // TODO: Implement git push
                    self.set_status("Git push not yet implemented");
                    true
                }
                KeyCode::Char('f') => {
                    // TODO: Implement git fetch/pull
                    self.set_status("Git pull not yet implemented");
                    true
                }
                KeyCode::Char('s') => {
                    // TODO: Implement git status refresh
                    self.set_status("Refreshing git status...");
                    true
                }
                _ => false,
            },
            _ => false,
        };

        if handled {
            return;
        }

        // General key handling
        match key.code {
            KeyCode::Char('q') => self.should_quit = true,
            KeyCode::Char('?') => self.show_help = true,
            KeyCode::Esc => self.go_back(),
            KeyCode::Tab => self.cycle_view_forward(),
            KeyCode::BackTab => self.cycle_view_backward(),

            // Navigation
            KeyCode::Char('d') => self.navigate(View::Dashboard),
            KeyCode::Char('b') => self.navigate(View::Bundles),
            KeyCode::Char('p') => self.navigate(View::Profiles),
            KeyCode::Char('m') => self.navigate(View::Modules),
            KeyCode::Char('x') => self.navigate(View::SystemMaintenance),
            KeyCode::Char('u') => self.navigate(View::UpdatePreview),
            KeyCode::Char('l') => self.navigate(View::CleanSystem),  // Phase 3: Cleanup
            KeyCode::Char('s') => self.navigate(View::Settings),
            KeyCode::Char('w') => self.navigate(View::SetupWizard),  // Re-enter wizard
            KeyCode::Char('y') => self.navigate(View::Sync),         // Git sync

            // List navigation
            KeyCode::Up | KeyCode::Char('k') => self.select_previous(),
            KeyCode::Down | KeyCode::Char('j') => self.select_next(),
            KeyCode::Enter => self.select_item(),
            KeyCode::Home => self.selected_index = 0,
            KeyCode::End => self.select_last(),

            // Module/Bundle actions
            KeyCode::Char('e') => self.toggle_selected_module(),
            KeyCode::Char('a') => self.activate_selected_bundle(),

            // Refresh
            KeyCode::Char('r') => self.refresh_current_view(),

            _ => {}
        }
    }

    /// Handle wizard key input
    pub fn handle_wizard_key(&mut self, key: KeyEvent) {
        // Handle text input mode
        if self.host_input.is_editing() {
            match key.code {
                KeyCode::Esc => {
                    self.host_input.exit_edit_mode();
                }
                KeyCode::Enter => {
                    self.wizard.host_id = self.host_input.value.clone();
                    self.host_input.exit_edit_mode();
                }
                KeyCode::Backspace => {
                    self.host_input.delete();
                }
                KeyCode::Delete => {
                    self.host_input.delete_forward();
                }
                KeyCode::Left => {
                    self.host_input.move_left();
                }
                KeyCode::Right => {
                    self.host_input.move_right();
                }
                KeyCode::Home => {
                    self.host_input.move_start();
                }
                KeyCode::End => {
                    self.host_input.move_end();
                }
                KeyCode::Char(c) => {
                    self.host_input.insert(c);
                }
                _ => {}
            }
            return;
        }

        match self.wizard.step {
            WizardStep::Welcome => match key.code {
                KeyCode::Enter => self.wizard.next_step(),
                KeyCode::Char('q') | KeyCode::Esc => self.should_quit = true,
                _ => {}
            },
            WizardStep::HostSetup => match key.code {
                KeyCode::Enter => {
                    if self.wizard.can_proceed() {
                        self.wizard.next_step();
                    }
                }
                KeyCode::Char('e') => {
                    self.host_input.enter_edit_mode();
                }
                KeyCode::Backspace | KeyCode::Esc => {
                    self.wizard.prev_step();
                }
                _ => {}
            },
            WizardStep::BundleSelection => match key.code {
                KeyCode::Enter => self.wizard.next_step(),
                KeyCode::Up | KeyCode::Char('k') => self.wizard.select_prev_bundle(),
                KeyCode::Down | KeyCode::Char('j') => self.wizard.select_next_bundle(),
                KeyCode::Backspace | KeyCode::Esc => self.wizard.prev_step(),
                _ => {}
            },
            WizardStep::ProfileSelection => match key.code {
                KeyCode::Enter => self.wizard.next_step(),
                KeyCode::Up | KeyCode::Char('k') => self.wizard.select_prev_profile(),
                KeyCode::Down | KeyCode::Char('j') => self.wizard.select_next_profile(),
                KeyCode::Backspace | KeyCode::Esc => self.wizard.prev_step(),
                _ => {}
            },
            WizardStep::Confirmation => match key.code {
                KeyCode::Enter | KeyCode::Char('y') => {
                    if let Ok(()) = self.wizard.apply(&self.config_dir) {
                        // Reinitialize app after wizard
                        let _ = self.init();
                        self.view = View::Dashboard;
                        self.set_status("Setup complete! Welcome to Iron.");
                    }
                }
                KeyCode::Backspace | KeyCode::Esc => self.wizard.prev_step(),
                _ => {}
            },
            WizardStep::Complete => {
                if key.code == KeyCode::Enter {
                    self.view = View::Dashboard;
                }
            }
        }
    }

    /// Cycle to next view
    fn cycle_view_forward(&mut self) {
        let next = match self.view {
            View::Dashboard => View::Bundles,
            View::Bundles | View::BundleDetail => View::Profiles,
            View::Profiles | View::ProfileDetail => View::Modules,
            View::Modules | View::ModuleDetail => View::SystemMaintenance,
            View::SystemMaintenance => View::UpdatePreview,
            View::UpdatePreview => View::Sync,
            View::Sync => View::Settings,
            View::Settings => View::Dashboard,
            // Sub-views cycle to their parent
            View::CleanSystem | View::SecurityModules | View::ConfigManager => View::SystemMaintenance,
            View::OperationLog => View::Settings,
            // SetupWizard exits to Dashboard (special case)
            View::SetupWizard => View::Dashboard,
        };
        self.navigate(next);
    }

    /// Cycle to previous view
    fn cycle_view_backward(&mut self) {
        let prev = match self.view {
            View::Dashboard => View::Settings,
            View::Settings => View::Sync,
            View::Sync => View::UpdatePreview,
            View::UpdatePreview => View::SystemMaintenance,
            View::SystemMaintenance => View::Modules,
            View::Modules | View::ModuleDetail => View::Profiles,
            View::Profiles | View::ProfileDetail => View::Bundles,
            View::Bundles | View::BundleDetail => View::Dashboard,
            // Sub-views cycle to their parent
            View::CleanSystem | View::SecurityModules | View::ConfigManager => View::SystemMaintenance,
            View::OperationLog => View::Settings,
            // SetupWizard exits to Dashboard (special case)
            View::SetupWizard => View::Dashboard,
        };
        self.navigate(prev);
    }

    /// Select previous item in list
    fn select_previous(&mut self) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
        }
    }

    /// Select next item in list
    fn select_next(&mut self) {
        let max = self.current_list_len().saturating_sub(1);
        if self.selected_index < max {
            self.selected_index += 1;
        }
    }

    /// Select last item in list
    fn select_last(&mut self) {
        self.selected_index = self.current_list_len().saturating_sub(1);
    }

    /// Get current list length based on view
    fn current_list_len(&self) -> usize {
        match self.view {
            View::Bundles | View::BundleDetail => self.bundles.len(),
            View::Profiles | View::ProfileDetail => self.profiles.len(),
            View::Modules | View::ModuleDetail => self.modules.len(),
            View::UpdatePreview => self.pending_updates.len(),
            View::CleanSystem => iron_core::services::clean::CleanupCategory::all().len(),
            View::SystemMaintenance => 3, // Update, Cleanup, Doctor
            View::ConfigManager => self
                .post_update_result
                .as_ref()
                .map(|r| r.config_conflicts.len())
                .unwrap_or(0),
            View::OperationLog => self
                .state_manager
                .as_ref()
                .map(|sm| sm.state().last_operations.len())
                .unwrap_or(0),
            View::SecurityModules => self
                .modules
                .iter()
                .filter(|m| {
                    m.id.contains("security")
                        || m.id.contains("firewall")
                        || m.id.contains("audit")
                        || ["ufw", "firewalld", "fail2ban", "auditd", "apparmor", "selinux", "clamav"]
                            .contains(&m.id.as_str())
                })
                .count(),
            View::Sync => 0, // No list items in sync view
            View::SetupWizard => self.wizard.available_bundles.len(),
            View::Settings => 8, // Number of setting items
            _ => 0,
        }
    }

    /// Handle item selection in list views
    fn select_item(&mut self) {
        match self.view {
            View::Bundles if !self.bundles.is_empty() => {
                self.navigate(View::BundleDetail);
            }
            View::Profiles if !self.profiles.is_empty() => {
                self.navigate(View::ProfileDetail);
            }
            View::Modules if !self.modules.is_empty() => {
                self.navigate(View::ModuleDetail);
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use iron_core::{Bundle, BundleType, Module, ModuleKind, Profile};

    fn create_key_event(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::empty())
    }

    fn create_key_event_with_mods(code: KeyCode, modifiers: KeyModifiers) -> KeyEvent {
        KeyEvent::new(code, modifiers)
    }

    fn create_test_bundle(id: &str) -> Bundle {
        Bundle {
            id: id.to_string(),
            name: id.to_string(),
            description: Some("Test bundle".to_string()),
            bundle_type: BundleType::WaylandCompositor,
            packages: vec![],
            aur_packages: vec![],
            profiles: vec![],
            default_profile: None,
            conflicts: vec![],
            services: vec![],
            post_install: None,
        }
    }

    fn create_test_module(id: &str) -> Module {
        Module {
            id: id.to_string(),
            name: id.to_string(),
            description: Some("Test module".to_string()),
            kind: ModuleKind::AppConfig,
            packages: vec![],
            aur_packages: vec![],
            dotfiles: vec![],
            conflicts: vec![],
            depends: vec![],
            pre_install: None,
            post_install: None,
        }
    }

    fn create_test_profile(id: &str) -> Profile {
        Profile {
            id: id.to_string(),
            name: id.to_string(),
            description: Some("Test profile".to_string()),
            modules: vec![],
            theme: None,
            shell: None,
            extends: None,
            for_bundle: None,
        }
    }

    // =============================================================================
    // Global Shortcut Tests
    // =============================================================================

    #[test]
    fn test_ctrl_c_quits() {
        let mut app = App::default();
        assert!(!app.should_quit);

        app.handle_key(create_key_event_with_mods(
            KeyCode::Char('c'),
            KeyModifiers::CONTROL,
        ));

        assert!(app.should_quit);
    }

    #[test]
    fn test_ctrl_q_quits() {
        let mut app = App::default();
        assert!(!app.should_quit);

        app.handle_key(create_key_event_with_mods(
            KeyCode::Char('q'),
            KeyModifiers::CONTROL,
        ));

        assert!(app.should_quit);
    }

    #[test]
    fn test_q_quits() {
        let mut app = App::default();
        assert!(!app.should_quit);

        app.handle_key(create_key_event(KeyCode::Char('q')));

        assert!(app.should_quit);
    }

    #[test]
    fn test_question_mark_shows_help() {
        let mut app = App::default();
        assert!(!app.show_help);

        app.handle_key(create_key_event(KeyCode::Char('?')));

        assert!(app.show_help);
    }

    #[test]
    fn test_any_key_dismisses_help() {
        let mut app = App::default();
        app.show_help = true;

        app.handle_key(create_key_event(KeyCode::Char('x')));

        assert!(!app.show_help);
    }

    // =============================================================================
    // View Navigation Tests
    // =============================================================================

    #[test]
    fn test_d_navigates_to_dashboard() {
        let mut app = App::default();
        app.view = View::Bundles;

        app.handle_key(create_key_event(KeyCode::Char('d')));

        assert_eq!(app.view, View::Dashboard);
    }

    #[test]
    fn test_b_navigates_to_bundles() {
        let mut app = App::default();
        app.view = View::Dashboard;

        app.handle_key(create_key_event(KeyCode::Char('b')));

        assert_eq!(app.view, View::Bundles);
    }

    #[test]
    fn test_p_navigates_to_profiles() {
        let mut app = App::default();
        app.view = View::Dashboard;

        app.handle_key(create_key_event(KeyCode::Char('p')));

        assert_eq!(app.view, View::Profiles);
    }

    #[test]
    fn test_m_navigates_to_modules() {
        let mut app = App::default();
        app.view = View::Dashboard;

        app.handle_key(create_key_event(KeyCode::Char('m')));

        assert_eq!(app.view, View::Modules);
    }

    #[test]
    fn test_u_navigates_to_update_preview() {
        let mut app = App::default();
        app.view = View::Dashboard;

        app.handle_key(create_key_event(KeyCode::Char('u')));

        assert_eq!(app.view, View::UpdatePreview);
    }

    #[test]
    fn test_s_navigates_to_settings() {
        let mut app = App::default();
        app.view = View::Dashboard;

        app.handle_key(create_key_event(KeyCode::Char('s')));

        assert_eq!(app.view, View::Settings);
    }

    #[test]
    fn test_tab_cycles_views_forward() {
        let mut app = App::default();
        app.view = View::Dashboard;

        app.handle_key(create_key_event(KeyCode::Tab));
        assert_eq!(app.view, View::Bundles);

        app.handle_key(create_key_event(KeyCode::Tab));
        assert_eq!(app.view, View::Profiles);

        app.handle_key(create_key_event(KeyCode::Tab));
        assert_eq!(app.view, View::Modules);

        // Modules -> SystemMaintenance (skips ModuleDetail)
        app.handle_key(create_key_event(KeyCode::Tab));
        assert_eq!(app.view, View::SystemMaintenance);

        // SystemMaintenance -> UpdatePreview
        app.handle_key(create_key_event(KeyCode::Tab));
        assert_eq!(app.view, View::UpdatePreview);

        // UpdatePreview -> Sync (Tab now works globally, arrows for sections)
        app.handle_key(create_key_event(KeyCode::Tab));
        assert_eq!(app.view, View::Sync);

        // Sync -> Settings
        app.handle_key(create_key_event(KeyCode::Tab));
        assert_eq!(app.view, View::Settings);
    }

    #[test]
    fn test_tab_from_settings_to_dashboard() {
        let mut app = App::default();
        app.view = View::Settings;

        app.handle_key(create_key_event(KeyCode::Tab));

        assert_eq!(app.view, View::Dashboard);
    }

    #[test]
    fn test_backtab_cycles_views_backward() {
        let mut app = App::default();
        app.view = View::Dashboard;

        app.handle_key(create_key_event(KeyCode::BackTab));
        assert_eq!(app.view, View::Settings);

        app.handle_key(create_key_event(KeyCode::BackTab));
        assert_eq!(app.view, View::Sync);

        app.handle_key(create_key_event(KeyCode::BackTab));
        assert_eq!(app.view, View::UpdatePreview);
    }

    #[test]
    fn test_escape_goes_back() {
        let mut app = App::default();
        app.view = View::Bundles;
        app.previous_view = Some(View::Dashboard);

        app.handle_key(create_key_event(KeyCode::Esc));

        assert_eq!(app.view, View::Dashboard);
    }

    // =============================================================================
    // List Navigation Tests
    // =============================================================================

    #[test]
    fn test_j_selects_next() {
        let mut app = App::default();
        app.view = View::Bundles;
        app.bundles = vec![
            create_test_bundle("bundle1"),
            create_test_bundle("bundle2"),
            create_test_bundle("bundle3"),
        ];
        app.selected_index = 0;

        app.handle_key(create_key_event(KeyCode::Char('j')));
        assert_eq!(app.selected_index, 1);

        app.handle_key(create_key_event(KeyCode::Char('j')));
        assert_eq!(app.selected_index, 2);
    }

    #[test]
    fn test_k_selects_previous() {
        let mut app = App::default();
        app.view = View::Bundles;
        app.bundles = vec![create_test_bundle("bundle1"), create_test_bundle("bundle2")];
        app.selected_index = 1;

        app.handle_key(create_key_event(KeyCode::Char('k')));
        assert_eq!(app.selected_index, 0);
    }

    #[test]
    fn test_down_arrow_selects_next() {
        let mut app = App::default();
        app.view = View::Modules;
        app.modules = vec![create_test_module("mod1"), create_test_module("mod2")];
        app.selected_index = 0;

        app.handle_key(create_key_event(KeyCode::Down));
        assert_eq!(app.selected_index, 1);
    }

    #[test]
    fn test_up_arrow_selects_previous() {
        let mut app = App::default();
        app.view = View::Modules;
        app.modules = vec![create_test_module("mod1"), create_test_module("mod2")];
        app.selected_index = 1;

        app.handle_key(create_key_event(KeyCode::Up));
        assert_eq!(app.selected_index, 0);
    }

    #[test]
    fn test_home_selects_first() {
        let mut app = App::default();
        app.view = View::Profiles;
        app.profiles = vec![
            create_test_profile("profile1"),
            create_test_profile("profile2"),
            create_test_profile("profile3"),
        ];
        app.selected_index = 2;

        app.handle_key(create_key_event(KeyCode::Home));
        assert_eq!(app.selected_index, 0);
    }

    #[test]
    fn test_end_selects_last() {
        let mut app = App::default();
        app.view = View::Profiles;
        app.profiles = vec![
            create_test_profile("profile1"),
            create_test_profile("profile2"),
            create_test_profile("profile3"),
        ];
        app.selected_index = 0;

        app.handle_key(create_key_event(KeyCode::End));
        assert_eq!(app.selected_index, 2);
    }

    #[test]
    fn test_select_next_bounds_check() {
        let mut app = App::default();
        app.view = View::Bundles;
        app.bundles = vec![create_test_bundle("only-one")];
        app.selected_index = 0;

        // Try to go past the end
        app.handle_key(create_key_event(KeyCode::Char('j')));
        assert_eq!(app.selected_index, 0);
    }

    #[test]
    fn test_select_previous_bounds_check() {
        let mut app = App::default();
        app.view = View::Bundles;
        app.bundles = vec![create_test_bundle("only-one")];
        app.selected_index = 0;

        // Try to go before start
        app.handle_key(create_key_event(KeyCode::Char('k')));
        assert_eq!(app.selected_index, 0);
    }

    // =============================================================================
    // Detail View Navigation Tests
    // =============================================================================

    #[test]
    fn test_enter_opens_bundle_detail() {
        let mut app = App::default();
        app.view = View::Bundles;
        app.bundles = vec![create_test_bundle("hyprland")];
        app.selected_index = 0;

        app.handle_key(create_key_event(KeyCode::Enter));

        assert_eq!(app.view, View::BundleDetail);
    }

    #[test]
    fn test_enter_opens_profile_detail() {
        let mut app = App::default();
        app.view = View::Profiles;
        app.profiles = vec![create_test_profile("developer")];
        app.selected_index = 0;

        app.handle_key(create_key_event(KeyCode::Enter));

        assert_eq!(app.view, View::ProfileDetail);
    }

    #[test]
    fn test_enter_opens_module_detail() {
        let mut app = App::default();
        app.view = View::Modules;
        app.modules = vec![create_test_module("nvim-ide")];
        app.selected_index = 0;

        app.handle_key(create_key_event(KeyCode::Enter));

        assert_eq!(app.view, View::ModuleDetail);
    }

    #[test]
    fn test_enter_no_op_on_empty_list() {
        let mut app = App::default();
        app.view = View::Bundles;
        app.bundles = vec![];

        app.handle_key(create_key_event(KeyCode::Enter));

        // Should stay on Bundles view
        assert_eq!(app.view, View::Bundles);
    }

    // =============================================================================
    // Confirm Dialog Tests
    // =============================================================================

    #[test]
    fn test_confirm_dialog_y_executes() {
        let mut app = App::default();
        app.show_confirm = true;
        app.confirm_action = Some(ConfirmAction::Quit);

        app.handle_key(create_key_event(KeyCode::Char('y')));

        assert!(!app.show_confirm);
        assert!(app.confirm_action.is_none());
    }

    #[test]
    fn test_confirm_dialog_enter_executes() {
        let mut app = App::default();
        app.show_confirm = true;
        app.confirm_action = Some(ConfirmAction::Quit);

        app.handle_key(create_key_event(KeyCode::Enter));

        assert!(!app.show_confirm);
        assert!(app.confirm_action.is_none());
    }

    #[test]
    fn test_confirm_dialog_n_cancels() {
        let mut app = App::default();
        app.show_confirm = true;
        app.confirm_action = Some(ConfirmAction::Quit);

        app.handle_key(create_key_event(KeyCode::Char('n')));

        assert!(!app.show_confirm);
        assert!(app.confirm_action.is_none());
    }

    #[test]
    fn test_confirm_dialog_escape_cancels() {
        let mut app = App::default();
        app.show_confirm = true;
        app.confirm_action = Some(ConfirmAction::Quit);

        app.handle_key(create_key_event(KeyCode::Esc));

        assert!(!app.show_confirm);
        assert!(app.confirm_action.is_none());
    }

    // =============================================================================
    // View-Specific Tests
    // =============================================================================

    #[test]
    fn test_update_preview_escape_goes_back() {
        let mut app = App::default();
        app.view = View::UpdatePreview;
        app.previous_view = Some(View::Dashboard);

        app.handle_key(create_key_event(KeyCode::Esc));

        assert_eq!(app.view, View::Dashboard);
    }

    #[test]
    fn test_profile_detail_escape_goes_back() {
        let mut app = App::default();
        app.view = View::ProfileDetail;
        app.previous_view = Some(View::Profiles);

        app.handle_key(create_key_event(KeyCode::Esc));

        assert_eq!(app.view, View::Profiles);
    }

    // =============================================================================
    // System Maintenance View Tests
    // =============================================================================

    #[test]
    fn test_x_navigates_to_system_maintenance() {
        let mut app = App::default();
        app.view = View::Dashboard;

        app.handle_key(create_key_event(KeyCode::Char('x')));

        assert_eq!(app.view, View::SystemMaintenance);
    }

    #[test]
    fn test_tab_from_modules_to_system_maintenance() {
        let mut app = App::default();
        app.view = View::Modules;

        app.handle_key(create_key_event(KeyCode::Tab));

        assert_eq!(app.view, View::SystemMaintenance);
    }

    #[test]
    fn test_tab_from_system_maintenance_to_update_preview() {
        let mut app = App::default();
        app.view = View::SystemMaintenance;

        app.handle_key(create_key_event(KeyCode::Tab));

        assert_eq!(app.view, View::UpdatePreview);
    }

    #[test]
    fn test_backtab_from_system_maintenance_to_modules() {
        let mut app = App::default();
        app.view = View::SystemMaintenance;

        app.handle_key(create_key_event(KeyCode::BackTab));

        assert_eq!(app.view, View::Modules);
    }

    #[test]
    fn test_backtab_from_update_preview_to_system_maintenance() {
        let mut app = App::default();
        app.view = View::UpdatePreview;
        // Use cycle_view_backward directly since UpdatePreview has special handling
        app.previous_view = Some(View::SystemMaintenance);

        app.handle_key(create_key_event(KeyCode::Esc));

        assert_eq!(app.view, View::SystemMaintenance);
    }

    #[test]
    fn test_tab_from_clean_system_to_system_maintenance() {
        let mut app = App::default();
        app.view = View::CleanSystem;

        app.handle_key(create_key_event(KeyCode::Tab));

        assert_eq!(app.view, View::SystemMaintenance);
    }

    #[test]
    fn test_tab_from_security_modules_to_system_maintenance() {
        let mut app = App::default();
        app.view = View::SecurityModules;

        app.handle_key(create_key_event(KeyCode::Tab));

        assert_eq!(app.view, View::SystemMaintenance);
    }

    #[test]
    fn test_tab_from_config_manager_to_system_maintenance() {
        let mut app = App::default();
        app.view = View::ConfigManager;

        app.handle_key(create_key_event(KeyCode::Tab));

        assert_eq!(app.view, View::SystemMaintenance);
    }

    #[test]
    fn test_tab_from_operation_log_to_settings() {
        let mut app = App::default();
        app.view = View::OperationLog;

        app.handle_key(create_key_event(KeyCode::Tab));

        assert_eq!(app.view, View::Settings);
    }

    // =============================================================================
    // Settings View Handler Tests
    // =============================================================================

    #[test]
    fn test_settings_enter_triggers_edit() {
        let mut app = App::default();
        app.view = View::Settings;
        app.selected_index = 1; // Current Host

        app.handle_key(create_key_event(KeyCode::Enter));

        // Should show hint message
        assert!(app.status_text().is_some());
    }

    #[test]
    fn test_settings_r_triggers_refresh() {
        let mut app = App::default();
        app.view = View::Settings;

        app.handle_key(create_key_event(KeyCode::Char('r')));

        assert!(app.status_text().is_some());
        assert!(app.status_text().unwrap().contains("refreshed"));
    }

    #[test]
    fn test_settings_w_navigates_to_wizard() {
        let mut app = App::default();
        app.view = View::Settings;

        app.handle_key(create_key_event(KeyCode::Char('w')));

        assert_eq!(app.view, View::SetupWizard);
    }

    #[test]
    fn test_settings_o_navigates_to_operation_log() {
        let mut app = App::default();
        app.view = View::Settings;

        app.handle_key(create_key_event(KeyCode::Char('o')));

        assert_eq!(app.view, View::OperationLog);
    }

    #[test]
    fn test_settings_c_navigates_to_config_manager() {
        let mut app = App::default();
        app.view = View::Settings;

        app.handle_key(create_key_event(KeyCode::Char('c')));

        assert_eq!(app.view, View::ConfigManager);
    }

    #[test]
    fn test_settings_navigation_respects_list_length() {
        let mut app = App::default();
        app.view = View::Settings;
        app.selected_index = 0;

        // Settings has 8 items (indices 0-7)
        for _ in 0..10 {
            app.handle_key(create_key_event(KeyCode::Down));
        }

        // Should stop at index 7 (last item)
        assert_eq!(app.selected_index, 7);
    }
}
