//! Iron CLI - Command-line interface for Iron
//!
//! Less is More - Turning your Arch into Iron.

mod cli;
mod commands;
mod context;
mod output;
pub mod progress;

use anyhow::Result;
use clap::{CommandFactory, Parser};
use clap_complete::generate;
use cli::{Cli, Commands};
use context::AppContext;
use iron_core::logging::{LogConfig, init_logging};

fn main() -> Result<()> {
    // Initialize structured JSON logging (NFR-9, NFR-10)
    let log_config = LogConfig::default();
    if let Err(e) = init_logging(&log_config) {
        eprintln!("Warning: Failed to initialize logging: {}", e);
        // Fall back to basic stderr logging
        tracing_subscriber::fmt::init();
    }

    let cli = Cli::parse();

    // Create application context
    let ctx = AppContext::new(
        &cli.root,
        cli.format,
        cli.verbose,
        cli.quiet,
        cli.no_color,
        cli.explain,
    )?;

    // Execute command
    match cli.command {
        Some(Commands::Init { id, name, force }) => commands::init::execute(&ctx, id, name, force),
        Some(Commands::Status { full, dry_run }) => commands::status::execute(&ctx, full, dry_run),
        Some(Commands::Update {
            dry_run,
            force,
            no_snapshot,
            resume,
            status,
            clear_progress,
            yes,
        }) => commands::update::execute(
            &ctx,
            dry_run,
            force,
            no_snapshot,
            resume,
            status,
            clear_progress,
            yes,
        ),
        Some(Commands::Bundle { action }) => commands::bundle::execute(&ctx, action),
        Some(Commands::Profile { action }) => commands::profile::execute(&ctx, action),
        Some(Commands::Module { action }) => commands::module::execute(&ctx, action),
        Some(Commands::Host { action }) => commands::host::execute(&ctx, action),
        Some(Commands::Sync { action }) => commands::sync::execute(&ctx, action),
        Some(Commands::Secrets { action }) => commands::secrets::execute(&ctx, action),
        Some(Commands::Apply {
            dry_run,
            module,
            yes,
            prune,
            prune_packages,
            prune_services,
            prune_dotfiles,
            force_hooks,
        }) => commands::apply::execute(
            &ctx,
            dry_run,
            module.as_deref(),
            yes,
            prune,
            prune_packages,
            prune_services,
            prune_dotfiles,
            force_hooks,
        ),
        Some(Commands::Diff {
            adopt,
            correct,
            dry_run,
            yes,
        }) => commands::diff::execute(&ctx, adopt, correct, dry_run, yes),
        Some(Commands::Snapshot { action }) => commands::snapshot::execute(&ctx, action),
        Some(Commands::Rollback {
            list,
            module,
            dry_run,
            yes,
        }) => commands::snapshot::execute_rollback(&ctx, list, module.as_deref(), dry_run, yes),
        Some(Commands::Plan {
            module,
            dry_run,
            prune,
            prune_packages,
            prune_services,
            prune_dotfiles,
        }) => commands::plan::execute(
            &ctx,
            module.as_deref(),
            dry_run,
            prune,
            prune_packages,
            prune_services,
            prune_dotfiles,
        ),
        Some(Commands::History { ref action, limit }) => {
            commands::history::execute(&ctx, action, limit)
        }
        Some(Commands::Security) => commands::security::execute(&ctx),
        Some(Commands::Validate) => commands::validate::execute(&ctx),
        Some(Commands::Doctor) => commands::doctor::execute(&ctx),
        Some(Commands::Scan) => commands::scan::execute(&ctx),
        Some(Commands::Clean {
            orphans,
            cache,
            symlinks,
            journal,
            logs,
            all,
            dry_run,
        }) => commands::clean::execute(
            &ctx,
            commands::clean::CleanOptions {
                orphans,
                cache,
                symlinks,
                journal,
                logs,
                all,
                dry_run,
            },
        ),
        Some(Commands::Recover {
            export,
            import,
            script,
            backup,
            restore,
        }) => commands::recover::execute(&ctx, export, import, script, backup, restore),
        Some(Commands::Go) => {
            ctx.output.info("Launching Iron TUI...");
            let root = std::path::PathBuf::from(&cli.root);
            let package_manager =
                std::sync::Arc::new(iron_pacman::DefaultPackageManager::default());
            let service_manager = std::sync::Arc::new(iron_systemd::SystemdServiceAdapter::user());
            iron_tui::run_with_config(root, package_manager, service_manager)
        }
        Some(Commands::Completions { shell }) => {
            let mut cmd = Cli::command();
            generate(shell, &mut cmd, "iron", &mut std::io::stdout());
            Ok(())
        }
        None => {
            // No command = launch TUI by default (F0-001)
            // JSON mode still outputs structured data for machine consumers
            if matches!(cli.format, cli::OutputFormat::Json) {
                let welcome = serde_json::json!({
                    "name": "iron",
                    "description": "Less is More - Turning your Arch into Iron",
                    "version": env!("CARGO_PKG_VERSION"),
                    "hint": "Run 'iron --help' for CLI commands"
                });
                println!(
                    "{}",
                    serde_json::to_string_pretty(&welcome).unwrap_or_default()
                );
                Ok(())
            } else {
                // Default: launch TUI dashboard
                let root = std::path::PathBuf::from(&cli.root);
                let package_manager =
                    std::sync::Arc::new(iron_pacman::DefaultPackageManager::default());
                let service_manager =
                    std::sync::Arc::new(iron_systemd::SystemdServiceAdapter::user());
                iron_tui::run_with_config(root, package_manager, service_manager)
            }
        }
    }
}
