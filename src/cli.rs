use clap::{Parser, Subcommand};

/// Personal development environment manager
///
/// devspace manages your dotfiles and development tools through declarative
/// manifests. It combines configuration management with tool installation in
/// a single, portable binary.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    /// Enable verbose output
    #[arg(short, long, global = true)]
    pub verbose: bool,

    /// Profile directory (defaults to $XDG_CONFIG_HOME/devspace)
    #[arg(short, long, global = true)]
    pub profile: Option<String>,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Initialize shell integration
    Init {
        /// Shell type (bash, zsh, fish)
        #[arg(value_name = "SHELL")]
        shell: Option<String>,
    },

    /// Export environment variables
    Env,

    /// Manage configuration files
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },

    /// Manage applications
    App {
        #[command(subcommand)]
        action: AppAction,
    },

    /// Manage profiles
    Profile {
        #[command(subcommand)]
        action: ProfileAction,
    },

    /// Check and repair environment
    Doctor,

    /// Show status of entire environment
    Status,
}

#[derive(Subcommand, Debug)]
pub enum ConfigAction {
    /// Show configuration status
    Status,
    /// Link configuration files
    Link,
    /// Unlink configuration files
    Unlink,
}

#[derive(Subcommand, Debug)]
pub enum AppAction {
    /// List all applications in manifests
    List,
    /// Show application status
    Status {
        /// Application name (shows all if not specified)
        name: Option<String>,
    },
    /// Install application(s)
    Install {
        /// Application name (installs all if not specified)
        name: Option<String>,
    },
    /// Update application(s)
    Update {
        /// Application name (updates all if not specified)
        name: Option<String>,
    },
    /// Uninstall application
    Uninstall {
        /// Application name
        name: String,
    },
}

#[derive(Subcommand, Debug)]
pub enum ProfileAction {
    /// List available profiles
    List,
    /// Show current profile
    Current,
    /// Clone a profile from a Git repository
    Clone {
        /// Git repository URL or GitHub shorthand (user/repo)
        repository: String,
        /// Profile name (defaults to repository name)
        #[arg(short, long)]
        name: Option<String>,
    },
    /// Activate a profile
    Activate {
        /// Profile name
        name: String,
    },
    /// Create a new profile
    Create {
        /// Profile name
        name: String,
    },
}
