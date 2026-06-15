//! Integrated Arch installation wizard state.

use crate::app::{App, View};
use crossterm::event::{KeyCode, KeyEvent};
use iron_core::{
    InstallEvent, InstallPhaseId, InstallPlan, InstallRunConfig, InstallRunMode, InstallRunner,
    InstallStatus,
};

/// Interactive install wizard state.
#[derive(Debug, Clone)]
pub struct InstallWizardState {
    /// Plan being reviewed/executed.
    pub plan: InstallPlan,
    /// Selected phase index.
    pub selected_phase: usize,
    /// Current phase statuses.
    pub phase_statuses: Vec<InstallStatus>,
    /// Current run mode.
    pub mode: InstallRunMode,
    /// Whether destructive/manual steps are explicitly confirmed.
    pub confirmed: bool,
    /// Typed confirmation buffer.
    pub confirmation_input: String,
    /// Whether the wizard is asking for INSTALL confirmation.
    pub awaiting_confirmation: bool,
    /// Whether a run is complete.
    pub completed: bool,
    /// Latest failure message, if any.
    pub failure: Option<String>,
    /// Human-readable log lines.
    pub logs: Vec<String>,
}

impl InstallWizardState {
    /// Create a wizard around an install plan.
    pub fn new(plan: InstallPlan) -> Self {
        let phase_statuses = vec![InstallStatus::Pending; plan.phases.len()];
        Self {
            plan,
            selected_phase: 0,
            phase_statuses,
            mode: InstallRunMode::DryRun,
            confirmed: false,
            confirmation_input: String::new(),
            awaiting_confirmation: false,
            completed: false,
            failure: None,
            logs: vec!["Wizard initialized. Start with dry-run review.".to_string()],
        }
    }

    /// Select previous phase.
    pub fn previous_phase(&mut self) {
        self.selected_phase = self.selected_phase.saturating_sub(1);
    }

    /// Select next phase.
    pub fn next_phase(&mut self) {
        let max = self.plan.phases.len().saturating_sub(1);
        self.selected_phase = (self.selected_phase + 1).min(max);
    }

    /// Run the full plan as a dry-run event stream.
    pub fn run_dry_run(&mut self) {
        self.mode = InstallRunMode::DryRun;
        self.completed = false;
        self.failure = None;
        self.logs.clear();
        self.phase_statuses.fill(InstallStatus::Pending);
        let runner = InstallRunner::real();
        let events = runner.run(&self.plan, &InstallRunConfig::default());
        self.apply_events(events);
    }

    /// Request typed confirmation for real execution.
    pub fn request_execute(&mut self) {
        self.awaiting_confirmation = true;
        self.confirmation_input.clear();
        self.logs
            .push("Type INSTALL and press Enter to run destructive-capable steps.".to_string());
    }

    /// Execute the full plan after typed confirmation.
    pub fn run_execute(&mut self) {
        self.mode = InstallRunMode::Execute;
        self.confirmed = true;
        self.awaiting_confirmation = false;
        self.completed = false;
        self.failure = None;
        self.logs.clear();
        self.phase_statuses.fill(InstallStatus::Pending);

        let runner = InstallRunner::real();
        let config = InstallRunConfig {
            mode: InstallRunMode::Execute,
            allow_destructive: true,
            confirm_manual: true,
            only_phase: None,
            from_phase: None,
        };
        let events = runner.run(&self.plan, &config);
        self.apply_events(events);
    }

    fn apply_events(&mut self, events: Vec<InstallEvent>) {
        for event in events {
            self.apply_event(event);
        }
    }

    fn apply_event(&mut self, event: InstallEvent) {
        match event {
            InstallEvent::PlanStarted {
                mode,
                host_id,
                target_mount,
            } => self.logs.push(format!(
                "Starting {:?} install plan for {} at {}",
                mode, host_id, target_mount
            )),
            InstallEvent::Warning { message } => self.logs.push(format!("WARN: {}", message)),
            InstallEvent::PhaseStarted {
                phase_id,
                phase_name,
            } => {
                self.set_phase_status(&phase_id, InstallStatus::Running);
                self.logs
                    .push(format!("==> {} [{}]", phase_name, phase_id.as_script_id()));
            }
            InstallEvent::PhaseSkipped {
                phase_id,
                phase_name,
            } => {
                self.set_phase_status(&phase_id, InstallStatus::Skipped);
                self.logs.push(format!(
                    "SKIP: {} [{}]",
                    phase_name,
                    phase_id.as_script_id()
                ));
            }
            InstallEvent::StepStarted {
                description,
                destructive,
                ..
            } => {
                let marker = if destructive { "!" } else { "-" };
                self.logs.push(format!(" {} {}", marker, description));
            }
            InstallEvent::CommandPreview { command, .. } => {
                self.logs.push(format!("   $ {}", command.join(" ")));
            }
            InstallEvent::CommandOutput {
                stdout,
                stderr,
                exit_code,
                ..
            } => {
                if !stdout.trim().is_empty() {
                    self.logs.push(stdout.trim().to_string());
                }
                if !stderr.trim().is_empty() {
                    self.logs.push(format!("stderr: {}", stderr.trim()));
                }
                self.logs.push(format!("exit: {}", exit_code));
            }
            InstallEvent::ManualCheckpoint { description, .. } => {
                self.logs
                    .push(format!("manual checkpoint: {}", description));
            }
            InstallEvent::StepCompleted { step_id } => {
                self.logs.push(format!("done: {}", step_id));
            }
            InstallEvent::StepBlocked { step_id, reason } => {
                self.failure = Some(format!("{} blocked: {}", step_id, reason));
                self.logs.push(format!("BLOCKED: {}: {}", step_id, reason));
            }
            InstallEvent::StepFailed { step_id, error } => {
                self.failure = Some(format!("{} failed: {}", step_id, error));
                self.logs.push(format!("FAILED: {}: {}", step_id, error));
                self.mark_running_failed();
            }
            InstallEvent::PhaseCompleted { phase_id } => {
                self.set_phase_status(&phase_id, InstallStatus::Success);
                self.logs
                    .push(format!("completed phase: {}", phase_id.as_script_id()));
            }
            InstallEvent::PlanCompleted => {
                self.completed = true;
                self.logs.push("Install plan finished.".to_string());
            }
        }
    }

    fn set_phase_status(&mut self, phase_id: &InstallPhaseId, status: InstallStatus) {
        if let Some(index) = self
            .plan
            .phases
            .iter()
            .position(|phase| &phase.id == phase_id)
        {
            self.phase_statuses[index] = status;
        }
    }

    fn mark_running_failed(&mut self) {
        for status in &mut self.phase_statuses {
            if *status == InstallStatus::Running {
                *status = InstallStatus::Failed;
            }
        }
    }
}

impl App {
    /// Enter the integrated install wizard view.
    pub fn open_install_wizard(&mut self, plan: InstallPlan) {
        self.install_wizard = Some(InstallWizardState::new(plan));
        self.view = View::InstallWizard;
    }

    /// Handle install wizard keyboard input.
    pub fn handle_install_wizard_key(&mut self, key: KeyEvent) {
        let Some(wizard) = self.install_wizard.as_mut() else {
            self.go_back();
            return;
        };

        if wizard.awaiting_confirmation {
            match key.code {
                KeyCode::Esc => {
                    wizard.awaiting_confirmation = false;
                    wizard.confirmation_input.clear();
                }
                KeyCode::Enter => {
                    if wizard.confirmation_input == "INSTALL" {
                        wizard.run_execute();
                    } else {
                        wizard
                            .logs
                            .push("Confirmation token did not match.".to_string());
                    }
                }
                KeyCode::Backspace => {
                    wizard.confirmation_input.pop();
                }
                KeyCode::Char(c) => {
                    wizard.confirmation_input.push(c);
                }
                _ => {}
            }
            return;
        }

        match key.code {
            KeyCode::Up | KeyCode::Char('k') => wizard.previous_phase(),
            KeyCode::Down | KeyCode::Char('j') => wizard.next_phase(),
            KeyCode::Char('d') => wizard.run_dry_run(),
            KeyCode::Char('r') => wizard.request_execute(),
            KeyCode::Esc => self.should_quit = true,
            _ => {}
        }
    }
}
