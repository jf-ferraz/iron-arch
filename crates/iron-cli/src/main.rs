//! Iron CLI - Command-line interface for Iron

use clap::{Parser, Subcommand};
use anyhow::Result;

#[derive(Parser)]
#[command(name = "iron")]
#[command(author, version, about = "Less is More - Turning your Arch into Iron")]
struct Cli {
    /// Iron root directory
    #[arg(short, long, default_value = "~/.config/iron")]
    root: String,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize Iron configuration
    Init,

    /// Show system status
    Status,

    /// Safe system update with risk assessment
    Update {
        /// Dry run (preview only)
        #[arg(long)]
        dry_run: bool,

        /// Skip risk assessment
        #[arg(long)]
        force: bool,
    },

    /// Bundle management
    Bundle {
        #[command(subcommand)]
        action: BundleAction,
    },

    /// Profile management
    Profile {
        #[command(subcommand)]
        action: ProfileAction,
    },

    /// Module management
    Module {
        #[command(subcommand)]
        action: ModuleAction,
    },

    /// Host management
    Host {
        #[command(subcommand)]
        action: HostAction,
    },

    /// Git sync operations
    Sync {
        #[command(subcommand)]
        action: SyncAction,
    },

    /// Secrets management
    Secrets {
        #[command(subcommand)]
        action: SecretsAction,
    },

    /// System doctor (health check)
    Doctor,

    /// System cleanup
    Clean,

    /// Recovery workflow
    Recover,

    /// Launch TUI dashboard
    Go,
}

#[derive(Subcommand)]
enum BundleAction {
    /// List available bundles
    List,
    /// Show bundle status
    Status { id: Option<String> },
    /// Install a bundle
    Install { id: String },
    /// Switch active bundle
    Switch { id: String },
    /// Remove a bundle
    Remove { id: String },
}

#[derive(Subcommand)]
enum ProfileAction {
    /// List available profiles
    List,
    /// Show profile details
    Show { id: String },
    /// Select/activate a profile
    Select { id: String },
    /// Create new profile
    Create { name: String },
    /// Edit existing profile
    Edit { id: String },
}

#[derive(Subcommand)]
enum ModuleAction {
    /// List all modules
    List,
    /// Show module details
    Show { id: String },
    /// Enable a module
    Enable { id: String },
    /// Disable a module
    Disable { id: String },
}

#[derive(Subcommand)]
enum HostAction {
    /// List configured hosts
    List,
    /// Show current host
    Current,
    /// Catalog hardware for current host
    Catalog,
    /// Select active host
    Select { id: String },
    /// Create snapshot
    Snapshot,
}

#[derive(Subcommand)]
enum SyncAction {
    /// Push changes to remote
    Push,
    /// Pull changes from remote
    Pull,
    /// Show sync status
    Status,
}

#[derive(Subcommand)]
enum SecretsAction {
    /// Unlock encrypted secrets
    Unlock,
    /// Lock secrets before push
    Lock,
    /// Link secrets to proper locations
    Link,
    /// Show secrets status
    Status,
}

fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Init) => {
            println!("Initializing Iron...");
            // TODO: Implement init
        }
        Some(Commands::Status) => {
            println!("Iron Status");
            println!("============");
            // TODO: Implement status
        }
        Some(Commands::Go) => {
            println!("Launching Iron TUI...");
            // TODO: Launch TUI
        }
        None => {
            // No command = launch TUI
            println!("Welcome to Iron!");
            println!("Run 'iron --help' for CLI commands, or 'iron go' for TUI.");
        }
        _ => {
            println!("Command not yet implemented.");
        }
    }

    Ok(())
}
