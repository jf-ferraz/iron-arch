//! Iron Import Command
//!
//! Scaffolds Iron modules from existing dotfiles. The first source is a
//! home-manager build output (`home-manager build` → `result`).

use crate::cli::ImportAction;
use crate::context::AppContext;
use crate::output::StatusBadge;
use anyhow::Result;
use iron_core::import::{HomeManagerImporter, add_modules_to_profile};
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
            into_profile,
            guess_packages,
            strip_store_paths,
        } => home_manager(
            ctx,
            &path,
            dry_run,
            force,
            only,
            into_profile,
            guess_packages,
            strip_store_paths,
        ),
    }
}

#[allow(clippy::too_many_arguments)]
fn home_manager(
    ctx: &AppContext,
    path: &str,
    dry_run: bool,
    force: bool,
    only: Option<Vec<String>>,
    into_profile: Option<String>,
    guess_packages: bool,
    strip_store_paths: bool,
) -> Result<()> {
    let output = &ctx.output;
    let input = expand_home(Path::new(path));
    let modules_dir = ctx.root.join("modules");

    let importer = HomeManagerImporter::new(&input, &modules_dir, only)?
        .with_package_guessing(guess_packages)
        .with_store_path_stripping(strip_store_paths);
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

    // Add the imported modules to a profile (non-dry-run only), so the result is
    // directly `iron apply`-able.
    let profile_update = match (&report, &into_profile) {
        (Some(_), Some(name)) => {
            let ids: Vec<String> = plan.modules.iter().map(|m| m.id.clone()).collect();
            Some(add_modules_to_profile(
                &ctx.root.join("profiles"),
                name,
                &ids,
            )?)
        }
        _ => None,
    };

    if output.is_json() {
        output.json(&serde_json::json!({
            "source": importer.home_files().display().to_string(),
            "modules_dir": modules_dir.display().to_string(),
            "dry_run": dry_run,
            "plan": plan,
            "report": report,
            "profile": profile_update,
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
                if m.store_refs > 0 {
                    m.store_refs.to_string()
                } else {
                    String::new()
                },
                if m.already_exists { "exists" } else { "new" }.to_string(),
            ]
        })
        .collect();
    output.table(
        &[
            "module",
            "kind",
            "target",
            "packages?",
            "files",
            "nix-paths",
            "status",
        ],
        &rows,
    );

    if !plan.skipped_dirs.is_empty() {
        output.info(&format!(
            "Skipped {} top-level director{} (import manually if needed): {}",
            plan.skipped_dirs.len(),
            if plan.skipped_dirs.len() == 1 {
                "y"
            } else {
                "ies"
            },
            plan.skipped_dirs.join(", "),
        ));
    }

    match report {
        None => {
            output
                .info("Dry run — nothing written. Re-run without --dry-run to create the modules.");
            if let Some(name) = &into_profile {
                output.info(&format!(
                    "Would add {} module(s) to profile '{}'.",
                    plan.modules.len(),
                    name
                ));
            }
            let with_refs = plan.modules.iter().filter(|m| m.store_refs > 0).count();
            if with_refs > 0 {
                output.info(&format!(
                    "{with_refs} module(s) reference /nix/store (see nix-paths column) — dead on \
                     Arch. Add --strip-store-paths to rewrite the `/bin/` cases on import.",
                ));
            }
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

            if report.stripped_refs > 0 {
                output.info(&format!(
                    "Stripped {} /nix/store/<pkg>/bin/ reference(s) across {} file(s).",
                    report.stripped_refs, report.stripped_files
                ));
            }

            if let Some(pu) = &profile_update {
                output.kv(
                    if pu.created {
                        "Profile created"
                    } else {
                        "Profile updated"
                    },
                    format!(
                        "{} ({} module(s), +{} new)",
                        pu.profile,
                        pu.total,
                        pu.added.len()
                    ),
                );
            }

            if guess_packages {
                output.info("Package guesses added — review each module.toml before applying.");
            } else {
                output.info(
                    "Modules are dotfiles-only (no packages). Use --guess-packages to add \
                     best-effort package names, or curate them per module.",
                );
            }
            if into_profile.is_none() {
                output.info(
                    "Tip: pass --into-profile <name> to make the import directly apply-able.",
                );
            }

            if !report.modules_with_store_refs.is_empty() {
                output.warning(&format!(
                    "{} module(s) still reference /nix/store (dead on Arch) — fix before applying: {}",
                    report.modules_with_store_refs.len(),
                    report.modules_with_store_refs.join(", "),
                ));
                if !strip_store_paths {
                    output.info(
                        "Re-run with --strip-store-paths to auto-rewrite the `/bin/` cases; \
                         the rest need a manual edit.",
                    );
                }
            }
        }
    }

    Ok(())
}
