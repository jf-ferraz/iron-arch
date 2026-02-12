//! Bundle Commands
//!
//! Desktop environment/bundle management.

use crate::cli::BundleAction;
use crate::context::{require_init, AppContext};
use crate::output::StatusBadge;
use anyhow::Result;
use iron_core::bundle::BundleState;
use iron_core::services::bundle::BundleService;
use serde::Serialize;
use std::io::{self, Write};

#[derive(Serialize)]
struct BundleInfo {
    id: String,
    name: String,
    description: Option<String>,
    bundle_type: String,
    state: String,
    packages: usize,
}

/// Execute bundle command
pub fn execute(ctx: &AppContext, action: BundleAction) -> Result<()> {
    require_init(ctx)?;

    match action {
        BundleAction::List { all } => list(ctx, all),
        BundleAction::Status { id } => status(ctx, id),
        BundleAction::Install { id, yes } => install(ctx, &id, yes),
        BundleAction::Switch { id, yes } => switch(ctx, &id, yes),
        BundleAction::Remove { id, yes } => remove(ctx, &id, yes),
    }
}

/// List bundles
fn list(ctx: &AppContext, all: bool) -> Result<()> {
    let output = &ctx.output;
    let bundle_service = ctx.bundle_service();

    let bundles = bundle_service.discover()?;

    if bundles.is_empty() {
        output.warning("No bundles found");
        output.info("Create bundles in ~/.config/iron/bundles/");
        return Ok(());
    }

    output.header("Available Bundles");

    if output.is_json() {
        let bundle_info: Vec<BundleInfo> = bundles.iter().map(|b| {
            let state = bundle_service.state(&b.id).unwrap_or(BundleState::NotInstalled);
            BundleInfo {
                id: b.id.clone(),
                name: b.name.clone(),
                description: b.description.clone(),
                bundle_type: format!("{:?}", b.bundle_type),
                state: format!("{:?}", state),
                packages: b.packages.len() + b.aur_packages.len(),
            }
        }).collect();
        output.json(&bundle_info);
        return Ok(());
    }

    // Get active bundle
    let active = bundle_service.active().ok().flatten();

    for bundle in &bundles {
        let state = bundle_service.state(&bundle.id).unwrap_or(BundleState::NotInstalled);
        let is_active = active.as_ref().map(|a| a.id == bundle.id).unwrap_or(false);

        let badge = match state {
            BundleState::Active => StatusBadge::Active,
            BundleState::Dormant => StatusBadge::Partial,
            BundleState::NotInstalled => StatusBadge::NotInstalled,
        };

        let active_marker = if is_active { " (active)" } else { "" };
        output.list_item_status(
            &format!("{} - {}{}", bundle.id, bundle.name, active_marker),
            badge,
        );

        if all || output.is_verbose() {
            if let Some(desc) = &bundle.description {
                output.verbose(&format!("  {}", desc));
            }
            output.verbose(&format!("  Type: {:?}", bundle.bundle_type));
            output.verbose(&format!("  Packages: {}", bundle.packages.len() + bundle.aur_packages.len()));
        }
    }

    Ok(())
}

/// Show bundle status
fn status(ctx: &AppContext, id: Option<String>) -> Result<()> {
    let output = &ctx.output;
    let bundle_service = ctx.bundle_service();

    // Get bundle ID (current if not specified)
    let bundle_id = match id {
        Some(id) => id,
        None => {
            if let Some(active) = bundle_service.active()? {
                active.id.clone()
            } else {
                output.warning("No active bundle. Specify a bundle ID.");
                return Ok(());
            }
        }
    };

    let bundle = bundle_service.load(&bundle_id)?;
    let state = bundle_service.state(&bundle_id)?;

    if output.is_json() {
        let info = BundleInfo {
            id: bundle.id.clone(),
            name: bundle.name.clone(),
            description: bundle.description.clone(),
            bundle_type: format!("{:?}", bundle.bundle_type),
            state: format!("{:?}", state),
            packages: bundle.packages.len() + bundle.aur_packages.len(),
        };
        output.json(&info);
        return Ok(());
    }

    output.header(&format!("Bundle: {}", bundle.name));

    output.kv("ID", &bundle.id);
    output.kv("Type", format!("{:?}", bundle.bundle_type));
    output.kv("State", format!("{:?}", state));

    if let Some(desc) = &bundle.description {
        output.kv("Description", desc);
    }

    if !bundle.packages.is_empty() {
        output.subheader("Packages");
        for pkg in &bundle.packages {
            output.list_item(pkg);
        }
    }

    if !bundle.aur_packages.is_empty() {
        output.subheader("AUR Packages");
        for pkg in &bundle.aur_packages {
            output.list_item(pkg);
        }
    }

    if !bundle.services.is_empty() {
        output.subheader("Services");
        for svc in &bundle.services {
            output.list_item(svc);
        }
    }

    if !bundle.profiles.is_empty() {
        output.subheader("Compatible Profiles");
        for profile in &bundle.profiles {
            output.list_item(profile);
        }
    }

    if !bundle.conflicts.is_empty() {
        output.subheader("Conflicts");
        for conflict in &bundle.conflicts {
            output.list_item(conflict);
        }
    }

    Ok(())
}

/// Install a bundle
fn install(ctx: &AppContext, id: &str, yes: bool) -> Result<()> {
    let output = &ctx.output;
    let bundle_service = ctx.bundle_service();

    let bundle = bundle_service.load(id)?;
    let state = bundle_service.state(id)?;

    if matches!(state, BundleState::Active) {
        output.warning(&format!("Bundle '{}' is already active", id));
        return Ok(());
    }

    output.header(&format!("Installing Bundle: {}", bundle.name));

    // Check conflicts
    let conflicts = bundle_service.check_conflicts(id)?;
    if !conflicts.is_empty() {
        output.error("Bundle conflicts with:");
        for conflict in &conflicts {
            output.list_item(conflict);
        }
        output.info("Remove conflicting bundles first");
        return Ok(());
    }

    // Show what will be installed
    output.subheader("Will install:");
    output.kv("Packages", bundle.packages.len());
    output.kv("AUR Packages", bundle.aur_packages.len());
    output.kv("Services", bundle.services.len());

    // Confirmation
    if !yes {
        print!("\nProceed? [y/N] ");
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;

        if !input.trim().eq_ignore_ascii_case("y") {
            output.info("Installation cancelled");
            return Ok(());
        }
    }

    // Activate bundle
    output.info("Activating bundle...");
    bundle_service.activate(id)?;

    output.success(&format!("Bundle '{}' installed and activated", id));

    Ok(())
}

/// Switch to a different bundle
fn switch(ctx: &AppContext, id: &str, yes: bool) -> Result<()> {
    let output = &ctx.output;
    let bundle_service = ctx.bundle_service();

    // Get current active bundle
    let current = bundle_service.active()?;
    if current.is_none() {
        output.info("No active bundle. Use 'iron bundle install' instead.");
        return install(ctx, id, yes);
    }

    let current = current.unwrap();
    if current.id == id {
        output.warning(&format!("Bundle '{}' is already active", id));
        return Ok(());
    }

    let target = bundle_service.load(id)?;

    output.header("Bundle Switch");
    output.kv("From", &current.name);
    output.kv("To", &target.name);

    // Check conflicts
    let conflicts = bundle_service.check_conflicts(id)?;
    if !conflicts.is_empty() {
        output.error("Target bundle conflicts with:");
        for conflict in &conflicts {
            output.list_item(conflict);
        }
        return Ok(());
    }

    // Confirmation
    if !yes {
        output.warning("This will deactivate the current bundle and activate the new one.");
        print!("\nProceed? [y/N] ");
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;

        if !input.trim().eq_ignore_ascii_case("y") {
            output.info("Switch cancelled");
            return Ok(());
        }
    }

    // Switch
    output.info("Switching bundles...");
    bundle_service.switch(&current.id, id)?;

    output.success(&format!("Switched to bundle '{}'", id));

    Ok(())
}

/// Remove a bundle
fn remove(ctx: &AppContext, id: &str, yes: bool) -> Result<()> {
    let output = &ctx.output;
    let bundle_service = ctx.bundle_service();

    let bundle = bundle_service.load(id)?;
    let state = bundle_service.state(id)?;

    if matches!(state, BundleState::NotInstalled) {
        output.warning(&format!("Bundle '{}' is not installed", id));
        return Ok(());
    }

    output.header(&format!("Removing Bundle: {}", bundle.name));

    // Confirmation
    if !yes {
        output.warning("This will deactivate the bundle and unlink its dotfiles.");
        print!("\nProceed? [y/N] ");
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;

        if !input.trim().eq_ignore_ascii_case("y") {
            output.info("Removal cancelled");
            return Ok(());
        }
    }

    // Deactivate
    output.info("Deactivating bundle...");
    bundle_service.deactivate(id)?;

    output.success(&format!("Bundle '{}' removed", id));

    Ok(())
}
