//! Host management - Hardware catalog and system configuration

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// Represents a physical or virtual machine
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Host {
    /// Unique identifier for this host
    pub id: String,

    /// Human-readable name
    pub name: String,

    /// Optional description
    pub description: Option<String>,

    /// Hardware specifications
    pub hardware: HardwareSpec,

    /// Arch installation parameters for recovery
    pub install_params: Option<InstallParams>,

    /// Installed bundles on this host
    pub installed_bundles: Vec<String>,

    /// Currently active bundle
    pub active_bundle: Option<String>,

    // ── F1-001: Desired-state fields (source of truth) ──────────────
    /// F1-001: Declared bundle for this host (desired state)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bundle: Option<String>,

    /// F1-001: Declared profile for this host (desired state)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub profile: Option<String>,

    /// F1-001: Extra modules beyond what the profile includes
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub extra_modules: Vec<String>,

    /// F1-001: Template variables for this host
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub variables: HashMap<String, String>,
}

/// Hardware specifications for a host
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct HardwareSpec {
    /// CPU model/vendor
    pub cpu: Option<String>,

    /// GPU model/vendor
    pub gpu: Option<String>,

    /// RAM in MB
    pub ram_mb: Option<u64>,

    /// Monitor configurations
    #[serde(default)]
    pub monitors: Vec<MonitorConfig>,

    /// Machine chassis type
    pub chassis: Option<ChassisType>,
}

/// Monitor configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitorConfig {
    /// Output name (e.g., "DP-1", "HDMI-1")
    pub output: String,

    /// Resolution (e.g., "2560x1440")
    pub resolution: String,

    /// Refresh rate in Hz
    pub refresh_rate: Option<u32>,

    /// Scale factor
    pub scale: Option<f32>,
}

/// Machine chassis type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ChassisType {
    Desktop,
    Laptop,
    Server,
    Tablet,
    Convertible,
    Unknown,
}

/// Arch Linux installation parameters for recovery
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallParams {
    /// Partition scheme
    pub partitions: Vec<PartitionConfig>,

    /// Bootloader type
    pub bootloader: BootloaderType,

    /// Kernel package
    pub kernel: String,

    /// Microcode package (intel-ucode, amd-ucode)
    pub microcode: Option<String>,

    /// GPU drivers
    pub gpu_drivers: Vec<String>,

    /// Filesystem type
    pub filesystem: String,

    /// Encryption enabled
    pub encrypted: bool,
}

/// Partition configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartitionConfig {
    pub device: String,
    pub mount_point: String,
    pub filesystem: String,
    pub size: String,
}

/// Bootloader type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BootloaderType {
    SystemdBoot,
    Grub,
    RefindBoot,
}

impl Host {
    /// Load host configuration from a directory
    pub fn load(path: &Path) -> anyhow::Result<Self> {
        let config_path = path.join("host.toml");
        let content = std::fs::read_to_string(&config_path)?;
        let host: Host = toml::from_str(&content)?;
        Ok(host)
    }

    /// Save host configuration to a directory
    pub fn save(&self, path: &Path) -> anyhow::Result<()> {
        let config_path = path.join("host.toml");
        let content = toml::to_string_pretty(self)?;
        std::fs::write(config_path, content)?;
        Ok(())
    }

    /// Check if host has a system snapshot
    pub fn has_snapshot(&self) -> bool {
        // TODO: Check timeshift/snapper for snapshots
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_host() -> Host {
        Host {
            id: "desktop".to_string(),
            name: "Desktop Workstation".to_string(),
            description: Some("Main development machine".to_string()),
            hardware: HardwareSpec {
                cpu: Some("AMD Ryzen 9 7950X".to_string()),
                gpu: Some("NVIDIA RTX 4080".to_string()),
                ram_mb: Some(65536),
                monitors: vec![MonitorConfig {
                    output: "DP-1".to_string(),
                    resolution: "2560x1440".to_string(),
                    refresh_rate: Some(165),
                    scale: Some(1.0),
                }],
                chassis: Some(ChassisType::Desktop),
            },
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
                gpu_drivers: vec!["nvidia".to_string(), "nvidia-utils".to_string()],
                filesystem: "btrfs".to_string(),
                encrypted: false,
            }),
            installed_bundles: vec!["hyprland".to_string(), "niri".to_string()],
            active_bundle: Some("hyprland".to_string()),
            bundle: None,
            profile: None,
            extra_modules: vec![],
            variables: HashMap::new(),
        }
    }

    #[test]
    fn test_host_creation() {
        let host = create_test_host();
        assert_eq!(host.id, "desktop");
        assert_eq!(host.name, "Desktop Workstation");
        assert!(host.description.is_some());
    }

    #[test]
    fn test_hardware_spec_default() {
        let spec = HardwareSpec::default();
        assert!(spec.cpu.is_none());
        assert!(spec.gpu.is_none());
        assert!(spec.ram_mb.is_none());
        assert!(spec.monitors.is_empty());
        assert!(spec.chassis.is_none());
    }

    #[test]
    fn test_chassis_type_variants() {
        let types = vec![
            ChassisType::Desktop,
            ChassisType::Laptop,
            ChassisType::Server,
            ChassisType::Tablet,
            ChassisType::Convertible,
            ChassisType::Unknown,
        ];

        for chassis in types {
            // All variants should be debuggable
            assert!(!format!("{:?}", chassis).is_empty());
        }
    }

    #[test]
    fn test_bootloader_type_variants() {
        let bootloaders = vec![
            BootloaderType::SystemdBoot,
            BootloaderType::Grub,
            BootloaderType::RefindBoot,
        ];

        for bootloader in bootloaders {
            assert!(!format!("{:?}", bootloader).is_empty());
        }
    }

    #[test]
    fn test_monitor_config() {
        let monitor = MonitorConfig {
            output: "HDMI-1".to_string(),
            resolution: "1920x1080".to_string(),
            refresh_rate: Some(60),
            scale: Some(1.25),
        };

        assert_eq!(monitor.output, "HDMI-1");
        assert_eq!(monitor.resolution, "1920x1080");
        assert_eq!(monitor.refresh_rate, Some(60));
        assert_eq!(monitor.scale, Some(1.25));
    }

    #[test]
    fn test_partition_config() {
        let partition = PartitionConfig {
            device: "/dev/sda1".to_string(),
            mount_point: "/home".to_string(),
            filesystem: "ext4".to_string(),
            size: "500G".to_string(),
        };

        assert_eq!(partition.device, "/dev/sda1");
        assert_eq!(partition.mount_point, "/home");
    }

    #[test]
    fn test_host_save_and_load() {
        let temp_dir = TempDir::new().unwrap();
        let host = create_test_host();

        // Save
        host.save(temp_dir.path()).unwrap();

        // Verify file exists
        let config_path = temp_dir.path().join("host.toml");
        assert!(config_path.exists());

        // Load and verify
        let loaded = Host::load(temp_dir.path()).unwrap();
        assert_eq!(loaded.id, host.id);
        assert_eq!(loaded.name, host.name);
        assert_eq!(loaded.hardware.cpu, host.hardware.cpu);
        assert_eq!(loaded.installed_bundles, host.installed_bundles);
    }

    #[test]
    fn test_host_toml_roundtrip() {
        let host = create_test_host();
        let serialized = toml::to_string_pretty(&host).unwrap();
        let deserialized: Host = toml::from_str(&serialized).unwrap();

        assert_eq!(deserialized.id, host.id);
        assert_eq!(deserialized.hardware.ram_mb, host.hardware.ram_mb);
    }

    #[test]
    fn test_host_minimal() {
        let minimal = Host {
            id: "minimal".to_string(),
            name: "Minimal Host".to_string(),
            description: None,
            hardware: HardwareSpec::default(),
            install_params: None,
            installed_bundles: vec![],
            active_bundle: None,
            bundle: None,
            profile: None,
            extra_modules: vec![],
            variables: HashMap::new(),
        };

        assert!(minimal.description.is_none());
        assert!(minimal.hardware.cpu.is_none());
        assert!(minimal.install_params.is_none());
        assert!(minimal.active_bundle.is_none());
        assert!(minimal.bundle.is_none());
        assert!(minimal.profile.is_none());
        assert!(minimal.extra_modules.is_empty());
        assert!(minimal.variables.is_empty());
    }

    #[test]
    fn test_has_snapshot_default() {
        let host = create_test_host();
        // Default implementation returns false
        assert!(!host.has_snapshot());
    }

    #[test]
    fn test_install_params_encryption() {
        let host = create_test_host();
        let params = host.install_params.unwrap();

        assert!(!params.encrypted);
        assert_eq!(params.kernel, "linux");
        assert_eq!(params.filesystem, "btrfs");
    }

    #[test]
    fn test_host_load_missing_file() {
        let temp_dir = TempDir::new().unwrap();
        let result = Host::load(temp_dir.path());
        assert!(result.is_err());
    }

    // ── F1-001: Desired-state fields tests ──────────────────────────

    #[test]
    fn test_host_backward_compat_no_new_fields() {
        let toml_str = r#"
            id = "legacy"
            name = "Legacy Host"
            installed_bundles = []
            [hardware]
        "#;
        let host: Host = toml::from_str(toml_str).unwrap();
        assert!(host.bundle.is_none());
        assert!(host.profile.is_none());
        assert!(host.extra_modules.is_empty());
        assert!(host.variables.is_empty());
    }

    #[test]
    fn test_host_roundtrip_with_desired_state() {
        let mut host = create_test_host();
        host.bundle = Some("hyprland".to_string());
        host.profile = Some("developer".to_string());
        host.extra_modules = vec!["gaming".to_string(), "vm-tools".to_string()];
        host.variables = HashMap::from([
            ("terminal".to_string(), "kitty".to_string()),
            ("primary_monitor".to_string(), "DP-1".to_string()),
        ]);

        let serialized = toml::to_string_pretty(&host).unwrap();
        let deserialized: Host = toml::from_str(&serialized).unwrap();

        assert_eq!(deserialized.bundle, Some("hyprland".to_string()));
        assert_eq!(deserialized.profile, Some("developer".to_string()));
        assert_eq!(deserialized.extra_modules, vec!["gaming", "vm-tools"]);
        assert_eq!(
            deserialized.variables.get("terminal"),
            Some(&"kitty".to_string())
        );
    }

    #[test]
    fn test_host_desired_state_save_load() {
        let temp_dir = TempDir::new().unwrap();
        let mut host = create_test_host();
        host.bundle = Some("niri".to_string());
        host.profile = Some("minimal".to_string());
        host.extra_modules = vec!["ssh-config".to_string()];
        host.variables = HashMap::from([("browser".to_string(), "firefox".to_string())]);

        host.save(temp_dir.path()).unwrap();
        let loaded = Host::load(temp_dir.path()).unwrap();

        assert_eq!(loaded.bundle, Some("niri".to_string()));
        assert_eq!(loaded.profile, Some("minimal".to_string()));
        assert_eq!(loaded.extra_modules, vec!["ssh-config"]);
        assert_eq!(
            loaded.variables.get("browser"),
            Some(&"firefox".to_string())
        );
    }

    #[test]
    fn test_host_skip_serializing_empty_fields() {
        let host = create_test_host(); // bundle/profile/extra_modules/variables are all empty/None
        let serialized = toml::to_string_pretty(&host).unwrap();
        // Empty optional/vec/map fields should NOT appear in TOML
        // Use line-start matching to avoid matching 'active_bundle'
        assert!(
            !serialized.lines().any(|l| l.starts_with("bundle ")),
            "bundle field should be skipped when None"
        );
        assert!(
            !serialized.lines().any(|l| l.starts_with("profile ")),
            "profile field should be skipped when None"
        );
        assert!(
            !serialized.contains("extra_modules"),
            "extra_modules should be skipped when empty"
        );
        assert!(
            !serialized.contains("[variables]"),
            "variables should be skipped when empty"
        );
    }
}
