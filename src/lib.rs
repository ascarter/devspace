// Public API
pub mod cli;
pub mod commands;

// Core domain types
mod config;
mod dotfiles;
mod environment;
mod installers;
mod lockfile;
mod manifest;
mod profile;
mod workspace;

// Re-export main types
pub use config::Config;
pub use dotfiles::{DotfileEntry, Dotfiles};
pub use environment::{Environment, Shell};
pub use lockfile::Lockfile;
pub use manifest::{InstallerKind, Manifest, ManifestEntry, ManifestSet, ToolDefinition};
pub use profile::Profile;
pub use workspace::{Workspace, WorkspacePath};
