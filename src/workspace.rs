use anyhow::{Context, Result};
use std::env;
use std::fs;
use std::path::PathBuf;

use crate::config::Config;
use crate::environment::{Environment, Shell};
use crate::lockfile::Lockfile;

/// Workspace - represents the devws installation
///
/// The workspace is rooted at $XDG_CONFIG_HOME/devws and represents your dotfiles.
/// No profiles, no switching - this IS your environment.
#[derive(Debug)]
pub struct Workspace {
    /// Workspace root: $XDG_CONFIG_HOME/devws (version controlled)
    workspace_dir: PathBuf,
    /// State directory: $XDG_STATE_HOME/devws (local execution state)
    state_dir: PathBuf,
}

impl Workspace {
    /// Create a new Workspace
    ///
    /// Initializes workspace with XDG-compliant directories:
    /// - Workspace: $XDG_CONFIG_HOME/devws (default: ~/.config/devws)
    /// - State: $XDG_STATE_HOME/devws (default: ~/.local/state/devws)
    pub fn new() -> Result<Self> {
        let workspace_dir = Self::get_workspace_dir()?;
        let state_dir = Self::get_state_dir()?;

        Ok(Self {
            workspace_dir,
            state_dir,
        })
    }

    /// Get the workspace directory (XDG_CONFIG_HOME/devws)
    fn get_workspace_dir() -> Result<PathBuf> {
        let base = env::var("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| {
                directories::BaseDirs::new()
                    .expect("Failed to get home directory")
                    .home_dir()
                    .join(".config")
            });

        Ok(base.join("devws"))
    }

    /// Get the state directory (XDG_STATE_HOME/devws)
    fn get_state_dir() -> Result<PathBuf> {
        let base = env::var("XDG_STATE_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| {
                directories::BaseDirs::new()
                    .expect("Failed to get home directory")
                    .home_dir()
                    .join(".local/state")
            });

        Ok(base.join("devws"))
    }

    /// Get the config directory (workspace_dir/config)
    pub fn config_dir(&self) -> PathBuf {
        self.workspace_dir.join("config")
    }

    /// Get the manifests directory (workspace_dir/manifests)
    pub fn manifests_dir(&self) -> PathBuf {
        self.workspace_dir.join("manifests")
    }

    /// Get the lockfile path
    pub fn lockfile_path(&self) -> PathBuf {
        self.state_dir.join("devws.lock")
    }

    /// Get the bin directory in state
    pub fn bin_dir(&self) -> PathBuf {
        self.state_dir.join("bin")
    }

    /// Get the share directory in state
    pub fn share_dir(&self) -> PathBuf {
        self.state_dir.join("share")
    }

    /// Check if workspace exists (has been initialized)
    pub fn exists(&self) -> bool {
        self.workspace_dir.exists()
    }

    /// Get the Config for managing dotfiles
    pub fn config(&self) -> Result<Config> {
        let config_dir = self.config_dir();

        // Target is $XDG_CONFIG_HOME (default: ~/.config)
        let target_dir = env::var("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| {
                directories::BaseDirs::new()
                    .expect("Failed to get home directory")
                    .home_dir()
                    .join(".config")
            });

        Ok(Config::new(config_dir, target_dir))
    }

    /// Get the Environment for shell integration
    pub fn environment(&self, shell: Shell) -> Result<Environment> {
        Environment::new_from_workspace(self, shell)
    }

    /// Install the workspace (symlink configs, install tools)
    pub fn install(&self) -> Result<()> {
        // Load or create lockfile
        let lockfile_path = self.lockfile_path();
        let mut lockfile = if lockfile_path.exists() {
            // Cleanup existing installation first
            let old_lockfile = Lockfile::load(&lockfile_path)?;
            self.cleanup_symlinks(&old_lockfile)?;
            Lockfile::new()
        } else {
            Lockfile::new()
        };

        // Install config entries and record in lockfile
        let config = self.config()?;
        let config_entries = config.discover_entries()?;

        for entry in &config_entries {
            entry.install()?;
            lockfile.add_config_symlink(entry.source.clone(), entry.target.clone());
        }

        // TODO: Install tools from manifests and add to lockfile

        // Save lockfile
        lockfile.save(&lockfile_path)?;

        Ok(())
    }

    /// Uninstall the workspace (remove all symlinks)
    pub fn uninstall(&self) -> Result<()> {
        let lockfile_path = self.lockfile_path();

        if !lockfile_path.exists() {
            return Ok(());
        }

        let lockfile = Lockfile::load(&lockfile_path)?;
        self.cleanup_symlinks(&lockfile)?;

        // Remove lockfile
        fs::remove_file(&lockfile_path)
            .with_context(|| format!("Failed to remove lockfile {:?}", lockfile_path))?;

        Ok(())
    }

    /// Remove all symlinks tracked in the lockfile
    fn cleanup_symlinks(&self, lockfile: &Lockfile) -> Result<()> {
        // Remove config symlinks
        for entry in lockfile.config_symlinks() {
            if entry.target.exists() || entry.target.symlink_metadata().is_ok() {
                fs::remove_file(&entry.target).with_context(|| {
                    format!("Failed to remove config symlink {:?}", entry.target)
                })?;
            }
        }

        // Remove tool symlinks
        for entry in lockfile.tool_symlinks() {
            if entry.target.exists() || entry.target.symlink_metadata().is_ok() {
                fs::remove_file(&entry.target).with_context(|| {
                    format!("Failed to remove tool symlink {:?}", entry.target)
                })?;
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use tempfile::TempDir;

    fn setup_test_env() -> TempDir {
        let temp = TempDir::new().unwrap();
        env::set_var("XDG_CONFIG_HOME", temp.path());
        env::set_var("XDG_STATE_HOME", temp.path().join("state"));
        temp
    }

    #[test]
    #[serial]
    fn test_workspace_new() {
        let _temp = setup_test_env();
        let workspace = Workspace::new().unwrap();

        assert!(workspace.workspace_dir.to_string_lossy().contains("devws"));
        assert!(workspace.state_dir.to_string_lossy().contains("devws"));
    }

    #[test]
    #[serial]
    fn test_workspace_paths() {
        let _temp = setup_test_env();
        let workspace = Workspace::new().unwrap();

        assert!(workspace.config_dir().to_string_lossy().contains("config"));
        assert!(workspace.manifests_dir().to_string_lossy().contains("manifests"));
        assert!(workspace.lockfile_path().to_string_lossy().contains("devws.lock"));
        assert!(workspace.bin_dir().to_string_lossy().contains("bin"));
    }

    #[test]
    #[serial]
    fn test_workspace_install() {
        let _temp = setup_test_env();
        let workspace = Workspace::new().unwrap();

        // Create workspace structure
        let config_dir = workspace.config_dir();
        fs::create_dir_all(&config_dir).unwrap();
        fs::create_dir_all(config_dir.join("zsh")).unwrap();
        fs::write(config_dir.join("zsh/.zshrc"), "test").unwrap();

        // Install
        workspace.install().unwrap();

        // Verify lockfile was created
        assert!(workspace.lockfile_path().exists());

        // Verify lockfile contents
        let lockfile = Lockfile::load(&workspace.lockfile_path()).unwrap();
        assert_eq!(lockfile.config_symlinks.len(), 1);
    }

    #[test]
    #[serial]
    fn test_workspace_uninstall() {
        let _temp = setup_test_env();
        let workspace = Workspace::new().unwrap();

        // Create and install workspace
        let config_dir = workspace.config_dir();
        fs::create_dir_all(&config_dir).unwrap();
        fs::create_dir_all(config_dir.join("zsh")).unwrap();
        fs::write(config_dir.join("zsh/.zshrc"), "test").unwrap();

        workspace.install().unwrap();

        // Get XDG_CONFIG_HOME
        let xdg_config = env::var("XDG_CONFIG_HOME").unwrap();
        let config_home = PathBuf::from(xdg_config);

        // Verify symlink exists
        assert!(config_home.join("zsh").exists());

        // Uninstall
        workspace.uninstall().unwrap();

        // Verify symlink removed
        assert!(!config_home.join("zsh").exists());

        // Verify lockfile removed
        assert!(!workspace.lockfile_path().exists());
    }

    #[test]
    #[serial]
    fn test_workspace_reinstall() {
        let _temp = setup_test_env();
        let workspace = Workspace::new().unwrap();

        // Create workspace with first config
        let config_dir = workspace.config_dir();
        fs::create_dir_all(&config_dir).unwrap();
        fs::create_dir_all(config_dir.join("zsh")).unwrap();
        fs::write(config_dir.join("zsh/.zshrc"), "test1").unwrap();

        workspace.install().unwrap();

        // Change config
        fs::remove_dir_all(config_dir.join("zsh")).unwrap();
        fs::create_dir_all(config_dir.join("fish")).unwrap();
        fs::write(config_dir.join("fish/config.fish"), "test2").unwrap();

        // Reinstall
        workspace.install().unwrap();

        // Get XDG_CONFIG_HOME
        let xdg_config = env::var("XDG_CONFIG_HOME").unwrap();
        let config_home = PathBuf::from(xdg_config);

        // Verify old symlink removed, new one created
        assert!(!config_home.join("zsh").exists());
        assert!(config_home.join("fish").exists());
    }
}
