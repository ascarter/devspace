use clap::{Parser, Subcommand};

/// Developer Workspace - Personal development environment manager
///
/// devws manages your dotfiles and development tools through declarative
/// manifests. Bootstrap new machines, sync configurations, and maintain your
/// dev environment with a single portable binary.
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
    /// Initialize shell integration and optionally clone a profile
    Init {
        /// Shell type (bash, zsh, fish)
        #[arg(value_name = "SHELL")]
        shell: Option<String>,

        /// Git repository URL or GitHub shorthand (user/repo) to clone
        #[arg(value_name = "REPOSITORY")]
        repository: Option<String>,

        /// Profile name (defaults to 'default')
        #[arg(short, long)]
        name: Option<String>,
    },

    /// Clone a profile from a Git repository
    Clone {
        /// Git repository URL or GitHub shorthand (user/repo)
        #[arg(value_name = "REPOSITORY")]
        repository: String,

        /// Profile name (defaults to 'default')
        #[arg(short, long)]
        name: Option<String>,
    },

    /// Switch to a different profile
    Use {
        /// Profile name to activate
        #[arg(value_name = "PROFILE")]
        name: String,
    },

    /// List available profiles
    List,

    /// Sync profile and install/update tools
    Sync,

    /// Update tools (respects version pins)
    Update {
        /// Tool name (updates all if not specified)
        #[arg(value_name = "TOOL")]
        name: Option<String>,
    },

    /// Show environment status
    Status,

    /// Check environment health and repair issues
    Doctor,

    /// Output environment setup (used in shell init)
    Env {
        /// Profile name (defaults to active profile)
        #[arg(value_name = "PROFILE")]
        profile: Option<String>,
    },

    /// Manage devws itself
    #[command(subcommand)]
    Self_(SelfAction),
}

#[derive(Subcommand, Debug)]
pub enum SelfAction {
    /// Show devws information (version, disk usage, profiles)
    #[command(name = "info")]
    Info,

    /// Update devws to latest version
    #[command(name = "update")]
    Update,

    /// Uninstall devws and remove all data
    #[command(name = "uninstall")]
    Uninstall,
}
