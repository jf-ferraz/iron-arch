//! Module Commands
//!
//! Module management.

use crate::cli::ModuleAction;
use crate::context::{AppContext, require_init};
use crate::output::StatusBadge;
use anyhow::Result;
use iron_core::module::ModuleState;
use iron_core::services::module::ModuleService;
use serde::Serialize;
use std::io::{self, Write};

#[derive(Serialize)]
struct ModuleInfo {
    id: String,
    name: String,
    description: Option<String>,
    kind: String,
    state: String,
    packages: usize,
    dotfiles: usize,
}

/// Execute module command
pub fn execute(ctx: &AppContext, action: ModuleAction) -> Result<()> {
    require_init(ctx)?;

    match action {
        ModuleAction::List {
            enabled,
            disabled,
            kind,
        } => list(ctx, enabled, disabled, kind),
        ModuleAction::Show { id } => show(ctx, &id),
        ModuleAction::Enable { id, force } => enable(ctx, &id, force),
        ModuleAction::Disable { id, yes } => disable(ctx, &id, yes),
        ModuleAction::Create {
            id,
            description,
            kind,
        } => create(ctx, &id, description.as_deref(), &kind),
    }
}

/// List modules
fn list(
    ctx: &AppContext,
    enabled_only: bool,
    disabled_only: bool,
    kind_filter: Option<String>,
) -> Result<()> {
    let output = &ctx.output;
    let module_service = ctx.module_service();

    let modules = module_service.discover()?;

    if modules.is_empty() {
        output.warning("No modules found");
        output.info("Create modules in ~/.config/iron/modules/");
        return Ok(());
    }

    // Filter modules
    let modules: Vec<_> = modules
        .into_iter()
        .filter(|m| {
            let state = module_service
                .status(&m.id)
                .unwrap_or(ModuleState::NotInstalled);
            let is_enabled = matches!(state, ModuleState::Installed | ModuleState::Partial);

            // Filter by enabled/disabled
            if enabled_only && !is_enabled {
                return false;
            }
            if disabled_only && is_enabled {
                return false;
            }

            // Filter by kind
            if let Some(kind) = &kind_filter {
                let kind_str = format!("{:?}", m.kind).to_lowercase();
                if !kind_str.contains(&kind.to_lowercase()) {
                    return false;
                }
            }

            true
        })
        .collect();

    output.header("Available Modules");

    if output.is_json() {
        let module_info: Vec<ModuleInfo> = modules
            .iter()
            .map(|m| {
                let state = module_service
                    .status(&m.id)
                    .unwrap_or(ModuleState::NotInstalled);
                ModuleInfo {
                    id: m.id.clone(),
                    name: m.name.clone(),
                    description: m.description.clone(),
                    kind: format!("{:?}", m.kind),
                    state: format!("{:?}", state),
                    packages: m.packages.len() + m.aur_packages.len(),
                    dotfiles: m.dotfiles.len(),
                }
            })
            .collect();
        output.json(&module_info);
        return Ok(());
    }

    // Group by kind
    let mut by_kind: std::collections::HashMap<String, Vec<_>> = std::collections::HashMap::new();
    for module in &modules {
        let kind = format!("{:?}", module.kind);
        by_kind.entry(kind).or_default().push(module);
    }

    for (kind, mods) in &by_kind {
        output.subheader(kind);

        for module in mods {
            let state = module_service
                .status(&module.id)
                .unwrap_or(ModuleState::NotInstalled);

            let badge = match state {
                ModuleState::Installed => StatusBadge::Installed,
                ModuleState::Partial => StatusBadge::Partial,
                ModuleState::NotInstalled => StatusBadge::NotInstalled,
            };

            output.list_item_status(&format!("{} - {}", module.id, module.name), badge);

            if output.is_verbose() {
                if let Some(desc) = &module.description {
                    output.verbose(&format!("  {}", desc));
                }
                output.verbose(&format!("  Dotfiles: {}", module.dotfiles.len()));
            }
        }
    }

    Ok(())
}

/// Show module details
fn show(ctx: &AppContext, id: &str) -> Result<()> {
    let output = &ctx.output;
    let module_service = ctx.module_service();

    let module = module_service.load(id)?;
    let state = module_service.status(id)?;

    if output.is_json() {
        let info = ModuleInfo {
            id: module.id.clone(),
            name: module.name.clone(),
            description: module.description.clone(),
            kind: format!("{:?}", module.kind),
            state: format!("{:?}", state),
            packages: module.packages.len() + module.aur_packages.len(),
            dotfiles: module.dotfiles.len(),
        };
        output.json(&info);
        return Ok(());
    }

    output.header(&format!("Module: {}", module.name));

    output.kv("ID", &module.id);
    output.kv("Kind", format!("{:?}", module.kind));
    output.kv("State", format!("{:?}", state));

    if let Some(desc) = &module.description {
        output.kv("Description", desc);
    }

    if !module.packages.is_empty() {
        output.subheader("Packages");
        for pkg in &module.packages {
            output.list_item(pkg);
        }
    }

    if !module.aur_packages.is_empty() {
        output.subheader("AUR Packages");
        for pkg in &module.aur_packages {
            output.list_item(pkg);
        }
    }

    if !module.dotfiles.is_empty() {
        output.subheader("Dotfiles");
        for df in &module.dotfiles {
            output.list_item(&format!("{} -> {}", df.source, df.target));
        }
    }

    if !module.depends.is_empty() {
        output.subheader("Dependencies");
        for dep in &module.depends {
            output.list_item(dep);
        }
    }

    if !module.conflicts.is_empty() {
        output.subheader("Conflicts");
        for conflict in &module.conflicts {
            output.list_item(conflict);
        }
    }

    Ok(())
}

/// Enable a module
fn enable(ctx: &AppContext, id: &str, force: bool) -> Result<()> {
    let output = &ctx.output;
    let module_service = ctx.module_service();

    let module = module_service.load(id)?;
    let state = module_service.status(id)?;

    if matches!(state, ModuleState::Installed) {
        output.warning(&format!("Module '{}' is already enabled", id));
        return Ok(());
    }

    output.header(&format!("Enabling Module: {}", module.name));

    // Check conflicts
    if !force {
        let conflicts = module_service.check_conflicts(id)?;
        if !conflicts.is_empty() {
            output.error("Module conflicts with:");
            for conflict in &conflicts {
                output.list_item(conflict);
            }
            output.info("Use --force to enable anyway");
            return Ok(());
        }
    }

    // Show what will happen
    if !module.dotfiles.is_empty() {
        output.subheader("Dotfiles to link:");
        for df in &module.dotfiles {
            output.list_item(&format!("{} -> {}", df.source, df.target));
        }
    }

    // Enable
    output.info("Enabling module...");
    module_service.enable(id)?;

    output.success(&format!("Module '{}' enabled", id));

    output.summary(&[
        ("packages", module.packages.len() + module.aur_packages.len()),
        ("configs linked", module.dotfiles.len()),
    ]);

    Ok(())
}

/// Disable a module
fn disable(ctx: &AppContext, id: &str, yes: bool) -> Result<()> {
    let output = &ctx.output;
    let module_service = ctx.module_service();

    let module = module_service.load(id)?;
    let state = module_service.status(id)?;

    if matches!(state, ModuleState::NotInstalled) {
        output.warning(&format!("Module '{}' is not enabled", id));
        return Ok(());
    }

    output.header(&format!("Disabling Module: {}", module.name));

    // Confirmation
    if !yes {
        output.warning("This will unlink the module's dotfiles.");
        print!("\nProceed? [y/N] ");
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;

        if !input.trim().eq_ignore_ascii_case("y") {
            output.info("Disable cancelled");
            return Ok(());
        }
    }

    // Disable
    output.info("Disabling module...");
    module_service.disable(id)?;

    output.success(&format!("Module '{}' disabled", id));

    output.summary(&[
        ("configs unlinked", module.dotfiles.len()),
    ]);

    Ok(())
}

/// Create a new module scaffold (C-004)
fn create(ctx: &AppContext, id: &str, description: Option<&str>, kind: &str) -> Result<()> {
    let output = &ctx.output;

    // Validate ID: lowercase alphanumeric + hyphens
    if !id
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        || id.is_empty()
        || id.starts_with('-')
    {
        output.error("Invalid module ID");
        output.info("IDs must be lowercase alphanumeric with hyphens (e.g. 'my-module')");
        return Ok(());
    }

    let module_dir = ctx.root.join("modules").join(id);
    if module_dir.exists() {
        output.error(&format!("Module '{}' already exists", id));
        return Ok(());
    }

    output.header(&format!("Creating Module: {}", id));

    // Create directory structure
    std::fs::create_dir_all(module_dir.join("config"))?;

    // Create module.toml
    let desc = description.unwrap_or("A new iron module");
    let toml_content = format!(
        r#"# Module: {id}
id = "{id}"
name = "{name}"
description = "{desc}"
kind = "{kind}"

packages = []
aur_packages = []
conflicts = []
depends = []

# Dotfiles mapping
# [[dotfiles]]
# source = "config/{id}"
# target = "~/.config/{id}"
# link = true
"#,
        id = id,
        name = id.replace('-', " "),
        desc = desc,
        kind = kind,
    );

    std::fs::write(module_dir.join("module.toml"), toml_content)?;

    output.success(&format!("Created module scaffold at modules/{}", id));
    output.info("Edit modules/{}/module.toml to configure packages and dotfiles");

    Ok(())
}
