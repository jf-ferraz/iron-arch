//! Sync Commands
//!
//! Git sync operations.

use crate::cli::SyncAction;
use crate::context::{AppContext, require_init};
use crate::output::StatusBadge;
use anyhow::Result;
use iron_core::services::sync::{SyncService, SyncStatus};
use serde::Serialize;

#[derive(Serialize)]
struct SyncInfo {
    status: String,
    ahead: usize,
    behind: usize,
    dirty_files: usize,
    branch: Option<String>,
    remote_branch: Option<String>,
}

/// Execute sync command
pub fn execute(ctx: &AppContext, action: SyncAction) -> Result<()> {
    require_init(ctx)?;

    match action {
        SyncAction::Status => status(ctx),
        SyncAction::Push { message } => push(ctx, message),
        SyncAction::Pull { stash } => pull(ctx, stash),
    }
}

/// Show sync status
fn status(ctx: &AppContext) -> Result<()> {
    let output = &ctx.output;
    let sync_service = ctx.sync_service();

    let info = sync_service.status()?;

    if matches!(info.status, SyncStatus::NotARepo) {
        output.warning("Iron directory is not a git repository");
        output.info("Initialize with: git init");
        return Ok(());
    }

    output.header("Sync Status");

    if output.is_json() {
        let sync_info = SyncInfo {
            status: format!("{:?}", info.status),
            ahead: info.commits_ahead,
            behind: info.commits_behind,
            dirty_files: info.dirty_files,
            branch: info.branch.clone(),
            remote_branch: info.remote_branch.clone(),
        };
        output.json(&sync_info);
        return Ok(());
    }

    // Show branch and remote
    if let Some(branch) = &info.branch {
        output.kv("Branch", branch);
    }
    if let Some(remote) = &info.remote_branch {
        output.kv("Remote", remote);
    }

    // Show status
    let badge = match &info.status {
        SyncStatus::UpToDate => StatusBadge::Ok,
        SyncStatus::Ahead => StatusBadge::Warning,
        SyncStatus::Behind => StatusBadge::Warning,
        SyncStatus::Diverged => StatusBadge::Error,
        SyncStatus::Dirty => StatusBadge::Warning,
        SyncStatus::NotARepo => StatusBadge::Error,
    };

    match &info.status {
        SyncStatus::UpToDate => {
            output.list_item_status("Up to date with remote", badge);
        }
        SyncStatus::Ahead => {
            output.list_item_status(
                &format!("{} commits ahead of remote", info.commits_ahead),
                badge,
            );
            output.info("Run 'iron sync push' to push changes");
        }
        SyncStatus::Behind => {
            output.list_item_status(
                &format!("{} commits behind remote", info.commits_behind),
                badge,
            );
            output.info("Run 'iron sync pull' to pull changes");
        }
        SyncStatus::Diverged => {
            output.list_item_status(
                &format!(
                    "Diverged: {} ahead, {} behind",
                    info.commits_ahead, info.commits_behind
                ),
                badge,
            );
            output.warning("Manual merge may be required");
        }
        SyncStatus::Dirty => {
            output.list_item_status(&format!("{} uncommitted changes", info.dirty_files), badge);
            output.info("Commit changes before syncing");
        }
        SyncStatus::NotARepo => {
            output.list_item_status("Not a git repository", badge);
        }
    }

    Ok(())
}

/// Push changes to remote
fn push(ctx: &AppContext, message: Option<String>) -> Result<()> {
    let output = &ctx.output;
    let sync_service = ctx.sync_service();

    let info = sync_service.status()?;

    if matches!(info.status, SyncStatus::NotARepo) {
        output.error("Not a git repository");
        return Ok(());
    }

    output.header("Push to Remote");

    match &info.status {
        SyncStatus::UpToDate => {
            output.info("Already up to date");
            return Ok(());
        }
        SyncStatus::Behind => {
            output.warning("Behind remote - pull first");
            return Ok(());
        }
        SyncStatus::Diverged => {
            output.error("Repository has diverged - manual merge required");
            return Ok(());
        }
        SyncStatus::Dirty => {
            output.info(&format!("{} uncommitted changes", info.dirty_files));

            // Commit changes
            let commit_msg = message.unwrap_or_else(|| "Iron sync".to_string());
            output.info(&format!("Committing with message: {}", commit_msg));

            sync_service.commit(&commit_msg)?;
            output.list_item_status("Changes committed", StatusBadge::Ok);
        }
        _ => {}
    }

    // Push
    output.info("Pushing to remote...");
    sync_service.push()?;

    output.success("Changes pushed to remote");

    Ok(())
}

/// Pull changes from remote
fn pull(ctx: &AppContext, stash: bool) -> Result<()> {
    let output = &ctx.output;
    let sync_service = ctx.sync_service();

    let info = sync_service.status()?;

    if matches!(info.status, SyncStatus::NotARepo) {
        output.error("Not a git repository");
        return Ok(());
    }

    output.header("Pull from Remote");

    let was_dirty = matches!(info.status, SyncStatus::Dirty);

    match &info.status {
        SyncStatus::UpToDate => {
            output.info("Already up to date");
            return Ok(());
        }
        SyncStatus::Dirty => {
            if stash {
                output.info(&format!(
                    "Stashing {} uncommitted changes",
                    info.dirty_files
                ));
                sync_service.stash()?;
                output.list_item_status("Changes stashed", StatusBadge::Ok);
            } else {
                output.warning("Uncommitted changes detected");
                print!("Stash and continue? [y/N] ");
                std::io::Write::flush(&mut std::io::stdout())?;

                let mut input = String::new();
                std::io::stdin().read_line(&mut input)?;

                if input.trim().eq_ignore_ascii_case("y") {
                    sync_service.stash()?;
                    output.list_item_status("Changes stashed", StatusBadge::Ok);
                } else {
                    output.info("Pull cancelled");
                    return Ok(());
                }
            }
        }
        SyncStatus::Diverged => {
            output.warning("Repository has diverged");
            output.info("Attempting to pull with rebase...");
        }
        _ => {}
    }

    // Pull
    output.info("Pulling from remote...");
    let result = sync_service.pull();

    match result {
        Ok(()) => {
            output.success("Changes pulled from remote");

            // Pop stash if we stashed
            if was_dirty && stash {
                output.info("Restoring stashed changes...");
                if sync_service.stash_pop().is_ok() {
                    output.list_item_status("Stashed changes restored", StatusBadge::Ok);
                } else {
                    output.warning("Could not restore stashed changes - check 'git stash list'");
                }
            }
        }
        Err(e) => {
            output.error(&format!("Pull failed: {}", e));
            output.info("Manual intervention may be required");
        }
    }

    Ok(())
}
