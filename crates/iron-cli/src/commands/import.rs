//! Iron Import Command
//!
//! Scaffolds Iron modules from existing dotfiles. The first source is a
//! home-manager build output (`home-manager build` → `result`).

use crate::cli::ImportAction;
use crate::context::AppContext;
use crate::output::StatusBadge;
use anyhow::Result;
use iron_core::import::HomeManagerImporter;
use iron_core::validation::expand_home;
use std::path::Path;

/// Execute the `import` command.
pub fn execute(ctx: &AppContext, action: ImportAction) -> Result<()> {
    match action {
        ImportAction::HomeManager {
            path,
            dry_run,
            force,
            only,
        } => home_manager(ctx, &path, dry_run, force, only),
    }
}

fn home_manager(
    ctx: &AppContext,
    path: &str,
    dry_run: bool,
    force: bool,
    only: Option<Vec<String>>,
) -> Result<()> {
    let output = &ctx.output;
    let input = expand_home(Path::new(path));
    let modules_dir = ctx.root.join("modules");

    let importer = HomeManagerImporter::new(&input, &modules_dir, only)?;
    let plan = importer.plan()?;

    if plan.modules.is_empty() {
        if output.is_json() {
            output.json(&serde_json::json!({ "modules": [], "skipped_dirs": plan.skipped_dirs }));
        } else {
            output.warning("No importable dotfiles found under the given path.");
        }
        return Ok(());
    }

    // Apply (unless dry-run) before emitting, so JSON carries the real outcome.
    let report = if dry_run {
        None
    } else {
        Some(importer.execute(&plan, force)?)
    };

    if output.is_json() {
        output.json(&serde_json::json!({
            "source": importer.home_files().display().to_string(),
            "modules_dir": modules_dir.display().to_string(),
            "dry_run": dry_run,
            "plan": plan,
            "report": report,
        }));
        return Ok(());
    }

    output.header("Import from home-manager");
    output.kv("Source", importer.home_files().display());
    output.kv("Modules dir", modules_dir.display());

    let rows: Vec<Vec<String>> = plan
        .modules
        .iter()
        .map(|m| {
            vec![
                m.id.clone(),
                format!("{:?}", m.kind),
                m.target.clone(),
                m.packages.join(", "),
                m.file_count.to_string(),
                if m.already_exists { "exists" } else { "new" }.to_string(),
            ]
        })
        .collect();
    output.table(
        &["module", "kind", "target", "packages?", "files", "status"],
        &rows,
    );

    if !plan.skipped_dirs.is_empty() {
        output.info(&format!(
            "Skipped {} top-level director{} (import manually if needed): {}",
            plan.skipped_dirs.len(),
            if plan.skipped_dirs.len() == 1 { "y" } else { "ies" },
            plan.skipped_dirs.join(", "),
        ));
    }

    match report {
        None => {
            output.info("Dry run — nothing written. Re-run without --dry-run to create the modules.");
        }
        Some(report) => {
            output.subheader("Result");
            for id in &report.created {
                output.list_item_status(id, StatusBadge::Ok);
            }
            for id in &report.skipped {
                output.list_item_status(
                    &format!("{id} (already exists — use --force to overwrite)"),
                    StatusBadge::Warning,
                );
            }
            output.success(&format!(
                "Created {} module(s), copied {} file(s).",
                report.created.len(),
                report.files_copied,
            ));
            output.info(
                "Packages are best-effort guesses — review each module.toml. \
                 Add the modules to a profile to apply them.",
            );
        }
    }

    Ok(())
}
