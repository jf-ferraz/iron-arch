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

    #[test]
    fn test_cli_update_dry_run() {
        let cli = Cli::try_parse_from(["iron", "update", "--dry-run"]).unwrap();
        if let Some(Commands::Update { dry_run, .. }) = cli.command {
            assert!(dry_run);
        } else {
            panic!("Expected Update command");
        }
    }

    #[test]
    fn test_cli_update_resume() {
        let cli = Cli::try_parse_from(["iron", "update", "--resume"]).unwrap();
        if let Some(Commands::Update { resume, .. }) = cli.command {
            assert!(resume);
        } else {
            panic!("Expected Update command");
        }
    }

    #[test]
    fn test_cli_update_status() {
        let cli = Cli::try_parse_from(["iron", "update", "--status"]).unwrap();
        if let Some(Commands::Update { status, .. }) = cli.command {
            assert!(status);
        } else {
            panic!("Expected Update command");
        }
    }

    #[test]
    fn test_cli_update_clear_progress() {
        let cli = Cli::try_parse_from(["iron", "update", "--clear-progress"]).unwrap();
        if let Some(Commands::Update { clear_progress, .. }) = cli.command {
            assert!(clear_progress);
        } else {
            panic!("Expected Update command");
        }
    }

    #[test]
    fn test_cli_update_yes() {
        let cli = Cli::try_parse_from(["iron", "update", "-y"]).unwrap();
        if let Some(Commands::Update { yes, .. }) = cli.command {
            assert!(yes);
        } else {
            panic!("Expected Update command");
        }
    }

    #[test]
    fn test_cli_bundle_list() {
        let cli = Cli::try_parse_from(["iron", "bundle", "list"]).unwrap();
        if let Some(Commands::Bundle {
            action: BundleAction::List { all },
        }) = cli.command
        {
            assert!(!all);
        } else {
            panic!("Expected Bundle List command");
        }
    }

    #[test]
    fn test_cli_bundle_list_all() {
        let cli = Cli::try_parse_from(["iron", "bundle", "list", "--all"]).unwrap();
        if let Some(Commands::Bundle {
            action: BundleAction::List { all },
        }) = cli.command
        {
            assert!(all);
        } else {
            panic!("Expected Bundle List command");
        }
    }

    #[test]
    fn test_cli_bundle_install() {
        let cli = Cli::try_parse_from(["iron", "bundle", "install", "hyprland"]).unwrap();
        if let Some(Commands::Bundle {
            action: BundleAction::Install { id, yes },
        }) = cli.command
        {
            assert_eq!(id, "hyprland");
            assert!(!yes);
        } else {
            panic!("Expected Bundle Install command");
        }
    }

    #[test]
    fn test_cli_bundle_switch() {
        let cli = Cli::try_parse_from(["iron", "bundle", "switch", "niri", "--yes"]).unwrap();
        if let Some(Commands::Bundle {
            action: BundleAction::Switch { id, yes },
        }) = cli.command
        {
            assert_eq!(id, "niri");
            assert!(yes);
        } else {
            panic!("Expected Bundle Switch command");
        }
    }

    #[test]
    fn test_cli_profile_list() {
        let cli = Cli::try_parse_from(["iron", "profile", "list"]).unwrap();
        if let Some(Commands::Profile {
            action: ProfileAction::List { bundle },
        }) = cli.command
        {
            assert!(bundle.is_none());
        } else {
            panic!("Expected Profile List command");
        }
    }

    #[test]
    fn test_cli_profile_select() {
        let cli = Cli::try_parse_from(["iron", "profile", "select", "developer"]).unwrap();
        if let Some(Commands::Profile {
            action: ProfileAction::Select { id },
        }) = cli.command
        {
            assert_eq!(id, "developer");
        } else {
            panic!("Expected Profile Select command");
        }
    }

    #[test]
    fn test_cli_module_list() {
        let cli = Cli::try_parse_from(["iron", "module", "list"]).unwrap();
        if let Some(Commands::Module {
            action:
                ModuleAction::List {
                    enabled,
                    disabled,
                    kind,
                },
        }) = cli.command
        {
            assert!(!enabled);
            assert!(!disabled);
            assert!(kind.is_none());
        } else {
            panic!("Expected Module List command");
        }
    }

    #[test]
    fn test_cli_module_enable() {
        let cli = Cli::try_parse_from(["iron", "module", "enable", "nvim-ide"]).unwrap();
        if let Some(Commands::Module {
            action: ModuleAction::Enable { id, force },
        }) = cli.command
        {
            assert_eq!(id, "nvim-ide");
            assert!(!force);
        } else {
            panic!("Expected Module Enable command");
        }
    }

    #[test]
    fn test_cli_module_disable() {
        let cli = Cli::try_parse_from(["iron", "module", "disable", "nvim-ide", "--yes"]).unwrap();
        if let Some(Commands::Module {
            action: ModuleAction::Disable { id, yes },
        }) = cli.command
        {
            assert_eq!(id, "nvim-ide");
            assert!(yes);
        } else {
            panic!("Expected Module Disable command");
        }
    }

    #[test]
    fn test_cli_host_list() {
        let cli = Cli::try_parse_from(["iron", "host", "list"]).unwrap();
        assert!(matches!(
            cli.command,
            Some(Commands::Host {
                action: HostAction::List
            })
        ));
    }

    #[test]
    fn test_cli_host_current() {
        let cli = Cli::try_parse_from(["iron", "host", "current"]).unwrap();
        assert!(matches!(
            cli.command,
            Some(Commands::Host {
                action: HostAction::Current
            })
        ));
    }

    #[test]
    fn test_cli_sync_status() {
        let cli = Cli::try_parse_from(["iron", "sync", "status"]).unwrap();
        assert!(matches!(
            cli.command,
            Some(Commands::Sync {
                action: SyncAction::Status
            })
        ));
    }

    #[test]
    fn test_cli_sync_push() {
        let cli = Cli::try_parse_from(["iron", "sync", "push", "-m", "Update config"]).unwrap();
        if let Some(Commands::Sync {
            action: SyncAction::Push { message },
        }) = cli.command
        {
            assert_eq!(message, Some("Update config".to_string()));
        } else {
            panic!("Expected Sync Push command");
        }
    }

    #[test]
    fn test_cli_secrets_status() {
        let cli = Cli::try_parse_from(["iron", "secrets", "status"]).unwrap();
        assert!(matches!(
            cli.command,
            Some(Commands::Secrets {
                action: SecretsAction::Status
            })
        ));
    }

    #[test]
    fn test_cli_clean_options() {
        let cli =
            Cli::try_parse_from(["iron", "clean", "--orphans", "--cache", "--symlinks"]).unwrap();
        if let Some(Commands::Clean {
            orphans,
            cache,
            symlinks,
            journal: _,
            logs: _,
            all,
            dry_run: _,
        }) = cli.command
        {
            assert!(orphans);
            assert!(cache);
            assert!(symlinks);
            assert!(!all);
        } else {
            panic!("Expected Clean command");
        }
    }

    #[test]
    fn test_cli_clean_all() {
        let cli = Cli::try_parse_from(["iron", "clean", "--all"]).unwrap();
        if let Some(Commands::Clean { all, .. }) = cli.command {
            assert!(all);
        } else {
            panic!("Expected Clean command");
        }
    }

    #[test]
    fn test_cli_recover_export() {
        let cli = Cli::try_parse_from(["iron", "recover", "--export"]).unwrap();
        if let Some(Commands::Recover {
            export,
            import,
            script,
            backup,
            restore,
        }) = cli.command
        {
            assert!(export);
            assert!(import.is_none());
            assert!(!script);
            assert!(!backup);
            assert!(restore.is_none());
        } else {
            panic!("Expected Recover command");
        }
    }

    #[test]
    fn test_cli_recover_import() {
        let cli =
            Cli::try_parse_from(["iron", "recover", "--import", "/path/to/backup.json"]).unwrap();
        if let Some(Commands::Recover {
            export,
            import,
            script,
            backup,
            restore,
        }) = cli.command
        {
            assert!(!export);
            assert_eq!(import, Some("/path/to/backup.json".to_string()));
            assert!(!script);
            assert!(!backup);
            assert!(restore.is_none());
        } else {
            panic!("Expected Recover command");
        }
    }

    #[test]
    fn test_cli_recover_backup() {
        let cli = Cli::try_parse_from(["iron", "recover", "--backup"]).unwrap();
        if let Some(Commands::Recover { backup, .. }) = cli.command {
            assert!(backup);
        } else {
            panic!("Expected Recover command");
        }
    }

    #[test]
    fn test_cli_recover_restore() {
        let cli = Cli::try_parse_from(["iron", "recover", "--restore", "./my-backup"]).unwrap();
        if let Some(Commands::Recover { restore, .. }) = cli.command {
            assert_eq!(restore, Some("./my-backup".to_string()));
        } else {
            panic!("Expected Recover command");
        }
    }

    #[test]
    fn test_cli_secrets_add_key() {
        let cli = Cli::try_parse_from(["iron", "secrets", "add-key", "ABCD1234"]).unwrap();
        if let Some(Commands::Secrets {
            action: SecretsAction::AddKey { key_id },
        }) = cli.command
        {
            assert_eq!(key_id, "ABCD1234");
        } else {
            panic!("Expected Secrets AddKey command");
        }
    }

    #[test]
    fn test_cli_secrets_export_key() {
        let cli = Cli::try_parse_from(["iron", "secrets", "export-key"]).unwrap();
        if let Some(Commands::Secrets {
            action: SecretsAction::ExportKey { output },
        }) = cli.command
        {
            assert_eq!(output, "iron-secrets.key");
        } else {
            panic!("Expected Secrets ExportKey command");
        }
    }

    #[test]
    fn test_cli_secrets_export_key_custom_path() {
        let cli =
            Cli::try_parse_from(["iron", "secrets", "export-key", "-o", "/tmp/my.key"]).unwrap();
        if let Some(Commands::Secrets {
            action: SecretsAction::ExportKey { output },
        }) = cli.command
        {
            assert_eq!(output, "/tmp/my.key");
        } else {
            panic!("Expected Secrets ExportKey command");
        }
    }

    #[test]
    fn test_cli_help_works() {
        // Verify the CLI can generate help without panicking
        Cli::command().debug_assert();
    }

    #[test]
    fn test_output_format_default() {
        let format = OutputFormat::default();
        assert!(matches!(format, OutputFormat::Text));
    }

    #[test]
    fn test_global_flags_with_subcommand() {
        let cli = Cli::try_parse_from(["iron", "-v", "--format", "json", "status"]).unwrap();
        assert!(cli.verbose);
        assert!(matches!(cli.format, OutputFormat::Json));
        assert!(matches!(cli.command, Some(Commands::Status { .. })));
    }

    // ── F1-007: Apply command tests ──────────────────────────────

    #[test]
    fn test_cli_apply_basic() {
        let cli = Cli::try_parse_from(["iron", "apply"]).unwrap();
        if let Some(Commands::Apply {
            dry_run,
            module,
            yes,
            prune,
            prune_packages,
            prune_services,
            prune_dotfiles,
            force_hooks,
        }) = cli.command
        {
            assert!(!dry_run);
            assert!(module.is_none());
            assert!(!yes);
            assert!(!prune);
            assert!(!prune_packages);
            assert!(!prune_services);
            assert!(!prune_dotfiles);
            assert!(!force_hooks);
        } else {
            panic!("Expected Apply command");
        }
    }

    #[test]
    fn test_cli_apply_dry_run() {
        let cli = Cli::try_parse_from(["iron", "apply", "--dry-run"]).unwrap();
        if let Some(Commands::Apply { dry_run, .. }) = cli.command {
            assert!(dry_run);
        } else {
            panic!("Expected Apply command");
        }
    }

    #[test]
    fn test_cli_apply_module() {
        let cli = Cli::try_parse_from(["iron", "apply", "--module", "nvim-ide"]).unwrap();
        if let Some(Commands::Apply { module, .. }) = cli.command {
            assert_eq!(module, Some("nvim-ide".to_string()));
        } else {
            panic!("Expected Apply command");
        }
    }

    #[test]
    fn test_cli_apply_yes() {
        let cli = Cli::try_parse_from(["iron", "apply", "-y"]).unwrap();
        if let Some(Commands::Apply { yes, .. }) = cli.command {
            assert!(yes);
        } else {
            panic!("Expected Apply command");
        }
    }

    #[test]
    fn test_cli_apply_prune() {
        let cli = Cli::try_parse_from(["iron", "apply", "--prune"]).unwrap();
        if let Some(Commands::Apply { prune, .. }) = cli.command {
            assert!(prune);
        } else {
            panic!("Expected Apply command");
        }
    }

    #[test]
    fn test_cli_apply_prune_packages() {
        let cli = Cli::try_parse_from(["iron", "apply", "--prune-packages"]).unwrap();
        if let Some(Commands::Apply { prune_packages, .. }) = cli.command {
            assert!(prune_packages);
        } else {
            panic!("Expected Apply command");
        }
    }

    #[test]
    fn test_cli_apply_prune_services() {
        let cli = Cli::try_parse_from(["iron", "apply", "--prune-services"]).unwrap();
        if let Some(Commands::Apply { prune_services, .. }) = cli.command {
            assert!(prune_services);
        } else {
            panic!("Expected Apply command");
        }
    }

    #[test]
    fn test_cli_apply_prune_dotfiles() {
        let cli = Cli::try_parse_from(["iron", "apply", "--prune-dotfiles"]).unwrap();
        if let Some(Commands::Apply { prune_dotfiles, .. }) = cli.command {
            assert!(prune_dotfiles);
        } else {
            panic!("Expected Apply command");
        }
    }

    // ── F1-015: Diff command tests ──────────────────────────────

    #[test]
    fn test_cli_diff_basic() {
        let cli = Cli::try_parse_from(["iron", "diff"]).unwrap();
        if let Some(Commands::Diff {
            adopt,
            correct,
            dry_run,
            yes,
        }) = cli.command
        {
            assert!(!adopt);
            assert!(!correct);
            assert!(!dry_run);
            assert!(!yes);
        } else {
            panic!("Expected Diff command");
        }
    }

    #[test]
    fn test_cli_diff_adopt() {
        let cli = Cli::try_parse_from(["iron", "diff", "--adopt"]).unwrap();
        if let Some(Commands::Diff { adopt, .. }) = cli.command {
            assert!(adopt);
        } else {
            panic!("Expected Diff command");
        }
    }

    #[test]
    fn test_cli_diff_correct_dry_run() {
        let cli = Cli::try_parse_from(["iron", "diff", "--correct", "--dry-run"]).unwrap();
        if let Some(Commands::Diff {
            correct, dry_run, ..
        }) = cli.command
        {
            assert!(correct);
            assert!(dry_run);
        } else {
            panic!("Expected Diff command");
        }
    }

    // ── F2-002/003/004/005: Snapshot command tests ──────────────

    #[test]
    fn test_cli_snapshot_create_named() {
        let cli = Cli::try_parse_from(["iron", "snapshot", "create", "pre-kde"]).unwrap();
        if let Some(Commands::Snapshot {
            action:
                SnapshotAction::Create {
                    name,
                    description,
                    dry_run,
                },
        }) = cli.command
        {
            assert_eq!(name, Some("pre-kde".to_string()));
            assert!(description.is_none());
            assert!(!dry_run);
        } else {
            panic!("Expected Snapshot Create command");
        }
    }

    #[test]
    fn test_cli_snapshot_create_no_name() {
        let cli = Cli::try_parse_from(["iron", "snapshot", "create"]).unwrap();
        if let Some(Commands::Snapshot {
            action: SnapshotAction::Create { name, .. },
        }) = cli.command
        {
            assert!(name.is_none());
        } else {
            panic!("Expected Snapshot Create command");
        }
    }

    #[test]
    fn test_cli_snapshot_create_dry_run() {
        let cli = Cli::try_parse_from(["iron", "snapshot", "create", "test", "--dry-run"]).unwrap();
        if let Some(Commands::Snapshot {
            action: SnapshotAction::Create { dry_run, .. },
        }) = cli.command
        {
            assert!(dry_run);
        } else {
            panic!("Expected Snapshot Create command");
        }
    }

    #[test]
    fn test_cli_snapshot_create_with_description() {
        let cli = Cli::try_parse_from([
            "iron",
            "snapshot",
            "create",
            "test",
            "-d",
            "Before KDE switch",
        ])
        .unwrap();
        if let Some(Commands::Snapshot {
            action: SnapshotAction::Create {
                name, description, ..
            },
        }) = cli.command
        {
            assert_eq!(name, Some("test".to_string()));
            assert_eq!(description, Some("Before KDE switch".to_string()));
        } else {
            panic!("Expected Snapshot Create command");
        }
    }

    #[test]
    fn test_cli_snapshot_list() {
        let cli = Cli::try_parse_from(["iron", "snapshot", "list"]).unwrap();
        assert!(matches!(
            cli.command,
            Some(Commands::Snapshot {
                action: SnapshotAction::List { json: false }
            })
        ));
    }

    #[test]
    fn test_cli_snapshot_list_json() {
        let cli = Cli::try_parse_from(["iron", "snapshot", "list", "--json"]).unwrap();
        if let Some(Commands::Snapshot {
            action: SnapshotAction::List { json },
        }) = cli.command
        {
            assert!(json);
        } else {
            panic!("Expected Snapshot List command");
        }
    }

    #[test]
    fn test_cli_snapshot_restore() {
        let cli = Cli::try_parse_from(["iron", "snapshot", "restore", "pre-kde"]).unwrap();
        if let Some(Commands::Snapshot {
            action: SnapshotAction::Restore { name, dry_run, yes },
        }) = cli.command
        {
            assert_eq!(name, "pre-kde");
            assert!(!dry_run);
            assert!(!yes);
        } else {
            panic!("Expected Snapshot Restore command");
        }
    }

    #[test]
    fn test_cli_snapshot_restore_dry_run_yes() {
        let cli = Cli::try_parse_from([
            "iron",
            "snapshot",
            "restore",
            "pre-kde",
            "--dry-run",
            "--yes",
        ])
        .unwrap();
        if let Some(Commands::Snapshot {
            action: SnapshotAction::Restore { dry_run, yes, .. },
        }) = cli.command
        {
            assert!(dry_run);
            assert!(yes);
        } else {
            panic!("Expected Snapshot Restore command");
        }
    }

    #[test]
    fn test_cli_snapshot_delete() {
        let cli = Cli::try_parse_from(["iron", "snapshot", "delete", "old-snap"]).unwrap();
        if let Some(Commands::Snapshot {
            action: SnapshotAction::Delete { name },
        }) = cli.command
        {
            assert_eq!(name, "old-snap");
        } else {
            panic!("Expected Snapshot Delete command");
        }
    }

    #[test]
    fn test_cli_snapshot_prune() {
        let cli = Cli::try_parse_from(["iron", "snapshot", "prune", "--keep", "5"]).unwrap();
        if let Some(Commands::Snapshot {
            action: SnapshotAction::Prune { keep },
        }) = cli.command
        {
            assert_eq!(keep, 5);
        } else {
            panic!("Expected Snapshot Prune command");
        }
    }

    #[test]
    fn test_cli_snapshot_prune_default() {
        let cli = Cli::try_parse_from(["iron", "snapshot", "prune"]).unwrap();
        if let Some(Commands::Snapshot {
            action: SnapshotAction::Prune { keep },
        }) = cli.command
        {
            assert_eq!(keep, 10);
        } else {
            panic!("Expected Snapshot Prune command");
        }
    }

    // ── F2-005: Rollback command tests ──────────────────────────

    #[test]
    fn test_cli_rollback_basic() {
        let cli = Cli::try_parse_from(["iron", "rollback"]).unwrap();
        if let Some(Commands::Rollback {
            list,
            module,
            dry_run,
            yes,
        }) = cli.command
        {
            assert!(!list);
            assert!(module.is_none());
            assert!(!dry_run);
            assert!(!yes);
        } else {
            panic!("Expected Rollback command");
        }
    }

    #[test]
    fn test_cli_rollback_list() {
        let cli = Cli::try_parse_from(["iron", "rollback", "--list"]).unwrap();
        if let Some(Commands::Rollback { list, .. }) = cli.command {
            assert!(list);
        } else {
            panic!("Expected Rollback command");
        }
    }

    #[test]
    fn test_cli_rollback_module() {
        let cli =
            Cli::try_parse_from(["iron", "rollback", "--module", "nvim-ide", "--dry-run"]).unwrap();
        if let Some(Commands::Rollback {
            module, dry_run, ..
        }) = cli.command
        {
            assert_eq!(module, Some("nvim-ide".to_string()));
            assert!(dry_run);
        } else {
            panic!("Expected Rollback command");
        }
    }

    // ── F3-004: Status command flag tests ────────────────────────

    #[test]
    fn test_cli_status_full_flag() {
        let cli = Cli::try_parse_from(["iron", "status", "--full"]).unwrap();
        if let Some(Commands::Status { full, dry_run }) = cli.command {
            assert!(full);
            assert!(!dry_run);
        } else {
            panic!("Expected Status command");
        }
    }

    #[test]
    fn test_cli_status_dry_run_flag() {
        let cli = Cli::try_parse_from(["iron", "status", "--dry-run"]).unwrap();
        if let Some(Commands::Status { full, dry_run }) = cli.command {
            assert!(!full);
            assert!(dry_run);
        } else {
            panic!("Expected Status command");
        }
    }

    // ── F3-005: Plan command tests ──────────────────────────────

    #[test]
    fn test_cli_plan_basic() {
        let cli = Cli::try_parse_from(["iron", "plan"]).unwrap();
        if let Some(Commands::Plan {
            module,
            dry_run,
            prune,
            ..
        }) = cli.command
        {
            assert!(module.is_none());
            assert!(!dry_run);
            assert!(!prune);
        } else {
            panic!("Expected Plan command");
        }
    }

    #[test]
    fn test_cli_plan_module() {
        let cli = Cli::try_parse_from(["iron", "plan", "--module", "nvim-ide"]).unwrap();
        if let Some(Commands::Plan {
            module, dry_run, ..
        }) = cli.command
        {
            assert_eq!(module, Some("nvim-ide".to_string()));
            assert!(!dry_run);
        } else {
            panic!("Expected Plan command");
        }
    }

    #[test]
    fn test_cli_plan_dry_run() {
        let cli = Cli::try_parse_from(["iron", "plan", "--dry-run"]).unwrap();
        if let Some(Commands::Plan {
            module, dry_run, ..
        }) = cli.command
        {
            assert!(module.is_none());
            assert!(dry_run);
        } else {
            panic!("Expected Plan command");
        }
    }

    #[test]
    fn test_cli_plan_prune() {
        let cli = Cli::try_parse_from(["iron", "plan", "--prune"]).unwrap();
        if let Some(Commands::Plan { prune, .. }) = cli.command {
            assert!(prune);
        } else {
            panic!("Expected Plan command");
        }
    }

    // -- F3-016: History command tests --

    #[test]
    fn test_cli_history_basic() {
        let cli = Cli::try_parse_from(["iron", "history"]).unwrap();
        if let Some(Commands::History { action, limit }) = cli.command {
            assert!(action.is_none());
            assert_eq!(limit, 20);
        } else {
            panic!("Expected History command");
        }
    }

    #[test]
    fn test_cli_history_list() {
        let cli = Cli::try_parse_from(["iron", "history", "list"]).unwrap();
        if let Some(Commands::History { action, .. }) = cli.command {
            assert!(matches!(action, Some(HistoryAction::List)));
        } else {
            panic!("Expected History command");
        }
    }

    #[test]
    fn test_cli_history_show() {
        let cli = Cli::try_parse_from(["iron", "history", "show", "3"]).unwrap();
        if let Some(Commands::History {
            action: Some(HistoryAction::Show { id }),
            ..
        }) = cli.command
        {
            assert_eq!(id, 3);
        } else {
            panic!("Expected History Show command");
        }
    }

    #[test]
    fn test_cli_history_last() {
        let cli = Cli::try_parse_from(["iron", "history", "last"]).unwrap();
        if let Some(Commands::History { action, .. }) = cli.command {
            assert!(matches!(action, Some(HistoryAction::Last)));
        } else {
            panic!("Expected History command");
        }
    }

    #[test]
    fn test_cli_history_limit() {
        let cli = Cli::try_parse_from(["iron", "history", "--limit", "5"]).unwrap();
        if let Some(Commands::History { limit, .. }) = cli.command {
            assert_eq!(limit, 5);
        } else {
            panic!("Expected History command");
        }
    }
}
