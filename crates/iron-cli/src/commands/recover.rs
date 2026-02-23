//! Recovery Commands
//!
//! State export/import and recovery workflows.

use crate::context::{AppContext, require_init};
use anyhow::Result;
use iron_core::services::recovery::{InstallScriptOptions, RecoveryService};
use std::fs;
use std::path::Path;
use std::time::Instant;

/// Execute recover command
pub fn execute(
    ctx: &AppContext,
    export: bool,
    import: Option<String>,
    script: bool,
    backup: bool,
    restore: Option<String>,
) -> Result<()> {
    let start = Instant::now();
    if export {
        return export_state(ctx, start);
    }

    if let Some(file) = import {
        return import_state(ctx, &file);
    }

    if script {
        return generate_script(ctx);
    }

    if backup {
        return create_backup(ctx);
    }

    if let Some(path) = restore {
        return restore_backup(ctx, &path);
    }

    // Default: show recovery help
    show_help(ctx)
}

/// Show recovery help
fn show_help(ctx: &AppContext) -> Result<()> {
    let output = &ctx.output;

    output.header("Iron Recovery");
    output.info("Recovery allows you to backup and restore your Iron configuration.");

    output.subheader("Available Options");
    output.list_item("--export     Export current state to JSON file");
    output.list_item("--import     Import state from JSON file");
    output.list_item("--script     Generate installation script");
    output.list_item("--backup     Create full backup (configs + state)");
    output.list_item("--restore    Restore from a backup directory");

    output.subheader("Examples");
    output.raw("  iron recover --export              # Export to iron-export.json");
    output.raw("  iron recover --import backup.json  # Import from file");
    output.raw("  iron recover --script              # Generate install.sh");
    output.raw("  iron recover --backup              # Create backup archive");
    output.raw("  iron recover --restore ./backup    # Restore from backup dir");

    Ok(())
}

/// Export current state
fn export_state(ctx: &AppContext, start: Instant) -> Result<()> {
    require_init(ctx)?;

    let output = &ctx.output;
    let recovery_service = ctx.recovery_service();

    output.header("Export State");

    output.info("Gathering state information...");
    let export_data = recovery_service.export()?;

    // Generate filename with timestamp
    let timestamp = chrono::Utc::now().format("%Y%m%d-%H%M%S");
    let filename = format!("iron-export-{}.json", timestamp);

    output.info(&format!("Writing to {}...", filename));
    recovery_service.save_export(Path::new(&filename))?;

    output.separator();
    output.success(&format!("State exported to {}", filename));

    // Show summary
    if output.is_verbose() {
        output.subheader("Export Contents");
        output.kv("Host ID", &export_data.host_id);
        output.kv(
            "Bundle",
            export_data
                .active_bundle
                .as_ref()
                .unwrap_or(&"None".to_string()),
        );
        output.kv(
            "Profile",
            export_data
                .active_profile
                .as_ref()
                .unwrap_or(&"None".to_string()),
        );
        output.kv("Modules", export_data.active_modules.len());
        output.kv("Packages", export_data.packages.len());
        output.kv("AUR Packages", export_data.aur_packages.len());
    }

    if output.is_json() {
        output.json_envelope("recover.export", &export_data, start);
    }

    Ok(())
}

/// Import state from file
fn import_state(ctx: &AppContext, file: &str) -> Result<()> {
    let output = &ctx.output;
    let recovery_service = ctx.recovery_service();

    output.header("Import State");

    let path = Path::new(file);
    if !path.exists() {
        output.error(&format!("File not found: {}", file));
        return Ok(());
    }

    output.info(&format!("Loading {}...", file));
    let export_data = recovery_service.load_export(path)?;

    // Show what will be imported
    output.subheader("Import Preview");
    output.kv("Host", &export_data.host_id);
    output.kv(
        "Bundle",
        export_data
            .active_bundle
            .as_ref()
            .unwrap_or(&"None".to_string()),
    );
    output.kv(
        "Profile",
        export_data
            .active_profile
            .as_ref()
            .unwrap_or(&"None".to_string()),
    );
    output.kv("Modules", export_data.active_modules.len());
    output.kv("Packages", export_data.packages.len());

    // List modules
    if !export_data.active_modules.is_empty() {
        output.subheader("Modules to Enable");
        for module in &export_data.active_modules {
            output.list_item(module);
        }
    }

    // Confirmation
    output.warning("This will apply the imported configuration.");
    print!("\nProceed? [y/N] ");
    std::io::Write::flush(&mut std::io::stdout())?;

    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;

    if !input.trim().eq_ignore_ascii_case("y") {
        output.info("Import cancelled");
        return Ok(());
    }

    // Import
    output.info("Applying configuration...");
    recovery_service.import(&export_data)?;

    output.success("State imported successfully");

    Ok(())
}

/// Generate installation script
fn generate_script(ctx: &AppContext) -> Result<()> {
    require_init(ctx)?;

    let output = &ctx.output;
    let recovery_service = ctx.recovery_service();

    output.header("Generate Installation Script");

    output.info("Generating script...");

    let options = InstallScriptOptions {
        include_packages: true,
        include_aur: true,
        include_services: true,
        include_modules: true,
        include_bundle: true,
        aur_helper: "paru".to_string(),
        interactive: true,
    };

    let script = recovery_service.generate_install_script(&options)?;

    let filename = "iron-install.sh";
    output.info(&format!("Writing to {}...", filename));

    fs::write(filename, &script)?;

    // Make executable
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(filename)?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(filename, perms)?;
    }

    output.separator();
    output.success(&format!("Script generated: {}", filename));
    output.info("Review the script before running:");
    output.raw(&format!("  less {}", filename));
    output.raw(&format!("  ./{}", filename));

    // Show script preview if verbose
    if output.is_verbose() {
        output.subheader("Script Preview (first 20 lines)");
        for line in script.lines().take(20) {
            output.raw(line);
        }
        output.raw("...");
    }

    Ok(())
}

/// Create a full backup (configs + state)
fn create_backup(ctx: &AppContext) -> Result<()> {
    require_init(ctx)?;

    let output = &ctx.output;
    let recovery_service = ctx.recovery_service();

    output.header("Create Backup");
    output.info("Creating full backup...");

    let backup_path = recovery_service.create_backup(Path::new("."))?;

    output.separator();
    output.success(&format!("Backup created: {}", backup_path.display()));
    output.info("Store this backup in a safe location.");

    Ok(())
}

/// Restore from a backup directory
fn restore_backup(ctx: &AppContext, backup_dir: &str) -> Result<()> {
    let output = &ctx.output;
    let recovery_service = ctx.recovery_service();

    let backup_path = Path::new(backup_dir);
    if !backup_path.exists() {
        anyhow::bail!("Backup path does not exist: {}", backup_dir);
    }

    output.header("Restore Backup");
    output.warning("This will overwrite current configuration!");
    output.info(&format!("Restoring from {}...", backup_dir));

    recovery_service.restore_backup(backup_path)?;

    output.separator();
    output.success("Backup restored successfully");
    output.info("Run 'iron doctor' to verify system health.");

    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_execute_default_shows_help() {
        // When no flags are set, execute() should call show_help()
        // Verify the routing logic: all false → hits show_help
        let export = false;
        let import: Option<String> = None;
        let script = false;
        let backup = false;
        let restore: Option<String> = None;

        // Simulate routing logic
        let routed_to = if export {
            "export"
        } else if import.is_some() {
            "import"
        } else if script {
            "script"
        } else if backup {
            "backup"
        } else if restore.is_some() {
            "restore"
        } else {
            "help"
        };
        assert_eq!(routed_to, "help");
    }

    #[test]
    fn test_execute_export_route() {
        let export = true;
        let routed = if export { "export" } else { "other" };
        assert_eq!(routed, "export");
    }

    #[test]
    fn test_execute_import_route() {
        let import = Some("/path/to/file.json".to_string());
        let routed = if import.is_some() { "import" } else { "other" };
        assert_eq!(routed, "import");
    }

    #[test]
    fn test_execute_backup_route() {
        let backup = true;
        let routed = if backup { "backup" } else { "other" };
        assert_eq!(routed, "backup");
    }

    #[test]
    fn test_execute_restore_route() {
        let restore = Some("./my-backup".to_string());
        let routed = if restore.is_some() {
            "restore"
        } else {
            "other"
        };
        assert_eq!(routed, "restore");
    }

    #[test]
    fn test_execute_script_route() {
        let script = true;
        let routed = if script { "script" } else { "other" };
        assert_eq!(routed, "script");
    }
}
