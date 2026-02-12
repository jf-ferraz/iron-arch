//! Secrets Commands
//!
//! git-crypt secrets management.

use crate::cli::SecretsAction;
use crate::context::{require_init, AppContext};
use crate::output::StatusBadge;
use anyhow::Result;
use iron_core::services::secrets::{SecretsService, SecretsStatus};
use serde::Serialize;
use std::path::Path;

#[derive(Serialize)]
struct SecretsInfo {
    status: String,
    initialized: bool,
    encrypted_files: Vec<String>,
    keys: Vec<KeyInfo>,
}

#[derive(Serialize)]
struct KeyInfo {
    id: String,
    user_id: String,
}

/// Execute secrets command
pub fn execute(ctx: &AppContext, action: SecretsAction) -> Result<()> {
    require_init(ctx)?;

    match action {
        SecretsAction::Status => status(ctx),
        SecretsAction::Unlock { key } => unlock(ctx, key),
        SecretsAction::Lock => lock(ctx),
        SecretsAction::Link => link(ctx),
    }
}

/// Show secrets status
fn status(ctx: &AppContext) -> Result<()> {
    let output = &ctx.output;
    let secrets_service = ctx.secrets_service();

    output.header("Secrets Status");

    let status = secrets_service.status()?;

    if output.is_json() {
        let encrypted = secrets_service.list_encrypted().unwrap_or_default();
        let keys = secrets_service.list_keys().unwrap_or_default();

        let info = SecretsInfo {
            status: format!("{:?}", status),
            initialized: !matches!(status, SecretsStatus::NotInitialized | SecretsStatus::NotAvailable),
            encrypted_files: encrypted.iter().map(|p| p.display().to_string()).collect(),
            keys: keys.iter().map(|k| KeyInfo {
                id: k.id.clone(),
                user_id: k.user_id.clone(),
            }).collect(),
        };
        output.json(&info);
        return Ok(());
    }

    let badge = match &status {
        SecretsStatus::Unlocked => StatusBadge::Unlocked,
        SecretsStatus::Locked => StatusBadge::Locked,
        SecretsStatus::NotInitialized => StatusBadge::Inactive,
        SecretsStatus::NotAvailable => StatusBadge::Error,
    };

    match &status {
        SecretsStatus::Unlocked => {
            output.list_item_status("Secrets are UNLOCKED", badge);
            output.info("Encrypted files are accessible");
        }
        SecretsStatus::Locked => {
            output.list_item_status("Secrets are LOCKED", badge);
            output.info("Run 'iron secrets unlock' to access encrypted files");
        }
        SecretsStatus::NotInitialized => {
            output.list_item_status("git-crypt not initialized", badge);
            output.info("Run 'git-crypt init' to set up secrets encryption");
        }
        SecretsStatus::NotAvailable => {
            output.list_item_status("git-crypt not available", badge);
            output.info("Install git-crypt: sudo pacman -S git-crypt");
        }
    }

    // Show encrypted files
    if !matches!(status, SecretsStatus::NotInitialized | SecretsStatus::NotAvailable) {
        let encrypted = secrets_service.list_encrypted()?;
        if !encrypted.is_empty() {
            output.subheader("Encrypted Files");
            for file in &encrypted {
                let is_locked = secrets_service.is_encrypted(file);
                let badge = if is_locked { StatusBadge::Locked } else { StatusBadge::Unlocked };
                output.list_item_status(&file.display().to_string(), badge);
            }
        }

        // Show keys
        let keys = secrets_service.list_keys()?;
        if !keys.is_empty() {
            output.subheader("Authorized Keys");
            for key in &keys {
                output.list_item(&format!("{} - {}", key.id, key.user_id));
            }
        }
    }

    Ok(())
}

/// Unlock secrets
fn unlock(ctx: &AppContext, key_file: Option<String>) -> Result<()> {
    let output = &ctx.output;
    let secrets_service = ctx.secrets_service();

    output.header("Unlock Secrets");

    // Check current status
    let status = secrets_service.status()?;

    match status {
        SecretsStatus::Unlocked => {
            output.info("Secrets are already unlocked");
            return Ok(());
        }
        SecretsStatus::NotInitialized => {
            output.error("git-crypt not initialized");
            output.info("Run 'git-crypt init' first");
            return Ok(());
        }
        SecretsStatus::NotAvailable => {
            output.error("git-crypt not available");
            output.info("Install: sudo pacman -S git-crypt");
            return Ok(());
        }
        SecretsStatus::Locked => {
            // Continue with unlock
        }
    }

    output.info("Unlocking secrets...");

    let key_path = key_file.as_ref().map(|k| Path::new(k.as_str()));
    match key_path {
        Some(key) => {
            output.verbose(&format!("Using key file: {}", key.display()));
            secrets_service.unlock(Some(key))?;
        }
        None => {
            output.verbose("Using GPG key");
            secrets_service.unlock(None)?;
        }
    }

    output.success("Secrets unlocked");

    // Show unlocked files
    let encrypted = secrets_service.list_encrypted()?;
    if !encrypted.is_empty() {
        output.info(&format!("{} encrypted files now accessible", encrypted.len()));
    }

    Ok(())
}

/// Lock secrets
fn lock(ctx: &AppContext) -> Result<()> {
    let output = &ctx.output;
    let secrets_service = ctx.secrets_service();

    output.header("Lock Secrets");

    let status = secrets_service.status()?;

    match status {
        SecretsStatus::Locked => {
            output.info("Secrets are already locked");
            return Ok(());
        }
        SecretsStatus::NotInitialized | SecretsStatus::NotAvailable => {
            output.warning("git-crypt not available or initialized");
            return Ok(());
        }
        SecretsStatus::Unlocked => {
            // Continue with lock
        }
    }

    output.info("Locking secrets...");
    secrets_service.lock()?;

    output.success("Secrets locked");
    output.info("Encrypted files are now inaccessible");

    Ok(())
}

/// Link secrets to proper locations
fn link(ctx: &AppContext) -> Result<()> {
    let output = &ctx.output;
    let secrets_service = ctx.secrets_service();

    output.header("Link Secrets");

    // Check if unlocked
    let status = secrets_service.status()?;
    if !matches!(status, SecretsStatus::Unlocked) {
        output.error("Secrets must be unlocked first");
        output.info("Run 'iron secrets unlock'");
        return Ok(());
    }

    // Get secrets directory
    let secrets_dir = ctx.root.join("secrets");
    if !secrets_dir.exists() {
        output.warning("No secrets directory found");
        output.info(&format!("Create secrets in: {}", secrets_dir.display()));
        return Ok(());
    }

    output.info("Linking secret files...");

    let mut linked = 0;
    for entry in walkdir::WalkDir::new(&secrets_dir)
        .min_depth(1)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
    {
        let relative = entry.path().strip_prefix(&secrets_dir)?;
        let target = iron_core::validation::expand_home(std::path::Path::new(&format!(
            "~/.{}",
            relative.display()
        )));

        // Create parent directories
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent)?;
        }

        // Create symlink
        if target.exists() && !target.is_symlink() {
            output.verbose(&format!("Skipping existing file: {}", target.display()));
            continue;
        }

        if target.is_symlink() {
            std::fs::remove_file(&target)?;
        }

        #[cfg(unix)]
        std::os::unix::fs::symlink(entry.path(), &target)?;

        output.list_item_status(&format!("{} -> {}", relative.display(), target.display()), StatusBadge::Ok);
        linked += 1;
    }

    if linked > 0 {
        output.success(&format!("Linked {} secret files", linked));
    } else {
        output.info("No secrets to link");
    }

    Ok(())
}
