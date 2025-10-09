// Public API
pub mod cli;
pub mod commands;

// Core domain types
mod config;
mod environment;
mod installers;
mod lockfile;
mod manifest;
mod workspace;

// Re-export main types
pub use config::{Config, ConfigEntry};
pub use environment::{Environment, Shell};
pub use lockfile::Lockfile;
pub use manifest::{InstallerKind, Manifest, ManifestEntry, ManifestSet, ToolDefinition};
pub use workspace::{Workspace, WorkspacePath};
