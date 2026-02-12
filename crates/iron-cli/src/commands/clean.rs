//! Iron Clean Command
//!
//! System cleanup operations.

use crate::context::{require_init, AppContext};
use crate::output::StatusBadge;
use anyhow::Result;
use iron_core::services::module::ModuleService;
use iron_core::validation::expand_home;
use std::path::Path;
use std::process::Command;

/// Execute clean command
pub fn execute(ctx: &AppContext, orphans: bool, cache: bool, symlinks: bool, all: bool) -> Result<()> {
    require_init(ctx)?;

    let output = &ctx.output;
    let do_all = all || (!orphans && !cache && !symlinks);

    output.header("Iron Cleanup");

    let mut cleaned = false;

    // Clean orphan packages
    if orphans || do_all {
        output.subheader("Orphan Packages");
        clean_orphans(ctx)?;
        cleaned = true;
    }

    // Clear package cache
    if cache || do_all {
        output.subheader("Package Cache");
        clean_cache(ctx)?;
        cleaned = true;
    }

    // Fix broken symlinks
    if symlinks || do_all {
        output.subheader("Broken Symlinks");
        clean_symlinks(ctx)?;
        cleaned = true;
    }

    if !cleaned {
        output.info("No cleanup operations specified.");
        output.info("Use --all or specify: --orphans, --cache, --symlinks");
    } else {
        output.separator();
        output.success("Cleanup complete");
    }

    Ok(())
}

/// Clean orphan packages
fn clean_orphans(ctx: &AppContext) -> Result<()> {
    let output = &ctx.output;

    // Check for orphans
    let orphan_check = Command::new("pacman")
        .args(["-Qtdq"])
        .output();

    match orphan_check {
        Ok(result) => {
            let orphans_output = String::from_utf8_lossy(&result.stdout);
            let orphans: Vec<&str> = orphans_output.lines().collect();

            if orphans.is_empty() {
                output.list_item_status("No orphan packages found", StatusBadge::Ok);
            } else {
                output.info(&format!("Found {} orphan packages:", orphans.len()));
                for pkg in &orphans {
                    output.list_item(pkg);
                }
                output.warning("Run 'sudo pacman -Rns $(pacman -Qtdq)' to remove");
            }
        }
        Err(_) => {
            output.warning("Could not check for orphan packages");
        }
    }

    Ok(())
}

/// Clean package cache
fn clean_cache(ctx: &AppContext) -> Result<()> {
    let output = &ctx.output;

    // Check cache size
    let cache_path = Path::new("/var/cache/pacman/pkg");
    if cache_path.exists() {
        if let Ok(entries) = std::fs::read_dir(cache_path) {
            let count = entries.count();
            output.info(&format!("Package cache contains {} files", count));
            output.warning("Run 'sudo paccache -r' to clean old versions");
            output.warning("Run 'sudo paccache -ruk0' to remove uninstalled packages");
        }
    } else {
        output.list_item_status("Package cache not found", StatusBadge::Warning);
    }

    Ok(())
}

/// Clean broken symlinks
fn clean_symlinks(ctx: &AppContext) -> Result<()> {
    let output = &ctx.output;
    let module_service = ctx.module_service();
    let modules = module_service.discover().unwrap_or_default();

    let mut broken = Vec::new();

    for module in &modules {
        for dotfile in &module.dotfiles {
            let target = expand_home(Path::new(&dotfile.target));
            if target.is_symlink() {
                if let Ok(link_target) = std::fs::read_link(&target) {
                    if !link_target.exists() {
                        broken.push(target.clone());
                    }
                }
            }
        }
    }

    if broken.is_empty() {
        output.list_item_status("No broken symlinks found", StatusBadge::Ok);
    } else {
        output.info(&format!("Found {} broken symlinks:", broken.len()));
        for link in &broken {
            output.list_item(&link.display().to_string());

            // Remove the broken symlink
            if std::fs::remove_file(link).is_ok() {
                output.verbose(&format!("  Removed: {}", link.display()));
            }
        }
        output.success(&format!("Removed {} broken symlinks", broken.len()));
    }

    Ok(())
}
