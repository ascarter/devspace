use anyhow::{Context, Result};
use std::fs;
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};

/// Built-in ignore patterns (always ignored, similar to Git's built-ins)
/// These are the bare minimum to prevent system files and VCS directories
const BUILTIN_IGNORES: &[&str] = &[
    ".git",
    ".DS_Store",
    ".dwsignore", // Don't symlink the ignore file itself
];

/// Represents a dotfile configuration entry that should be installed
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigEntry {
    /// Source path in workspace (e.g., $XDG_CONFIG_HOME/dws/config/zsh/.zshrc)
    pub source: PathBuf,
    /// Target path in XDG_CONFIG_HOME (e.g., $XDG_CONFIG_HOME/zsh/.zshrc)
    pub target: PathBuf,
}

impl ConfigEntry {
    /// Create a new config entry
    pub fn new(source: PathBuf, target: PathBuf) -> Self {
        Self { source, target }
    }

    /// Check if this entry's filename matches a pattern
    pub fn matches_pattern(&self, pattern: &str) -> bool {
        let filename = match self.source.file_name().and_then(|n| n.to_str()) {
            Some(name) => name,
            None => return false,
        };

        // Exact match
        if pattern == filename {
            return true;
        }

        // Support simple wildcards like *.log
        if pattern.contains('*') {
            let pattern_parts: Vec<&str> = pattern.split('*').collect();
            if pattern_parts.len() == 2 {
                let (prefix, suffix) = (pattern_parts[0], pattern_parts[1]);
                if filename.starts_with(prefix) && filename.ends_with(suffix) {
                    return true;
                }
            }
        }

        false
    }

    /// Check if this entry should be ignored based on patterns
    pub fn should_ignore(&self, user_patterns: &[String]) -> bool {
        let filename = match self.source.file_name().and_then(|n| n.to_str()) {
            Some(name) => name,
            None => return false,
        };

        // Check built-in ignores first
        if BUILTIN_IGNORES.contains(&filename) {
            return true;
        }

        // Check user patterns
        user_patterns.iter().any(|p| self.matches_pattern(p))
    }

    /// Install this config entry (currently via symlink)
    pub fn install(&self) -> Result<()> {
        // Ensure parent directory exists
        if let Some(parent) = self.target.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create parent directory {:?}", parent))?;
        }

        // Remove existing file/symlink if it exists
        if self.target.exists() || self.target.symlink_metadata().is_ok() {
            fs::remove_file(&self.target).with_context(|| {
                format!("Failed to remove existing config entry {:?}", self.target)
            })?;
        }

        // Install via symlink (implementation detail)
        symlink(&self.source, &self.target).with_context(|| {
            format!(
                "Failed to install config entry from {:?} to {:?}",
                self.source, self.target
            )
        })?;

        Ok(())
    }
}

/// Manages configuration files (dotfiles) for the workspace
pub struct Config {
    /// Path to the workspace's config directory
    config_dir: PathBuf,
    /// Target directory ($XDG_CONFIG_HOME, default: ~/.config)
    target_dir: PathBuf,
    /// Ignore patterns loaded from .dwsignore
    ignore_patterns: Vec<String>,
}

impl Config {
    /// Create a new profile configuration
    pub fn new(config_dir: PathBuf, target_dir: PathBuf) -> Self {
        let ignore_patterns = Self::load_ignore_patterns(&config_dir);
        Self {
            config_dir,
            target_dir,
            ignore_patterns,
        }
    }

    /// Load ignore patterns from .dwsignore file
    fn load_ignore_patterns(config_dir: &Path) -> Vec<String> {
        let ignore_file = config_dir.join(".dwsignore");

        if !ignore_file.exists() {
            return Vec::new();
        }

        fs::read_to_string(&ignore_file)
            .unwrap_or_default()
            .lines()
            .map(|line| line.trim())
            .filter(|line| !line.is_empty() && !line.starts_with('#'))
            .map(|line| line.to_string())
            .collect()
    }

    /// Discover all config entries in the profile's config directory
    pub fn discover_entries(&self) -> Result<Vec<ConfigEntry>> {
        let mut entries = Vec::new();

        if !self.config_dir.exists() {
            return Ok(entries);
        }

        for entry in fs::read_dir(&self.config_dir)
            .with_context(|| format!("Failed to read config directory {:?}", self.config_dir))?
        {
            let entry = entry?;
            let source = entry.path();

            let relative = source
                .strip_prefix(&self.config_dir)
                .context("Failed to get relative path")?;

            let target = self.target_dir.join(relative);

            let config_entry = ConfigEntry::new(source, target);

            // Skip ignored entries
            if config_entry.should_ignore(&self.ignore_patterns) {
                continue;
            }

            entries.push(config_entry);
        }

        Ok(entries)
    }

    /// Install all discovered config entries
    pub fn install(&self) -> Result<Vec<PathBuf>> {
        let entries = self.discover_entries()?;
        let mut installed = Vec::new();

        for entry in &entries {
            entry.install()?;
            installed.push(entry.target.clone());
        }

        Ok(installed)
    }

    /// Uninstall all config entries that belong to this profile
    pub fn uninstall(&self) -> Result<Vec<PathBuf>> {
        let mut removed = Vec::new();

        if !self.target_dir.exists() {
            return Ok(removed);
        }

        for entry in fs::read_dir(&self.target_dir)
            .with_context(|| format!("Failed to read target directory {:?}", self.target_dir))?
        {
            let entry = entry?;
            let path = entry.path();

            // Check if it's a symlink pointing to our config directory
            if let Ok(metadata) = fs::symlink_metadata(&path) {
                if metadata.is_symlink() {
                    if let Ok(link_target) = fs::read_link(&path) {
                        // Make absolute if relative
                        let absolute_target = if link_target.is_absolute() {
                            link_target
                        } else {
                            self.target_dir.join(&link_target)
                        };

                        // Check if it points into our config directory
                        if absolute_target.starts_with(&self.config_dir) {
                            fs::remove_file(&path).with_context(|| {
                                format!("Failed to uninstall config entry {:?}", path)
                            })?;
                            removed.push(path);
                        }
                    }
                }
            }
        }

        Ok(removed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_profile_configuration_discover_entries_empty() {
        let temp = TempDir::new().unwrap();
        let config_dir = temp.path().join("config");
        let home_dir = temp.path().join("home");

        fs::create_dir_all(&config_dir).unwrap();
        fs::create_dir_all(&home_dir).unwrap();

        let config = Config::new(config_dir, home_dir);
        let entries = config.discover_entries().unwrap();
        assert!(entries.is_empty());
    }

    #[test]
    fn test_profile_configuration_discover_entries_with_files() {
        let temp = TempDir::new().unwrap();
        let profile_config_dir = temp.path().join("profile/config");
        let xdg_config_home = temp.path().join("xdg_config");

        fs::create_dir_all(&profile_config_dir).unwrap();
        fs::create_dir_all(&xdg_config_home).unwrap();

        // Create XDG-style structure: config/zsh/.zshrc, config/git/config
        fs::create_dir_all(profile_config_dir.join("zsh")).unwrap();
        fs::create_dir_all(profile_config_dir.join("git")).unwrap();
        fs::write(profile_config_dir.join("zsh/.zshrc"), "test").unwrap();
        fs::write(profile_config_dir.join("git/config"), "test").unwrap();

        let config = Config::new(profile_config_dir, xdg_config_home.clone());
        let entries = config.discover_entries().unwrap();
        assert_eq!(entries.len(), 2);

        // Check that entries point to XDG_CONFIG_HOME locations
        let has_zsh_dir = entries
            .iter()
            .any(|e| e.target == xdg_config_home.join("zsh"));
        let has_git_dir = entries
            .iter()
            .any(|e| e.target == xdg_config_home.join("git"));

        assert!(has_zsh_dir);
        assert!(has_git_dir);
    }

    #[test]
    fn test_profile_configuration_with_dwsignore() {
        let temp = TempDir::new().unwrap();
        let profile_config_dir = temp.path().join("profile/config");
        let xdg_config_home = temp.path().join("xdg_config");

        fs::create_dir_all(&profile_config_dir).unwrap();
        fs::create_dir_all(&xdg_config_home).unwrap();

        // Create .dwsignore to ignore .gitkeep
        fs::write(profile_config_dir.join(".dwsignore"), ".gitkeep\n").unwrap();

        fs::write(profile_config_dir.join(".gitkeep"), "").unwrap();
        fs::create_dir_all(profile_config_dir.join("zsh")).unwrap();
        fs::write(profile_config_dir.join("zsh/.zshrc"), "test").unwrap();

        let config = Config::new(profile_config_dir, xdg_config_home.clone());
        let entries = config.discover_entries().unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].target, xdg_config_home.join("zsh"));
    }

    #[test]
    fn test_profile_configuration_builtin_ignores() {
        let temp = TempDir::new().unwrap();
        let profile_config_dir = temp.path().join("profile/config");
        let xdg_config_home = temp.path().join("xdg_config");

        fs::create_dir_all(&profile_config_dir).unwrap();
        fs::create_dir_all(&xdg_config_home).unwrap();

        // Create files/dirs that should be ignored by built-in rules
        fs::create_dir_all(profile_config_dir.join(".git")).unwrap();
        fs::write(profile_config_dir.join(".DS_Store"), "").unwrap();

        // Create XDG structure that should be discovered
        fs::create_dir_all(profile_config_dir.join("zsh")).unwrap();
        fs::create_dir_all(profile_config_dir.join("git")).unwrap();
        fs::write(profile_config_dir.join("zsh/.zshrc"), "test").unwrap();
        fs::write(profile_config_dir.join("git/config"), "test").unwrap();

        let config = Config::new(profile_config_dir, xdg_config_home.clone());
        let entries = config.discover_entries().unwrap();
        assert_eq!(entries.len(), 2);

        let has_zsh = entries
            .iter()
            .any(|e| e.target == xdg_config_home.join("zsh"));
        let has_git = entries
            .iter()
            .any(|e| e.target == xdg_config_home.join("git"));

        assert!(has_zsh);
        assert!(has_git);
    }

    #[test]
    fn test_config_entry_matches_pattern_exact() {
        let entry = ConfigEntry::new(
            PathBuf::from("/config/README.md"),
            PathBuf::from("/home/README.md"),
        );

        assert!(entry.matches_pattern("README.md"));
        assert!(!entry.matches_pattern("LICENSE"));
    }

    #[test]
    fn test_config_entry_matches_pattern_wildcard() {
        let entry = ConfigEntry::new(
            PathBuf::from("/config/test.log"),
            PathBuf::from("/home/test.log"),
        );

        assert!(entry.matches_pattern("*.log"));
        assert!(!entry.matches_pattern("*.tmp"));
    }

    #[test]
    fn test_config_entry_with_user_patterns() {
        let patterns = vec!["README.md".to_string(), "*.log".to_string()];

        let readme = ConfigEntry::new(
            PathBuf::from("/config/README.md"),
            PathBuf::from("/home/README.md"),
        );
        assert!(readme.should_ignore(&patterns));

        let log = ConfigEntry::new(
            PathBuf::from("/config/test.log"),
            PathBuf::from("/home/test.log"),
        );
        assert!(log.should_ignore(&patterns));

        let zshrc = ConfigEntry::new(
            PathBuf::from("/config/.zshrc"),
            PathBuf::from("/home/.zshrc"),
        );
        assert!(!zshrc.should_ignore(&patterns));
    }

    #[test]
    fn test_config_entry_with_builtin_patterns() {
        let patterns = vec![];

        let git = ConfigEntry::new(PathBuf::from("/config/.git"), PathBuf::from("/home/.git"));
        assert!(git.should_ignore(&patterns));

        let ds_store = ConfigEntry::new(
            PathBuf::from("/config/.DS_Store"),
            PathBuf::from("/home/.DS_Store"),
        );
        assert!(ds_store.should_ignore(&patterns));
    }

    #[test]
    fn test_profile_configuration_install() {
        let temp = TempDir::new().unwrap();
        let profile_config_dir = temp.path().join("profile/config");
        let xdg_config_home = temp.path().join("xdg_config");

        fs::create_dir_all(profile_config_dir.join("zsh")).unwrap();
        fs::create_dir_all(&xdg_config_home).unwrap();

        fs::write(profile_config_dir.join("zsh/.zshrc"), "test").unwrap();

        let config = Config::new(profile_config_dir.clone(), xdg_config_home.clone());
        let installed = config.install().unwrap();

        assert_eq!(installed.len(), 1);
        assert_eq!(installed[0], xdg_config_home.join("zsh"));

        // Verify symlink was created
        let link_target = fs::read_link(&installed[0]).unwrap();
        assert_eq!(link_target, profile_config_dir.join("zsh"));
    }

    #[test]
    fn test_profile_configuration_uninstall() {
        let temp = TempDir::new().unwrap();
        let profile_config_dir = temp.path().join("profile/config");
        let xdg_config_home = temp.path().join("xdg_config");

        fs::create_dir_all(profile_config_dir.join("zsh")).unwrap();
        fs::create_dir_all(&xdg_config_home).unwrap();

        // Create a file in profile config
        let config_zsh_dir = profile_config_dir.join("zsh");
        fs::write(config_zsh_dir.join(".zshrc"), "content").unwrap();

        // Create symlink in XDG_CONFIG_HOME pointing to profile config
        let entry_path = xdg_config_home.join("zsh");
        symlink(&config_zsh_dir, &entry_path).unwrap();

        // Create a regular directory (should not be removed)
        let regular_dir = xdg_config_home.join("other");
        fs::create_dir_all(&regular_dir).unwrap();

        // Uninstall
        let config = Config::new(profile_config_dir, xdg_config_home);
        let removed = config.uninstall().unwrap();

        assert_eq!(removed.len(), 1);
        assert_eq!(removed[0], entry_path);
        assert!(!entry_path.exists());
        assert!(regular_dir.exists());
    }
}
