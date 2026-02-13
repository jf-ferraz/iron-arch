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

        // View-specific key handling
        match self.view {
            View::UpdatePreview => {
                match key.code {
                    KeyCode::Char('r') => self.refresh_updates(),
                    KeyCode::Enter | KeyCode::Char('u') => {
                        self.request_confirm(ConfirmAction::RunUpdate);
                    }
                    KeyCode::Esc => self.go_back(),
                    KeyCode::Char('?') => self.show_help = true,
                    KeyCode::Char('q') => self.should_quit = true,
                    _ => {}
                }
                return;
            }
            View::ProfileDetail => {
                match key.code {
                    KeyCode::Enter | KeyCode::Char('a') => self.activate_selected_profile(),
                    KeyCode::Esc => self.go_back(),
                    KeyCode::Char('?') => self.show_help = true,
                    KeyCode::Char('q') => self.should_quit = true,
                    _ => {}
                }
                return;
            }
            _ => {}
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
            KeyCode::Char('u') => self.navigate(View::UpdatePreview),
            KeyCode::Char('s') => self.navigate(View::Settings),

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
            View::Modules | View::ModuleDetail => View::UpdatePreview,
            View::UpdatePreview => View::Settings,
            View::Settings => View::Dashboard,
            _ => View::Dashboard,
        };
        self.navigate(next);
    }

    /// Cycle to previous view
    fn cycle_view_backward(&mut self) {
        let prev = match self.view {
            View::Dashboard => View::Settings,
            View::Settings => View::UpdatePreview,
            View::UpdatePreview => View::Modules,
            View::Modules | View::ModuleDetail => View::Profiles,
            View::Profiles | View::ProfileDetail => View::Bundles,
            View::Bundles | View::BundleDetail => View::Dashboard,
            _ => View::Dashboard,
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
            View::Bundles => self.bundles.len(),
            View::Profiles => self.profiles.len(),
            View::Modules => self.modules.len(),
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
