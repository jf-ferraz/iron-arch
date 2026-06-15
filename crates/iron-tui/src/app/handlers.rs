//! Key handling for Iron TUI
//!
//! Contains all keyboard input handling logic.

use super::{App, ConfirmAction, ConfirmStyle, View};
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

        // Divergence popup (S1-P3-002)
        if self.show_divergence_popup {
            match key.code {
                KeyCode::Esc => {
                    self.show_divergence_popup = false;
                }
                KeyCode::Char('j') | KeyCode::Down => {
                    let max = self.diverged_modules.len().saturating_sub(1);
                    if self.divergence_selected < max {
                        self.divergence_selected += 1;
                    }
                }
                KeyCode::Char('k') | KeyCode::Up => {
                    self.divergence_selected = self.divergence_selected.saturating_sub(1);
                }
                KeyCode::Char('r') => {
                    if let Some(module_id) = self.diverged_modules.get(self.divergence_selected) {
                        self.set_status(format!(
                            "Restore '{}': run `iron sync pull` to reset managed files",
                            module_id
                        ));
                    }
                    self.show_divergence_popup = false;
                }
                KeyCode::Char('a') => {
                    if let Some(module_id) = self.diverged_modules.get(self.divergence_selected) {
                        self.set_status(format!(
                            "Accept '{}': run `iron sync push` to commit current changes",
                            module_id
                        ));
                    }
                    self.show_divergence_popup = false;
                }
                KeyCode::Char('d') => {
                    if let Some(module_id) = self.diverged_modules.get(self.divergence_selected) {
                        self.set_status(format!(
                            "Diff '{}': run `git diff` in config dir to view changes",
                            module_id
                        ));
                    }
                    self.show_divergence_popup = false;
                }
                _ => {}
            }
            return;
        }

        // Help overlay
        if self.show_help {
            self.show_help = false;
            return;
        }

        // Confirm dialog (risk-differentiated)
        if self.show_confirm {
            match self.confirm_style {
                ConfirmStyle::TypedConfirmation => {
                    // Critical risk: user must type "CONFIRM" and press Enter
                    match key.code {
                        KeyCode::Char(c) => {
                            self.confirm_typed_input.push(c);
                        }
                        KeyCode::Backspace => {
                            self.confirm_typed_input.pop();
                        }
                        KeyCode::Enter => {
                            if self.confirm_typed_input == "CONFIRM" {
                                self.execute_confirm_action();
                                self.show_confirm = false;
                                self.confirm_action = None;
                                self.confirm_typed_input.clear();
                            }
                            // If text doesn't match, ignore Enter (don't dismiss)
                        }
                        KeyCode::Esc => {
                            self.show_confirm = false;
                            self.confirm_action = None;
                            self.confirm_typed_input.clear();
                        }
                        _ => {}
                    }
                }
                ConfirmStyle::EnhancedWarning | ConfirmStyle::Simple => {
                    // High/Medium/Low risk: Y/N confirmation
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
                }
            }
            return;
        }

        // Wizard handling
        if self.view == View::SetupWizard {
            self.handle_wizard_key(key);
            return;
        }

        if self.view == View::InstallWizard {
            self.handle_install_wizard_key(key);
            return;
        }

        // Profile Builder handling
        if self.view == View::ProfileBuilder {
            self.handle_profile_builder_key(key);
            return;
        }

        // Module Creator handling
        if self.view == View::ModuleCreator {
            self.handle_module_creator_key(key);
            return;
        }

        // D-004: GPG key input mode
        if self.gpg_key_input.is_editing() {
            match key.code {
                KeyCode::Esc => {
                    self.gpg_key_input.exit_edit_mode();
                    self.set_status("GPG key input cancelled");
                }
                KeyCode::Enter => {
                    let key_id = self.gpg_key_input.value.clone();
                    self.gpg_key_input.exit_edit_mode();
                    if !key_id.is_empty() {
                        self.secrets_add_gpg_key(&key_id);
                    }
                }
                KeyCode::Backspace => self.gpg_key_input.delete(),
                KeyCode::Char(c) => self.gpg_key_input.insert(c),
                _ => {}
            }
            return;
        }

        // D-003: Import path input mode
        if self.import_path_input.is_editing() {
            match key.code {
                KeyCode::Esc => {
                    self.import_path_input.exit_edit_mode();
                    self.set_status("Import cancelled");
                }
                KeyCode::Enter => {
                    let path = self.import_path_input.value.clone();
                    self.import_path_input.exit_edit_mode();
                    if !path.is_empty() {
                        self.recovery_import(&path);
                    }
                }
                KeyCode::Backspace => self.import_path_input.delete(),
                KeyCode::Char(c) => self.import_path_input.insert(c),
                _ => {}
            }
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
            // CleanupPreview view handlers
            View::CleanupPreview => match key.code {
                // Execute cleanup from preview
                KeyCode::Char('c') => {
                    if !self.cleanup_categories.is_empty() {
                        self.request_confirm(ConfirmAction::RunCleanup);
                    }
                    true
                }
                _ => false,
            },
            // CleanupResults — read-only, Esc handled globally
            View::CleanupResults => false,
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
                    self.navigate(View::Doctor);
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
                        2 => self.navigate(View::Doctor),
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
                    self.set_info("Diff viewer: use 'diff' command on the files shown");
                    true
                }
                KeyCode::Char('r') => {
                    self.refresh_config_conflicts();
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
                    self.cycle_operation_filter();
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
                    // Install = same as enable for now
                    self.toggle_selected_module();
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
                    self.init_wizard();
                    true
                }
                KeyCode::Char('s') => {
                    self.navigate(View::SystemScan);
                    true
                }
                _ => false,
            },
            // Sync view handlers
            View::Sync => match key.code {
                KeyCode::Char('p') => {
                    // D-006: Confirm before push
                    self.request_confirm(ConfirmAction::SyncPush);
                    true
                }
                KeyCode::Char('f') => {
                    // D-006: Confirm before pull
                    self.request_confirm(ConfirmAction::SyncPull);
                    true
                }
                KeyCode::Char('s') => {
                    self.refresh_sync_status();
                    true
                }
                KeyCode::Char('l') if !self.sync_conflicts.is_empty() => {
                    self.resolve_conflicts_keep_local();
                    true
                }
                KeyCode::Char('r') if !self.sync_conflicts.is_empty() => {
                    self.resolve_conflicts_keep_remote();
                    true
                }
                _ => false,
            },
            // Secrets view handlers
            View::Secrets => match key.code {
                KeyCode::Char('i') => {
                    self.secrets_init();
                    true
                }
                KeyCode::Char('u') => {
                    self.secrets_unlock();
                    true
                }
                KeyCode::Char('l') => {
                    self.secrets_lock();
                    true
                }
                KeyCode::Char('r') => {
                    self.refresh_secrets();
                    self.set_status("Secrets status refreshed");
                    true
                }
                KeyCode::Char('a') => {
                    // D-004: Add GPG key — prompt via input mode
                    self.gpg_key_input = crate::wizard::TextInput::new("");
                    self.gpg_key_input.enter_edit_mode();
                    self.set_info("Enter GPG key ID, then press Enter:");
                    true
                }
                _ => false,
            },
            // Recovery view handlers
            View::Recovery => match key.code {
                KeyCode::Char('e') => {
                    self.recovery_export();
                    true
                }
                KeyCode::Char('g') => {
                    self.recovery_generate_script();
                    true
                }
                KeyCode::Char('s') => {
                    self.recovery_create_snapshot();
                    true
                }
                KeyCode::Char('i') => {
                    // D-003: Import recovery — prompt for file path
                    self.import_path_input = crate::wizard::TextInput::new("");
                    self.import_path_input.enter_edit_mode();
                    self.set_info("Enter import file path, then press Enter:");
                    true
                }
                _ => false,
            },
            // SystemScan view handlers (S1-P1.5-004)
            View::SystemScan => match key.code {
                KeyCode::Enter => {
                    self.navigate(View::Dashboard);
                    true
                }
                KeyCode::Char('r') => {
                    self.rescan_system();
                    true
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    self.scan_scroll = self.scan_scroll.saturating_sub(1);
                    true
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    self.scan_scroll = self.scan_scroll.saturating_add(1);
                    true
                }
                _ => false,
            },
            // Doctor view handlers (S1-P7-003)
            View::Doctor => match key.code {
                KeyCode::Char('r') => {
                    self.refresh_current_view();
                    true
                }
                _ => false,
            },
            // HostSelection view handlers (S1-P2-001)
            View::HostSelection => match key.code {
                KeyCode::Enter => {
                    if let Some(host) = self.discovered_hosts.get(self.selected_index) {
                        let host_id = host.id.clone();
                        if let Some(ref sm) = self.state_manager {
                            let _ = sm.set_current_host(&host_id);
                            let _ = self.init();
                            self.set_status(format!("Switched to host '{}'", host_id));
                        }
                        self.navigate(View::Dashboard);
                    }
                    true
                }
                KeyCode::Char('c') => {
                    self.navigate(View::SetupWizard);
                    self.init_wizard();
                    true
                }
                _ => false,
            },
            // Dashboard divergence tooltip (S1-P3-002)
            View::Dashboard => match key.code {
                KeyCode::Char('i') if self.diverged_count() > 0 => {
                    self.show_divergence_popup = true;
                    self.divergence_selected = 0;
                    true
                }
                // F1-010: Apply from Dashboard
                KeyCode::Char('a') => {
                    self.navigate(View::Apply);
                    true
                }
                // F1-018: Drift detail from Dashboard
                KeyCode::Char('D') => {
                    self.navigate(View::DriftDetail);
                    true
                }
                // F2-007: Snapshot timeline from Dashboard
                KeyCode::Char('t') => {
                    self.navigate(View::Snapshots);
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

            // Navigation ('d' is context-sensitive: deactivate in BundleDetail, else Dashboard)
            KeyCode::Char('d') => {
                if matches!(self.view, View::BundleDetail) {
                    // B-006: Deactivate bundle without switching
                    if let Some(bundle) = self.selected_bundle() {
                        let id = bundle.id.clone();
                        self.request_confirm(ConfirmAction::RemoveBundle(id));
                    }
                } else {
                    self.navigate(View::Dashboard);
                }
            }
            KeyCode::Char('b') => self.navigate(View::Bundles),
            KeyCode::Char('p') => self.navigate(View::Profiles),
            KeyCode::Char('m') => self.navigate(View::Modules),
            KeyCode::Char('x') => self.navigate(View::SystemMaintenance),
            KeyCode::Char('u') => self.navigate(View::UpdatePreview),
            KeyCode::Char('l') => self.navigate(View::CleanSystem), // Phase 3: Cleanup
            KeyCode::Char('s') => self.navigate(View::Settings),
            KeyCode::Char('w') => {
                self.navigate(View::SetupWizard);
                self.init_wizard();
            }
            KeyCode::Char('y') => self.navigate(View::Sync), // Git sync
            KeyCode::Char('S') => self.navigate(View::Secrets), // Shift+S = Secrets
            KeyCode::Char('R') => self.navigate(View::Recovery), // Shift+R = Recovery
            KeyCode::Char('H') => {
                self.load_hosts();
                self.navigate(View::HostSelection);
            }

            // List navigation
            KeyCode::Up | KeyCode::Char('k') => self.select_previous(),
            KeyCode::Down | KeyCode::Char('j') => self.select_next(),
            KeyCode::Enter => self.select_item(),
            KeyCode::Home => self.selected_index = 0,
            KeyCode::End => self.select_last(),

            // Module/Bundle actions (scoped to relevant views)
            KeyCode::Char('e') => {
                if matches!(
                    self.view,
                    View::Modules | View::ModuleDetail | View::SecurityModules
                ) {
                    self.toggle_selected_module();
                }
            }
            KeyCode::Char('a') => {
                if matches!(self.view, View::Bundles | View::BundleDetail) {
                    self.activate_selected_bundle();
                }
            }

            // New item wizard (context-sensitive)
            KeyCode::Char('n') => {
                if matches!(self.view, View::Profiles | View::ProfileDetail) {
                    self.open_profile_builder();
                } else if matches!(self.view, View::Modules | View::ModuleDetail) {
                    self.open_module_creator();
                }
            }

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
                    if let Ok(()) = self.wizard.apply(
                        &self.config_dir,
                        self.package_manager.clone(),
                        self.service_manager.clone(),
                    ) {
                        // Reinitialize app after wizard
                        let _ = self.init();
                        // Run system scan and transition to scan view (S1-P1.5-004)
                        self.run_post_wizard_scan();
                        self.view = View::SystemScan;
                        self.set_status("Setup complete! Scanning system...");
                    }
                }
                KeyCode::Backspace | KeyCode::Esc => self.wizard.prev_step(),
                _ => {}
            },
            WizardStep::Complete => {
                if key.code == KeyCode::Enter {
                    // Also route through scan view on late completion
                    self.run_post_wizard_scan();
                    self.view = View::SystemScan;
                }
            }
        }
    }

    /// Handle keys for the Profile Builder wizard
    pub fn handle_profile_builder_key(&mut self, key: crossterm::event::KeyEvent) {
        use crossterm::event::KeyCode;
        match self.profile_builder_step {
            // Step 0: name/description input
            0 => match key.code {
                KeyCode::Esc => {
                    self.navigate(View::Profiles);
                }
                KeyCode::Tab => {
                    self.profile_builder_editing_desc = !self.profile_builder_editing_desc;
                }
                KeyCode::Enter => {
                    if self.profile_builder_name.trim().is_empty() {
                        self.set_error("Profile name is required");
                    } else {
                        self.profile_builder_step = 1;
                    }
                }
                KeyCode::Backspace => {
                    if self.profile_builder_editing_desc {
                        self.profile_builder_description.pop();
                    } else {
                        self.profile_builder_name.pop();
                    }
                }
                KeyCode::Char(c) => {
                    if self.profile_builder_editing_desc {
                        self.profile_builder_description.push(c);
                    } else {
                        // D-010: Enforce [a-z0-9][a-z0-9-]* pattern
                        let c_lower = c.to_ascii_lowercase();
                        if c_lower.is_ascii_lowercase()
                            || c_lower.is_ascii_digit()
                            || (c == '-' && !self.profile_builder_name.is_empty())
                        {
                            self.profile_builder_name.push(c_lower);
                        }
                    }
                }
                _ => {}
            },
            // Step 1: module checklist
            1 => match key.code {
                KeyCode::Esc => {
                    self.profile_builder_step = 0;
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    if self.profile_builder_module_cursor > 0 {
                        self.profile_builder_module_cursor -= 1;
                    }
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    if self.profile_builder_module_cursor + 1 < self.modules.len() {
                        self.profile_builder_module_cursor += 1;
                    }
                }
                KeyCode::Char(' ') => {
                    if let Some(module) = self.modules.get(self.profile_builder_module_cursor) {
                        let id = module.id.clone();
                        if let Some(pos) = self
                            .profile_builder_selected_modules
                            .iter()
                            .position(|m| m == &id)
                        {
                            self.profile_builder_selected_modules.remove(pos);
                        } else {
                            // F-009: Check for conflicts with already-selected modules
                            let mut conflict_warnings: Vec<String> = Vec::new();
                            for selected_id in &self.profile_builder_selected_modules {
                                // Check if the new module conflicts with selected
                                if module.conflicts_with(selected_id) {
                                    conflict_warnings
                                        .push(format!("'{}' conflicts with '{}'", id, selected_id));
                                }
                                // Check if selected module conflicts with the new one
                                if let Some(sel_mod) =
                                    self.modules.iter().find(|m| m.id == *selected_id)
                                    && sel_mod.conflicts_with(&id)
                                {
                                    conflict_warnings
                                        .push(format!("'{}' conflicts with '{}'", selected_id, id));
                                }
                            }
                            // Still add (user may choose to override), but show warning
                            self.profile_builder_selected_modules.push(id.clone());
                            if !conflict_warnings.is_empty() {
                                // Deduplicate warnings
                                conflict_warnings.sort();
                                conflict_warnings.dedup();
                                self.set_error(format!(
                                    "Warning: {}",
                                    conflict_warnings.join("; ")
                                ));
                            } else {
                                // D-011: Auto-suggest missing dependencies
                                let missing_deps: Vec<String> = module
                                    .depends
                                    .iter()
                                    .filter(|dep| {
                                        !self.profile_builder_selected_modules.contains(dep)
                                    })
                                    .cloned()
                                    .collect();
                                if !missing_deps.is_empty() {
                                    self.set_status(format!(
                                        "Tip: '{}' depends on: {}",
                                        id,
                                        missing_deps.join(", ")
                                    ));
                                }
                            }
                        }
                    }
                }
                KeyCode::Enter => {
                    self.profile_builder_step = 2;
                }
                _ => {}
            },
            // Step 2: preview
            _ => match key.code {
                KeyCode::Esc => {
                    self.profile_builder_step = 1;
                }
                KeyCode::Enter => {
                    self.create_profile_from_builder();
                }
                _ => {}
            },
        }
    }

    /// Handle keys for the Module Creator wizard
    pub fn handle_module_creator_key(&mut self, key: crossterm::event::KeyEvent) {
        use crossterm::event::KeyCode;
        match self.module_creator_step {
            0 => match key.code {
                KeyCode::Esc => {
                    self.navigate(View::Modules);
                }
                KeyCode::Tab => {
                    self.module_creator_active_field = (self.module_creator_active_field + 1) % 4;
                }
                KeyCode::Enter => {
                    if self.module_creator_name.trim().is_empty() {
                        self.set_error("Module ID is required");
                    } else {
                        self.module_creator_step = 1; // D-012: go to dotfiles step
                    }
                }
                KeyCode::Backspace => match self.module_creator_active_field {
                    0 => {
                        self.module_creator_name.pop();
                    }
                    1 => {
                        self.module_creator_description.pop();
                    }
                    _ => {
                        self.module_creator_packages.pop();
                    }
                },
                KeyCode::Char(c) => match self.module_creator_active_field {
                    0 => {
                        // D-010: Enforce [a-z0-9][a-z0-9-]* pattern
                        let c_lower = c.to_ascii_lowercase();
                        if c_lower.is_ascii_lowercase()
                            || c_lower.is_ascii_digit()
                            || (c == '-' && !self.module_creator_name.is_empty())
                        {
                            self.module_creator_name.push(c_lower);
                        }
                    }
                    1 => {
                        self.module_creator_description.push(c);
                    }
                    2 => {
                        self.module_creator_packages.push(c);
                    }
                    _ => {
                        // F-010: Kind field — use left/right or j/k to cycle
                        // ModuleKind has 6 variants
                        match c {
                            'l' | 'j' => {
                                self.module_creator_kind_index =
                                    (self.module_creator_kind_index + 1) % 6;
                            }
                            'h' | 'k' => {
                                self.module_creator_kind_index =
                                    (self.module_creator_kind_index + 5) % 6;
                            }
                            _ => {}
                        }
                    }
                },
                // F-010: Left/Right to cycle kind when kind field is active
                KeyCode::Left => {
                    if self.module_creator_active_field == 3 {
                        self.module_creator_kind_index = (self.module_creator_kind_index + 5) % 6;
                    }
                }
                KeyCode::Right => {
                    if self.module_creator_active_field == 3 {
                        self.module_creator_kind_index = (self.module_creator_kind_index + 1) % 6;
                    }
                }
                _ => {}
            },
            // D-012 / F0-011: Step 1 — Dotfile mapping editor
            1 => match key.code {
                KeyCode::Esc => {
                    self.module_creator_step = 0;
                }
                KeyCode::Tab => {
                    self.module_creator_dotfile_field = 1 - self.module_creator_dotfile_field;
                }
                KeyCode::Enter => {
                    let src = self.module_creator_dotfile_source.trim().to_string();
                    let tgt = self.module_creator_dotfile_target.trim().to_string();
                    if !src.is_empty() && !tgt.is_empty() {
                        // Both fields filled → add the mapping, clear inputs, stay on step 1
                        self.module_creator_dotfiles.push((src, tgt));
                        self.module_creator_dotfile_source.clear();
                        self.module_creator_dotfile_target.clear();
                        self.module_creator_dotfile_field = 0;
                    } else {
                        // Empty inputs → advance to preview (step 2)
                        self.module_creator_step = 2;
                    }
                }
                KeyCode::Backspace => {
                    // Delete character from active field; if empty, pop last dotfile entry
                    match self.module_creator_dotfile_field {
                        0 => {
                            if self.module_creator_dotfile_source.is_empty() {
                                self.module_creator_dotfiles.pop();
                            } else {
                                self.module_creator_dotfile_source.pop();
                            }
                        }
                        _ => {
                            if self.module_creator_dotfile_target.is_empty() {
                                self.module_creator_dotfiles.pop();
                            } else {
                                self.module_creator_dotfile_target.pop();
                            }
                        }
                    }
                }
                KeyCode::Char(c) => match self.module_creator_dotfile_field {
                    0 => self.module_creator_dotfile_source.push(c),
                    _ => self.module_creator_dotfile_target.push(c),
                },
                _ => {}
            },
            // Step 2 — Preview
            _ => match key.code {
                KeyCode::Esc => {
                    self.module_creator_step = 1;
                }
                KeyCode::Enter => {
                    self.create_module_from_creator();
                }
                _ => {}
            },
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
            View::CleanSystem
            | View::CleanupPreview
            | View::CleanupResults
            | View::SecurityModules
            | View::ConfigManager => View::SystemMaintenance,
            View::OperationLog => View::Settings,
            // SetupWizard exits to Dashboard (special case)
            View::SetupWizard => View::Dashboard,
            // New Phase-4 views cycle back to Dashboard
            View::Doctor | View::Secrets | View::Recovery | View::SystemScan => View::Dashboard,
            // HostSelection cycles back to Dashboard
            View::HostSelection => View::Dashboard,
            // Direct installer cycles back to Dashboard
            View::InstallWizard => View::Dashboard,
            // Wizard sub-views go back to their parent list
            View::ProfileBuilder => View::Profiles,
            View::ModuleCreator => View::Modules,
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
            View::CleanSystem
            | View::CleanupPreview
            | View::CleanupResults
            | View::SecurityModules
            | View::ConfigManager => View::SystemMaintenance,
            View::OperationLog => View::Settings,
            // SetupWizard exits to Dashboard (special case)
            View::SetupWizard => View::Dashboard,
            // New Phase-4 views cycle back to Dashboard
            View::Doctor | View::Secrets | View::Recovery | View::SystemScan => View::Dashboard,
            // HostSelection cycles back to Dashboard
            View::HostSelection => View::Dashboard,
            // Direct installer cycles back to Dashboard
            View::InstallWizard => View::Dashboard,
            // Wizard sub-views go back to their parent list
            View::ProfileBuilder => View::Profiles,
            View::ModuleCreator => View::Modules,
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
                        || [
                            "ufw",
                            "firewalld",
                            "fail2ban",
                            "auditd",
                            "apparmor",
                            "selinux",
                            "clamav",
                        ]
                        .contains(&m.id.as_str())
                })
                .count(),
            View::Sync => 0, // No list items in sync view
            View::SetupWizard => self.wizard.available_bundles.len(),
            View::Settings => 8, // Number of setting items
            View::HostSelection => self.discovered_hosts.len(),
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
            pre_uninstall: None,
            status_check: None,
            priority: None,
            requires_root: false,
            security_points: 0,
            hook_behavior: iron_core::module::HookBehavior::default(),
            dotfiles_sync: false,
            dotfiles_sync_target: None,
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
    // Risk-Differentiated Confirm Dialog Tests
    // =============================================================================

    #[test]
    fn test_request_confirm_sets_simple_for_low_risk_update() {
        use iron_core::RiskLevel;

        let mut app = App::default();
        app.update_risk = RiskLevel::Low;
        app.request_confirm(ConfirmAction::RunUpdate);

        assert_eq!(app.confirm_style, ConfirmStyle::Simple);
        assert!(app.show_confirm);
    }

    #[test]
    fn test_request_confirm_sets_enhanced_for_high_risk_update() {
        use iron_core::RiskLevel;

        let mut app = App::default();
        app.update_risk = RiskLevel::High;
        app.request_confirm(ConfirmAction::RunUpdate);

        assert_eq!(app.confirm_style, ConfirmStyle::EnhancedWarning);
        assert!(app.show_confirm);
    }

    #[test]
    fn test_request_confirm_sets_typed_for_critical_risk_update() {
        use iron_core::RiskLevel;

        let mut app = App::default();
        app.update_risk = RiskLevel::Critical;
        app.request_confirm(ConfirmAction::RunUpdate);

        assert_eq!(app.confirm_style, ConfirmStyle::TypedConfirmation);
        assert!(app.show_confirm);
    }

    #[test]
    fn test_request_confirm_sets_simple_for_non_update_actions() {
        let mut app = App::default();
        app.request_confirm(ConfirmAction::Quit);
        assert_eq!(app.confirm_style, ConfirmStyle::Simple);

        app.show_confirm = false;
        app.request_confirm(ConfirmAction::EnableModule("test".to_string()));
        assert_eq!(app.confirm_style, ConfirmStyle::Simple);
    }

    #[test]
    fn test_typed_confirm_accepts_correct_input() {
        let mut app = App::default();
        app.show_confirm = true;
        app.confirm_style = ConfirmStyle::TypedConfirmation;
        app.confirm_action = Some(ConfirmAction::Quit);

        // Type "CONFIRM"
        for ch in "CONFIRM".chars() {
            app.handle_key(create_key_event(KeyCode::Char(ch)));
        }
        assert!(app.show_confirm); // Still showing until Enter

        app.handle_key(create_key_event(KeyCode::Enter));

        assert!(!app.show_confirm);
        assert!(app.confirm_action.is_none());
    }

    #[test]
    fn test_typed_confirm_rejects_wrong_input() {
        let mut app = App::default();
        app.show_confirm = true;
        app.confirm_style = ConfirmStyle::TypedConfirmation;
        app.confirm_action = Some(ConfirmAction::Quit);

        // Type "WRONG"
        for ch in "WRONG".chars() {
            app.handle_key(create_key_event(KeyCode::Char(ch)));
        }
        app.handle_key(create_key_event(KeyCode::Enter));

        // Should still be showing - not dismissed
        assert!(app.show_confirm);
        assert!(app.confirm_action.is_some());
    }

    #[test]
    fn test_typed_confirm_escape_cancels() {
        let mut app = App::default();
        app.show_confirm = true;
        app.confirm_style = ConfirmStyle::TypedConfirmation;
        app.confirm_action = Some(ConfirmAction::Quit);

        // Type partial input then escape
        app.handle_key(create_key_event(KeyCode::Char('C')));
        app.handle_key(create_key_event(KeyCode::Char('O')));
        app.handle_key(create_key_event(KeyCode::Esc));

        assert!(!app.show_confirm);
        assert!(app.confirm_action.is_none());
        assert!(app.confirm_typed_input.is_empty());
    }

    #[test]
    fn test_typed_confirm_backspace_deletes() {
        let mut app = App::default();
        app.show_confirm = true;
        app.confirm_style = ConfirmStyle::TypedConfirmation;
        app.confirm_action = Some(ConfirmAction::Quit);

        app.handle_key(create_key_event(KeyCode::Char('C')));
        app.handle_key(create_key_event(KeyCode::Char('X'))); // wrong char
        app.handle_key(create_key_event(KeyCode::Backspace)); // delete it
        assert_eq!(app.confirm_typed_input, "C");

        // Continue with correct input
        for ch in "ONFIRM".chars() {
            app.handle_key(create_key_event(KeyCode::Char(ch)));
        }
        app.handle_key(create_key_event(KeyCode::Enter));

        assert!(!app.show_confirm);
    }

    #[test]
    fn test_enhanced_warning_y_executes() {
        let mut app = App::default();
        app.show_confirm = true;
        app.confirm_style = ConfirmStyle::EnhancedWarning;
        app.confirm_action = Some(ConfirmAction::Quit);

        app.handle_key(create_key_event(KeyCode::Char('y')));

        assert!(!app.show_confirm);
        assert!(app.confirm_action.is_none());
    }

    #[test]
    fn test_enhanced_warning_n_cancels() {
        let mut app = App::default();
        app.show_confirm = true;
        app.confirm_style = ConfirmStyle::EnhancedWarning;
        app.confirm_action = Some(ConfirmAction::Quit);

        app.handle_key(create_key_event(KeyCode::Char('n')));

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

    // =============================================================================
    // Secrets View Handler Tests
    // =============================================================================

    /// Helper: check that the handler produced some feedback (status or error)
    fn has_feedback(app: &App) -> bool {
        app.status_text().is_some() || app.error_text().is_some()
    }

    #[test]
    fn test_secrets_i_triggers_init() {
        let mut app = App::default();
        app.view = View::Secrets;

        app.handle_key(create_key_event(KeyCode::Char('i')));

        // Should stay on Secrets view and produce feedback
        assert_eq!(app.view, View::Secrets);
        assert!(has_feedback(&app));
    }

    #[test]
    fn test_secrets_u_triggers_unlock() {
        let mut app = App::default();
        app.view = View::Secrets;

        app.handle_key(create_key_event(KeyCode::Char('u')));

        assert_eq!(app.view, View::Secrets);
        assert!(has_feedback(&app));
    }

    #[test]
    fn test_secrets_l_triggers_lock() {
        let mut app = App::default();
        app.view = View::Secrets;

        app.handle_key(create_key_event(KeyCode::Char('l')));

        assert_eq!(app.view, View::Secrets);
        assert!(has_feedback(&app));
    }

    #[test]
    fn test_secrets_r_triggers_refresh() {
        let mut app = App::default();
        app.view = View::Secrets;

        app.handle_key(create_key_event(KeyCode::Char('r')));

        assert_eq!(app.view, View::Secrets);
        // 'r' always sets status via handler, regardless of service result
        assert!(app.status_text().is_some());
        assert!(app.status_text().unwrap().contains("refreshed"));
    }

    #[test]
    fn test_secrets_unhandled_key_stays_on_view() {
        let mut app = App::default();
        app.view = View::Secrets;

        // 'z' is not bound in secrets view or globally
        app.handle_key(create_key_event(KeyCode::Char('z')));

        assert_eq!(app.view, View::Secrets);
    }

    // =============================================================================
    // Recovery View Handler Tests
    // =============================================================================

    #[test]
    fn test_recovery_e_triggers_export() {
        let mut app = App::default();
        app.view = View::Recovery;

        app.handle_key(create_key_event(KeyCode::Char('e')));

        assert_eq!(app.view, View::Recovery);
        assert!(has_feedback(&app));
    }

    #[test]
    fn test_recovery_g_triggers_generate_script() {
        let mut app = App::default();
        app.view = View::Recovery;

        app.handle_key(create_key_event(KeyCode::Char('g')));

        assert_eq!(app.view, View::Recovery);
        assert!(has_feedback(&app));
    }

    #[test]
    fn test_recovery_s_triggers_snapshot() {
        let mut app = App::default();
        app.view = View::Recovery;

        app.handle_key(create_key_event(KeyCode::Char('s')));

        assert_eq!(app.view, View::Recovery);
        assert!(has_feedback(&app));
    }

    #[test]
    fn test_recovery_unhandled_key_stays_on_view() {
        let mut app = App::default();
        app.view = View::Recovery;

        // 'z' is not bound in recovery view or globally
        app.handle_key(create_key_event(KeyCode::Char('z')));

        assert_eq!(app.view, View::Recovery);
    }

    // Doctor view handler tests (S1-P7-003)

    #[test]
    fn test_doctor_r_refreshes_checks() {
        let tmp = tempfile::tempdir().unwrap();
        // Write a minimal state.json so StateManager::new succeeds with a host
        std::fs::write(
            tmp.path().join("state.json"),
            r#"{"current_host":"test-host","active_bundles":{},"active_profiles":{},"active_modules":[],"last_operations":[],"maintenance":{"last_update":null,"last_clean":null,"last_doctor":null,"last_snapshot":null,"last_sync":null}}"#,
        )
        .unwrap();
        // Create a hosts dir with a host file so load_hosts finds it
        let hosts_dir = tmp.path().join("hosts");
        std::fs::create_dir_all(&hosts_dir).unwrap();
        std::fs::write(
            hosts_dir.join("test-host.toml"),
            "[host]\nid = \"test-host\"\nname = \"Test\"\n",
        )
        .unwrap();

        let mut app = App::default();
        app.config_dir = tmp.path().to_path_buf();
        app.view = View::Doctor;

        app.handle_key(create_key_event(KeyCode::Char('r')));

        // Should stay on Doctor view with status feedback
        assert_eq!(app.view, View::Doctor);
        assert!(has_feedback(&app));
    }

    #[test]
    fn test_doctor_unhandled_key_stays_on_view() {
        let mut app = App::default();
        app.view = View::Doctor;

        // 'z' is not bound in doctor view or globally
        app.handle_key(create_key_event(KeyCode::Char('z')));

        assert_eq!(app.view, View::Doctor);
    }

    // SystemScan re-scan handler tests (S1-P1.5-005)

    #[test]
    fn test_system_scan_r_rescans() {
        let mut app = App::default();
        app.view = View::SystemScan;

        app.handle_key(create_key_event(KeyCode::Char('r')));

        // Should stay on SystemScan view with status feedback
        assert_eq!(app.view, View::SystemScan);
        assert!(has_feedback(&app));
    }

    #[test]
    fn test_settings_s_navigates_to_system_scan() {
        let mut app = App::default();
        app.view = View::Settings;

        app.handle_key(create_key_event(KeyCode::Char('s')));

        assert_eq!(app.view, View::SystemScan);
    }

    // HostSelection view handler tests (S1-P2-001)

    #[test]
    fn test_host_selection_enter_selects_host() {
        use iron_core::host::{HardwareSpec, Host};

        let mut app = App::default();
        app.view = View::HostSelection;
        app.discovered_hosts = vec![
            Host {
                id: "desktop".to_string(),
                name: "Desktop Workstation".to_string(),
                description: None,
                hardware: HardwareSpec::default(),
                install_params: None,
                installed_bundles: vec![],
                active_bundle: None,
                bundle: None,
                profile: None,
                extra_modules: vec![],
                variables: std::collections::HashMap::new(),
            },
            Host {
                id: "laptop".to_string(),
                name: "Laptop".to_string(),
                description: None,
                hardware: HardwareSpec::default(),
                install_params: None,
                installed_bundles: vec![],
                active_bundle: None,
                bundle: None,
                profile: None,
                extra_modules: vec![],
                variables: std::collections::HashMap::new(),
            },
        ];
        app.selected_index = 0;

        app.handle_key(create_key_event(KeyCode::Enter));

        // Should navigate to Dashboard after selection
        assert_eq!(app.view, View::Dashboard);
    }

    #[test]
    fn test_host_selection_c_opens_wizard() {
        let mut app = App::default();
        app.view = View::HostSelection;

        app.handle_key(create_key_event(KeyCode::Char('c')));

        assert_eq!(app.view, View::SetupWizard);
    }

    #[test]
    fn test_shift_h_navigates_to_host_selection() {
        let mut app = App::default();
        app.view = View::Dashboard;

        app.handle_key(create_key_event(KeyCode::Char('H')));

        assert_eq!(app.view, View::HostSelection);
    }

    #[test]
    fn test_host_selection_list_len() {
        use iron_core::host::{HardwareSpec, Host};

        let mut app = App::default();
        app.view = View::HostSelection;
        app.discovered_hosts = vec![Host {
            id: "test".to_string(),
            name: "Test".to_string(),
            description: None,
            hardware: HardwareSpec::default(),
            install_params: None,
            installed_bundles: vec![],
            active_bundle: None,
            bundle: None,
            profile: None,
            extra_modules: vec![],
            variables: std::collections::HashMap::new(),
        }];

        assert_eq!(app.current_list_len(), 1);
    }

    // Divergence popup tests (S1-P3-002)

    #[test]
    fn test_dashboard_i_opens_divergence_popup() {
        let mut app = App::default();
        app.view = View::Dashboard;
        app.diverged_modules = vec!["nvim-ide".to_string()];

        app.handle_key(create_key_event(KeyCode::Char('i')));

        assert!(app.show_divergence_popup);
        assert_eq!(app.divergence_selected, 0);
    }

    #[test]
    fn test_dashboard_i_noop_when_no_divergence() {
        let mut app = App::default();
        app.view = View::Dashboard;
        // No diverged modules

        app.handle_key(create_key_event(KeyCode::Char('i')));

        assert!(!app.show_divergence_popup);
    }

    #[test]
    fn test_divergence_popup_esc_closes() {
        let mut app = App::default();
        app.show_divergence_popup = true;
        app.diverged_modules = vec!["test".to_string()];

        app.handle_key(create_key_event(KeyCode::Esc));

        assert!(!app.show_divergence_popup);
    }

    #[test]
    fn test_divergence_popup_j_k_navigation() {
        let mut app = App::default();
        app.show_divergence_popup = true;
        app.diverged_modules = vec!["mod-a".to_string(), "mod-b".to_string()];
        app.divergence_selected = 0;

        app.handle_key(create_key_event(KeyCode::Char('j')));
        assert_eq!(app.divergence_selected, 1);

        app.handle_key(create_key_event(KeyCode::Char('k')));
        assert_eq!(app.divergence_selected, 0);
    }

    #[test]
    fn test_divergence_popup_r_restore_guidance() {
        let mut app = App::default();
        app.show_divergence_popup = true;
        app.diverged_modules = vec!["nvim-ide".to_string()];
        app.divergence_selected = 0;

        app.handle_key(create_key_event(KeyCode::Char('r')));

        assert!(!app.show_divergence_popup);
        assert!(has_feedback(&app));
    }

    #[test]
    fn test_divergence_popup_a_accept_guidance() {
        let mut app = App::default();
        app.show_divergence_popup = true;
        app.diverged_modules = vec!["kitty-dev".to_string()];
        app.divergence_selected = 0;

        app.handle_key(create_key_event(KeyCode::Char('a')));

        assert!(!app.show_divergence_popup);
        assert!(has_feedback(&app));
    }

    #[test]
    fn test_divergence_popup_d_diff_guidance() {
        let mut app = App::default();
        app.show_divergence_popup = true;
        app.diverged_modules = vec!["tmux-config".to_string()];
        app.divergence_selected = 0;

        app.handle_key(create_key_event(KeyCode::Char('d')));

        assert!(!app.show_divergence_popup);
        assert!(has_feedback(&app));
    }

    // ======================================================================
    // E-010: Wizard handler integration tests
    // ======================================================================

    #[test]
    fn test_wizard_welcome_enter_advances() {
        let mut app = App::default();
        app.view = View::SetupWizard;
        app.wizard.step = WizardStep::Welcome;

        app.handle_key(create_key_event(KeyCode::Enter));

        assert_eq!(app.wizard.step, WizardStep::HostSetup);
    }

    #[test]
    fn test_wizard_welcome_q_quits() {
        let mut app = App::default();
        app.view = View::SetupWizard;
        app.wizard.step = WizardStep::Welcome;

        app.handle_key(create_key_event(KeyCode::Char('q')));

        assert!(app.should_quit);
    }

    #[test]
    fn test_wizard_welcome_esc_quits() {
        let mut app = App::default();
        app.view = View::SetupWizard;
        app.wizard.step = WizardStep::Welcome;

        app.handle_key(create_key_event(KeyCode::Esc));

        assert!(app.should_quit);
    }

    #[test]
    fn test_wizard_host_setup_e_enters_edit_mode() {
        let mut app = App::default();
        app.view = View::SetupWizard;
        app.wizard.step = WizardStep::HostSetup;

        app.handle_key(create_key_event(KeyCode::Char('e')));

        assert!(app.host_input.is_editing());
    }

    #[test]
    fn test_wizard_host_input_typing() {
        let mut app = App::default();
        app.view = View::SetupWizard;
        app.wizard.step = WizardStep::HostSetup;
        app.host_input.enter_edit_mode();

        // Type "desktop"
        for c in "desktop".chars() {
            app.handle_key(create_key_event(KeyCode::Char(c)));
        }

        assert_eq!(app.host_input.value, "desktop");
    }

    #[test]
    fn test_wizard_host_input_enter_confirms() {
        let mut app = App::default();
        app.view = View::SetupWizard;
        app.wizard.step = WizardStep::HostSetup;
        app.host_input.enter_edit_mode();
        app.host_input.value = "my-host".to_string();

        app.handle_key(create_key_event(KeyCode::Enter));

        assert_eq!(app.wizard.host_id, "my-host");
        assert!(!app.host_input.is_editing());
    }

    #[test]
    fn test_wizard_host_input_esc_cancels_edit() {
        let mut app = App::default();
        app.view = View::SetupWizard;
        app.wizard.step = WizardStep::HostSetup;
        app.host_input.enter_edit_mode();
        app.host_input.value = "typing".to_string();

        app.handle_key(create_key_event(KeyCode::Esc));

        assert!(!app.host_input.is_editing());
    }

    #[test]
    fn test_wizard_host_input_backspace_deletes() {
        let mut app = App::default();
        app.view = View::SetupWizard;
        app.wizard.step = WizardStep::HostSetup;
        app.host_input.enter_edit_mode();
        app.host_input.value = "abc".to_string();
        app.host_input.cursor = 3;

        app.handle_key(create_key_event(KeyCode::Backspace));

        assert_eq!(app.host_input.value, "ab");
    }

    #[test]
    fn test_wizard_host_setup_backspace_goes_back() {
        let mut app = App::default();
        app.view = View::SetupWizard;
        app.wizard.step = WizardStep::HostSetup;

        app.handle_key(create_key_event(KeyCode::Backspace));

        assert_eq!(app.wizard.step, WizardStep::Welcome);
    }

    #[test]
    fn test_wizard_bundle_selection_navigation() {
        let mut app = App::default();
        app.view = View::SetupWizard;
        app.wizard.step = WizardStep::BundleSelection;
        app.wizard.available_bundles = vec!["hyprland".to_string(), "niri".to_string()];
        app.wizard.selected_bundle_index = 0;

        app.handle_key(create_key_event(KeyCode::Down));
        assert_eq!(app.wizard.selected_bundle_index, 1);

        app.handle_key(create_key_event(KeyCode::Up));
        assert_eq!(app.wizard.selected_bundle_index, 0);
    }

    #[test]
    fn test_wizard_bundle_selection_j_k_navigation() {
        let mut app = App::default();
        app.view = View::SetupWizard;
        app.wizard.step = WizardStep::BundleSelection;
        app.wizard.available_bundles = vec!["a".to_string(), "b".to_string()];
        app.wizard.selected_bundle_index = 0;

        app.handle_key(create_key_event(KeyCode::Char('j')));
        assert_eq!(app.wizard.selected_bundle_index, 1);

        app.handle_key(create_key_event(KeyCode::Char('k')));
        assert_eq!(app.wizard.selected_bundle_index, 0);
    }

    #[test]
    fn test_wizard_profile_selection_navigation() {
        let mut app = App::default();
        app.view = View::SetupWizard;
        app.wizard.step = WizardStep::ProfileSelection;
        app.wizard.available_profiles = vec!["developer".to_string(), "minimal".to_string()];
        app.wizard.selected_profile_index = 0;

        app.handle_key(create_key_event(KeyCode::Down));
        assert_eq!(app.wizard.selected_profile_index, 1);
    }

    #[test]
    fn test_wizard_confirmation_backspace_goes_back() {
        let mut app = App::default();
        app.view = View::SetupWizard;
        app.wizard.step = WizardStep::Confirmation;

        app.handle_key(create_key_event(KeyCode::Backspace));

        assert_eq!(app.wizard.step, WizardStep::ProfileSelection);
    }
}
