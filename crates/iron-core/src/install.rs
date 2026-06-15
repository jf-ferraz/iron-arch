//! Arch install planning.
//!
//! This module turns Iron host configuration into a typed, reviewable install
//! plan. Execution is intentionally separate so destructive operations can be
//! guarded and tested independently.

use crate::host::{BootloaderType, Host, InstallParams, PartitionConfig};
use crate::resilience::{CommandConfig, CommandError, CommandExecutor, RealCommandExecutor};
use crate::{ConfigError, IronError, IronResult};
use serde::{Deserialize, Serialize};

/// A typed Arch installation plan derived from an Iron host.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct InstallPlan {
    /// Host ID this plan targets.
    pub host_id: String,
    /// Human-readable host name.
    pub host_name: String,
    /// Target mountpoint used by Arch install scripts.
    pub target_mount: String,
    /// Ordered phases to execute.
    pub phases: Vec<InstallPhase>,
    /// Warnings that require operator review.
    pub warnings: Vec<String>,
}

/// One coarse-grained install phase.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct InstallPhase {
    /// Stable phase identifier.
    pub id: InstallPhaseId,
    /// Human-readable phase name.
    pub name: String,
    /// Ordered steps in this phase.
    pub steps: Vec<InstallStep>,
}

/// Stable install phase identifiers.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum InstallPhaseId {
    Preflight,
    Disk,
    Bootstrap,
    SystemConfig,
    IronBootstrap,
    Validation,
}

/// A reviewable command or manual checkpoint.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct InstallStep {
    /// Stable step identifier.
    pub id: String,
    /// Human-readable description.
    pub description: String,
    /// Command to run, if automated.
    pub command: Option<Vec<String>>,
    /// Whether this step can destroy data.
    pub destructive: bool,
}

/// Install execution mode.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum InstallRunMode {
    /// Emit the complete event stream without executing commands.
    DryRun,
    /// Execute commands through the configured command runner.
    Execute,
}

/// Runtime configuration for executing an install plan.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct InstallRunConfig {
    /// Execution mode.
    pub mode: InstallRunMode,
    /// Allow destructive automated steps to run.
    pub allow_destructive: bool,
    /// Treat manual checkpoints as already reviewed.
    pub confirm_manual: bool,
    /// Optional phase to run exclusively.
    pub only_phase: Option<InstallPhaseId>,
    /// Optional phase to start from and continue.
    pub from_phase: Option<InstallPhaseId>,
}

impl Default for InstallRunConfig {
    fn default() -> Self {
        Self {
            mode: InstallRunMode::DryRun,
            allow_destructive: false,
            confirm_manual: false,
            only_phase: None,
            from_phase: None,
        }
    }
}

/// Status for install phases and steps.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum InstallStatus {
    Pending,
    Running,
    Success,
    Skipped,
    Failed,
    Blocked,
}

/// Typed event emitted while validating or executing an install plan.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case", tag = "type")]
pub enum InstallEvent {
    PlanStarted {
        mode: InstallRunMode,
        host_id: String,
        target_mount: String,
    },
    Warning {
        message: String,
    },
    PhaseStarted {
        phase_id: InstallPhaseId,
        phase_name: String,
    },
    PhaseSkipped {
        phase_id: InstallPhaseId,
        phase_name: String,
    },
    StepStarted {
        phase_id: InstallPhaseId,
        step_id: String,
        description: String,
        destructive: bool,
    },
    CommandPreview {
        step_id: String,
        command: Vec<String>,
    },
    CommandOutput {
        step_id: String,
        stdout: String,
        stderr: String,
        exit_code: i32,
    },
    ManualCheckpoint {
        step_id: String,
        description: String,
        destructive: bool,
    },
    StepCompleted {
        step_id: String,
    },
    StepBlocked {
        step_id: String,
        reason: String,
    },
    StepFailed {
        step_id: String,
        error: String,
    },
    PhaseCompleted {
        phase_id: InstallPhaseId,
    },
    PlanCompleted,
}

/// Executes an install plan and emits typed events for CLI/TUI consumers.
pub struct InstallRunner<E> {
    executor: E,
}

impl InstallRunner<RealCommandExecutor> {
    /// Create a production install runner with conservative command settings.
    pub fn real() -> Self {
        Self {
            executor: RealCommandExecutor::new(CommandConfig::strict().with_max_retries(0)),
        }
    }
}

impl<E: CommandExecutor> InstallRunner<E> {
    /// Create a runner with an injected command executor.
    pub fn new(executor: E) -> Self {
        Self { executor }
    }

    /// Run a plan and return the full event stream.
    pub fn run(&self, plan: &InstallPlan, config: &InstallRunConfig) -> Vec<InstallEvent> {
        let mut events = vec![InstallEvent::PlanStarted {
            mode: config.mode,
            host_id: plan.host_id.clone(),
            target_mount: plan.target_mount.clone(),
        }];

        for warning in &plan.warnings {
            events.push(InstallEvent::Warning {
                message: warning.clone(),
            });
        }

        for phase in &plan.phases {
            if !should_run_phase_id(&phase.id, config, plan) {
                events.push(InstallEvent::PhaseSkipped {
                    phase_id: phase.id.clone(),
                    phase_name: phase.name.clone(),
                });
                continue;
            }

            events.push(InstallEvent::PhaseStarted {
                phase_id: phase.id.clone(),
                phase_name: phase.name.clone(),
            });

            for step in &phase.steps {
                events.push(InstallEvent::StepStarted {
                    phase_id: phase.id.clone(),
                    step_id: step.id.clone(),
                    description: step.description.clone(),
                    destructive: step.destructive,
                });

                if step.destructive
                    && config.mode == InstallRunMode::Execute
                    && !config.allow_destructive
                {
                    events.push(InstallEvent::StepBlocked {
                        step_id: step.id.clone(),
                        reason: "destructive step requires explicit confirmation".to_string(),
                    });
                    return events;
                }

                match &step.command {
                    Some(command) => {
                        events.push(InstallEvent::CommandPreview {
                            step_id: step.id.clone(),
                            command: command.clone(),
                        });

                        if config.mode == InstallRunMode::Execute {
                            match execute_command(&self.executor, command) {
                                Ok(output) => {
                                    events.push(InstallEvent::CommandOutput {
                                        step_id: step.id.clone(),
                                        stdout: output.stdout,
                                        stderr: output.stderr,
                                        exit_code: output.exit_code,
                                    });
                                    events.push(InstallEvent::StepCompleted {
                                        step_id: step.id.clone(),
                                    });
                                }
                                Err(error) => {
                                    events.push(InstallEvent::StepFailed {
                                        step_id: step.id.clone(),
                                        error: error.to_string(),
                                    });
                                    return events;
                                }
                            }
                        } else {
                            events.push(InstallEvent::StepCompleted {
                                step_id: step.id.clone(),
                            });
                        }
                    }
                    None => {
                        events.push(InstallEvent::ManualCheckpoint {
                            step_id: step.id.clone(),
                            description: step.description.clone(),
                            destructive: step.destructive,
                        });

                        if config.mode == InstallRunMode::Execute && !config.confirm_manual {
                            events.push(InstallEvent::StepBlocked {
                                step_id: step.id.clone(),
                                reason: "manual checkpoint has not been confirmed".to_string(),
                            });
                            return events;
                        }

                        events.push(InstallEvent::StepCompleted {
                            step_id: step.id.clone(),
                        });
                    }
                }
            }

            events.push(InstallEvent::PhaseCompleted {
                phase_id: phase.id.clone(),
            });
        }

        events.push(InstallEvent::PlanCompleted);
        events
    }
}

impl InstallPlan {
    /// Build an install plan from a host definition.
    pub fn from_host(host: &Host, target_mount: impl Into<String>) -> IronResult<Self> {
        let target_mount = target_mount.into();
        validate_target_mount(&target_mount)?;

        let install = host.install_params.as_ref().ok_or_else(|| {
            IronError::Config(ConfigError::MissingField {
                field: "install_params".to_string(),
            })
        })?;

        validate_install_params(install)?;

        let mut warnings = vec![
            "Review backup status before running disk or bootstrap phases.".to_string(),
            "Disk partitioning is planned as manual by default; no wipe command is generated."
                .to_string(),
        ];

        if install.encrypted {
            warnings.push(
                "Encryption is requested; LUKS formatting/opening must be confirmed manually in phase 2."
                    .to_string(),
            );
        }

        Ok(Self {
            host_id: host.id.clone(),
            host_name: host.name.clone(),
            target_mount: target_mount.clone(),
            phases: vec![
                preflight_phase(),
                disk_phase(install, &target_mount),
                bootstrap_phase(install, &target_mount),
                system_config_phase(host, install, &target_mount),
                iron_bootstrap_phase(host, &target_mount),
                validation_phase(&target_mount),
            ],
            warnings,
        })
    }

    /// Render this plan as a shell script. The script is conservative: it keeps
    /// disk preparation as manual checkpoints, confirms destructive steps, and
    /// uses `set -Eeuo pipefail`.
    pub fn to_shell_script(&self) -> String {
        let mut script = String::from(
            r#"#!/usr/bin/env bash
set -Eeuo pipefail

LOG_FILE="${LOG_FILE:-/tmp/iron-install.log}"
ASSUME_YES="${ASSUME_YES:-false}"
DRY_RUN="${DRY_RUN:-false}"
IRON_BIN="${IRON_BIN:-$(command -v iron || true)}"
IRON_CONFIG_SRC="${IRON_CONFIG_SRC:-$(pwd)}"
STATE_FILE="${STATE_FILE:-/tmp/iron-install-state}"
ONLY_PHASE=""
FROM_PHASE=""
PHASE_STARTED=false
RUN_ALL=false
MENU_REQUESTED=false

trap 'echo "[ERROR] line ${LINENO}: ${BASH_COMMAND}" | tee -a "$LOG_FILE"' ERR

log() {
  printf '[%s] %s\n' "$(date -Is)" "$*" | tee -a "$LOG_FILE"
}

warn() {
  printf '[%s] WARNING: %s\n' "$(date -Is)" "$*" | tee -a "$LOG_FILE"
}

die() {
  printf '[%s] ERROR: %s\n' "$(date -Is)" "$*" | tee -a "$LOG_FILE" >&2
  exit 1
}

confirm() {
  local prompt="$1"
  if [[ "$DRY_RUN" == "true" ]]; then
    log "Dry-run confirmed: $prompt"
    return 0
  fi
  if [[ "$ASSUME_YES" == "true" ]]; then
    log "Auto-confirmed: $prompt"
    return 0
  fi
  read -r -p "$prompt [y/N] " response
  [[ "$response" =~ ^[Yy]$ ]]
}

run_step() {
  log "RUN: $*"
  if [[ "$DRY_RUN" == "true" ]]; then
    return 0
  fi
  "$@" 2>&1 | tee -a "$LOG_FILE"
}

run_shell() {
  log "RUN: $*"
  if [[ "$DRY_RUN" == "true" ]]; then
    return 0
  fi
  bash -lc "$*" 2>&1 | tee -a "$LOG_FILE"
}

manual_step() {
  local prompt="$1"
  confirm "$prompt" || {
    log "Skipped/manual checkpoint not confirmed: $prompt"
    return 1
  }
}

usage() {
  cat <<'EOF'
Iron generated Arch installer

Options:
  --menu             Open the interactive menu
  --run             Run the full plan without opening the menu
  --dry-run          Print/log commands without executing them
  --yes             Auto-confirm prompts
  --only PHASE      Run only one phase
  --from PHASE      Start from a phase and continue
  --list-phases     List phases and exit
  -h, --help        Show this help

Environment:
  IRON_BIN          Built iron binary to copy into target
  IRON_CONFIG_SRC   Iron config/repo source to copy into target
  LOG_FILE          Installer log file
  STATE_FILE        Last completed phase state file
EOF
}

parse_args() {
  while [[ $# -gt 0 ]]; do
    case "$1" in
      --menu)
        MENU_REQUESTED=true
        shift
        ;;
      --run)
        RUN_ALL=true
        shift
        ;;
      --dry-run)
        DRY_RUN=true
        shift
        ;;
      --yes)
        ASSUME_YES=true
        shift
        ;;
      --only)
        ONLY_PHASE="${2:-}"
        [[ -n "$ONLY_PHASE" ]] || die "--only requires a phase id"
        shift 2
        ;;
      --from)
        FROM_PHASE="${2:-}"
        [[ -n "$FROM_PHASE" ]] || die "--from requires a phase id"
        shift 2
        ;;
      --list-phases)
        LIST_PHASES=true
        shift
        ;;
      -h|--help)
        usage
        exit 0
        ;;
      *)
        die "Unknown argument: $1"
        ;;
    esac
  done
}

reset_phase_filters() {
  ONLY_PHASE=""
  FROM_PHASE=""
  PHASE_STARTED=false
}

phase_known() {
  local wanted="$1"
  for phase_id in "${PHASE_IDS[@]}"; do
    [[ "$phase_id" == "$wanted" ]] && return 0
  done
  return 1
}

list_phases() {
  for i in "${!PHASE_IDS[@]}"; do
    printf '%s\t%s\n' "${PHASE_IDS[$i]}" "${PHASE_NAMES[$i]}"
  done
}

validate_phase_args() {
  [[ -z "${ONLY_PHASE:-}" || -z "${FROM_PHASE:-}" ]] || die "--only and --from cannot be used together"
  [[ -z "${ONLY_PHASE:-}" ]] || phase_known "$ONLY_PHASE" || die "Unknown --only phase: $ONLY_PHASE"
  [[ -z "${FROM_PHASE:-}" ]] || phase_known "$FROM_PHASE" || die "Unknown --from phase: $FROM_PHASE"
}

should_run_phase() {
  local phase_id="$1"

  if [[ -n "${ONLY_PHASE:-}" ]]; then
    [[ "$phase_id" == "$ONLY_PHASE" ]]
    return
  fi

  if [[ -n "${FROM_PHASE:-}" && "$PHASE_STARTED" != "true" ]]; then
    if [[ "$phase_id" == "$FROM_PHASE" ]]; then
      PHASE_STARTED=true
      return 0
    fi
    return 1
  fi

  return 0
}

begin_phase() {
  local phase_id="$1"
  local phase_name="$2"
  if should_run_phase "$phase_id"; then
    log "==> $phase_name [$phase_id]"
    if [[ "$DRY_RUN" != "true" ]]; then
      printf '%s\n' "$phase_id" > "$STATE_FILE"
    fi
    return 0
  fi
  log "Skipping phase: $phase_name [$phase_id]"
  return 1
}

finish_phase() {
  local phase_id="$1"
  log "Completed phase: $phase_id"
  if [[ "$DRY_RUN" != "true" ]]; then
    printf '%s\n' "$phase_id:completed" > "$STATE_FILE"
  fi
}

require_runtime() {
  if [[ "$DRY_RUN" == "true" ]]; then
    warn "Dry-run mode enabled; no commands will be executed."
    return 0
  fi

  [[ $EUID -eq 0 ]] || die "Run this installer as root from the Arch ISO."

  local missing=()
  for cmd in pacstrap genfstab arch-chroot mount mkdir test ping timedatectl install rsync; do
    command -v "$cmd" >/dev/null 2>&1 || missing+=("$cmd")
  done

  if ((${#missing[@]} > 0)); then
    die "Missing required commands: ${missing[*]}"
  fi
}

print_intro() {
  cat <<EOF

╔════════════════════════════════════════════════════════════╗
║                    Iron Arch Installer                    ║
╚════════════════════════════════════════════════════════════╝

Target mount : $TARGET_MOUNT
Iron binary  : ${IRON_BIN:-<unset>}
Iron config  : $IRON_CONFIG_SRC
Log file     : $LOG_FILE
State file   : $STATE_FILE

EOF
}

show_status() {
  echo
  echo "Plan phases:"
  for i in "${!PHASE_IDS[@]}"; do
    printf '  %d) %-14s %s\n' "$((i + 1))" "${PHASE_IDS[$i]}" "${PHASE_NAMES[$i]}"
  done
  echo
  if [[ -f "$STATE_FILE" ]]; then
    echo "Last state: $(cat "$STATE_FILE")"
  else
    echo "Last state: none"
  fi
  echo
}

pick_phase() {
  local prompt="$1"
  local choice
  show_status
  read -r -p "$prompt [1-${#PHASE_IDS[@]}] " choice
  [[ "$choice" =~ ^[0-9]+$ ]] || die "Invalid phase number: $choice"
  ((choice >= 1 && choice <= ${#PHASE_IDS[@]})) || die "Phase number out of range: $choice"
  printf '%s\n' "${PHASE_IDS[$((choice - 1))]}"
}

final_install_confirmation() {
  echo
  echo "This will run the full install plan against $TARGET_MOUNT."
  echo "Disk formatting is still manual, but mount/bootstrap/system steps affect the target."
  echo
  read -r -p "Type INSTALL to continue: " token
  [[ "$token" == "INSTALL" ]] || die "Install cancelled"
}

interactive_menu() {
  while true; do
    print_intro
    show_status
    cat <<'EOF'
Choose an action:
  1) Dry-run full plan
  2) Run preflight checks
  3) Run one phase
  4) Resume from phase
  5) Run full install
  6) Show help
  7) Quit
EOF
    echo
    read -r -p "Selection [1-7]: " selection

    case "$selection" in
      1)
        reset_phase_filters
        DRY_RUN=true
        run_plan
        DRY_RUN=false
        ;;
      2)
        reset_phase_filters
        ONLY_PHASE="preflight"
        run_plan
        reset_phase_filters
        ;;
      3)
        reset_phase_filters
        ONLY_PHASE="$(pick_phase "Phase to run")"
        run_plan
        reset_phase_filters
        ;;
      4)
        reset_phase_filters
        FROM_PHASE="$(pick_phase "Resume from phase")"
        run_plan
        reset_phase_filters
        ;;
      5)
        reset_phase_filters
        final_install_confirmation
        run_plan
        ;;
      6)
        usage
        ;;
      7)
        log "Installer menu exited"
        exit 0
        ;;
      *)
        echo "Invalid selection: $selection"
        ;;
    esac

    echo
    read -r -p "Press Enter to return to the menu..." _
  done
}

"#,
        );
        script.push_str(&format!("# Iron install plan for {}\n", self.host_id));
        script.push_str(&format!("TARGET_MOUNT=\"{}\"\n\n", self.target_mount));
        script.push_str("LIST_PHASES=false\n");
        script.push_str("PHASE_IDS=(");
        for phase in &self.phases {
            script.push_str(&format!(
                " '{}'",
                shell_escape_single(&phase.id.as_script_id())
            ));
        }
        script.push_str(" )\n");
        script.push_str("PHASE_NAMES=(");
        for phase in &self.phases {
            script.push_str(&format!(" '{}'", shell_escape_single(&phase.name)));
        }
        script.push_str(" )\n\n");
        script.push_str("parse_args \"$@\"\n");
        script.push_str("validate_phase_args\n");
        script.push_str("if [[ \"$LIST_PHASES\" == \"true\" ]]; then list_phases; exit 0; fi\n");
        script.push_str("run_plan() {\n");
        script.push_str("print_intro\n");
        script.push_str("PHASE_STARTED=false\n");
        script.push_str("require_runtime\n");

        for warning in &self.warnings {
            script.push_str(&format!("warn '{}'\n", shell_escape_single(warning)));
        }
        script.push('\n');

        for phase in &self.phases {
            script.push_str(&format!(
                "\nif begin_phase '{}' '{}'; then\n",
                shell_escape_single(&phase.id.as_script_id()),
                shell_escape_single(&phase.name)
            ));
            for step in &phase.steps {
                script.push_str(&format!(
                    "  log ' -> {}'\n",
                    shell_escape_single(&step.description)
                ));
                if let Some(command) = &step.command {
                    if step.destructive {
                        script.push_str(&format!(
                            "  confirm '{}' || exit 1\n",
                            shell_escape_single(&format!(
                                "Run destructive step '{}'?",
                                step.description
                            ))
                        ));
                    }
                    script.push_str("  ");
                    script.push_str(&render_script_command(command));
                    script.push('\n');
                } else {
                    script.push_str(&format!(
                        "  manual_step '{}'\n",
                        shell_escape_single(&format!(
                            "Manual checkpoint complete: {}?",
                            step.description
                        ))
                    ));
                }
            }
            script.push_str(&format!(
                "  finish_phase '{}'\n",
                shell_escape_single(&phase.id.as_script_id())
            ));
            script.push_str("fi\n");
        }
        script.push_str("\nlog 'Install plan finished. Review logs before rebooting.'\n");
        script.push_str("}\n\n");
        script.push_str("if [[ \"$MENU_REQUESTED\" == \"true\" || ( \"$RUN_ALL\" != \"true\" && \"$DRY_RUN\" != \"true\" && -z \"$ONLY_PHASE\" && -z \"$FROM_PHASE\" ) ]]; then\n");
        script.push_str("  interactive_menu\n");
        script.push_str("else\n");
        script.push_str("  run_plan\n");
        script.push_str("fi\n");

        script
    }
}

impl InstallPhaseId {
    /// Stable kebab-case identifier for scripts, logs, and UI state.
    pub fn as_script_id(&self) -> &'static str {
        match self {
            InstallPhaseId::Preflight => "preflight",
            InstallPhaseId::Disk => "disk",
            InstallPhaseId::Bootstrap => "bootstrap",
            InstallPhaseId::SystemConfig => "system-config",
            InstallPhaseId::IronBootstrap => "iron-bootstrap",
            InstallPhaseId::Validation => "validation",
        }
    }
}

fn should_run_phase_id(
    phase_id: &InstallPhaseId,
    config: &InstallRunConfig,
    plan: &InstallPlan,
) -> bool {
    if let Some(only_phase) = &config.only_phase {
        return phase_id == only_phase;
    }

    if let Some(from_phase) = &config.from_phase {
        let phase_position = plan.phases.iter().position(|phase| &phase.id == phase_id);
        let from_position = plan.phases.iter().position(|phase| &phase.id == from_phase);
        return matches!((phase_position, from_position), (Some(phase), Some(from)) if phase >= from);
    }

    true
}

fn execute_command<E: CommandExecutor>(
    executor: &E,
    command: &[String],
) -> Result<crate::resilience::CommandOutput, CommandError> {
    let Some(program) = command.first() else {
        return Err(CommandError::SpawnFailed {
            command: "<empty>".to_string(),
            message: "install step command is empty".to_string(),
        });
    };
    let args: Vec<&str> = command.iter().skip(1).map(String::as_str).collect();
    executor.execute_full(program, &args)
}

fn preflight_phase() -> InstallPhase {
    InstallPhase {
        id: InstallPhaseId::Preflight,
        name: "Preflight checks".to_string(),
        steps: vec![
            command_step(
                "check-uefi",
                "Check UEFI firmware availability",
                vec!["test", "-d", "/sys/firmware/efi/efivars"],
                false,
            ),
            command_step(
                "check-network",
                "Check network connectivity",
                vec!["ping", "-c", "1", "archlinux.org"],
                false,
            ),
            command_step(
                "sync-clock",
                "Enable NTP clock synchronization",
                vec!["timedatectl", "set-ntp", "true"],
                false,
            ),
        ],
    }
}

fn disk_phase(install: &InstallParams, target_mount: &str) -> InstallPhase {
    let mut steps = vec![manual_step(
        "review-partitions",
        "Review partition table and confirm target devices",
        true,
    )];

    for partition in &install.partitions {
        steps.push(manual_step(
            &format!("prepare-{}", mountpoint_id(&partition.mount_point)),
            &format!(
                "Prepare {} as {} for {} ({})",
                partition.device, partition.filesystem, partition.mount_point, partition.size
            ),
            true,
        ));
    }

    for partition in sorted_partitions_for_mount(&install.partitions) {
        if partition.mount_point != "/" {
            steps.push(command_step(
                &format!(
                    "create-mountpoint-{}",
                    mountpoint_id(&partition.mount_point)
                ),
                &format!("Create mountpoint for {}", partition.mount_point),
                vec![
                    "mkdir".to_string(),
                    "-p".to_string(),
                    format!("{target_mount}{}", partition.mount_point),
                ],
                true,
            ));
        }

        steps.push(command_step(
            &format!("mount-{}", mountpoint_id(&partition.mount_point)),
            &format!("Mount {} at {}", partition.device, partition.mount_point),
            mount_command_for(partition, target_mount),
            true,
        ));
    }

    InstallPhase {
        id: InstallPhaseId::Disk,
        name: "Disk preparation".to_string(),
        steps,
    }
}

fn bootstrap_phase(install: &InstallParams, target_mount: &str) -> InstallPhase {
    let mut packages = vec![
        "base".to_string(),
        install.kernel.clone(),
        "linux-firmware".to_string(),
        "networkmanager".to_string(),
        "sudo".to_string(),
        "git".to_string(),
        "rsync".to_string(),
    ];

    if let Some(microcode) = &install.microcode {
        packages.push(microcode.clone());
    }
    packages.extend(install.gpu_drivers.clone());
    dedup_preserve_order(&mut packages);

    let mut command = vec![
        "pacstrap".to_string(),
        "-K".to_string(),
        target_mount.to_string(),
    ];
    command.extend(packages);

    InstallPhase {
        id: InstallPhaseId::Bootstrap,
        name: "Base system bootstrap".to_string(),
        steps: vec![InstallStep {
            id: "pacstrap-base".to_string(),
            description: "Install base packages into target root".to_string(),
            command: Some(command),
            destructive: false,
        }],
    }
}

fn system_config_phase(host: &Host, install: &InstallParams, target_mount: &str) -> InstallPhase {
    let mut steps = vec![
        command_step(
            "genfstab",
            "Generate fstab using UUIDs",
            vec![
                "sh",
                "-c",
                &format!("genfstab -U {target_mount} >> {target_mount}/etc/fstab"),
            ],
            false,
        ),
        command_step(
            "set-hostname",
            "Set target hostname",
            vec![
                "arch-chroot",
                target_mount,
                "sh",
                "-c",
                &format!("printf '%s\\n' '{}' > /etc/hostname", host.id),
            ],
            false,
        ),
        command_step(
            "enable-networkmanager",
            "Enable NetworkManager",
            vec![
                "arch-chroot",
                target_mount,
                "systemctl",
                "enable",
                "NetworkManager.service",
            ],
            false,
        ),
    ];

    match install.bootloader {
        BootloaderType::SystemdBoot => steps.push(command_step(
            "install-systemd-boot",
            "Install systemd-boot",
            vec!["arch-chroot", target_mount, "bootctl", "install"],
            false,
        )),
        BootloaderType::Grub => steps.push(manual_step(
            "install-grub",
            "Install and configure GRUB for this machine firmware",
            false,
        )),
        BootloaderType::RefindBoot => steps.push(manual_step(
            "install-refind",
            "Install and configure rEFInd",
            false,
        )),
    }

    InstallPhase {
        id: InstallPhaseId::SystemConfig,
        name: "System configuration".to_string(),
        steps,
    }
}

fn iron_bootstrap_phase(host: &Host, target_mount: &str) -> InstallPhase {
    let mut steps = vec![
        command_step(
            "install-iron-binary",
            "Install Iron binary into target system",
            vec![
                "sh",
                "-c",
                &format!(
                    "if [[ -z \"${{IRON_BIN:-}}\" || ! -x \"${{IRON_BIN}}\" ]]; then echo 'IRON_BIN must point to a built iron binary' >&2; exit 1; fi; install -Dm755 \"${{IRON_BIN}}\" {target_mount}/usr/local/bin/iron"
                ),
            ],
            false,
        ),
        command_step(
            "copy-iron-config",
            "Copy Iron configuration into target system",
            vec![
                "sh",
                "-c",
                &format!(
                    "if [[ ! -d \"${{IRON_CONFIG_SRC}}\" ]]; then echo 'IRON_CONFIG_SRC must point to the Iron config directory' >&2; exit 1; fi; mkdir -p {target_mount}/opt/iron-config; rsync -rt --delete --exclude target/ --exclude .git/ \"${{IRON_CONFIG_SRC}}/\" {target_mount}/opt/iron-config/"
                ),
            ],
            false,
        ),
    ];

    if let Some(bundle) = &host.active_bundle {
        steps.push(command_step(
            "apply-active-bundle",
            "Apply active Iron bundle",
            vec![
                "arch-chroot",
                target_mount,
                "iron",
                "--root",
                "/opt/iron-config",
                "bundle",
                "install",
                bundle,
                "--yes",
            ],
            false,
        ));
    }

    InstallPhase {
        id: InstallPhaseId::IronBootstrap,
        name: "Iron bootstrap".to_string(),
        steps,
    }
}

fn validation_phase(target_mount: &str) -> InstallPhase {
    InstallPhase {
        id: InstallPhaseId::Validation,
        name: "Validation".to_string(),
        steps: vec![
            command_step(
                "check-fstab",
                "Verify fstab was generated",
                vec!["test", "-s", &format!("{target_mount}/etc/fstab")],
                false,
            ),
            command_step(
                "check-kernel",
                "Verify kernel image exists",
                vec!["test", "-e", &format!("{target_mount}/boot/vmlinuz-linux")],
                false,
            ),
        ],
    }
}

fn mount_command_for(partition: &PartitionConfig, target_mount: &str) -> Vec<String> {
    let mountpoint = if partition.mount_point == "/" {
        target_mount.to_string()
    } else {
        format!("{target_mount}{}", partition.mount_point)
    };
    vec!["mount".to_string(), partition.device.clone(), mountpoint]
}

fn mountpoint_id(mountpoint: &str) -> String {
    if mountpoint == "/" {
        "root".to_string()
    } else {
        mountpoint.trim_matches('/').replace('/', "-")
    }
}

fn sorted_partitions_for_mount(partitions: &[PartitionConfig]) -> Vec<&PartitionConfig> {
    let mut sorted: Vec<&PartitionConfig> = partitions.iter().collect();
    sorted.sort_by_key(|partition| {
        if partition.mount_point == "/" {
            (0, 0)
        } else {
            (1, partition.mount_point.matches('/').count())
        }
    });
    sorted
}

fn command_step<S>(id: &str, description: &str, command: Vec<S>, destructive: bool) -> InstallStep
where
    S: Into<String>,
{
    InstallStep {
        id: id.to_string(),
        description: description.to_string(),
        command: Some(command.into_iter().map(Into::into).collect()),
        destructive,
    }
}

fn manual_step(id: &str, description: &str, destructive: bool) -> InstallStep {
    InstallStep {
        id: id.to_string(),
        description: description.to_string(),
        command: None,
        destructive,
    }
}

fn dedup_preserve_order(values: &mut Vec<String>) {
    let mut seen = std::collections::HashSet::new();
    values.retain(|value| seen.insert(value.clone()));
}

fn render_script_command(command: &[String]) -> String {
    if command.first().map(|cmd| cmd == "sh").unwrap_or(false)
        && command.get(1).map(|arg| arg == "-c").unwrap_or(false)
        && let Some(shell_command) = command.get(2)
    {
        return format!("run_shell '{}'", shell_escape_single(shell_command));
    }

    format!(
        "run_step {}",
        command
            .iter()
            .map(|part| shell_word(part))
            .collect::<Vec<_>>()
            .join(" ")
    )
}

fn shell_word(value: &str) -> String {
    if value
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || "-_./:=@".contains(ch))
    {
        value.to_string()
    } else {
        format!("'{}'", shell_escape_single(value))
    }
}

fn validate_target_mount(target_mount: &str) -> IronResult<()> {
    if !target_mount.starts_with('/') {
        return Err(ConfigError::InvalidValue {
            field: "target_mount".to_string(),
            message: "target mount must be an absolute path".to_string(),
        }
        .into());
    }

    if target_mount == "/" {
        return Err(ConfigError::InvalidValue {
            field: "target_mount".to_string(),
            message: "target mount cannot be /".to_string(),
        }
        .into());
    }

    Ok(())
}

fn validate_install_params(install: &InstallParams) -> IronResult<()> {
    if install.partitions.is_empty() {
        return Err(ConfigError::MissingField {
            field: "install_params.partitions".to_string(),
        }
        .into());
    }

    if install.kernel.trim().is_empty() {
        return Err(ConfigError::MissingField {
            field: "install_params.kernel".to_string(),
        }
        .into());
    }

    if !install
        .partitions
        .iter()
        .any(|partition| partition.mount_point == "/")
    {
        return Err(ConfigError::MissingField {
            field: "install_params.partitions[/]".to_string(),
        }
        .into());
    }

    let mut mountpoints = std::collections::HashSet::new();
    for partition in &install.partitions {
        validate_partition(partition)?;
        if !mountpoints.insert(partition.mount_point.clone()) {
            return Err(ConfigError::InvalidValue {
                field: "install_params.partitions.mount_point".to_string(),
                message: format!("duplicate mount point {}", partition.mount_point),
            }
            .into());
        }
    }

    Ok(())
}

fn validate_partition(partition: &PartitionConfig) -> IronResult<()> {
    if !partition.device.starts_with("/dev/") && !partition.device.starts_with("UUID=") {
        return Err(ConfigError::InvalidValue {
            field: "install_params.partitions.device".to_string(),
            message: format!(
                "{} is not a /dev path or UUID= identifier",
                partition.device
            ),
        }
        .into());
    }

    if !partition.mount_point.starts_with('/') {
        return Err(ConfigError::InvalidValue {
            field: "install_params.partitions.mount_point".to_string(),
            message: format!("{} is not absolute", partition.mount_point),
        }
        .into());
    }

    if partition.filesystem.trim().is_empty() {
        return Err(ConfigError::MissingField {
            field: "install_params.partitions.filesystem".to_string(),
        }
        .into());
    }

    Ok(())
}

fn shell_escape_single(value: &str) -> String {
    value.replace('\'', "'\"'\"'")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::host::{HardwareSpec, PartitionConfig};
    use crate::resilience::{MockCommandExecutor, MockResponse};

    fn test_host() -> Host {
        Host {
            id: "desktop".to_string(),
            name: "Desktop".to_string(),
            description: None,
            hardware: HardwareSpec::default(),
            install_params: Some(InstallParams {
                partitions: vec![
                    PartitionConfig {
                        device: "/dev/nvme0n1p1".to_string(),
                        mount_point: "/boot".to_string(),
                        filesystem: "vfat".to_string(),
                        size: "512M".to_string(),
                    },
                    PartitionConfig {
                        device: "/dev/nvme0n1p2".to_string(),
                        mount_point: "/".to_string(),
                        filesystem: "btrfs".to_string(),
                        size: "remaining".to_string(),
                    },
                ],
                bootloader: BootloaderType::SystemdBoot,
                kernel: "linux".to_string(),
                microcode: Some("amd-ucode".to_string()),
                gpu_drivers: vec!["nvidia".to_string()],
                filesystem: "btrfs".to_string(),
                encrypted: true,
            }),
            installed_bundles: vec!["hyprland".to_string()],
            active_bundle: Some("hyprland".to_string()),
        }
    }

    #[test]
    fn builds_plan_from_host_install_params() {
        let plan = InstallPlan::from_host(&test_host(), "/mnt").unwrap();

        assert_eq!(plan.host_id, "desktop");
        assert_eq!(plan.phases.len(), 6);
        assert!(
            plan.warnings
                .iter()
                .any(|warning| warning.contains("Encryption"))
        );

        let bootstrap = plan
            .phases
            .iter()
            .find(|phase| phase.id == InstallPhaseId::Bootstrap)
            .unwrap();
        let command = bootstrap.steps[0].command.as_ref().unwrap();

        assert!(command.contains(&"pacstrap".to_string()));
        assert!(command.contains(&"amd-ucode".to_string()));
        assert!(command.contains(&"nvidia".to_string()));
    }

    #[test]
    fn rejects_hosts_without_install_params() {
        let mut host = test_host();
        host.install_params = None;

        let result = InstallPlan::from_host(&host, "/mnt");

        assert!(result.is_err());
    }

    #[test]
    fn dry_run_emits_completed_event_without_executing_commands() {
        let plan = InstallPlan::from_host(&test_host(), "/mnt").unwrap();
        let mock = MockCommandExecutor::new();
        mock.set_default_response(MockResponse::success("ok"));
        let runner = InstallRunner::new(mock);

        let events = runner.run(&plan, &InstallRunConfig::default());

        assert!(matches!(
            events.first(),
            Some(InstallEvent::PlanStarted { .. })
        ));
        assert!(matches!(events.last(), Some(InstallEvent::PlanCompleted)));
        assert!(
            events
                .iter()
                .any(|event| matches!(event, InstallEvent::CommandPreview { .. }))
        );
    }

    #[test]
    fn execute_blocks_destructive_steps_without_confirmation() {
        let plan = InstallPlan::from_host(&test_host(), "/mnt").unwrap();
        let mock = MockCommandExecutor::new();
        mock.set_default_response(MockResponse::success("ok"));
        let runner = InstallRunner::new(mock);

        let events = runner.run(
            &plan,
            &InstallRunConfig {
                mode: InstallRunMode::Execute,
                ..InstallRunConfig::default()
            },
        );

        assert!(
            events
                .iter()
                .any(|event| matches!(event, InstallEvent::StepBlocked { .. }))
        );
        assert!(!matches!(events.last(), Some(InstallEvent::PlanCompleted)));
    }

    #[test]
    fn execute_stops_on_command_failure() {
        let plan = InstallPlan::from_host(&test_host(), "/mnt").unwrap();
        let mock = MockCommandExecutor::new();
        mock.set_default_response(MockResponse::success("ok"));
        mock.add_response(
            "test",
            &["-d", "/sys/firmware/efi/efivars"],
            MockResponse::exit_error(1, "not uefi"),
        );
        let runner = InstallRunner::new(mock);

        let events = runner.run(
            &plan,
            &InstallRunConfig {
                mode: InstallRunMode::Execute,
                allow_destructive: true,
                confirm_manual: true,
                only_phase: Some(InstallPhaseId::Preflight),
                from_phase: None,
            },
        );

        assert!(
            events
                .iter()
                .any(|event| matches!(event, InstallEvent::StepFailed { step_id, .. } if step_id == "check-uefi"))
        );
        assert!(!matches!(events.last(), Some(InstallEvent::PlanCompleted)));
    }

    #[test]
    fn renders_reviewable_shell_script() {
        let plan = InstallPlan::from_host(&test_host(), "/mnt").unwrap();
        let script = plan.to_shell_script();

        assert!(script.contains("set -Eeuo pipefail"));
        assert!(script.contains("IRON_BIN="));
        assert!(script.contains("IRON_CONFIG_SRC="));
        assert!(script.contains("interactive_menu"));
        assert!(script.contains("Type INSTALL to continue"));
        assert!(script.contains("confirm"));
        assert!(script.contains("--list-phases"));
        assert!(script.contains("pacstrap -K /mnt base linux linux-firmware"));
        assert!(script.contains("install -Dm755"));
        assert!(script.contains("/opt/iron-config"));
        assert!(script.contains("run_step arch-chroot /mnt bootctl install"));
        assert!(script.contains("manual_step 'Manual checkpoint complete: Review partition table"));
    }

    #[test]
    fn rejects_relative_target_mount() {
        let result = InstallPlan::from_host(&test_host(), "mnt");

        assert!(result.is_err());
    }

    #[test]
    fn rejects_missing_root_partition() {
        let mut host = test_host();
        let install = host.install_params.as_mut().unwrap();
        install
            .partitions
            .retain(|partition| partition.mount_point != "/");

        let result = InstallPlan::from_host(&host, "/mnt");

        assert!(result.is_err());
    }

    #[test]
    fn mounts_root_before_nested_mountpoints() {
        let plan = InstallPlan::from_host(&test_host(), "/mnt").unwrap();
        let disk = plan
            .phases
            .iter()
            .find(|phase| phase.id == InstallPhaseId::Disk)
            .unwrap();

        let mount_root = disk
            .steps
            .iter()
            .position(|step| step.id == "mount-root")
            .unwrap();
        let mount_boot = disk
            .steps
            .iter()
            .position(|step| step.id == "mount-boot")
            .unwrap();

        assert!(mount_root < mount_boot);
    }
}
