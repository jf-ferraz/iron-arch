//! Iron Status Command
//!
//! Shows system status overview.
//! F3-004: Enhanced with package/service/dotfile counts, security level,
//! --full flag for ActualState scan, --dry-run for testing.

use crate::context::{AppContext, require_init};
use crate::output::{Output, StatusBadge};
use anyhow::Result;
use iron_core::availability::{AvailabilityStatus, ServiceAvailability};
use iron_core::services::apply::resolve_desired_state;
use iron_core::services::bundle::BundleService;
use iron_core::services::host::HostService;
use iron_core::services::module::ModuleService;
use iron_core::services::profile::ProfileService;
use iron_core::services::secrets::SecretsService;
use iron_core::services::security::SecurityService;
use iron_core::services::sync::{SyncService, SyncStatus};
use serde::Serialize;
use std::time::Instant;

/// Status data for JSON output
#[derive(Serialize)]
struct StatusData {
    host: HostStatus,
    bundle: Option<BundleStatus>,
    profile: Option<ProfileStatus>,
    modules: ModulesStatus,
    #[serde(default)]
    packages: PackagesStatus,
    #[serde(default)]
    security: Option<SecurityStatus>,
    sync: SyncStatusData,
    secrets: SecretsStatusData,
    services: ServicesStatusData,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    drift: Option<DriftSummary>,
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

#[derive(Serialize, Default)]
struct PackagesStatus {
    declared: usize,
    declared_aur: usize,
    #[serde(default)]
    services_declared: usize,
    #[serde(default)]
    dotfiles_declared: usize,
    #[serde(default)]
    managed_packages: usize,
    #[serde(default)]
    managed_services: usize,
    #[serde(default)]
    managed_dotfiles: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    last_apply: Option<String>,
}

#[derive(Serialize)]
struct SecurityStatus {
    level: String,
    score: u32,
    max_score: u32,
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

#[derive(Serialize)]
struct ServiceStatusData {
    name: String,
    status: String,
    reason: Option<String>,
}

#[derive(Serialize)]
struct ServicesStatusData {
    secrets: ServiceStatusData,
    sync: ServiceStatusData,
    snapshots: ServiceStatusData,
    aur: ServiceStatusData,
}

#[derive(Serialize)]
struct DriftSummary {
    total_drifts: usize,
    packages_missing: usize,
    packages_extra: usize,
    configs_drifted: usize,
    services_drifted: usize,
}

/// Format service status for display
fn format_service_status(name: &str, status: &AvailabilityStatus, output: &Output) {
    match status {
        AvailabilityStatus::Available => {
            output.list_item_status(name, StatusBadge::Ok);
        }
        AvailabilityStatus::Degraded(reason) => {
            output.list_item_status(&format!("{}: {}", name, reason), StatusBadge::Warning);
        }
        AvailabilityStatus::Unavailable(reason) => {
            output.list_item_status(&format!("{}: {}", name, reason), StatusBadge::Error);
        }
    }
}

/// Convert AvailabilityStatus to ServiceStatusData for JSON output
fn availability_to_service_data(name: &str, status: &AvailabilityStatus) -> ServiceStatusData {
    match status {
        AvailabilityStatus::Available => ServiceStatusData {
            name: name.to_string(),
            status: "available".to_string(),
            reason: None,
        },
        AvailabilityStatus::Degraded(reason) => ServiceStatusData {
            name: name.to_string(),
            status: "degraded".to_string(),
            reason: Some(reason.clone()),
        },
        AvailabilityStatus::Unavailable(reason) => ServiceStatusData {
            name: name.to_string(),
            status: "unavailable".to_string(),
            reason: Some(reason.clone()),
        },
    }
}

/// Execute status command
pub fn execute(ctx: &AppContext, full: bool, dry_run: bool) -> Result<()> {
    let start = Instant::now();
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

    // Resolve desired state for package/service/dotfile counts
    let desired = host
        .as_ref()
        .and_then(|h| resolve_desired_state(&ctx.root, h).ok());

    // Managed resource counts from state tracking
    let managed_pkg_count = ctx.state.managed_packages().len();
    let managed_svc_count = ctx.state.managed_services().len();
    let managed_dot_count = ctx.state.managed_dotfiles().len();
    let last_apply_ts = ctx.state.state().last_apply.map(|ts| ts.to_rfc3339());

    let packages_status = desired
        .as_ref()
        .map(|d| PackagesStatus {
            declared: d.packages.len(),
            declared_aur: d.aur_packages.len(),
            services_declared: d.services.len(),
            dotfiles_declared: d.dotfiles.len(),
            managed_packages: managed_pkg_count,
            managed_services: managed_svc_count,
            managed_dotfiles: managed_dot_count,
            last_apply: last_apply_ts.clone(),
        })
        .unwrap_or_else(|| PackagesStatus {
            managed_packages: managed_pkg_count,
            managed_services: managed_svc_count,
            managed_dotfiles: managed_dot_count,
            last_apply: last_apply_ts.clone(),
            ..Default::default()
        });

    // Security level
    let security_status = ctx
        .security_service()
        .calculate()
        .ok()
        .map(|r| SecurityStatus {
            level: r.level.label().to_string(),
            score: r.score,
            max_score: r.max_score,
        });

    // Get sync status
    let sync_info = sync_service.status().ok();

    // Get secrets status
    let secrets_status = secrets_service.status().ok();

    // Check service availability (NFR-11)
    let availability = ServiceAvailability::check();

    // Full scan for drift (only with --full and not --dry-run)
    let drift_summary = if full && !dry_run {
        let drift_service = ctx.drift_service();
        use iron_core::services::drift::DriftService;
        drift_service
            .detect(&host_id)
            .ok()
            .map(|report| DriftSummary {
                total_drifts: report.summary.total_drifts,
                packages_missing: report.summary.packages_missing,
                packages_extra: report.summary.packages_extra,
                configs_drifted: report.summary.configs_drifted,
                services_drifted: report.summary.services_drifted,
            })
    } else {
        None
    };

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
            packages: packages_status,
            security: security_status,
            sync: SyncStatusData {
                status: sync_info
                    .as_ref()
                    .map(|s| format!("{:?}", s.status))
                    .unwrap_or_default(),
                behind: sync_info.as_ref().map(|s| s.commits_behind).unwrap_or(0),
                ahead: sync_info.as_ref().map(|s| s.commits_ahead).unwrap_or(0),
            },
            secrets: SecretsStatusData {
                status: secrets_status
                    .as_ref()
                    .map(|s| format!("{:?}", s))
                    .unwrap_or_default(),
            },
            services: ServicesStatusData {
                secrets: availability_to_service_data("Secrets (git-crypt)", &availability.secrets),
                sync: availability_to_service_data("Sync (git remote)", &availability.sync),
                snapshots: availability_to_service_data("Snapshots", &availability.snapshots),
                aur: availability_to_service_data("AUR Helper", &availability.aur),
            },
            drift: drift_summary,
        };
        output.json_envelope("status", &data, start);
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
        output.list_item_status(
            &format!("{} ({})", bundle.name, bundle.id),
            StatusBadge::Active,
        );
    } else {
        output.list_item_status("No bundle active", StatusBadge::Inactive);
    }

    // Profile info
    output.subheader("Profile");
    if let Some(profile) = &active_profile {
        output.list_item_status(
            &format!("{} ({})", profile.name, profile.id),
            StatusBadge::Active,
        );
    } else {
        output.list_item_status("No profile active", StatusBadge::Inactive);
    }

    // Module stats
    output.subheader("Modules");
    output.kv("Total", all_modules.len());
    output.kv("Enabled", enabled_modules.len());

    // Package/service/dotfile counts (from DesiredState + managed tracking)
    if let Some(d) = &desired {
        output.subheader("Declared State");
        output.kv("Packages (declared)", d.packages.len());
        if !d.aur_packages.is_empty() {
            output.kv("AUR Packages", d.aur_packages.len());
        }
        output.kv("Services", d.services.len());
        output.kv("Dotfiles", d.dotfiles.len());
    }

    // Managed resource counts
    if managed_pkg_count > 0 || managed_svc_count > 0 || managed_dot_count > 0 {
        output.subheader("Managed Resources");
        output.kv("Packages", managed_pkg_count);
        output.kv("Services", managed_svc_count);
        output.kv("Dotfiles", managed_dot_count);
    }

    // Last apply timestamp
    if let Some(ref ts) = last_apply_ts {
        output.kv("Last Apply", ts);
    }

    // Security level
    if let Some(sec) = &security_status {
        output.subheader("Security");
        output.kv(
            "Level",
            format!("{} ({}/{} pts)", sec.level, sec.score, sec.max_score),
        );
    }

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

    // Drift summary (--full only)
    if let Some(drift) = &drift_summary {
        output.subheader("Drift");
        if drift.total_drifts == 0 {
            output.list_item_status("No drift detected", StatusBadge::Ok);
        } else {
            output.list_item_status(
                &format!(
                    "{} item(s) drifted -- run `iron diff` for details",
                    drift.total_drifts
                ),
                StatusBadge::Warning,
            );
            output.verbose(&format!(
                "  {} pkg missing, {} pkg extra, {} config, {} service",
                drift.packages_missing,
                drift.packages_extra,
                drift.configs_drifted,
                drift.services_drifted,
            ));
        }
    } else if full && dry_run {
        output.subheader("Drift");
        output.info("[DRY RUN] Drift scan skipped.");
    }

    // Services availability (NFR-11: Graceful degradation)
    output.subheader("Services");
    format_service_status("Secrets (git-crypt)", &availability.secrets, output);
    format_service_status("Sync (git remote)", &availability.sync, output);
    format_service_status("Snapshots", &availability.snapshots, output);
    format_service_status("AUR Helper", &availability.aur, output);

    Ok(())
}
