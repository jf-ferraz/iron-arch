//! Host management - Hardware catalog and system configuration

use serde::{Deserialize, Serialize};
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
}

/// Hardware specifications for a host
#[derive(Debug, Clone, Serialize, Deserialize)]
#[derive(Default)]
pub struct HardwareSpec {
    /// CPU model/vendor
    pub cpu: Option<String>,

    /// GPU model/vendor
    pub gpu: Option<String>,

    /// RAM in MB
    pub ram_mb: Option<u64>,

    /// Monitor configurations
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
#[derive(Debug, Clone, Serialize, Deserialize)]
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

