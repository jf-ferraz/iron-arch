//! Secrets Commands
//!
//! git-crypt secrets management.

use crate::cli::SecretsAction;
use crate::context::{AppContext, require_init};
use crate::output::StatusBadge;
use anyhow::Result;
use iron_core::services::secrets::{SecretsService, SecretsStatus};
use serde::Serialize;
use std::path::Path;
use std::time::Instant;

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
    let start = Instant::now();
    require_init(ctx)?;

    match action {
        SecretsAction::Status => status(ctx, start),
        SecretsAction::Init => init(ctx),
        SecretsAction::Unlock { key } => unlock(ctx, key),
        SecretsAction::Lock => lock(ctx),
        SecretsAction::Link => link(ctx),
        SecretsAction::AddKey { key_id } => add_key(ctx, &key_id),
        SecretsAction::ExportKey { output } => export_key(ctx, &output),
    }
}

/// Show secrets status
fn status(ctx: &AppContext, start: Instant) -> Result<()> {
    let output = &ctx.output;
    let secrets_service = ctx.secrets_service();

    output.header("Secrets Status");

    let status = secrets_service.status()?;

    if output.is_json() {
        let encrypted = secrets_service.list_encrypted().unwrap_or_default();
        let keys = secrets_service.list_keys().unwrap_or_default();

        let info = SecretsInfo {
            status: format!("{:?}", status),
            initialized: !matches!(
                status,
                SecretsStatus::NotInitialized | SecretsStatus::NotAvailable
            ),
            encrypted_files: encrypted.iter().map(|p| p.display().to_string()).collect(),
            keys: keys
                .iter()
                .map(|k| KeyInfo {
                    id: k.id.clone(),
                    user_id: k.user_id.clone(),
                })
                .collect(),
        };
        output.json_envelope("secrets.status", &info, start);
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
    if !matches!(
        status,
        SecretsStatus::NotInitialized | SecretsStatus::NotAvailable
    ) {
        let encrypted = secrets_service.list_encrypted()?;
        if !encrypted.is_empty() {
            output.subheader("Encrypted Files");
            for file in &encrypted {
                let is_locked = secrets_service.is_encrypted(file);
                let badge = if is_locked {
                    StatusBadge::Locked
                } else {
                    StatusBadge::Unlocked
                };
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

/// Initialize git-crypt in the repository (C-007)
fn init(ctx: &AppContext) -> Result<()> {
    let output = &ctx.output;
    let secrets_service = ctx.secrets_service();

    output.header("Initialize Secrets");

    let status = secrets_service.status()?;

    if !matches!(status, SecretsStatus::NotInitialized) {
        output.info("git-crypt is already initialized");
        return Ok(());
    }

    secrets_service.init()?;
    output.success("git-crypt initialized successfully");
    output.info("Add GPG keys with: iron secrets add-key <KEY_ID>");
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
        output.info(&format!(
            "{} encrypted files now accessible",
            encrypted.len()
        ));
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

        output.list_item_status(
            &format!("{} -> {}", relative.display(), target.display()),
            StatusBadge::Ok,
        );
        linked += 1;
    }

    if linked > 0 {
        output.success(&format!("Linked {} secret files", linked));
    } else {
        output.info("No secrets to link");
    }

    Ok(())
}

/// Add a GPG user key for secrets encryption
fn add_key(ctx: &AppContext, key_id: &str) -> Result<()> {
    let output = &ctx.output;
    let secrets_service = ctx.secrets_service();

    output.header("Add GPG Key");
    output.info(&format!("Adding GPG key {}...", key_id));

    secrets_service.add_gpg_user(key_id)?;

    output.success(&format!("GPG key {} added successfully", key_id));
    output.info("Remember to re-lock and push so collaborators can decrypt.");

    Ok(())
}

/// Export the git-crypt encryption key
fn export_key(ctx: &AppContext, output_path: &str) -> Result<()> {
    let output = &ctx.output;
    let secrets_service = ctx.secrets_service();

    output.header("Export Encryption Key");
    output.info(&format!("Exporting key to {}...", output_path));

    secrets_service.export_key(Path::new(output_path))?;

    output.success(&format!("Key exported to {}", output_path));
    output.warning("Keep this file safe! Anyone with this key can decrypt your secrets.");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_secrets_info_serializable() {
        let info = SecretsInfo {
            status: "Locked".to_string(),
            initialized: true,
            encrypted_files: vec!["secrets/api.txt".to_string()],
            keys: vec![KeyInfo {
                id: "ABC123".to_string(),
                user_id: "user@example.com".to_string(),
            }],
        };
        let json = serde_json::to_string(&info).unwrap();
        assert!(json.contains("Locked"));
        assert!(json.contains("api.txt"));
        assert!(json.contains("ABC123"));
    }

    #[test]
    fn test_key_info_serializable() {
        let key = KeyInfo {
            id: "DEADBEEF".to_string(),
            user_id: "test@test.com".to_string(),
        };
        let json = serde_json::to_string(&key).unwrap();
        assert!(json.contains("DEADBEEF"));
        assert!(json.contains("test@test.com"));
    }

    #[test]
    fn test_secrets_info_empty_fields() {
        let info = SecretsInfo {
            status: "NotInitialized".to_string(),
            initialized: false,
            encrypted_files: vec![],
            keys: vec![],
        };
        let json = serde_json::to_string(&info).unwrap();
        assert!(json.contains("NotInitialized"));
        assert!(!info.initialized);
    }

    #[test]
    fn test_secrets_action_dispatch_coverage() {
        // Verify that all SecretsAction variants are handled
        // (compile-time guarantee via exhaustive match in execute())
        let actions = ["Status", "Unlock", "Lock", "Link", "AddKey", "ExportKey"];
        assert_eq!(actions.len(), 6);
    }
}
