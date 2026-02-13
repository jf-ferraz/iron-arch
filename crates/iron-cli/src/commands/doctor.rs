//! Iron Doctor Command
//!
//! System health check and diagnostics.
//! Implements FR-10.1 through FR-10.8 health diagnostics requirements.

use crate::context::{AppContext, require_init};
use crate::output::StatusBadge;
use anyhow::Result;
use chrono::Utc;
use iron_core::services::bundle::BundleService;
use iron_core::services::host::HostService;
use iron_core::services::module::ModuleService;
use serde::Serialize;
use std::path::Path;
use std::process::Command;

/// Health check status values (FR-10.8)
#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "lowercase")]
enum CheckStatus {
    Pass,
    Warn,
    Fail,
}

impl CheckStatus {
    fn as_str(&self) -> &'static str {
        match self {
            CheckStatus::Pass => "pass",
            CheckStatus::Warn => "warn",
            CheckStatus::Fail => "fail",
        }
    }
}

/// Structured health report (FR-10.8)
#[derive(Serialize)]
struct HealthReport {
    checks: Vec<HealthCheck>,
    overall: String,
    timestamp: String,
}

/// Individual health check result
#[derive(Serialize)]
struct HealthCheck {
    name: String,
    status: String,
    message: String,
}

/// Execute doctor command
pub fn execute(ctx: &AppContext) -> Result<()> {
    require_init(ctx)?;

    let output = &ctx.output;
    let mut checks: Vec<HealthCheck> = Vec::new();
    let mut errors = 0;
    let mut warnings = 0;

    output.header("Iron Health Check");

    // Check 1: State file validation (FR-10.1)
    let state_path = ctx.root.join("state.json");
    if state_path.exists() {
        // Validate state file is parseable
        match std::fs::read_to_string(&state_path) {
            Ok(content) => {
                if serde_json::from_str::<serde_json::Value>(&content).is_ok() {
                    output.list_item_status("State file valid", StatusBadge::Ok);
                    checks.push(HealthCheck {
                        name: "state_file".to_string(),
                        status: CheckStatus::Pass.as_str().to_string(),
                        message: "state.json valid".to_string(),
                    });
                } else {
                    output.list_item_status("State file invalid JSON", StatusBadge::Error);
                    errors += 1;
                    checks.push(HealthCheck {
                        name: "state_file".to_string(),
                        status: CheckStatus::Fail.as_str().to_string(),
                        message: "state.json invalid JSON".to_string(),
                    });
                }
            }
            Err(_) => {
                output.list_item_status("State file unreadable", StatusBadge::Error);
                errors += 1;
                checks.push(HealthCheck {
                    name: "state_file".to_string(),
                    status: CheckStatus::Fail.as_str().to_string(),
                    message: "state.json unreadable".to_string(),
                });
            }
        }
    } else {
        output.list_item_status("State file missing", StatusBadge::Error);
        errors += 1;
        checks.push(HealthCheck {
            name: "state_file".to_string(),
            status: CheckStatus::Fail.as_str().to_string(),
            message: "State file missing".to_string(),
        });
    }

    // Check 2: Config directory check (FR-10.5)
    output.subheader("Directory Structure");
    let dirs = ["modules", "profiles", "bundles", "hosts"];
    let mut all_dirs_exist = true;
    for dir in &dirs {
        let path = ctx.root.join(dir);
        if path.exists() {
            output.list_item_status(&format!("{} directory", dir), StatusBadge::Ok);
        } else {
            output.list_item_status(&format!("{} directory missing", dir), StatusBadge::Warning);
            warnings += 1;
            all_dirs_exist = false;
        }
    }
    checks.push(HealthCheck {
        name: "directories".to_string(),
        status: if all_dirs_exist {
            CheckStatus::Pass.as_str().to_string()
        } else {
            CheckStatus::Warn.as_str().to_string()
        },
        message: if all_dirs_exist {
            "all directories exist".to_string()
        } else {
            "some directories missing".to_string()
        },
    });

    // Check 3: Current host configured
    output.subheader("Host Configuration");
    if let Some(host_id) = ctx.current_host() {
        let host_service = ctx.host_service();
        if host_service.load_host(&host_id).is_ok() {
            output.list_item_status(&format!("Current host: {}", host_id), StatusBadge::Ok);
            checks.push(HealthCheck {
                name: "current_host".to_string(),
                status: CheckStatus::Pass.as_str().to_string(),
                message: format!("Current host: {}", host_id),
            });
        } else {
            output.list_item_status(
                &format!("Host {} config missing", host_id),
                StatusBadge::Error,
            );
            errors += 1;
            checks.push(HealthCheck {
                name: "current_host".to_string(),
                status: CheckStatus::Fail.as_str().to_string(),
                message: "Host config missing".to_string(),
            });
        }
    } else {
        output.list_item_status("No current host set", StatusBadge::Warning);
        warnings += 1;
        checks.push(HealthCheck {
            name: "current_host".to_string(),
            status: CheckStatus::Warn.as_str().to_string(),
            message: "No current host set".to_string(),
        });
    }

    // Check 4: Git repository check (FR-10.6)
    output.subheader("Git Status");
    if ctx.root.join(".git").exists() {
        // Check for uncommitted changes
        let git_status = Command::new("git")
            .args(["-C", ctx.root.to_str().unwrap_or("."), "status", "--porcelain"])
            .output();

        match git_status {
            Ok(output_result) if output_result.status.success() => {
                let status_output = String::from_utf8_lossy(&output_result.stdout);
                if status_output.trim().is_empty() {
                    output.list_item_status("Git repository clean", StatusBadge::Ok);
                    checks.push(HealthCheck {
                        name: "git".to_string(),
                        status: CheckStatus::Pass.as_str().to_string(),
                        message: "repository clean".to_string(),
                    });
                } else {
                    let changed_count = status_output.lines().count();
                    output.list_item_status(
                        &format!("Git: {} uncommitted changes", changed_count),
                        StatusBadge::Warning,
                    );
                    warnings += 1;
                    checks.push(HealthCheck {
                        name: "git".to_string(),
                        status: CheckStatus::Warn.as_str().to_string(),
                        message: "uncommitted changes".to_string(),
                    });
                }
            }
            _ => {
                output.list_item_status("Git repository initialized", StatusBadge::Ok);
                checks.push(HealthCheck {
                    name: "git".to_string(),
                    status: CheckStatus::Pass.as_str().to_string(),
                    message: "repository initialized".to_string(),
                });
            }
        }
    } else {
        output.list_item_status("Not a git repository", StatusBadge::Warning);
        warnings += 1;
        checks.push(HealthCheck {
            name: "git".to_string(),
            status: CheckStatus::Warn.as_str().to_string(),
            message: "not a git repository".to_string(),
        });
    }

    // Check 5: External tools (required)
    output.subheader("External Tools");

    let tools = [("pacman", "Package manager"), ("git", "Version control")];
    let mut all_required_tools = true;

    for (tool, desc) in &tools {
        if Command::new("which")
            .arg(tool)
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
        {
            output.list_item_status(&format!("{} ({})", tool, desc), StatusBadge::Ok);
        } else {
            output.list_item_status(&format!("{} not found", tool), StatusBadge::Error);
            errors += 1;
            all_required_tools = false;
        }
    }

    checks.push(HealthCheck {
        name: "tools".to_string(),
        status: if all_required_tools {
            CheckStatus::Pass.as_str().to_string()
        } else {
            CheckStatus::Fail.as_str().to_string()
        },
        message: if all_required_tools {
            "required tools available".to_string()
        } else {
            "missing required tools".to_string()
        },
    });

    // Check 6: Package installation check (FR-10.3)
    output.subheader("Package Installation");
    let (packages_status, packages_message): (CheckStatus, String) = if let Some(host_id) = ctx.current_host() {
        if let Some(bundle_id) = ctx.state.active_bundle(&host_id) {
            let bundle_service = ctx.bundle_service();
            match bundle_service.load(&bundle_id) {
                Ok(bundle) => {
                    let all_packages: Vec<String> = bundle
                        .packages
                        .iter()
                        .chain(bundle.aur_packages.iter())
                        .cloned()
                        .collect();

                    if all_packages.is_empty() {
                        output.list_item_status("No packages to verify", StatusBadge::Ok);
                        (CheckStatus::Pass, "no packages required".to_string())
                    } else {
                        let mut missing_packages: Vec<String> = Vec::new();
                        for pkg in &all_packages {
                            let installed = Command::new("pacman")
                                .args(["-Q", pkg])
                                .output()
                                .map(|o| o.status.success())
                                .unwrap_or(false);
                            if !installed {
                                missing_packages.push(pkg.clone());
                            }
                        }

                        if missing_packages.is_empty() {
                            output.list_item_status(
                                &format!("{} packages verified", all_packages.len()),
                                StatusBadge::Ok,
                            );
                            (
                                CheckStatus::Pass,
                                format!("{} packages verified", all_packages.len()),
                            )
                        } else {
                            for pkg in &missing_packages {
                                output.list_item_status(
                                    &format!("Missing: {}", pkg),
                                    StatusBadge::Warning,
                                );
                            }
                            warnings += missing_packages.len();
                            (
                                CheckStatus::Warn,
                                format!(
                                    "{} missing packages: {}",
                                    missing_packages.len(),
                                    missing_packages.join(", ")
                                ),
                            )
                        }
                    }
                }
                Err(_) => {
                    output.list_item_status(
                        &format!("Cannot load bundle '{}'", bundle_id),
                        StatusBadge::Warning,
                    );
                    warnings += 1;
                    (CheckStatus::Warn, format!("cannot load bundle '{}'", bundle_id))
                }
            }
        } else {
            output.list_item_status("No active bundle", StatusBadge::Ok);
            (CheckStatus::Pass, "no active bundle".to_string())
        }
    } else {
        output.list_item_status("No host configured", StatusBadge::Ok);
        (CheckStatus::Pass, "no host configured".to_string())
    };

    checks.push(HealthCheck {
        name: "packages".to_string(),
        status: packages_status.as_str().to_string(),
        message: packages_message,
    });

    // Check 7: Snapshot backend check (FR-10.4)
    // Note: Checks renumbered - was Check 6
    output.subheader("Snapshot Backend");
    let timeshift_available = Command::new("which")
        .arg("timeshift")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);
    let snapper_available = Command::new("which")
        .arg("snapper")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);

    if timeshift_available {
        output.list_item_status("timeshift available", StatusBadge::Ok);
        checks.push(HealthCheck {
            name: "snapshot".to_string(),
            status: CheckStatus::Pass.as_str().to_string(),
            message: "timeshift available".to_string(),
        });
    } else if snapper_available {
        output.list_item_status("snapper available", StatusBadge::Ok);
        checks.push(HealthCheck {
            name: "snapshot".to_string(),
            status: CheckStatus::Pass.as_str().to_string(),
            message: "snapper available".to_string(),
        });
    } else {
        output.list_item_status("No snapshot backend (timeshift/snapper)", StatusBadge::Warning);
        warnings += 1;
        checks.push(HealthCheck {
            name: "snapshot".to_string(),
            status: CheckStatus::Warn.as_str().to_string(),
            message: "no snapshot backend available".to_string(),
        });
    }

    // Check 8: Secrets status check (FR-10.7)
    output.subheader("Secrets Status");
    let secrets_dir = ctx.root.join("secrets");
    let gitcrypt_available = Command::new("which")
        .arg("git-crypt")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);

    if secrets_dir.exists() {
        if gitcrypt_available {
            // Check if git-crypt is unlocked by trying to read .git-crypt directory
            let gitcrypt_dir = ctx.root.join(".git-crypt");
            if gitcrypt_dir.exists() {
                // Check lock status by looking for key files
                let keys_dir = gitcrypt_dir.join("keys");
                if keys_dir.exists() {
                    output.list_item_status("git-crypt configured", StatusBadge::Ok);
                    checks.push(HealthCheck {
                        name: "secrets".to_string(),
                        status: CheckStatus::Pass.as_str().to_string(),
                        message: "git-crypt configured".to_string(),
                    });
                } else {
                    output.list_item_status("git-crypt not initialized", StatusBadge::Warning);
                    warnings += 1;
                    checks.push(HealthCheck {
                        name: "secrets".to_string(),
                        status: CheckStatus::Warn.as_str().to_string(),
                        message: "git-crypt not initialized".to_string(),
                    });
                }
            } else {
                output.list_item_status("secrets dir exists, git-crypt not configured", StatusBadge::Warning);
                warnings += 1;
                checks.push(HealthCheck {
                    name: "secrets".to_string(),
                    status: CheckStatus::Warn.as_str().to_string(),
                    message: "git-crypt not configured".to_string(),
                });
            }
        } else {
            output.list_item_status("git-crypt not available", StatusBadge::Warning);
            warnings += 1;
            checks.push(HealthCheck {
                name: "secrets".to_string(),
                status: CheckStatus::Warn.as_str().to_string(),
                message: "git-crypt not available".to_string(),
            });
        }
    } else {
        output.list_item_status("No secrets directory (optional)", StatusBadge::Ok);
        checks.push(HealthCheck {
            name: "secrets".to_string(),
            status: CheckStatus::Pass.as_str().to_string(),
            message: "no secrets configured".to_string(),
        });
    }

    // Optional AUR helpers
    let paru_available = Command::new("which")
        .arg("paru")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);
    let yay_available = Command::new("which")
        .arg("yay")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);

    if paru_available || yay_available {
        let helper = if paru_available { "paru" } else { "yay" };
        output.verbose(&format!("AUR helper: {}", helper));
    }

    // Check 9: Symlink integrity check (FR-10.2)
    output.subheader("Symlink Integrity");
    let module_service = ctx.module_service();
    let modules = module_service.discover().unwrap_or_default();
    let mut broken_links = 0;
    let mut total_links = 0;

    for module in &modules {
        for dotfile in &module.dotfiles {
            let target = iron_core::validation::expand_home(Path::new(&dotfile.target));
            if target.is_symlink() {
                total_links += 1;
                if let Ok(link_target) = std::fs::read_link(&target) {
                    if !link_target.exists() {
                        output.list_item_status(
                            &format!("Broken: {}", target.display()),
                            StatusBadge::Error,
                        );
                        broken_links += 1;
                    }
                }
            }
        }
    }

    if broken_links == 0 {
        if total_links > 0 {
            output.list_item_status(
                &format!("{} symlinks verified", total_links),
                StatusBadge::Ok,
            );
        } else {
            output.list_item_status("No symlinks to check", StatusBadge::Ok);
        }
        checks.push(HealthCheck {
            name: "symlinks".to_string(),
            status: CheckStatus::Pass.as_str().to_string(),
            message: format!("{} symlinks verified", total_links),
        });
    } else {
        errors += broken_links;
        checks.push(HealthCheck {
            name: "symlinks".to_string(),
            status: CheckStatus::Warn.as_str().to_string(),
            message: format!("{} broken symlinks found", broken_links),
        });
    }

    // Summary
    let overall = if errors > 0 {
        "fail"
    } else if warnings > 0 {
        "warn"
    } else {
        "pass"
    };

    // Output JSON report if requested (FR-10.8)
    // Skip text summary when in JSON mode - the report contains all info
    if output.is_json() {
        let report = HealthReport {
            checks,
            overall: overall.to_string(),
            timestamp: Utc::now().to_rfc3339(),
        };
        output.json(&report);
    } else {
        // Output text summary only in non-JSON modes
        output.separator();
        if errors > 0 {
            output.error(&format!("{} errors, {} warnings", errors, warnings));
        } else if warnings > 0 {
            output.warning(&format!("{} warnings", warnings));
        } else {
            output.success("All checks passed");
        }
    }

    if errors > 0 {
        std::process::exit(1);
    }

    Ok(())
}
