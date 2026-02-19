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
        assert!(matches!(cli.command, Some(Commands::Status)));
    }

    #[test]
    fn test_cli_doctor_command() {
        let cli = Cli::try_parse_from(["iron", "doctor"]).unwrap();
        assert!(matches!(cli.command, Some(Commands::Doctor)));
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
            all,
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
        }) = cli.command
        {
            assert!(export);
            assert!(import.is_none());
            assert!(!script);
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
        }) = cli.command
        {
            assert!(!export);
            assert_eq!(import, Some("/path/to/backup.json".to_string()));
            assert!(!script);
        } else {
            panic!("Expected Recover command");
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
        assert!(matches!(cli.command, Some(Commands::Status)));
    }
}
