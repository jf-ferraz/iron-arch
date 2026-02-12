//! CLI Argument Parsing
//!
//! Defines the command-line interface structure using clap.

use clap::{Parser, Subcommand, ValueEnum};

/// Iron - Declarative Arch Linux Configuration Management
#[derive(Parser)]
#[command(name = "iron")]
#[command(author, version)]
#[command(about = "Less is More - Turning your Arch into Iron")]
#[command(long_about = "Iron is a declarative configuration management tool for Arch Linux.\n\n\
    It manages your dotfiles, packages, and system configuration through a \
    hierarchy of Hosts, Bundles, Profiles, and Modules.")]
pub struct Cli {
    /// Iron root directory
    #[arg(short, long, default_value = "~/.config/iron", global = true)]
    pub root: String,

    /// Output format
    #[arg(short, long, value_enum, default_value = "text", global = true)]
    pub format: OutputFormat,

    /// Verbose output (show details)
    #[arg(short, long, global = true)]
    pub verbose: bool,

    /// Quiet output (minimal)
    #[arg(short, long, global = true)]
    pub quiet: bool,

    /// No color output
    #[arg(long, global = true)]
    pub no_color: bool,

    #[command(subcommand)]
    pub command: Option<Commands>,
}

/// Output format options
#[derive(Clone, Copy, Debug, ValueEnum, Default)]
pub enum OutputFormat {
    /// Human-readable text
    #[default]
    Text,
    /// JSON output
    Json,
    /// Minimal output (IDs only)
    Minimal,
}

/// Top-level commands
#[derive(Subcommand)]
pub enum Commands {
    /// Initialize Iron on this host
    Init {
        /// Host identifier (defaults to hostname)
        #[arg(short, long)]
        id: Option<String>,

        /// Host display name
        #[arg(short, long)]
        name: Option<String>,

        /// Force re-initialization
        #[arg(long)]
        force: bool,
    },

    /// Show system status overview
    Status,

    /// Safe system update with risk assessment
    Update {
        /// Preview only (dry run)
        #[arg(long)]
        dry_run: bool,

        /// Skip risk assessment
        #[arg(long)]
        force: bool,

        /// Skip snapshot creation
        #[arg(long)]
        no_snapshot: bool,
    },

    /// Bundle management (desktop environments)
    Bundle {
        #[command(subcommand)]
        action: BundleAction,
    },

    /// Profile management (configuration presets)
    Profile {
        #[command(subcommand)]
        action: ProfileAction,
    },

    /// Module management (config modules)
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

    /// Secrets management (git-crypt)
    Secrets {
        #[command(subcommand)]
        action: SecretsAction,
    },

    /// System health check
    Doctor,

    /// System cleanup
    Clean {
        /// Remove orphaned packages
        #[arg(long)]
        orphans: bool,

        /// Clear package cache
        #[arg(long)]
        cache: bool,

        /// Remove broken symlinks
        #[arg(long)]
        symlinks: bool,

        /// All cleanup operations
        #[arg(short, long)]
        all: bool,
    },

    /// Recovery workflow
    Recover {
        /// Export current state
        #[arg(long)]
        export: bool,

        /// Import from file
        #[arg(long)]
        import: Option<String>,

        /// Generate install script
        #[arg(long)]
        script: bool,
    },

    /// Launch TUI dashboard
    Go,
}

/// Bundle subcommands
#[derive(Subcommand)]
pub enum BundleAction {
    /// List available bundles
    List {
        /// Show all details
        #[arg(short, long)]
        all: bool,
    },

    /// Show bundle details/status
    Status {
        /// Bundle ID (current if omitted)
        id: Option<String>,
    },

    /// Install and activate a bundle
    Install {
        /// Bundle ID to install
        id: String,

        /// Skip confirmation
        #[arg(short, long)]
        yes: bool,
    },

    /// Switch to a different bundle
    Switch {
        /// Target bundle ID
        id: String,

        /// Skip confirmation
        #[arg(short, long)]
        yes: bool,
    },

    /// Remove/deactivate a bundle
    Remove {
        /// Bundle ID to remove
        id: String,

        /// Skip confirmation
        #[arg(short, long)]
        yes: bool,
    },
}

/// Profile subcommands
#[derive(Subcommand)]
pub enum ProfileAction {
    /// List available profiles
    List {
        /// Filter by bundle
        #[arg(short, long)]
        bundle: Option<String>,
    },

    /// Show profile details
    Show {
        /// Profile ID
        id: String,

        /// Show inherited modules
        #[arg(long)]
        effective: bool,
    },

    /// Select/activate a profile
    Select {
        /// Profile ID to activate
        id: String,
    },

    /// Create a new profile
    Create {
        /// Profile ID
        id: String,

        /// Profile display name
        #[arg(short, long)]
        name: Option<String>,

        /// Parent profile to extend
        #[arg(short, long)]
        extends: Option<String>,
    },

    /// Edit profile in $EDITOR
    Edit {
        /// Profile ID
        id: String,
    },
}

/// Module subcommands
#[derive(Subcommand)]
pub enum ModuleAction {
    /// List all modules
    List {
        /// Show enabled only
        #[arg(short, long)]
        enabled: bool,

        /// Show disabled only
        #[arg(short, long)]
        disabled: bool,

        /// Filter by kind
        #[arg(short, long)]
        kind: Option<String>,
    },

    /// Show module details
    Show {
        /// Module ID
        id: String,
    },

    /// Enable a module
    Enable {
        /// Module ID
        id: String,

        /// Skip conflict check
        #[arg(long)]
        force: bool,
    },

    /// Disable a module
    Disable {
        /// Module ID
        id: String,

        /// Skip confirmation
        #[arg(short, long)]
        yes: bool,
    },
}

/// Host subcommands
#[derive(Subcommand)]
pub enum HostAction {
    /// List configured hosts
    List,

    /// Show current host info
    Current,

    /// Catalog current hardware
    Catalog {
        /// Update existing host config
        #[arg(long)]
        update: bool,
    },

    /// Select active host
    Select {
        /// Host ID
        id: String,
    },

    /// Create a system snapshot
    Snapshot {
        /// Snapshot description
        #[arg(short, long)]
        description: Option<String>,
    },
}

/// Sync subcommands
#[derive(Subcommand)]
pub enum SyncAction {
    /// Show sync status
    Status,

    /// Push local changes to remote
    Push {
        /// Commit message
        #[arg(short, long)]
        message: Option<String>,
    },

    /// Pull changes from remote
    Pull {
        /// Stash local changes
        #[arg(long)]
        stash: bool,
    },
}

/// Secrets subcommands
#[derive(Subcommand)]
pub enum SecretsAction {
    /// Show secrets status
    Status,

    /// Unlock encrypted secrets
    Unlock {
        /// GPG key file
        #[arg(short, long)]
        key: Option<String>,
    },

    /// Lock secrets before push
    Lock,

    /// Link secrets to proper locations
    Link,
}
