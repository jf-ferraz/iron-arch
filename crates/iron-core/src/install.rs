//! Arch install planning.
//!
//! This module turns Iron host configuration into a typed, reviewable install
//! plan. Execution is intentionally separate so destructive operations can be
//! guarded and tested independently.

use crate::host::{BootloaderType, Host, InstallParams, PartitionConfig};
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
IRON_BIN="${IRON_BIN:-$(command -v iron || true)}"

trap 'echo "[ERROR] line ${LINENO}: ${BASH_COMMAND}" | tee -a "$LOG_FILE"' ERR

log() {
  printf '[%s] %s\n' "$(date -Is)" "$*" | tee -a "$LOG_FILE"
}

confirm() {
  local prompt="$1"
  if [[ "$ASSUME_YES" == "true" ]]; then
    log "Auto-confirmed: $prompt"
    return 0
  fi
  read -r -p "$prompt [y/N] " response
  [[ "$response" =~ ^[Yy]$ ]]
}

run_step() {
  log "RUN: $*"
  "$@" 2>&1 | tee -a "$LOG_FILE"
}

run_shell() {
  log "RUN: $*"
  bash -lc "$*" 2>&1 | tee -a "$LOG_FILE"
}

manual_step() {
  local prompt="$1"
  confirm "$prompt" || {
    log "Skipped/manual checkpoint not confirmed: $prompt"
    return 1
  }
}

"#,
        );
        script.push_str(&format!("# Iron install plan for {}\n", self.host_id));
        script.push_str(&format!("TARGET_MOUNT=\"{}\"\n\n", self.target_mount));

        for warning in &self.warnings {
            script.push_str(&format!(
                "echo 'WARNING: {}'\n",
                shell_escape_single(warning)
            ));
        }
        script.push('\n');

        for phase in &self.phases {
            script.push_str(&format!(
                "\nlog '==> {}'\n",
                shell_escape_single(&phase.name)
            ));
            for step in &phase.steps {
                script.push_str(&format!(
                    "log ' -> {}'\n",
                    shell_escape_single(&step.description)
                ));
                if let Some(command) = &step.command {
                    if step.destructive {
                        script.push_str(&format!(
                            "confirm '{}' || exit 1\n",
                            shell_escape_single(&format!(
                                "Run destructive step '{}'?",
                                step.description
                            ))
                        ));
                    }
                    script.push_str(&render_script_command(command));
                    script.push('\n');
                } else {
                    script.push_str(&format!(
                        "manual_step '{}'\n",
                        shell_escape_single(&format!(
                            "Manual checkpoint complete: {}?",
                            step.description
                        ))
                    ));
                }
            }
        }

        script
    }
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
                &format!("create-mountpoint-{}", mountpoint_id(&partition.mount_point)),
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
        manual_step(
            "copy-iron-config",
            "Copy or clone Iron configuration into the installed system",
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
            bundle: None,
            profile: None,
            extra_modules: Vec::new(),
            variables: Default::default(),
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
    fn renders_reviewable_shell_script() {
        let plan = InstallPlan::from_host(&test_host(), "/mnt").unwrap();
        let script = plan.to_shell_script();

        assert!(script.contains("set -Eeuo pipefail"));
        assert!(script.contains("IRON_BIN="));
        assert!(script.contains("confirm"));
        assert!(script.contains("pacstrap -K /mnt base linux linux-firmware"));
        assert!(script.contains("install -Dm755"));
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
