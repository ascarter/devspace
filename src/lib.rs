// Public API
pub mod cli;
pub mod commands;

// Core domain types
mod config;
mod environment;
mod lockfile;
mod workspace;

// Re-export main types
pub use config::{Config, ConfigEntry};
pub use environment::{Environment, Shell};
pub use lockfile::Lockfile;
pub use workspace::{Workspace, WorkspacePath};
