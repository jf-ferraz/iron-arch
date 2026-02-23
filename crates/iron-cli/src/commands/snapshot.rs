//! Snapshot & Rollback Commands
//!
//! F2-002: iron snapshot create <name>
//! F2-003: iron snapshot list
//! F2-004: iron snapshot restore <name>
//! F2-005: iron rollback
//! F2-006: per-module rollback

use crate::cli::SnapshotAction;
use crate::context::{AppContext, require_init};
use anyhow::Result;
use iron_core::services::snapshot_service::SnapshotService;
use std::time::Instant;

/// Execute snapshot subcommand
pub fn execute(ctx: &AppContext, action: SnapshotAction) -> Result<()> {
    let start = Instant::now();
    require_init(ctx)?;

    match action {
        SnapshotAction::Create {
            name,
            description,
            dry_run,
        } => execute_create(ctx, name, description.as_deref(), dry_run),
        SnapshotAction::List { json } => execute_list(ctx, json, start),
        SnapshotAction::Restore { name, dry_run, yes } => execute_restore(ctx, &name, dry_run, yes),
        SnapshotAction::Delete { name } => execute_delete(ctx, &name),
        SnapshotAction::Prune { keep } => execute_prune(ctx, keep),
    }
}

/// F2-002: Create a named snapshot
fn execute_create(
    ctx: &AppContext,
    name: Option<String>,
    description: Option<&str>,
    dry_run: bool,
) -> Result<()> {
    let output = &ctx.output;
    let service = ctx.snapshot_service();

    output.header("Snapshot Create");

    let snapshot_name =
        name.unwrap_or_else(|| chrono::Utc::now().format("snap-%Y%m%d-%H%M%S").to_string());

    if dry_run {
        output.info(&format!("Would create snapshot '{}'", snapshot_name));
        output.kv("Host", ctx.current_host().unwrap_or_else(|| "none".into()));
        output.kv("Modules", format!("{}", ctx.state.active_modules().len()));
        output.success("[DRY RUN] No snapshot created.");
        return Ok(());
    }

    let spinner = if !output.is_json() {
        let s = crate::progress::ProgressReporter::spinner("Creating snapshot...");
        Some(s)
    } else {
        None
    };

    let record = service.create(&snapshot_name, description)?;

    if let Some(s) = spinner {
        s.finish("Snapshot created");
    }

    output.success(&format!("Snapshot '{}' created", record.name));
    output.kv("ID", &record.id);
    output.kv(
        "Timestamp",
        record.timestamp.format("%Y-%m-%d %H:%M:%S UTC"),
    );
    output.kv("Modules", format!("{}", record.active_modules.len()));
    output.kv("Packages", format!("{}", record.explicit_packages.len()));

    if let Some(bundle) = &record.active_bundle {
        output.kv("Bundle", bundle);
    }
    if let Some(desc) = &record.description {
        output.kv("Description", desc);
    }

    Ok(())
}

/// F2-003: List all snapshots
fn execute_list(ctx: &AppContext, json: bool, start: Instant) -> Result<()> {
    let output = &ctx.output;
    let service = ctx.snapshot_service();

    let records = service.list()?;

    if json || output.is_json() {
        output.json_envelope("snapshot.list", &records, start);
        return Ok(());
    }

    output.header("Snapshots");

    if records.is_empty() {
        output.info("No snapshots found. Create one with 'iron snapshot create <name>'");
        return Ok(());
    }

    let rows: Vec<Vec<String>> = records
        .iter()
        .map(|r| {
            let snap_type = if r.auto { "auto" } else { "manual" };
            vec![
                truncate_str(&r.name, 23),
                r.timestamp.format("%Y-%m-%d %H:%M").to_string(),
                r.active_modules.len().to_string(),
                r.explicit_packages.len().to_string(),
                snap_type.to_string(),
            ]
        })
        .collect();

    output.table(&["NAME", "DATE", "MODULES", "PACKAGES", "TYPE"], &rows);

    output.info(&format!("{} snapshot(s) total", records.len()));

    Ok(())
}

/// F2-004: Restore from a snapshot
fn execute_restore(ctx: &AppContext, name: &str, dry_run: bool, yes: bool) -> Result<()> {
    let output = &ctx.output;
    let service = ctx.snapshot_service();

    output.header("Snapshot Restore");

    // Load the target snapshot
    let snapshot = service.get(name)?;

    output.info(&format!("Restoring to snapshot '{}'", snapshot.name));
    output.kv(
        "Created",
        snapshot.timestamp.format("%Y-%m-%d %H:%M:%S UTC"),
    );
    output.kv("Modules", format!("{}", snapshot.active_modules.len()));
    output.kv("Packages", format!("{}", snapshot.explicit_packages.len()));

    if let Some(bundle) = &snapshot.active_bundle {
        output.kv("Bundle", bundle);
    }

    // Show what would change
    output.subheader("Changes");

    let current_modules = ctx.state.active_modules();
    let snapshot_modules: std::collections::HashSet<_> =
        snapshot.active_modules.iter().cloned().collect();
    let current_set: std::collections::HashSet<_> = current_modules.iter().cloned().collect();

    let to_enable: Vec<_> = snapshot_modules.difference(&current_set).collect();
    let to_disable: Vec<_> = current_set.difference(&snapshot_modules).collect();

    if to_enable.is_empty() && to_disable.is_empty() {
        output.success("Module state already matches snapshot — nothing to do.");
        return Ok(());
    }

    for m in &to_enable {
        output.info(&format!("  + Enable module: {}", m));
    }
    for m in &to_disable {
        output.info(&format!("  - Disable module: {}", m));
    }

    if dry_run {
        output.success("[DRY RUN] No changes made.");
        return Ok(());
    }

    // Confirm
    if !yes {
        output.info("Proceed with restore? [y/N]");
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        if !input.trim().eq_ignore_ascii_case("y") {
            output.info("Cancelled.");
            return Ok(());
        }
    }

    // Auto-snapshot current state before restore
    output.info("Creating pre-restore snapshot...");
    let _ = service.create_auto("pre-restore");

    // Restore module state
    // Disable modules not in snapshot
    for m in &to_disable {
        ctx.state.disable_module(m)?;
        output.info(&format!("  Disabled: {}", m));
    }

    // Enable modules from snapshot
    for m in &to_enable {
        ctx.state.enable_module(m)?;
        output.info(&format!("  Enabled: {}", m));
    }

    // Restore bundle/profile if present
    if let Some(host_id) = &snapshot.host_id {
        if let Some(bundle) = &snapshot.active_bundle {
            ctx.state.set_active_bundle(host_id, bundle)?;
        }
        if let Some(profile) = &snapshot.active_profile {
            ctx.state.set_active_profile(host_id, profile)?;
        }
    }

    output.separator();
    output.success(&format!(
        "Restored to snapshot '{}' ({} modules enabled, {} disabled)",
        snapshot.name,
        to_enable.len(),
        to_disable.len()
    ));

    // Converge system state after metadata restore
    output.separator();
    output.info("Converging system state...");

    let apply_svc = ctx.apply_service();
    let host_id = ctx.current_host().unwrap_or_default();

    if !host_id.is_empty() {
        use iron_core::services::apply::ApplyService;

        let spinner = if !output.is_json() {
            Some(crate::progress::ProgressReporter::spinner(
                "Computing convergence plan...",
            ))
        } else {
            None
        };

        match apply_svc.plan(&host_id) {
            Ok(plan) if plan.is_empty() => {
                if let Some(s) = spinner {
                    s.finish("No changes needed");
                }
                output.success("System already matches restored state.");
            }
            Ok(plan) => {
                if let Some(s) = &spinner {
                    s.tick(&format!("Applying {} actions...", plan.actions.len()));
                }
                output.info(&format!("  {} actions to converge", plan.actions.len()));
                match apply_svc.execute(&plan) {
                    Ok(_) => {
                        if let Some(s) = spinner {
                            s.finish("Converged");
                        }
                        output.success("System state converged.");
                    }
                    Err(e) => {
                        if let Some(s) = spinner {
                            s.abandon("Failed");
                        }
                        output.warning(&format!(
                            "Convergence failed: {}. Run 'iron apply' manually.",
                            e
                        ));
                    }
                }
            }
            Err(e) => {
                if let Some(s) = spinner {
                    s.abandon("Failed");
                }
                output.warning(&format!(
                    "Could not compute plan: {}. Run 'iron apply' manually.",
                    e
                ));
            }
        }
    } else {
        output.warning("No active host set. Run 'iron apply' to converge system state.");
    }

    Ok(())
}

/// F2-005: iron rollback (quick rollback to last auto-snapshot)
pub fn execute_rollback(
    ctx: &AppContext,
    list: bool,
    module: Option<&str>,
    dry_run: bool,
    yes: bool,
) -> Result<()> {
    require_init(ctx)?;

    let output = &ctx.output;
    let service = ctx.snapshot_service();

    if list {
        output.header("Recent Auto-Snapshots");
        let records = service.list()?;
        let auto_records: Vec<_> = records.iter().filter(|r| r.auto).collect();

        if auto_records.is_empty() {
            output.info("No auto-snapshots found.");
            return Ok(());
        }

        for record in auto_records.iter().take(10) {
            output.list_item(&record.summary());
        }
        return Ok(());
    }

    // Find the most recent auto-snapshot
    let records = service.list()?;
    let latest_auto = records.iter().find(|r| r.auto);

    let Some(snapshot) = latest_auto else {
        output.error("No auto-snapshots available. Nothing to roll back to.");
        output.info("Create a snapshot manually: iron snapshot create <name>");
        return Ok(());
    };

    // F2-006: Per-module rollback
    if let Some(mod_id) = module {
        return execute_module_rollback(ctx, snapshot, mod_id, dry_run, yes);
    }

    output.header("Rollback");
    output.info(&format!(
        "Rolling back to: {} ({})",
        snapshot.name,
        snapshot.timestamp.format("%Y-%m-%d %H:%M")
    ));

    // Delegate to restore logic
    execute_restore(ctx, &snapshot.name, dry_run, yes)
}

/// F2-006: Per-module rollback
fn execute_module_rollback(
    ctx: &AppContext,
    snapshot: &iron_core::services::snapshot_service::SnapshotRecord,
    mod_id: &str,
    dry_run: bool,
    yes: bool,
) -> Result<()> {
    let output = &ctx.output;

    output.header(&format!("Module Rollback: {}", mod_id));

    let was_active = snapshot.active_modules.contains(&mod_id.to_string());
    let is_active = ctx.state.is_module_active(mod_id);

    if was_active == is_active {
        output.success(&format!(
            "Module '{}' is already in snapshot state ({}). Nothing to do.",
            mod_id,
            if is_active { "active" } else { "inactive" }
        ));
        return Ok(());
    }

    if was_active {
        output.info(&format!(
            "  + Enable module: {} (was active in snapshot)",
            mod_id
        ));
    } else {
        output.info(&format!(
            "  - Disable module: {} (was inactive in snapshot)",
            mod_id
        ));
    }

    if dry_run {
        output.success("[DRY RUN] No changes made.");
        return Ok(());
    }

    if !yes {
        output.info("Proceed with module rollback? [y/N]");
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        if !input.trim().eq_ignore_ascii_case("y") {
            output.info("Cancelled.");
            return Ok(());
        }
    }

    if was_active {
        ctx.state.enable_module(mod_id)?;
        output.success(&format!("Enabled module '{}'", mod_id));
    } else {
        ctx.state.disable_module(mod_id)?;
        output.success(&format!("Disabled module '{}'", mod_id));
    }

    Ok(())
}

/// Delete a snapshot
fn execute_delete(ctx: &AppContext, name: &str) -> Result<()> {
    let output = &ctx.output;
    let service = ctx.snapshot_service();

    service.delete(name)?;
    output.success(&format!("Snapshot '{}' deleted", name));

    Ok(())
}

/// Prune old auto-snapshots
fn execute_prune(ctx: &AppContext, keep: usize) -> Result<()> {
    let output = &ctx.output;
    let service = ctx.snapshot_service();

    let pruned = service.prune_auto(keep)?;

    if pruned == 0 {
        output.info(&format!(
            "No auto-snapshots to prune (keeping up to {})",
            keep
        ));
    } else {
        output.success(&format!(
            "Pruned {} auto-snapshot(s), keeping {} most recent",
            pruned, keep
        ));
    }

    Ok(())
}

use crate::output::truncate_str;
