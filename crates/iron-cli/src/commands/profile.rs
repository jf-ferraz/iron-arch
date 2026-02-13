//! Profile Commands
//!
//! Profile management.

use crate::cli::ProfileAction;
use crate::context::{AppContext, require_init};
use crate::output::StatusBadge;
use anyhow::Result;
use iron_core::profile::ProfileState;
use iron_core::services::profile::ProfileService;
use serde::Serialize;
use std::fs;
use std::process::Command;

#[derive(Serialize)]
struct ProfileInfo {
    id: String,
    name: String,
    description: Option<String>,
    extends: Option<String>,
    modules: Vec<String>,
    state: String,
}

/// Execute profile command
pub fn execute(ctx: &AppContext, action: ProfileAction) -> Result<()> {
    require_init(ctx)?;

    match action {
        ProfileAction::List { bundle } => list(ctx, bundle),
        ProfileAction::Show { id, effective } => show(ctx, &id, effective),
        ProfileAction::Select { id } => select(ctx, &id),
        ProfileAction::Create { id, name, extends } => create(ctx, &id, name, extends),
        ProfileAction::Edit { id } => edit(ctx, &id),
    }
}

/// List profiles
fn list(ctx: &AppContext, bundle_filter: Option<String>) -> Result<()> {
    let output = &ctx.output;
    let profile_service = ctx.profile_service();

    let profiles = profile_service.discover()?;

    if profiles.is_empty() {
        output.warning("No profiles found");
        output.info("Create profiles in ~/.config/iron/profiles/");
        return Ok(());
    }

    // Filter by bundle if specified
    let profiles: Vec<_> = if let Some(bundle) = &bundle_filter {
        profiles
            .into_iter()
            .filter(|p| p.for_bundle.as_ref().map(|b| b == bundle).unwrap_or(true))
            .collect()
    } else {
        profiles
    };

    output.header("Available Profiles");

    if output.is_json() {
        let profile_info: Vec<ProfileInfo> = profiles
            .iter()
            .map(|p| {
                let state = profile_service
                    .state(&p.id)
                    .unwrap_or(ProfileState::Inactive);
                ProfileInfo {
                    id: p.id.clone(),
                    name: p.name.clone(),
                    description: p.description.clone(),
                    extends: p.extends.clone(),
                    modules: p.modules.clone(),
                    state: format!("{:?}", state),
                }
            })
            .collect();
        output.json(&profile_info);
        return Ok(());
    }

    // Get active profile
    let active = profile_service.active().ok().flatten();

    for profile in &profiles {
        let state = profile_service
            .state(&profile.id)
            .unwrap_or(ProfileState::Inactive);
        let is_active = active.as_ref().map(|a| a.id == profile.id).unwrap_or(false);

        let badge = match state {
            ProfileState::Active => StatusBadge::Active,
            ProfileState::Partial => StatusBadge::Partial,
            ProfileState::Inactive => StatusBadge::Inactive,
        };

        let active_marker = if is_active { " (active)" } else { "" };
        let extends_info = profile
            .extends
            .as_ref()
            .map(|e| format!(" [extends: {}]", e))
            .unwrap_or_default();

        output.list_item_status(
            &format!(
                "{} - {}{}{}",
                profile.id, profile.name, extends_info, active_marker
            ),
            badge,
        );

        if output.is_verbose() {
            if let Some(desc) = &profile.description {
                output.verbose(&format!("  {}", desc));
            }
            output.verbose(&format!("  Modules: {}", profile.modules.len()));
        }
    }

    Ok(())
}

/// Show profile details
fn show(ctx: &AppContext, id: &str, effective: bool) -> Result<()> {
    let output = &ctx.output;
    let profile_service = ctx.profile_service();

    let profile = profile_service.load(id)?;
    let state = profile_service.state(id)?;

    if output.is_json() {
        let modules = if effective {
            profile_service.effective_modules(id)?
        } else {
            profile.modules.clone()
        };

        let info = ProfileInfo {
            id: profile.id.clone(),
            name: profile.name.clone(),
            description: profile.description.clone(),
            extends: profile.extends.clone(),
            modules,
            state: format!("{:?}", state),
        };
        output.json(&info);
        return Ok(());
    }

    output.header(&format!("Profile: {}", profile.name));

    output.kv("ID", &profile.id);
    output.kv("State", format!("{:?}", state));

    if let Some(desc) = &profile.description {
        output.kv("Description", desc);
    }

    if let Some(extends) = &profile.extends {
        output.kv("Extends", extends);
    }

    if let Some(bundle) = &profile.for_bundle {
        output.kv("For Bundle", bundle);
    }

    // Show modules
    if effective {
        output.subheader("Effective Modules (including inherited)");
        let effective_modules = profile_service.effective_modules(id)?;
        for module in &effective_modules {
            output.list_item(module);
        }
    } else {
        output.subheader("Direct Modules");
        for module in &profile.modules {
            output.list_item(module);
        }
    }

    // Show inheritance chain
    if profile.extends.is_some() {
        output.subheader("Inheritance Chain");
        let chain = profile_service.resolve_inheritance(id)?;
        for (i, profile_id) in chain.iter().enumerate() {
            let prefix = if i == 0 { "→" } else { "  ↳" };
            output.list_item(&format!("{} {}", prefix, profile_id));
        }
    }

    Ok(())
}

/// Select/activate a profile
fn select(ctx: &AppContext, id: &str) -> Result<()> {
    let output = &ctx.output;
    let profile_service = ctx.profile_service();

    let profile = profile_service.load(id)?;

    output.header(&format!("Activating Profile: {}", profile.name));

    // Show effective modules
    let effective_modules = profile_service.effective_modules(id)?;
    output.info(&format!("Will enable {} modules:", effective_modules.len()));
    for module in &effective_modules {
        output.list_item(module);
    }

    // Apply profile
    output.info("Applying profile...");
    profile_service.apply(id)?;

    output.success(&format!("Profile '{}' activated", id));

    Ok(())
}

/// Create a new profile
fn create(ctx: &AppContext, id: &str, name: Option<String>, extends: Option<String>) -> Result<()> {
    let output = &ctx.output;

    // Validate ID
    if !id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
        anyhow::bail!("Profile ID must be alphanumeric with hyphens only");
    }

    let profile_dir = ctx.root.join("profiles").join(id);
    if profile_dir.exists() {
        anyhow::bail!("Profile '{}' already exists", id);
    }

    let profile_name = name.unwrap_or_else(|| id.replace('-', " ").to_string());

    output.header(&format!("Creating Profile: {}", profile_name));

    // Create directory
    fs::create_dir_all(&profile_dir)?;

    // Create profile.toml
    let mut content = format!(
        r#"id = "{}"
name = "{}"
"#,
        id, profile_name
    );

    if let Some(ext) = &extends {
        content.push_str(&format!("extends = \"{}\"\n", ext));
    }

    content.push_str("\nmodules = []\n");

    let profile_path = profile_dir.join("profile.toml");
    fs::write(&profile_path, content)?;

    output.success(&format!("Profile '{}' created", id));
    output.info(&format!("Edit: {}", profile_path.display()));

    Ok(())
}

/// Edit profile in $EDITOR
fn edit(ctx: &AppContext, id: &str) -> Result<()> {
    let output = &ctx.output;
    let profile_service = ctx.profile_service();

    // Verify profile exists
    let _ = profile_service.load(id)?;

    let profile_path = ctx.root.join("profiles").join(id).join("profile.toml");

    let editor = std::env::var("EDITOR").unwrap_or_else(|_| "nano".to_string());

    output.info(&format!(
        "Opening {} in {}...",
        profile_path.display(),
        editor
    ));

    let status = Command::new(&editor).arg(&profile_path).status()?;

    if status.success() {
        output.success("Profile saved");
    } else {
        output.warning("Editor exited with non-zero status");
    }

    Ok(())
}
