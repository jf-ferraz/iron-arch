//! Iron Init Command
//!
//! Initializes Iron on the current host.

use crate::context::AppContext;
use crate::output::StatusBadge;
use anyhow::{Context, Result};
use iron_core::services::host::HostService;
use std::fs;

/// Execute init command
pub fn execute(
    ctx: &AppContext,
    id: Option<String>,
    name: Option<String>,
    force: bool,
) -> Result<()> {
    let output = &ctx.output;

    // Check if already initialized
    if ctx.is_initialized() && !force {
        output.warning("Iron is already initialized on this host.");
        output.info("Use --force to reinitialize.");
        return Ok(());
    }

    output.header("Iron Initialization");

    // Determine host ID
    let host_service = ctx.host_service();
    let hostname = host_service
        .hostname()
        .unwrap_or_else(|_| "unknown".to_string());
    let host_id = id.unwrap_or_else(|| hostname.clone());
    let host_name = name.unwrap_or_else(|| format!("{}'s Iron Host", hostname));

    output.kv("Host ID", &host_id);
    output.kv("Host Name", &host_name);

    // Create directory structure
    output.subheader("Creating directories");
    let dirs = ["modules", "profiles", "bundles", "hosts", "secrets"];
    for dir in &dirs {
        let path = ctx.root.join(dir);
        fs::create_dir_all(&path)
            .with_context(|| format!("Failed to create {}", path.display()))?;
        output.list_item_status(dir, StatusBadge::Ok);
    }

    // Detect hardware
    output.subheader("Detecting hardware");
    let hardware = host_service.detect_hardware()?;

    if let Some(cpu) = &hardware.cpu {
        output.kv("CPU", cpu);
    }
    if let Some(gpu) = &hardware.gpu {
        output.kv("GPU", gpu);
    }
    if let Some(ram) = hardware.ram_mb {
        output.kv("RAM", format!("{} MB", ram));
    }
    if let Some(chassis) = &hardware.chassis {
        output.kv("Chassis", format!("{:?}", chassis));
    }
    if !hardware.monitors.is_empty() {
        output.kv("Monitors", hardware.monitors.len());
        for mon in &hardware.monitors {
            output.verbose(&format!("  {} ({})", mon.output, mon.resolution));
        }
    }

    // Create host configuration
    output.subheader("Creating host configuration");
    let host = host_service.create_from_current(&host_id, &host_name)?;
    output.list_item_status(
        &format!("Host config: hosts/{}.toml", host.id),
        StatusBadge::Ok,
    );

    // Set current host
    ctx.state.set_current_host(&host_id)?;
    output.list_item_status("Set as active host", StatusBadge::Ok);

    output.separator();
    output.success(&format!("Iron initialized for host '{}'", host_id));
    output.info("Next steps:");
    output.list_item("iron bundle list    - View available bundles");
    output.list_item("iron bundle install - Install a desktop environment");
    output.list_item("iron go             - Launch TUI dashboard");

    Ok(())
}
