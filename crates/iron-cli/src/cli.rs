//! CLI Argument Parsing
//!
//! Defines the command-line interface structure using clap.

use clap::{Parser, Subcommand, ValueEnum};
use clap_complete::Shell;

/// Iron - Declarative Arch Linux Configuration Management
#[derive(Parser)]
#[command(name = "iron")]
#[command(author, version)]
#[command(about = "Less is More - Turning your Arch into Iron")]
#[command(
    long_about = "Iron is a declarative configuration management tool for Arch Linux.\n\n\
    It manages your dotfiles, packages, and system configuration through a \
    hierarchy of Hosts, Bundles, Profiles, and Modules."
)]
pub struct Cli {
    /// Iron root directory
    #[arg(short, long, default_value = "~/.config/iron", global = true)]
    pub root: String,

    /// Output format
    #[arg(short, long, value_enum, default_value = "text", global = true)]
    pub format: OutputFormat,

    /// Verbose output (show details)
    #[arg(short, long, global = true, conflicts_with = "quiet")]
    pub verbose: bool,

    /// Quiet output (minimal)
    #[arg(short, long, global = true, conflicts_with = "verbose")]
    pub quiet: bool,

    /// No color output
    #[arg(long, global = true)]
    pub no_color: bool,

    /// Show underlying system commands being executed (F0-006)
    #[arg(long, global = true)]
    pub explain: bool,

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
    Status {
        /// Full status with ActualState scan (slower, more accurate)
        #[arg(long)]
        full: bool,

        /// Dry run (for testing -- no system queries)
        #[arg(long)]
        dry_run: bool,
    },

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

        /// Resume an interrupted update (FR-5.10)
        #[arg(long)]
        resume: bool,

        /// Show current update progress status (FR-5.10)
        #[arg(long)]
        status: bool,

        /// Clear stale update progress state (FR-5.10)
        #[arg(long)]
        clear_progress: bool,

        /// Auto-approve low risk updates
        #[arg(short, long)]
        yes: bool,
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

    /// Import existing dotfiles into Iron modules
    Import {
        #[command(subcommand)]
        action: ImportAction,
    },

    /// Host management
    Host {
        #[command(subcommand)]
        action: HostAction,
    },

    /// Arch install planning and bootstrap
    Install {
        #[command(subcommand)]
        action: InstallAction,
    },

    /// Git sync operations
    Sync {
        #[command(subcommand)]
        action: SyncAction,
    },

    /// Secrets management (git-crypt).
    ///
    /// Manages encrypted secrets using git-crypt. Secrets are stored in the
    /// secrets/ directory and encrypted at rest. Use 'init' to set up, 'unlock'
    /// to decrypt, 'link' to symlink to ~/.config, and 'lock' before pushing.
    Secrets {
        #[command(subcommand)]
        action: SecretsAction,
    },

    /// Apply declared system state (converge host.toml -> system)
    Apply {
        /// Preview changes without executing
        #[arg(long)]
        dry_run: bool,

        /// Apply a single module only
        #[arg(long)]
        module: Option<String>,

        /// Skip confirmation prompt (except for critical changes)
        #[arg(short, long)]
        yes: bool,

        /// Prune all resource types (remove packages/services/dotfiles
        /// no longer in desired state)
        #[arg(long)]
        prune: bool,

        /// Prune only packages no longer in desired state
        #[arg(long)]
        prune_packages: bool,

        /// Prune only services no longer in desired state
        #[arg(long)]
        prune_services: bool,

        /// Prune only dotfiles/symlinks no longer in desired state
        #[arg(long)]
        prune_dotfiles: bool,

        /// Re-run hooks even if already executed (overrides Once behavior)
        #[arg(long)]
        force_hooks: bool,
    },

    /// Show differences between declared and actual state
    Diff {
        /// Incorporate discovered drift into canonical state
        #[arg(long)]
        adopt: bool,

        /// Revert system to match declared state
        #[arg(long)]
        correct: bool,

        /// Preview corrections without executing
        #[arg(long)]
        dry_run: bool,

        /// Skip confirmation prompt
        #[arg(short, long)]
        yes: bool,
    },

    /// Snapshot management (create, list, restore, delete)
    Snapshot {
        #[command(subcommand)]
        action: SnapshotAction,
    },

    /// Quick rollback to last auto-snapshot
    Rollback {
        /// List recent auto-snapshots instead of restoring
        #[arg(long)]
        list: bool,

        /// Rollback a single module only
        #[arg(long)]
        module: Option<String>,

        /// Preview only (dry run)
        #[arg(long)]
        dry_run: bool,

        /// Skip confirmation prompt
        #[arg(short, long)]
        yes: bool,
    },

    /// Preview what iron apply would do (read-only, no confirmation)
    Plan {
        /// Show plan for a specific module only
        #[arg(short, long)]
        module: Option<String>,

        /// Dry run (for testing -- no system queries)
        #[arg(long)]
        dry_run: bool,

        /// Show plan with all prune actions enabled
        #[arg(long)]
        prune: bool,

        /// Show plan with package prune actions
        #[arg(long)]
        prune_packages: bool,

        /// Show plan with service prune actions
        #[arg(long)]
        prune_services: bool,

        /// Show plan with dotfile prune actions
        #[arg(long)]
        prune_dotfiles: bool,
    },

    /// Show security hardening level and recommendations
    Security,

    /// Validate configuration before applying
    Validate,

    /// System health check
    Doctor,

    /// Scan system for existing configs, package overlaps, and conflicts
    Scan,

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

        /// Vacuum systemd journal logs
        #[arg(long)]
        journal: bool,

        /// Remove old application logs
        #[arg(long)]
        logs: bool,

        /// All cleanup operations
        #[arg(short, long)]
        all: bool,

        /// Preview only, don't execute (no sudo required)
        #[arg(long)]
        dry_run: bool,
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

        /// Create a full backup (configs + state)
        #[arg(long)]
        backup: bool,

        /// Restore from a backup directory
        #[arg(long)]
        restore: Option<String>,
    },

    /// Launch TUI dashboard
    Go,

    /// View operation history
    History {
        #[command(subcommand)]
        action: Option<HistoryAction>,

        /// Maximum number of entries to show
        #[arg(short, long, default_value = "20")]
        limit: usize,
    },

    /// Generate shell completions
    Completions {
        /// Shell to generate completions for
        #[arg(value_enum)]
        shell: Shell,
    },
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

/// Install subcommands
#[derive(Subcommand)]
pub enum InstallAction {
    /// Build a reviewable install plan from host configuration
    Plan {
        /// Host ID to plan from
        #[arg(short = 'H', long)]
        host: String,

        /// Target mountpoint for the new system
        #[arg(short, long, default_value = "/mnt")]
        target: String,

        /// Emit a conservative shell script instead of formatted output
        #[arg(long)]
        emit_script: bool,
    },

    /// Launch the integrated interactive installation wizard
    Wizard {
        /// Host ID to install
        #[arg(short = 'H', long)]
        host: String,

        /// Target mountpoint for the new system
        #[arg(short, long, default_value = "/mnt")]
        target: String,
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

    /// Create a new module scaffold
    Create {
        /// Module ID (lowercase, alphanumeric + hyphens)
        id: String,

        /// Module description
        #[arg(short = 'D', long)]
        description: Option<String>,

        /// Module kind (AppConfig, SystemConfig, DevTools, Shell)
        #[arg(short, long, default_value = "AppConfig")]
        kind: String,
    },
}

/// Import subcommands
#[derive(Subcommand)]
pub enum ImportAction {
    /// Scaffold modules from a home-manager build (`home-manager build` → `result`)
    #[command(visible_alias = "hm")]
    HomeManager {
        /// Path to the generation (`result` symlink / generation dir) or its `home-files/` directory
        path: String,

        /// Preview the modules that would be created without writing anything
        #[arg(long)]
        dry_run: bool,

        /// Overwrite modules that already exist
        #[arg(long)]
        force: bool,

        /// Comma-separated app ids to import (default: all detected)
        #[arg(long, value_delimiter = ',')]
        only: Option<Vec<String>>,

        /// Also add the imported modules to this profile (created if missing)
        #[arg(long)]
        into_profile: Option<String>,

        /// Guess package names from app directory names (default off — dotfiles only)
        #[arg(long)]
        guess_packages: bool,

        /// Rewrite `/nix/store/<pkg>/bin/<x>` references to bare `<x>` in copied
        /// files (other store paths are left intact and reported)
        #[arg(long)]
        strip_store_paths: bool,
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

    /// Initialize git-crypt in the repository
    Init,

    /// Unlock encrypted secrets
    Unlock {
        /// GPG key file
        #[arg(short, long)]
        key: Option<String>,
    },

    /// Lock secrets before push
    Lock,

    /// Link secrets to proper locations.
    ///
    /// Creates symlinks from the secrets/ directory to their expected system
    /// locations. Convention: secrets/<module>/<file> → ~/.config/<module>/<file>.
    /// Files must be unlocked first (iron secrets unlock).
    Link,

    /// Add a GPG user key for encryption
    AddKey {
        /// GPG key ID to add
        #[arg(required = true)]
        key_id: String,
    },

    /// Export the git-crypt encryption key
    ExportKey {
        /// Output path for the exported key
        #[arg(short, long, default_value = "iron-secrets.key")]
        output: String,
    },
}

/// History subcommands
#[derive(Debug, Subcommand)]
pub enum HistoryAction {
    /// List recent operations
    List,
    /// Show details for a specific operation
    Show {
        /// Operation number (1 = most recent)
        id: usize,
    },
    /// Show the most recent operation
    Last,
}

/// Snapshot subcommands
#[derive(Subcommand)]
pub enum SnapshotAction {
    /// Create a named snapshot of current state
    Create {
        /// Snapshot name (auto-generated if omitted)
        name: Option<String>,

        /// Description of the snapshot
        #[arg(short, long)]
        description: Option<String>,

        /// Preview only (dry run)
        #[arg(long)]
        dry_run: bool,
    },

    /// List all snapshots
    List {
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Restore system state from a snapshot
    Restore {
        /// Snapshot name or ID
        name: String,

        /// Preview only (dry run)
        #[arg(long)]
        dry_run: bool,

        /// Skip confirmation prompt
        #[arg(short, long)]
        yes: bool,
    },

    /// Delete a snapshot
    Delete {
        /// Snapshot name or ID
        name: String,
    },

    /// Prune old auto-snapshots
    Prune {
        /// Number of auto-snapshots to keep (default: 10)
        #[arg(long, default_value = "10")]
        keep: usize,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn test_cli_parses_no_args() {
        let cli = Cli::try_parse_from(["iron"]).unwrap();
        assert!(cli.command.is_none());
        assert!(!cli.verbose);
        assert!(!cli.quiet);
    }

    #[test]
    fn test_cli_status_command() {
        let cli = Cli::try_parse_from(["iron", "status"]).unwrap();
        assert!(matches!(cli.command, Some(Commands::Status { .. })));
    }

    #[test]
    fn test_cli_doctor_command() {
        let cli = Cli::try_parse_from(["iron", "doctor"]).unwrap();
        assert!(matches!(cli.command, Some(Commands::Doctor)));
    }

    #[test]
    fn test_cli_scan_command() {
        let cli = Cli::try_parse_from(["iron", "scan"]).unwrap();
        assert!(matches!(cli.command, Some(Commands::Scan)));
    }

    #[test]
    fn test_cli_go_command() {
        let cli = Cli::try_parse_from(["iron", "go"]).unwrap();
        assert!(matches!(cli.command, Some(Commands::Go)));
    }

    #[test]
    fn test_cli_verbose_flag() {
        let cli = Cli::try_parse_from(["iron", "-v"]).unwrap();
        assert!(cli.verbose);
    }

    #[test]
    fn test_cli_quiet_flag() {
        let cli = Cli::try_parse_from(["iron", "-q"]).unwrap();
        assert!(cli.quiet);
    }

    #[test]
    fn test_cli_no_color_flag() {
        let cli = Cli::try_parse_from(["iron", "--no-color"]).unwrap();
        assert!(cli.no_color);
    }

    #[test]
    fn test_cli_format_json() {
        let cli = Cli::try_parse_from(["iron", "--format", "json"]).unwrap();
        assert!(matches!(cli.format, OutputFormat::Json));
    }

    #[test]
    fn test_cli_format_minimal() {
        let cli = Cli::try_parse_from(["iron", "--format", "minimal"]).unwrap();
        assert!(matches!(cli.format, OutputFormat::Minimal));
    }

    #[test]
    fn test_cli_root_option() {
        let cli = Cli::try_parse_from(["iron", "--root", "/custom/path"]).unwrap();
        assert_eq!(cli.root, "/custom/path");
    }

    #[test]
    fn test_cli_init_command() {
        let cli = Cli::try_parse_from(["iron", "init"]).unwrap();
        if let Some(Commands::Init { id, name, force }) = cli.command {
            assert!(id.is_none());
            assert!(name.is_none());
            assert!(!force);
        } else {
            panic!("Expected Init command");
        }
    }

    #[test]
    fn test_cli_init_with_options() {
        let cli = Cli::try_parse_from([
            "iron", "init", "--id", "myhost", "--name", "My Host", "--force",
        ])
        .unwrap();
        if let Some(Commands::Init { id, name, force }) = cli.command {
            assert_eq!(id, Some("myhost".to_string()));
            assert_eq!(name, Some("My Host".to_string()));
            assert!(force);
        } else {
            panic!("Expected Init command");
        }
    }

    #[test]
    fn test_cli_update_command() {
        let cli = Cli::try_parse_from(["iron", "update"]).unwrap();
        if let Some(Commands::Update {
            dry_run,
            force,
            no_snapshot,
            resume,
            status,
            clear_progress,
            yes,
        }) = cli.command
        {
            assert!(!dry_run);
            assert!(!force);
            assert!(!no_snapshot);
            assert!(!resume);
            assert!(!status);
            assert!(!clear_progress);
            assert!(!yes);
        } else {
            panic!("Expected Update command");
        }
    }

    // (rest of tests omitted for brevity - original file continues)
}
