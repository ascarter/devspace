use clap::{Parser, Subcommand};

/// Developer Workspace - Personal development workspace manager
///
/// dws manages your dotfiles and development tools through declarative
/// manifests (`dws.toml` per profile with workspace overrides in `config.toml`).
/// Bootstrap new machines, sync configurations, and maintain your
/// workspace with a single portable binary.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    /// Enable verbose output
    #[arg(short, long, global = true)]
    pub verbose: bool,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Initialize workspace and shell integration
    ///
    /// Creates workspace from template or clones from repository.
    /// If workspace exists, verifies it and updates shell integration.
    Init {
        /// Git repository URL or GitHub shorthand (user/repo) to clone
        #[arg(value_name = "REPOSITORY")]
        repository: Option<String>,

        /// Shell type (auto-detects from $SHELL if not specified)
        #[arg(short, long, value_name = "SHELL")]
        shell: Option<String>,

        /// Profile name (defaults to active profile or repository slug)
        #[arg(short, long, value_name = "PROFILE")]
        profile: Option<String>,
    },

    /// Clone a profile repository without activating it
    Clone {
        /// Git repository URL or GitHub shorthand (user/repo) to clone
        #[arg(value_name = "REPOSITORY")]
        repository: String,

        /// Profile name (defaults to repository slug)
        #[arg(short, long, value_name = "PROFILE")]
        profile: Option<String>,
    },

    /// Switch to a different profile
    Use {
        /// Profile name to activate
        #[arg(value_name = "PROFILE")]
        profile: String,
    },

    /// List available profiles
    Profiles,

    /// Sync workspace (git pull + reinstall configs/tools)
    Sync,

    /// Reset workspace (clean git state + reinstall everything)
    Reset {
        /// Force reset without confirmation
        #[arg(short, long)]
        force: bool,
    },

    /// Update tools (respects version pins)
    Update {
        /// Tool name (updates all if not specified)
        #[arg(value_name = "TOOL")]
        name: Option<String>,
    },

    /// Show workspace status
    Status,

    /// Clean up unused cache and orphaned symlinks
    Cleanup,

    /// Validate profile and workspace manifests
    Check,

    /// Output environment setup (used in shell init)
    Env {
        /// Shell type (zsh, bash, fish)
        #[arg(short, long, value_name = "SHELL", default_value = "zsh")]
        shell: String,
    },

    /// Manage dws itself
    #[command(subcommand)]
    Self_(SelfAction),
}

#[derive(Subcommand, Debug)]
pub enum SelfAction {
    /// Show dws information (version, disk usage)
    #[command(name = "info")]
    Info,

    /// Update dws to latest version
    #[command(name = "update")]
    Update,

    /// Uninstall dws and remove all data
    #[command(name = "uninstall")]
    Uninstall,
}
