//! Iron CLI - Command-line interface for Iron
//!
//! Less is More - Turning your Arch into Iron.

mod cli;
mod commands;
mod context;
mod output;

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
    let ctx = AppContext::new(&cli.root, cli.format, cli.verbose, cli.quiet, cli.no_color)?;

    // Execute command
    match cli.command {
        Some(Commands::Init { id, name, force }) => commands::init::execute(&ctx, id, name, force),
        Some(Commands::Status) => commands::status::execute(&ctx),
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
        Some(Commands::Doctor) => commands::doctor::execute(&ctx),
        Some(Commands::Scan) => commands::scan::execute(&ctx),
        Some(Commands::Clean {
            orphans,
            cache,
            symlinks,
            all,
        }) => commands::clean::execute(&ctx, orphans, cache, symlinks, all),
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
            // No command = show welcome message
            if matches!(cli.format, cli::OutputFormat::Json) {
                // Structured JSON for machine consumption
                let welcome = serde_json::json!({
                    "name": "iron",
                    "description": "Less is More - Turning your Arch into Iron",
                    "version": env!("CARGO_PKG_VERSION"),
                    "hint": "Run 'iron --help' for CLI commands, 'iron go' for TUI"
                });
                println!(
                    "{}",
                    serde_json::to_string_pretty(&welcome).unwrap_or_default()
                );
            } else {
                ctx.output.header("Welcome to Iron");
                ctx.output
                    .info("Less is More - Turning your Arch into Iron");
                ctx.output.raw("");
                ctx.output.info("Run 'iron --help' for CLI commands");
                ctx.output
                    .info("Run 'iron init' to initialize Iron on this host");
                ctx.output.info("Run 'iron go' to launch the TUI dashboard");
            }
            Ok(())
        }
    }
}
