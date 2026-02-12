//! Iron Status Command
//!
//! Shows system status overview.

use crate::context::{require_init, AppContext};
use crate::output::StatusBadge;
use anyhow::Result;
use iron_core::services::bundle::BundleService;
use iron_core::services::host::HostService;
use iron_core::services::module::ModuleService;
use iron_core::services::profile::ProfileService;
use iron_core::services::secrets::SecretsService;
use iron_core::services::sync::{SyncService, SyncStatus};
use serde::Serialize;

/// Status data for JSON output
#[derive(Serialize)]
struct StatusData {
    host: HostStatus,
    bundle: Option<BundleStatus>,
    profile: Option<ProfileStatus>,
    modules: ModulesStatus,
    sync: SyncStatusData,
    secrets: SecretsStatusData,
}

#[derive(Serialize)]
struct HostStatus {
    id: String,
    name: String,
}

#[derive(Serialize)]
struct BundleStatus {
    id: String,
    name: String,
    state: String,
}

#[derive(Serialize)]
struct ProfileStatus {
    id: String,
    name: String,
    state: String,
}

#[derive(Serialize)]
struct ModulesStatus {
    total: usize,
    enabled: usize,
}

#[derive(Serialize)]
struct SyncStatusData {
    status: String,
    behind: usize,
    ahead: usize,
}

#[derive(Serialize)]
struct SecretsStatusData {
    status: String,
}

/// Execute status command
pub fn execute(ctx: &AppContext) -> Result<()> {
    require_init(ctx)?;

    let output = &ctx.output;
    let host_service = ctx.host_service();
    let bundle_service = ctx.bundle_service();
    let profile_service = ctx.profile_service();
    let module_service = ctx.module_service();
    let sync_service = ctx.sync_service();
    let secrets_service = ctx.secrets_service();

    // Get current host
    let host_id = ctx.current_host().unwrap_or_else(|| "unknown".to_string());
    let host = host_service.load_host(&host_id).ok();

    // Get active bundle
    let active_bundle = bundle_service.active().ok().flatten();

    // Get active profile
    let active_profile = profile_service.active().ok().flatten();

    // Get module stats
    let all_modules = module_service.discover().unwrap_or_default();
    let enabled_modules = module_service.list_enabled().unwrap_or_default();

    // Get sync status
    let sync_info = sync_service.status().ok();

    // Get secrets status
    let secrets_status = secrets_service.status().ok();

    if output.is_json() {
        let data = StatusData {
            host: HostStatus {
                id: host_id.clone(),
                name: host.as_ref().map(|h| h.name.clone()).unwrap_or_default(),
            },
            bundle: active_bundle.as_ref().map(|b| BundleStatus {
                id: b.id.clone(),
                name: b.name.clone(),
                state: "active".to_string(),
            }),
            profile: active_profile.as_ref().map(|p| ProfileStatus {
                id: p.id.clone(),
                name: p.name.clone(),
                state: "active".to_string(),
            }),
            modules: ModulesStatus {
                total: all_modules.len(),
                enabled: enabled_modules.len(),
            },
            sync: SyncStatusData {
                status: sync_info.as_ref().map(|s| format!("{:?}", s.status)).unwrap_or_default(),
                behind: sync_info.as_ref().map(|s| s.commits_behind).unwrap_or(0),
                ahead: sync_info.as_ref().map(|s| s.commits_ahead).unwrap_or(0),
            },
            secrets: SecretsStatusData {
                status: secrets_status.as_ref().map(|s| format!("{:?}", s)).unwrap_or_default(),
            },
        };
        output.json(&data);
        return Ok(());
    }

    output.header("Iron Status");

    // Host info
    output.subheader("Host");
    output.kv("ID", &host_id);
    if let Some(h) = &host {
        output.kv("Name", &h.name);
        if let Some(cpu) = &h.hardware.cpu {
            output.kv("CPU", cpu);
        }
    }

    // Bundle info
    output.subheader("Bundle");
    if let Some(bundle) = &active_bundle {
        output.list_item_status(&format!("{} ({})", bundle.name, bundle.id), StatusBadge::Active);
    } else {
        output.list_item_status("No bundle active", StatusBadge::Inactive);
    }

    // Profile info
    output.subheader("Profile");
    if let Some(profile) = &active_profile {
        output.list_item_status(&format!("{} ({})", profile.name, profile.id), StatusBadge::Active);
    } else {
        output.list_item_status("No profile active", StatusBadge::Inactive);
    }

    // Module stats
    output.subheader("Modules");
    output.kv("Total", all_modules.len());
    output.kv("Enabled", enabled_modules.len());

    // Sync status
    output.subheader("Sync");
    if let Some(info) = &sync_info {
        let badge = match info.status {
            SyncStatus::UpToDate => StatusBadge::Ok,
            SyncStatus::Ahead => StatusBadge::Warning,
            SyncStatus::Behind => StatusBadge::Warning,
            SyncStatus::Diverged => StatusBadge::Error,
            SyncStatus::Dirty => StatusBadge::Warning,
            SyncStatus::NotARepo => StatusBadge::Inactive,
        };
        output.list_item_status(&format!("{:?}", info.status), badge);
    } else {
        output.list_item_status("Unknown", StatusBadge::Inactive);
    }

    // Secrets status
    output.subheader("Secrets");
    if let Some(status) = &secrets_status {
        let badge = match status {
            iron_core::services::secrets::SecretsStatus::Unlocked => StatusBadge::Unlocked,
            iron_core::services::secrets::SecretsStatus::Locked => StatusBadge::Locked,
            iron_core::services::secrets::SecretsStatus::NotInitialized => StatusBadge::Inactive,
            iron_core::services::secrets::SecretsStatus::NotAvailable => StatusBadge::Inactive,
        };
        output.list_item_status(&format!("{:?}", status), badge);
    } else {
        output.list_item_status("Unknown", StatusBadge::Inactive);
    }

    Ok(())
}
