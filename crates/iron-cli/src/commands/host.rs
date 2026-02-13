//! Host Commands
//!
//! Host management and hardware detection.

use crate::cli::HostAction;
use crate::context::{AppContext, require_init};
use crate::output::StatusBadge;
use anyhow::Result;
use iron_core::services::host::HostService;
use serde::Serialize;

#[derive(Serialize)]
struct HostInfo {
    id: String,
    name: String,
    description: Option<String>,
    hardware: HardwareInfo,
    bundles: Vec<String>,
    active_bundle: Option<String>,
}

#[derive(Serialize)]
struct HardwareInfo {
    cpu: Option<String>,
    gpu: Option<String>,
    ram_mb: Option<u64>,
    chassis: Option<String>,
    monitors: usize,
}

/// Execute host command
pub fn execute(ctx: &AppContext, action: HostAction) -> Result<()> {
    match action {
        HostAction::List => list(ctx),
        HostAction::Current => current(ctx),
        HostAction::Catalog { update } => catalog(ctx, update),
        HostAction::Select { id } => select(ctx, &id),
        HostAction::Snapshot { description } => snapshot(ctx, description),
    }
}

/// List configured hosts
fn list(ctx: &AppContext) -> Result<()> {
    let output = &ctx.output;
    let host_service = ctx.host_service();

    let hosts = host_service.list_hosts()?;

    if hosts.is_empty() {
        output.warning("No hosts configured");
        output.info("Run 'iron init' to configure this host");
        return Ok(());
    }

    let current = ctx.current_host();

    output.header("Configured Hosts");

    if output.is_json() {
        let host_info: Vec<HostInfo> = hosts
            .iter()
            .map(|h| HostInfo {
                id: h.id.clone(),
                name: h.name.clone(),
                description: h.description.clone(),
                hardware: HardwareInfo {
                    cpu: h.hardware.cpu.clone(),
                    gpu: h.hardware.gpu.clone(),
                    ram_mb: h.hardware.ram_mb,
                    chassis: h.hardware.chassis.as_ref().map(|c| format!("{:?}", c)),
                    monitors: h.hardware.monitors.len(),
                },
                bundles: h.installed_bundles.clone(),
                active_bundle: h.active_bundle.clone(),
            })
            .collect();
        output.json(&host_info);
        return Ok(());
    }

    for host in &hosts {
        let is_current = current.as_ref().map(|c| c == &host.id).unwrap_or(false);
        let badge = if is_current {
            StatusBadge::Active
        } else {
            StatusBadge::Inactive
        };
        let current_marker = if is_current { " (current)" } else { "" };

        output.list_item_status(
            &format!("{} - {}{}", host.id, host.name, current_marker),
            badge,
        );

        if output.is_verbose() {
            if let Some(cpu) = &host.hardware.cpu {
                output.verbose(&format!("  CPU: {}", cpu));
            }
            if let Some(chassis) = &host.hardware.chassis {
                output.verbose(&format!("  Chassis: {:?}", chassis));
            }
        }
    }

    Ok(())
}

/// Show current host
fn current(ctx: &AppContext) -> Result<()> {
    require_init(ctx)?;

    let output = &ctx.output;
    let host_service = ctx.host_service();

    let host_id = ctx
        .current_host()
        .ok_or_else(|| anyhow::anyhow!("No current host"))?;
    let host = host_service.load_host(&host_id)?;

    if output.is_json() {
        let info = HostInfo {
            id: host.id.clone(),
            name: host.name.clone(),
            description: host.description.clone(),
            hardware: HardwareInfo {
                cpu: host.hardware.cpu.clone(),
                gpu: host.hardware.gpu.clone(),
                ram_mb: host.hardware.ram_mb,
                chassis: host.hardware.chassis.as_ref().map(|c| format!("{:?}", c)),
                monitors: host.hardware.monitors.len(),
            },
            bundles: host.installed_bundles.clone(),
            active_bundle: host.active_bundle.clone(),
        };
        output.json(&info);
        return Ok(());
    }

    output.header(&format!("Current Host: {}", host.name));

    output.kv("ID", &host.id);
    output.kv("Name", &host.name);

    if let Some(desc) = &host.description {
        output.kv("Description", desc);
    }

    output.subheader("Hardware");
    if let Some(cpu) = &host.hardware.cpu {
        output.kv("CPU", cpu);
    }
    if let Some(gpu) = &host.hardware.gpu {
        output.kv("GPU", gpu);
    }
    if let Some(ram) = host.hardware.ram_mb {
        output.kv("RAM", format!("{} MB", ram));
    }
    if let Some(chassis) = &host.hardware.chassis {
        output.kv("Chassis", format!("{:?}", chassis));
    }

    if !host.hardware.monitors.is_empty() {
        output.subheader("Monitors");
        for mon in &host.hardware.monitors {
            output.list_item(&format!(
                "{}: {} @ {}Hz (scale: {})",
                mon.output,
                mon.resolution,
                mon.refresh_rate.unwrap_or(60),
                mon.scale.unwrap_or(1.0)
            ));
        }
    }

    if let Some(bundle) = &host.active_bundle {
        output.subheader("Configuration");
        output.kv("Active Bundle", bundle);
    }

    if !host.installed_bundles.is_empty() {
        output.kv("Installed Bundles", host.installed_bundles.join(", "));
    }

    Ok(())
}

/// Catalog current hardware
fn catalog(ctx: &AppContext, update: bool) -> Result<()> {
    let output = &ctx.output;
    let host_service = ctx.host_service();

    output.header("Hardware Catalog");
    output.info("Detecting hardware...");

    let hardware = host_service.detect_hardware()?;

    if output.is_json() {
        let info = HardwareInfo {
            cpu: hardware.cpu.clone(),
            gpu: hardware.gpu.clone(),
            ram_mb: hardware.ram_mb,
            chassis: hardware.chassis.as_ref().map(|c| format!("{:?}", c)),
            monitors: hardware.monitors.len(),
        };
        output.json(&info);
        return Ok(());
    }

    output.subheader("Detected Hardware");

    if let Some(cpu) = &hardware.cpu {
        output.kv("CPU", cpu);
    } else {
        output.kv("CPU", "Unknown");
    }

    if let Some(gpu) = &hardware.gpu {
        output.kv("GPU", gpu);
    } else {
        output.kv("GPU", "Unknown");
    }

    if let Some(ram) = hardware.ram_mb {
        output.kv("RAM", format!("{} MB ({:.1} GB)", ram, ram as f64 / 1024.0));
    } else {
        output.kv("RAM", "Unknown");
    }

    let chassis = host_service.detect_chassis();
    output.kv("Chassis", format!("{:?}", chassis));

    let monitors = host_service.detect_monitors()?;
    if !monitors.is_empty() {
        output.subheader("Monitors");
        for mon in &monitors {
            output.list_item(&format!(
                "{}: {} @ {}Hz",
                mon.output,
                mon.resolution,
                mon.refresh_rate.unwrap_or(60)
            ));
        }
    } else {
        output.kv("Monitors", "None detected");
    }

    // Update host config if requested
    if update {
        if let Some(host_id) = ctx.current_host() {
            output.info("Updating host configuration...");
            let mut host = host_service.load_host(&host_id)?;
            host.hardware = hardware;
            host_service.save_host(&host)?;
            output.success("Host configuration updated");
        } else {
            output.warning("No current host to update");
        }
    }

    Ok(())
}

/// Select active host
fn select(ctx: &AppContext, id: &str) -> Result<()> {
    let output = &ctx.output;
    let host_service = ctx.host_service();

    // Verify host exists
    let host = host_service.load_host(id)?;

    output.info(&format!("Selecting host: {}", host.name));

    ctx.state.set_current_host(id)?;

    output.success(&format!("Switched to host '{}'", id));

    Ok(())
}

/// Create a system snapshot
fn snapshot(ctx: &AppContext, description: Option<String>) -> Result<()> {
    require_init(ctx)?;

    let output = &ctx.output;

    output.header("Creating System Snapshot");

    // Detect snapshot backend
    let backend = iron_core::snapshot::detect_backend();
    let backend_name = backend.name();

    if backend_name == "None" {
        output.error("No snapshot tool available");
        output.info("Install timeshift or snapper for snapshot support");
        return Ok(());
    }

    output.kv("Backend", backend_name);

    let desc = description.unwrap_or_else(|| "Iron manual snapshot".to_string());
    output.info(&format!("Description: {}", desc));

    output.info("Creating snapshot...");
    let manager = iron_core::snapshot::create_manager();
    let snapshot_info = manager.create(&desc)?;

    output.success(&format!("Snapshot created: {}", snapshot_info.id));

    Ok(())
}
