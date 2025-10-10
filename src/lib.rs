// Public API
pub mod cli;
pub mod commands;

// Core domain types
mod config;
mod dotfiles;
mod environment;
mod installers;
mod lockfile;
mod profile;
mod toolset;
mod workspace;

// Re-export main types
pub use config::Config;
pub use dotfiles::{DotfileEntry, Dotfiles};
pub use environment::{Environment, Shell};
pub use lockfile::Lockfile;
pub use profile::Profile;
pub use toolset::{InstallerKind, ToolDefinition, ToolEntry, ToolSet};
pub use workspace::{Workspace, WorkspacePath};
