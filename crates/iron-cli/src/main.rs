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

fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();

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
        }) => commands::update::execute(&ctx, dry_run, force, no_snapshot),
        Some(Commands::Bundle { action }) => commands::bundle::execute(&ctx, action),
        Some(Commands::Profile { action }) => commands::profile::execute(&ctx, action),
        Some(Commands::Module { action }) => commands::module::execute(&ctx, action),
        Some(Commands::Host { action }) => commands::host::execute(&ctx, action),
        Some(Commands::Sync { action }) => commands::sync::execute(&ctx, action),
        Some(Commands::Secrets { action }) => commands::secrets::execute(&ctx, action),
        Some(Commands::Doctor) => commands::doctor::execute(&ctx),
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
        }) => commands::recover::execute(&ctx, export, import, script),
        Some(Commands::Go) => {
            ctx.output.info("Launching Iron TUI...");
            ctx.output
                .warning("TUI not yet implemented. Use CLI commands for now.");
            ctx.output.info("Run 'iron --help' for available commands.");
            Ok(())
        }
        Some(Commands::Completions { shell }) => {
            let mut cmd = Cli::command();
            generate(shell, &mut cmd, "iron", &mut std::io::stdout());
            Ok(())
        }
        None => {
            // No command = show welcome message
            ctx.output.header("Welcome to Iron");
            ctx.output
                .info("Less is More - Turning your Arch into Iron");
            ctx.output.raw("");
            ctx.output.info("Run 'iron --help' for CLI commands");
            ctx.output
                .info("Run 'iron init' to initialize Iron on this host");
            ctx.output
                .info("Run 'iron go' for TUI dashboard (coming soon)");
            Ok(())
        }
    }
}
