//! Iron Doctor Command
//!
//! System health check and diagnostics.

use crate::context::{AppContext, require_init};
use crate::output::StatusBadge;
use anyhow::Result;
use iron_core::services::host::HostService;
use iron_core::services::module::ModuleService;
use serde::Serialize;
use std::path::Path;
use std::process::Command;

#[derive(Serialize)]
struct HealthReport {
    overall: String,
    checks: Vec<HealthCheck>,
}

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

    // Check 1: State file exists
    let state_path = ctx.root.join("state.json");
    if state_path.exists() {
        output.list_item_status("State file exists", StatusBadge::Ok);
        checks.push(HealthCheck {
            name: "state_file".to_string(),
            status: "ok".to_string(),
            message: "State file exists".to_string(),
        });
    } else {
        output.list_item_status("State file missing", StatusBadge::Error);
        errors += 1;
        checks.push(HealthCheck {
            name: "state_file".to_string(),
            status: "error".to_string(),
            message: "State file missing".to_string(),
        });
    }

    // Check 2: Directory structure
    let dirs = ["modules", "profiles", "bundles", "hosts"];
    for dir in &dirs {
        let path = ctx.root.join(dir);
        if path.exists() {
            output.list_item_status(&format!("{} directory", dir), StatusBadge::Ok);
            checks.push(HealthCheck {
                name: format!("{}_dir", dir),
                status: "ok".to_string(),
                message: format!("{} directory exists", dir),
            });
        } else {
            output.list_item_status(&format!("{} directory missing", dir), StatusBadge::Warning);
            warnings += 1;
            checks.push(HealthCheck {
                name: format!("{}_dir", dir),
                status: "warning".to_string(),
                message: format!("{} directory missing", dir),
            });
        }
    }

    // Check 3: Current host configured
    if let Some(host_id) = ctx.current_host() {
        let host_service = ctx.host_service();
        if host_service.load_host(&host_id).is_ok() {
            output.list_item_status(&format!("Current host: {}", host_id), StatusBadge::Ok);
            checks.push(HealthCheck {
                name: "current_host".to_string(),
                status: "ok".to_string(),
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
                status: "error".to_string(),
                message: "Host config missing".to_string(),
            });
        }
    } else {
        output.list_item_status("No current host set", StatusBadge::Warning);
        warnings += 1;
        checks.push(HealthCheck {
            name: "current_host".to_string(),
            status: "warning".to_string(),
            message: "No current host set".to_string(),
        });
    }

    // Check 4: Git repository
    output.subheader("Git Status");
    if ctx.root.join(".git").exists() {
        output.list_item_status("Git repository initialized", StatusBadge::Ok);
        checks.push(HealthCheck {
            name: "git_repo".to_string(),
            status: "ok".to_string(),
            message: "Git repository initialized".to_string(),
        });
    } else {
        output.list_item_status("Not a git repository", StatusBadge::Warning);
        warnings += 1;
        checks.push(HealthCheck {
            name: "git_repo".to_string(),
            status: "warning".to_string(),
            message: "Not a git repository".to_string(),
        });
    }

    // Check 5: External tools
    output.subheader("External Tools");

    let tools = [("pacman", "Package manager"), ("git", "Version control")];

    for (tool, desc) in &tools {
        if Command::new("which")
            .arg(tool)
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
        {
            output.list_item_status(&format!("{} ({})", tool, desc), StatusBadge::Ok);
            checks.push(HealthCheck {
                name: format!("tool_{}", tool),
                status: "ok".to_string(),
                message: format!("{} available", tool),
            });
        } else {
            output.list_item_status(&format!("{} not found", tool), StatusBadge::Error);
            errors += 1;
            checks.push(HealthCheck {
                name: format!("tool_{}", tool),
                status: "error".to_string(),
                message: format!("{} not found", tool),
            });
        }
    }

    // Optional tools
    let optional_tools = [
        ("paru", "AUR helper"),
        ("yay", "AUR helper"),
        ("git-crypt", "Secrets encryption"),
        ("timeshift", "Snapshots"),
        ("snapper", "Snapshots"),
    ];

    for (tool, desc) in &optional_tools {
        if Command::new("which")
            .arg(tool)
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
        {
            output.list_item_status(&format!("{} ({})", tool, desc), StatusBadge::Ok);
        } else {
            output.verbose(&format!("{} not available (optional)", tool));
        }
    }

    // Check 6: Broken symlinks
    output.subheader("Symlink Integrity");
    let module_service = ctx.module_service();
    let modules = module_service.discover().unwrap_or_default();
    let mut broken_links = 0;

    for module in &modules {
        for dotfile in &module.dotfiles {
            let target = iron_core::validation::expand_home(Path::new(&dotfile.target));
            if target.is_symlink()
                && let Ok(link_target) = std::fs::read_link(&target)
                && !link_target.exists()
            {
                output
                    .list_item_status(&format!("Broken: {}", target.display()), StatusBadge::Error);
                broken_links += 1;
            }
        }
    }

    if broken_links == 0 {
        output.list_item_status("No broken symlinks found", StatusBadge::Ok);
        checks.push(HealthCheck {
            name: "symlinks".to_string(),
            status: "ok".to_string(),
            message: "No broken symlinks".to_string(),
        });
    } else {
        errors += broken_links;
        checks.push(HealthCheck {
            name: "symlinks".to_string(),
            status: "error".to_string(),
            message: format!("{} broken symlinks", broken_links),
        });
    }

    // Summary
    output.separator();
    let overall = if errors > 0 {
        output.error(&format!("{} errors, {} warnings", errors, warnings));
        "error"
    } else if warnings > 0 {
        output.warning(&format!("{} warnings", warnings));
        "warning"
    } else {
        output.success("All checks passed");
        "ok"
    };

    if output.is_json() {
        let report = HealthReport {
            overall: overall.to_string(),
            checks,
        };
        output.json(&report);
    }

    if errors > 0 {
        std::process::exit(1);
    }

    Ok(())
}
