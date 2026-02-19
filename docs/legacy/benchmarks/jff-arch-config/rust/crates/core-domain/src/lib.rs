use std::collections::HashSet;
use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use toml::Value;
use walkdir::WalkDir;

pub mod state;
pub mod module_toggle;
pub mod maintenance;
pub mod hooks;

pub use state::{
    ActiveModules, HookBehavior, HookHashes, HookState, MaintenanceRecord, MaintenanceState,
    ModuleState, OperationRecord, ProcessingMode, ThemeDescriptor, ThemeState,
};
pub use module_toggle::{
    apply_toggles, get_activation_stats, get_all_modules_status, prepare_toggles, ModuleToggle,
};
pub use maintenance::{
    format_time_ago, get_maintenance_statuses, record_maintenance_operation, time_since_last,
    MaintenanceStatus,
};
pub use hooks::{
    execute_hook, list_tracked_hooks, reset_hook, set_hook_behavior, HookTracker,
};

#[derive(Debug, Clone)]
pub struct ModuleId(pub String);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
}

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Severity::Error => write!(f, "error"),
            Severity::Warning => write!(f, "warning"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ValidationIssue {
    pub severity: Severity,
    pub path: PathBuf,
    pub message: String,
}

#[derive(Debug, Clone)]
pub struct ValidationReport {
    pub files_checked: usize,
    pub issues: Vec<ValidationIssue>,
}

impl ValidationReport {
    pub fn error_count(&self) -> usize {
        self.issues
            .iter()
            .filter(|i| i.severity == Severity::Error)
            .count()
    }

    pub fn warning_count(&self) -> usize {
        self.issues
            .iter()
            .filter(|i| i.severity == Severity::Warning)
            .count()
    }

    pub fn ok(&self) -> bool {
        self.error_count() == 0
    }
}

#[derive(Debug, Clone)]
pub struct StatusSummary {
    pub root: PathBuf,
    pub manifest_count: usize,
    pub module_count: usize,
    pub default_host: Option<String>,
    pub active_host: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ModuleOperation {
    pub id: String,
    pub description: String,
    pub command: String,
}

#[derive(Debug, Clone)]
pub struct ModuleDescriptor {
    pub id: String,
    pub name: String,
    pub kind: String,
    pub status: String,
    pub manifest_path: PathBuf,
    pub operations: Vec<ModuleOperation>,
}

#[derive(Debug, Clone)]
pub struct PlannedOperation {
    pub order: usize,
    pub module_id: String,
    pub operation_id: String,
    pub command: String,
}

#[derive(Debug, Clone)]
pub struct ServiceEntry {
    pub id: String,
    pub service_type: String,
    pub unit: Option<String>,
    pub command: Option<String>,
    pub enabled: bool,
}

pub fn validate_module_id(id: &str) -> bool {
    !id.trim().is_empty() && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
}

pub fn validate_repository(root: &Path) -> io::Result<ValidationReport> {
    let mut report = ValidationReport {
        files_checked: 0,
        issues: Vec::new(),
    };

    let manifest_dir = root.join("app/manifests");
    let module_root = root.join("modules");

    if !manifest_dir.is_dir() {
        report.issues.push(ValidationIssue {
            severity: Severity::Error,
            path: manifest_dir,
            message: "missing directory app/manifests".to_string(),
        });
    } else {
        for entry in fs::read_dir(&manifest_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("toml") {
                report.files_checked += 1;
                validate_manifest_file(root, &path, &mut report);
            }
        }
    }

    if !module_root.is_dir() {
        report.issues.push(ValidationIssue {
            severity: Severity::Error,
            path: module_root,
            message: "missing directory modules".to_string(),
        });
    } else {
        for entry in WalkDir::new(&module_root)
            .into_iter()
            .filter_map(Result::ok)
        {
            if entry.file_type().is_file() && entry.file_name() == "module.toml" {
                let path = entry.into_path();
                report.files_checked += 1;
                validate_module_file(root, &path, &mut report);
            }
        }
    }

    Ok(report)
}

pub fn get_status_summary(root: &Path) -> io::Result<StatusSummary> {
    let manifest_dir = root.join("app/manifests");
    let module_root = root.join("modules");

    let manifest_count = if manifest_dir.is_dir() {
        fs::read_dir(&manifest_dir)?
            .filter_map(Result::ok)
            .filter(|e| e.path().extension().and_then(|s| s.to_str()) == Some("toml"))
            .count()
    } else {
        0
    };

    let module_count = if module_root.is_dir() {
        WalkDir::new(&module_root)
            .into_iter()
            .filter_map(Result::ok)
            .filter(|e| e.file_type().is_file() && e.file_name() == "module.toml")
            .count()
    } else {
        0
    };

    let default_host = read_default_host(root);
    let active_host = read_active_host(root);

    Ok(StatusSummary {
        root: root.to_path_buf(),
        manifest_count,
        module_count,
        default_host,
        active_host,
    })
}

pub fn list_hosts(root: &Path) -> io::Result<Vec<String>> {
    let value = read_hosts_manifest(root)?;
    let mut hosts = Vec::new();
    if let Some(arr) = value.get("hosts").and_then(Value::as_array) {
        for entry in arr {
            if let Some(id) = entry.get("id").and_then(Value::as_str) {
                hosts.push(id.to_string());
            }
        }
    }
    Ok(hosts)
}

pub fn get_default_host(root: &Path) -> io::Result<Option<String>> {
    let value = read_hosts_manifest(root)?;
    Ok(value
        .get("default_host")
        .and_then(Value::as_str)
        .map(ToString::to_string))
}

pub fn get_active_host(root: &Path) -> io::Result<Option<String>> {
    let path = active_host_path(root);
    match fs::read_to_string(path) {
        Ok(raw) => {
            let trimmed = raw.trim();
            if trimmed.is_empty() {
                Ok(None)
            } else {
                Ok(Some(trimmed.to_string()))
            }
        }
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e),
    }
}

pub fn set_active_host(root: &Path, host_id: &str) -> io::Result<()> {
    let hosts = list_hosts(root)?;
    if !hosts.iter().any(|h| h == host_id) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("unknown host id '{host_id}'"),
        ));
    }

    let path = active_host_path(root);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, format!("{host_id}\n"))
}

pub fn load_modules(root: &Path) -> io::Result<Vec<ModuleDescriptor>> {
    let module_root = root.join("modules");
    if !module_root.is_dir() {
        return Ok(Vec::new());
    }

    let mut modules = Vec::new();
    for entry in WalkDir::new(&module_root)
        .into_iter()
        .filter_map(Result::ok)
    {
        if entry.file_type().is_file() && entry.file_name() == "module.toml" {
            let path = entry.into_path();
            let raw = fs::read_to_string(&path)?;
            let value: Value = raw.parse().map_err(|e| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("failed to parse {}: {e}", path.display()),
                )
            })?;
            let table = value.as_table().ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("{} must be a TOML table", path.display()),
                )
            })?;

            let id = table
                .get("id")
                .and_then(Value::as_str)
                .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "module id missing"))?
                .to_string();
            let name = table
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or(&id)
                .to_string();
            let kind = table
                .get("kind")
                .and_then(Value::as_str)
                .unwrap_or("unknown")
                .to_string();
            let status = table
                .get("status")
                .and_then(Value::as_str)
                .unwrap_or("unknown")
                .to_string();

            let mut operations = Vec::new();
            if let Some(ops) = table.get("operations").and_then(Value::as_array) {
                for op in ops {
                    if let Some(op_table) = op.as_table() {
                        let Some(op_id) = op_table.get("id").and_then(Value::as_str) else {
                            continue;
                        };
                        let Some(command) = op_table.get("command").and_then(Value::as_str) else {
                            continue;
                        };
                        let description = op_table
                            .get("description")
                            .and_then(Value::as_str)
                            .unwrap_or("")
                            .to_string();
                        operations.push(ModuleOperation {
                            id: op_id.to_string(),
                            description,
                            command: command.to_string(),
                        });
                    }
                }
            }

            modules.push(ModuleDescriptor {
                id,
                name,
                kind,
                status,
                manifest_path: relative_or_absolute(root, &path),
                operations,
            });
        }
    }

    modules.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(modules)
}

pub fn build_plan(
    root: &Path,
    module_filter: Option<&str>,
    operation_filter: Option<&str>,
) -> io::Result<Vec<PlannedOperation>> {
    let modules = load_modules(root)?;
    let mut planned = Vec::new();
    let mut order = 1usize;

    for module in modules {
        if module_filter.is_some() && module_filter != Some(module.id.as_str()) {
            continue;
        }

        for op in module.operations {
            if operation_filter.is_some() && operation_filter != Some(op.id.as_str()) {
                continue;
            }

            planned.push(PlannedOperation {
                order,
                module_id: module.id.clone(),
                operation_id: op.id,
                command: op.command,
            });
            order += 1;
        }
    }

    Ok(planned)
}

pub fn load_services(root: &Path) -> io::Result<Vec<ServiceEntry>> {
    let path = root.join("app/manifests/services.toml");
    let raw = fs::read_to_string(&path)?;
    let value: Value = raw.parse().map_err(|e| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("failed to parse {}: {e}", path.display()),
        )
    })?;
    let table = value.as_table().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            "services manifest must be a TOML table",
        )
    })?;

    let mut services = Vec::new();
    if let Some(items) = table.get("services").and_then(Value::as_array) {
        for item in items {
            let Some(t) = item.as_table() else {
                continue;
            };
            let Some(id) = t.get("id").and_then(Value::as_str) else {
                continue;
            };
            let service_type = t
                .get("type")
                .and_then(Value::as_str)
                .unwrap_or("unknown")
                .to_string();
            let unit = t
                .get("unit")
                .and_then(Value::as_str)
                .map(ToString::to_string);
            let command = t
                .get("command")
                .and_then(Value::as_str)
                .map(ToString::to_string);
            let enabled = t.get("enabled").and_then(Value::as_bool).unwrap_or(false);

            services.push(ServiceEntry {
                id: id.to_string(),
                service_type,
                unit,
                command,
                enabled,
            });
        }
    }

    Ok(services)
}

fn validate_manifest_file(root: &Path, path: &Path, report: &mut ValidationReport) {
    let Some(value) = parse_toml(root, path, report) else {
        return;
    };

    let Some(table) = value.as_table() else {
        report.issues.push(ValidationIssue {
            severity: Severity::Error,
            path: relative_or_absolute(root, path),
            message: "top-level TOML value must be a table".to_string(),
        });
        return;
    };

    match table.get("version") {
        Some(Value::Integer(_)) => {}
        _ => report.issues.push(ValidationIssue {
            severity: Severity::Error,
            path: relative_or_absolute(root, path),
            message: "missing or invalid integer key: version".to_string(),
        }),
    }

    if path.file_name().and_then(|n| n.to_str()) == Some("hosts.toml") {
        validate_hosts_manifest(root, path, table, report);
    }
    if path.file_name().and_then(|n| n.to_str()) == Some("services.toml") {
        validate_services_manifest(root, path, table, report);
    }
}

fn validate_module_file(root: &Path, path: &Path, report: &mut ValidationReport) {
    let Some(value) = parse_toml(root, path, report) else {
        return;
    };

    let Some(table) = value.as_table() else {
        report.issues.push(ValidationIssue {
            severity: Severity::Error,
            path: relative_or_absolute(root, path),
            message: "top-level TOML value must be a table".to_string(),
        });
        return;
    };

    validate_required_int(table, "version", root, path, report);

    let id = match table.get("id").and_then(Value::as_str) {
        Some(id) => {
            if !validate_module_id(id) {
                report.issues.push(ValidationIssue {
                    severity: Severity::Error,
                    path: relative_or_absolute(root, path),
                    message: format!("invalid module id '{id}' (allowed: [a-zA-Z0-9-])"),
                });
            }
            Some(id)
        }
        None => {
            report.issues.push(ValidationIssue {
                severity: Severity::Error,
                path: relative_or_absolute(root, path),
                message: "missing string key: id".to_string(),
            });
            None
        }
    };

    validate_required_string(table, "name", root, path, report);
    let kind = validate_required_string(table, "kind", root, path, report);
    let status = validate_required_string(table, "status", root, path, report);
    validate_required_string(table, "owner", root, path, report);

    if let Some(kind) = kind.as_deref() {
        let allowed = [
            "config",
            "service",
            "operation",
            "package",
            "backup",
            "theme",
        ];
        if !allowed.contains(&kind) {
            report.issues.push(ValidationIssue {
                severity: Severity::Error,
                path: relative_or_absolute(root, path),
                message: format!("invalid kind '{kind}' (allowed: {})", allowed.join(", ")),
            });
        }
    }

    if let Some(status) = status.as_deref() {
        let allowed = ["draft", "active", "deprecated"];
        if !allowed.contains(&status) {
            report.issues.push(ValidationIssue {
                severity: Severity::Error,
                path: relative_or_absolute(root, path),
                message: format!(
                    "invalid status '{status}' (allowed: {})",
                    allowed.join(", ")
                ),
            });
        }
    }

    if !matches!(table.get("paths"), Some(Value::Table(_))) {
        report.issues.push(ValidationIssue {
            severity: Severity::Error,
            path: relative_or_absolute(root, path),
            message: "missing table: [paths]".to_string(),
        });
    } else if let Some(paths_table) = table.get("paths").and_then(Value::as_table) {
        if let Some(root_path) = paths_table.get("root").and_then(Value::as_str) {
            let abs = root.join(root_path);
            if !abs.exists() {
                report.issues.push(ValidationIssue {
                    severity: Severity::Error,
                    path: relative_or_absolute(root, path),
                    message: format!("paths.root does not exist: {root_path}"),
                });
            }
        } else {
            report.issues.push(ValidationIssue {
                severity: Severity::Error,
                path: relative_or_absolute(root, path),
                message: "missing string key [paths].root".to_string(),
            });
        }
    }

    if !matches!(table.get("operations"), Some(Value::Array(_))) {
        report.issues.push(ValidationIssue {
            severity: Severity::Warning,
            path: relative_or_absolute(root, path),
            message: "missing operations array-of-table: [[operations]]".to_string(),
        });
    } else if let Some(ops) = table.get("operations").and_then(Value::as_array) {
        let mut op_ids = HashSet::new();
        for op in ops {
            let Some(op_table) = op.as_table() else {
                report.issues.push(ValidationIssue {
                    severity: Severity::Error,
                    path: relative_or_absolute(root, path),
                    message: "operation entry must be a table".to_string(),
                });
                continue;
            };

            let Some(op_id) = op_table.get("id").and_then(Value::as_str) else {
                report.issues.push(ValidationIssue {
                    severity: Severity::Error,
                    path: relative_or_absolute(root, path),
                    message: "operation missing required string key: id".to_string(),
                });
                continue;
            };

            if op_id.trim().is_empty() {
                report.issues.push(ValidationIssue {
                    severity: Severity::Error,
                    path: relative_or_absolute(root, path),
                    message: "operation id cannot be empty".to_string(),
                });
            } else if !op_ids.insert(op_id.to_string()) {
                report.issues.push(ValidationIssue {
                    severity: Severity::Error,
                    path: relative_or_absolute(root, path),
                    message: format!("duplicate operation id: {op_id}"),
                });
            }

            if !matches!(op_table.get("command"), Some(Value::String(s)) if !s.trim().is_empty()) {
                report.issues.push(ValidationIssue {
                    severity: Severity::Error,
                    path: relative_or_absolute(root, path),
                    message: format!("operation '{op_id}' missing non-empty command"),
                });
            }
        }
    }

    if let Some(id) = id {
        let dir_name = path
            .parent()
            .and_then(Path::file_name)
            .and_then(|n| n.to_str())
            .unwrap_or_default();
        if dir_name != id && !dir_name.contains(id) {
            report.issues.push(ValidationIssue {
                severity: Severity::Warning,
                path: relative_or_absolute(root, path),
                message: format!("module id '{id}' does not match directory name '{dir_name}'"),
            });
        }
    }
}

fn validate_required_int(
    table: &toml::map::Map<String, Value>,
    key: &str,
    root: &Path,
    path: &Path,
    report: &mut ValidationReport,
) {
    if !matches!(table.get(key), Some(Value::Integer(_))) {
        report.issues.push(ValidationIssue {
            severity: Severity::Error,
            path: relative_or_absolute(root, path),
            message: format!("missing or invalid integer key: {key}"),
        });
    }
}

fn validate_required_string(
    table: &toml::map::Map<String, Value>,
    key: &str,
    root: &Path,
    path: &Path,
    report: &mut ValidationReport,
) -> Option<String> {
    if !matches!(table.get(key), Some(Value::String(_))) {
        report.issues.push(ValidationIssue {
            severity: Severity::Error,
            path: relative_or_absolute(root, path),
            message: format!("missing or invalid string key: {key}"),
        });
        return None;
    }
    table
        .get(key)
        .and_then(Value::as_str)
        .map(ToString::to_string)
}

fn parse_toml(root: &Path, path: &Path, report: &mut ValidationReport) -> Option<Value> {
    let raw = match fs::read_to_string(path) {
        Ok(raw) => raw,
        Err(e) => {
            report.issues.push(ValidationIssue {
                severity: Severity::Error,
                path: relative_or_absolute(root, path),
                message: format!("failed to read file: {e}"),
            });
            return None;
        }
    };

    match raw.parse::<Value>() {
        Ok(v) => Some(v),
        Err(e) => {
            report.issues.push(ValidationIssue {
                severity: Severity::Error,
                path: relative_or_absolute(root, path),
                message: format!("invalid TOML: {e}"),
            });
            None
        }
    }
}

fn read_default_host(root: &Path) -> Option<String> {
    let path = root.join("app/manifests/hosts.toml");
    let raw = fs::read_to_string(path).ok()?;
    let value: Value = raw.parse().ok()?;
    value
        .as_table()?
        .get("default_host")?
        .as_str()
        .map(ToString::to_string)
}

fn validate_hosts_manifest(
    root: &Path,
    path: &Path,
    table: &toml::map::Map<String, Value>,
    report: &mut ValidationReport,
) {
    let default_host = table.get("default_host").and_then(Value::as_str);
    let Some(hosts) = table.get("hosts").and_then(Value::as_array) else {
        report.issues.push(ValidationIssue {
            severity: Severity::Error,
            path: relative_or_absolute(root, path),
            message: "hosts.toml missing [[hosts]] entries".to_string(),
        });
        return;
    };

    let mut ids = HashSet::new();
    for host in hosts {
        let Some(host_table) = host.as_table() else {
            report.issues.push(ValidationIssue {
                severity: Severity::Error,
                path: relative_or_absolute(root, path),
                message: "hosts entry must be a table".to_string(),
            });
            continue;
        };

        let Some(id) = host_table.get("id").and_then(Value::as_str) else {
            report.issues.push(ValidationIssue {
                severity: Severity::Error,
                path: relative_or_absolute(root, path),
                message: "host entry missing string key: id".to_string(),
            });
            continue;
        };

        if !validate_module_id(id) {
            report.issues.push(ValidationIssue {
                severity: Severity::Error,
                path: relative_or_absolute(root, path),
                message: format!("invalid host id '{id}'"),
            });
        }

        if !ids.insert(id.to_string()) {
            report.issues.push(ValidationIssue {
                severity: Severity::Error,
                path: relative_or_absolute(root, path),
                message: format!("duplicate host id: {id}"),
            });
        }

        if let Some(overlay_dir) = host_table.get("overlay_dir").and_then(Value::as_str) {
            let abs = root.join(overlay_dir);
            if !abs.exists() {
                report.issues.push(ValidationIssue {
                    severity: Severity::Warning,
                    path: relative_or_absolute(root, path),
                    message: format!("host overlay_dir does not exist: {overlay_dir}"),
                });
            }
        }
    }

    match default_host {
        Some(default) if ids.contains(default) => {}
        Some(default) => report.issues.push(ValidationIssue {
            severity: Severity::Error,
            path: relative_or_absolute(root, path),
            message: format!("default_host '{default}' not found in [[hosts]] ids"),
        }),
        None => report.issues.push(ValidationIssue {
            severity: Severity::Error,
            path: relative_or_absolute(root, path),
            message: "hosts.toml missing string key: default_host".to_string(),
        }),
    }
}

fn validate_services_manifest(
    root: &Path,
    path: &Path,
    table: &toml::map::Map<String, Value>,
    report: &mut ValidationReport,
) {
    let Some(services) = table.get("services").and_then(Value::as_array) else {
        report.issues.push(ValidationIssue {
            severity: Severity::Error,
            path: relative_or_absolute(root, path),
            message: "services.toml missing [[services]] entries".to_string(),
        });
        return;
    };

    let mut ids = HashSet::new();
    for item in services {
        let Some(t) = item.as_table() else {
            report.issues.push(ValidationIssue {
                severity: Severity::Error,
                path: relative_or_absolute(root, path),
                message: "service entry must be a table".to_string(),
            });
            continue;
        };

        let Some(id) = t.get("id").and_then(Value::as_str) else {
            report.issues.push(ValidationIssue {
                severity: Severity::Error,
                path: relative_or_absolute(root, path),
                message: "service entry missing string key: id".to_string(),
            });
            continue;
        };
        if !validate_module_id(id) {
            report.issues.push(ValidationIssue {
                severity: Severity::Error,
                path: relative_or_absolute(root, path),
                message: format!("invalid service id '{id}'"),
            });
        }
        if !ids.insert(id.to_string()) {
            report.issues.push(ValidationIssue {
                severity: Severity::Error,
                path: relative_or_absolute(root, path),
                message: format!("duplicate service id: {id}"),
            });
        }

        match t.get("type").and_then(Value::as_str) {
            Some("systemd-user") | Some("systemd-system") => {
                if !matches!(t.get("unit"), Some(Value::String(_))) {
                    report.issues.push(ValidationIssue {
                        severity: Severity::Error,
                        path: relative_or_absolute(root, path),
                        message: format!("service '{id}' missing required string key: unit"),
                    });
                }
            }
            Some("session-autostart") | Some("health-check") => {
                if !matches!(t.get("command"), Some(Value::String(_))) {
                    report.issues.push(ValidationIssue {
                        severity: Severity::Error,
                        path: relative_or_absolute(root, path),
                        message: format!("service '{id}' missing required string key: command"),
                    });
                }
            }
            Some(other) => {
                report.issues.push(ValidationIssue {
                    severity: Severity::Error,
                    path: relative_or_absolute(root, path),
                    message: format!(
                        "service '{id}' has invalid type '{other}' (allowed: systemd-user, systemd-system, session-autostart, health-check)"
                    ),
                });
            }
            None => {
                report.issues.push(ValidationIssue {
                    severity: Severity::Error,
                    path: relative_or_absolute(root, path),
                    message: format!("service '{id}' missing required string key: type"),
                });
            }
        }
    }
}

fn read_active_host(root: &Path) -> Option<String> {
    get_active_host(root).ok().flatten()
}

fn read_hosts_manifest(root: &Path) -> io::Result<toml::value::Table> {
    let path = root.join("app/manifests/hosts.toml");
    let raw = fs::read_to_string(path)?;
    let value: Value = raw.parse().map_err(|e| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("failed to parse app/manifests/hosts.toml: {e}"),
        )
    })?;

    match value.as_table() {
        Some(table) => Ok(table.clone()),
        None => Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "hosts.toml must be a TOML table",
        )),
    }
}

fn active_host_path(root: &Path) -> PathBuf {
    root.join("app/state/run/active-host")
}

fn relative_or_absolute(root: &Path, path: &Path) -> PathBuf {
    path.strip_prefix(root)
        .map_or_else(|_| path.to_path_buf(), Path::to_path_buf)
}

// ── Theme Management ──────────────────────────────────────────

/// Load available themes from themes.toml
pub fn load_themes(root: &Path) -> io::Result<Vec<ThemeDescriptor>> {
    let path = root.join("app/manifests/themes.toml");
    let raw = fs::read_to_string(&path)?;
    let value: Value = raw.parse().map_err(|e| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("failed to parse themes.toml: {e}"),
        )
    })?;

    let table = value.as_table().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            "themes.toml must be a TOML table",
        )
    })?;

    let mut themes = Vec::new();
    if let Some(items) = table.get("themes").and_then(Value::as_array) {
        for item in items {
            let Some(t) = item.as_table() else {
                continue;
            };
            let Some(id) = t.get("id").and_then(Value::as_str) else {
                continue;
            };
            let name = t
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or(id)
                .to_string();
            let description = t
                .get("description")
                .and_then(Value::as_str)
                .map(ToString::to_string);
            let shell = t
                .get("shell")
                .and_then(Value::as_str)
                .unwrap_or("bash")
                .to_string();
            let dotfiles_module = t
                .get("dotfiles_module")
                .and_then(Value::as_str)
                .map(ToString::to_string);

            themes.push(ThemeDescriptor {
                id: id.to_string(),
                name,
                description,
                shell,
                dotfiles_module,
            });
        }
    }

    Ok(themes)
}

/// Get currently active theme
pub fn get_active_theme(root: &Path) -> io::Result<Option<String>> {
    match ThemeState::load(root) {
        Ok(state) => Ok(state.theme_id),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e),
    }
}

/// Set active theme
pub fn set_active_theme(root: &Path, theme_id: &str) -> io::Result<()> {
    // Validate theme exists
    let themes = load_themes(root)?;
    if !themes.iter().any(|t| t.id == theme_id) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("unknown theme id '{theme_id}'"),
        ));
    }

    // Update state
    let mut state = ThemeState::load(root).unwrap_or_default();
    state.set_theme(theme_id.to_string());
    state.save(root)?;

    Ok(())
}

/// List available themes
pub fn list_available_themes(root: &Path) -> io::Result<Vec<String>> {
    let themes = load_themes(root)?;
    Ok(themes.into_iter().map(|t| t.id).collect())
}

// ── Module Activation/Deactivation ────────────────────────────

/// Get list of active modules
pub fn get_active_modules(root: &Path) -> io::Result<Vec<String>> {
    let path = root.join("app/state/tracking/active_modules.json");
    match state::load_state::<ActiveModules>(&path) {
        Ok(state) => Ok(state.active),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(Vec::new()),
        Err(e) => Err(e),
    }
}

/// Set active modules
pub fn set_active_modules(root: &Path, module_ids: Vec<String>) -> io::Result<()> {
    let path = root.join("app/state/tracking/active_modules.json");
    let state = ActiveModules {
        active: module_ids,
        last_updated: Some(chrono::Utc::now()),
    };
    state::save_state(&path, &state)
}

/// Activate a module
pub fn activate_module(root: &Path, module_id: &str) -> io::Result<()> {
    let mut active = get_active_modules(root)?;
    if !active.iter().any(|id| id == module_id) {
        active.push(module_id.to_string());
        set_active_modules(root, active)?;
    }
    Ok(())
}

/// Deactivate a module
pub fn deactivate_module(root: &Path, module_id: &str) -> io::Result<()> {
    let mut active = get_active_modules(root)?;
    active.retain(|id| id != module_id);
    set_active_modules(root, active)?;
    Ok(())
}

/// Check if a module is active
pub fn is_module_active(root: &Path, module_id: &str) -> io::Result<bool> {
    let active = get_active_modules(root)?;
    Ok(active.iter().any(|id| id == module_id))
}

// ── Maintenance State ──────────────────────────────────────────

/// Get maintenance state
pub fn get_maintenance_state(root: &Path) -> io::Result<MaintenanceState> {
    let path = root.join("app/state/tracking/maintenance_state.json");
    match state::load_state(&path) {
        Ok(state) => Ok(state),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(MaintenanceState::default()),
        Err(e) => Err(e),
    }
}

/// Update maintenance state
pub fn update_maintenance_state(
    root: &Path,
    operation: &str,
    status: &str,
    details: Option<String>,
) -> io::Result<()> {
    let path = root.join("app/state/tracking/maintenance_state.json");
    let mut state = get_maintenance_state(root)?;

    let record = MaintenanceRecord {
        operation: operation.to_string(),
        timestamp: chrono::Utc::now(),
        status: status.to_string(),
        details,
    };

    // Update last_* timestamp based on operation
    match operation {
        "clean" | "system-clean" => state.last_clean = Some(record.timestamp),
        "doctor" | "system-doctor" => state.last_doctor = Some(record.timestamp),
        "update" | "system-update" => state.last_update = Some(record.timestamp),
        _ => {}
    }

    state.operations.push(record);

    // Keep only last 100 operations
    if state.operations.len() > 100 {
        state.operations = state.operations.split_off(state.operations.len() - 100);
    }

    state::save_state(&path, &state)
}
