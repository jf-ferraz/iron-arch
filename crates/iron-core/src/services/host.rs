//! Host Service - Hardware detection and host management
//!
//! Provides hardware detection (CPU, GPU, RAM, monitors) and host configuration.

use crate::host::{ChassisType, HardwareSpec, Host, MonitorConfig};
use crate::{IronResult, StateError};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Host service trait
pub trait HostService {
    /// Detect hardware specifications
    fn detect_hardware(&self) -> IronResult<HardwareSpec>;

    /// Detect chassis type (laptop, desktop, etc.)
    fn detect_chassis(&self) -> ChassisType;

    /// Detect connected monitors
    fn detect_monitors(&self) -> IronResult<Vec<MonitorConfig>>;

    /// Get current hostname
    fn hostname(&self) -> IronResult<String>;

    /// Load host configuration
    fn load_host(&self, id: &str) -> IronResult<Host>;

    /// Save host configuration
    fn save_host(&self, host: &Host) -> IronResult<()>;

    /// List all hosts
    fn list_hosts(&self) -> IronResult<Vec<Host>>;

    /// Find host by hostname
    fn find_by_hostname(&self, hostname: &str) -> IronResult<Option<Host>>;

    /// Create a new host from current hardware
    fn create_from_current(&self, id: &str, name: &str) -> IronResult<Host>;
}

/// Default host service implementation
pub struct DefaultHostService {
    /// Hosts directory
    hosts_dir: PathBuf,
}

impl DefaultHostService {
    /// Create a new host service
    pub fn new(iron_root: &Path) -> Self {
        Self {
            hosts_dir: iron_root.join("hosts"),
        }
    }

    /// Get host file path
    fn host_path(&self, id: &str) -> PathBuf {
        self.hosts_dir.join(format!("{}.toml", id))
    }

    /// Read a file from /sys or /proc
    fn read_sys_file(&self, path: &str) -> Option<String> {
        fs::read_to_string(path).ok().map(|s| s.trim().to_string())
    }

    /// Run a command and capture output
    fn run_command(&self, cmd: &str, args: &[&str]) -> Option<String> {
        Command::new(cmd)
            .args(args)
            .output()
            .ok()
            .filter(|o| o.status.success())
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
    }
}

impl HostService for DefaultHostService {
    fn detect_hardware(&self) -> IronResult<HardwareSpec> {
        // Detect CPU
        let cpu = self.read_sys_file("/proc/cpuinfo").and_then(|info| {
            info.lines()
                .find(|l| l.starts_with("model name"))
                .and_then(|l| l.split(':').nth(1))
                .map(|s| s.trim().to_string())
        });

        // Detect GPU using lspci
        let gpu = self.run_command("lspci", &[]).and_then(|output| {
            output
                .lines()
                .find(|l| l.contains("VGA") || l.contains("3D"))
                .and_then(|l| l.split(':').next_back())
                .map(|s| s.trim().to_string())
        });

        // Detect RAM in MB
        let ram_mb = self
            .read_sys_file("/proc/meminfo")
            .and_then(|info| {
                info.lines()
                    .find(|l| l.starts_with("MemTotal"))
                    .and_then(|l| l.split_whitespace().nth(1))
                    .and_then(|s| s.parse::<u64>().ok())
            })
            .map(|kb| kb / 1024);

        // Detect chassis
        let chassis = Some(self.detect_chassis());

        // Detect monitors
        let monitors = self.detect_monitors().unwrap_or_default();

        Ok(HardwareSpec {
            cpu,
            gpu,
            ram_mb,
            monitors,
            chassis,
        })
    }

    fn detect_chassis(&self) -> ChassisType {
        // Read DMI chassis type
        if let Some(chassis) = self.read_sys_file("/sys/class/dmi/id/chassis_type") {
            match chassis.as_str() {
                "3" | "4" | "6" | "7" | "15" | "16" => return ChassisType::Desktop,
                "8" | "9" | "10" | "14" | "31" => return ChassisType::Laptop,
                "11" | "30" | "32" => return ChassisType::Tablet,
                "17" | "23" => return ChassisType::Server,
                "13" => return ChassisType::Convertible,
                _ => {}
            }
        }

        // Fallback: check for battery (laptop indicator)
        if Path::new("/sys/class/power_supply/BAT0").exists()
            || Path::new("/sys/class/power_supply/BAT1").exists()
        {
            return ChassisType::Laptop;
        }

        ChassisType::Unknown
    }

    fn detect_monitors(&self) -> IronResult<Vec<MonitorConfig>> {
        let mut monitors = Vec::new();

        // Try wlr-randr for Wayland
        if let Some(output) = self.run_command("wlr-randr", &[]) {
            let mut current_output = String::new();
            let mut current_resolution = String::new();
            let mut current_refresh: Option<u32> = None;
            let mut current_scale: Option<f32> = None;

            for line in output.lines() {
                let line = line.trim();
                if !line.starts_with(' ') && !line.is_empty() {
                    // Save previous monitor
                    if !current_output.is_empty() {
                        monitors.push(MonitorConfig {
                            output: current_output.clone(),
                            resolution: current_resolution.clone(),
                            refresh_rate: current_refresh,
                            scale: current_scale,
                        });
                    }
                    current_output = line.split_whitespace().next().unwrap_or("").to_string();
                    current_resolution = String::new();
                    current_refresh = Some(60);
                    current_scale = Some(1.0);
                } else if line.contains("current") {
                    // Parse resolution and refresh
                    if let Some(res) = line.split_whitespace().next() {
                        current_resolution = res.to_string();
                    }
                    if line.contains("Hz")
                        && let Some(hz) = line.split_whitespace().find(|s| s.ends_with("Hz"))
                        && let Ok(rate) = hz.trim_end_matches("Hz").parse::<f32>()
                    {
                        current_refresh = Some(rate as u32);
                    }
                } else if line.starts_with("Scale:")
                    && let Some(scale) = line.split(':').nth(1)
                    && let Ok(s) = scale.trim().parse()
                {
                    current_scale = Some(s);
                }
            }

            // Save last monitor
            if !current_output.is_empty() {
                monitors.push(MonitorConfig {
                    output: current_output,
                    resolution: current_resolution,
                    refresh_rate: current_refresh,
                    scale: current_scale,
                });
            }
        }

        // Fallback to xrandr for X11
        if monitors.is_empty()
            && let Some(output) = self.run_command("xrandr", &["--query"])
        {
            for line in output.lines() {
                if line.contains(" connected") {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if !parts.is_empty() {
                        let output_name = parts[0].to_string();
                        let resolution = parts
                            .iter()
                            .find(|s| {
                                s.contains('x')
                                    && s.chars().all(|c| c.is_numeric() || c == 'x' || c == '+')
                            })
                            .map(|s| s.split('+').next().unwrap_or(*s))
                            .unwrap_or("unknown")
                            .to_string();

                        monitors.push(MonitorConfig {
                            output: output_name,
                            resolution,
                            refresh_rate: Some(60),
                            scale: Some(1.0),
                        });
                    }
                }
            }
        }

        Ok(monitors)
    }

    fn hostname(&self) -> IronResult<String> {
        self.read_sys_file("/etc/hostname")
            .or_else(|| self.run_command("hostname", &[]))
            .ok_or_else(|| crate::IronError::OperationFailed {
                message: "Could not determine hostname".to_string(),
            })
    }

    fn load_host(&self, id: &str) -> IronResult<Host> {
        let path = self.host_path(id);
        if !path.exists() {
            return Err(StateError::HostNotFound { id: id.to_string() }.into());
        }

        let content = fs::read_to_string(&path)
            .map_err(|_| StateError::HostNotFound { id: id.to_string() })?;

        toml::from_str(&content).map_err(|e| {
            crate::ConfigError::ParseError {
                path,
                message: e.to_string(),
            }
            .into()
        })
    }

    fn save_host(&self, host: &Host) -> IronResult<()> {
        fs::create_dir_all(&self.hosts_dir).ok();
        let path = self.host_path(&host.id);
        let content = toml::to_string_pretty(host).map_err(|e| crate::ConfigError::ParseError {
            path: path.clone(),
            message: e.to_string(),
        })?;

        fs::write(&path, content).map_err(|_| crate::FsError::PermissionDenied { path })?;

        Ok(())
    }

    fn list_hosts(&self) -> IronResult<Vec<Host>> {
        let mut hosts = Vec::new();

        if self.hosts_dir.exists() {
            for entry in fs::read_dir(&self.hosts_dir)
                .into_iter()
                .flatten()
                .flatten()
            {
                if entry
                    .path()
                    .extension()
                    .map(|e| e == "toml")
                    .unwrap_or(false)
                    && let Some(id) = entry.path().file_stem().and_then(|s| s.to_str())
                    && let Ok(host) = self.load_host(id)
                {
                    hosts.push(host);
                }
            }
        }

        Ok(hosts)
    }

    fn find_by_hostname(&self, hostname: &str) -> IronResult<Option<Host>> {
        let hosts = self.list_hosts()?;
        Ok(hosts
            .into_iter()
            .find(|h| h.id == hostname || h.name == hostname))
    }

    fn create_from_current(&self, id: &str, name: &str) -> IronResult<Host> {
        let hardware = self.detect_hardware()?;

        let host = Host {
            id: id.to_string(),
            name: name.to_string(),
            description: None,
            hardware,
            install_params: None,
            installed_bundles: vec![],
            active_bundle: None,
        };

        self.save_host(&host)?;

        Ok(host)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_service() -> (DefaultHostService, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let service = DefaultHostService::new(temp_dir.path());
        (service, temp_dir)
    }

    #[test]
    fn test_detect_chassis() {
        let (service, _temp) = create_test_service();
        let chassis = service.detect_chassis();
        // Just verify it returns a valid variant
        let _ = format!("{:?}", chassis);
    }

    #[test]
    fn test_host_save_load() {
        let (service, _temp) = create_test_service();

        let host = Host {
            id: "test-host".to_string(),
            name: "Test Host".to_string(),
            description: None,
            hardware: HardwareSpec::default(),
            install_params: None,
            installed_bundles: vec![],
            active_bundle: None,
        };

        service.save_host(&host).unwrap();
        let loaded = service.load_host("test-host").unwrap();

        assert_eq!(loaded.id, "test-host");
        assert_eq!(loaded.name, "Test Host");
    }

    #[test]
    fn test_list_hosts() {
        let (service, _temp) = create_test_service();

        let host1 = Host {
            id: "host1".to_string(),
            name: "Host 1".to_string(),
            description: None,
            hardware: HardwareSpec::default(),
            install_params: None,
            installed_bundles: vec![],
            active_bundle: None,
        };

        let host2 = Host {
            id: "host2".to_string(),
            name: "Host 2".to_string(),
            description: None,
            hardware: HardwareSpec::default(),
            install_params: None,
            installed_bundles: vec![],
            active_bundle: None,
        };

        service.save_host(&host1).unwrap();
        service.save_host(&host2).unwrap();

        let hosts = service.list_hosts().unwrap();
        assert_eq!(hosts.len(), 2);
    }

    #[test]
    fn test_find_by_hostname() {
        let (service, _temp) = create_test_service();

        let host = Host {
            id: "myhost".to_string(),
            name: "My Host".to_string(),
            description: None,
            hardware: HardwareSpec::default(),
            install_params: None,
            installed_bundles: vec![],
            active_bundle: None,
        };

        service.save_host(&host).unwrap();

        // Find by ID
        let found = service.find_by_hostname("myhost").unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().id, "myhost");

        // Not found
        let found = service.find_by_hostname("unknown").unwrap();
        assert!(found.is_none());
    }
}
