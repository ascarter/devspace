use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

/// Lockfile format (similar to Cargo.lock)
/// Records the resolved state of the installed workspace
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Lockfile {
    /// Version of the lockfile format
    version: u32,
    /// Metadata about when this was generated
    pub metadata: Metadata,
    /// Config file symlinks (dotfiles)
    #[serde(default)]
    pub config_symlinks: Vec<SymlinkEntry>,
    /// Tool binary symlinks
    #[serde(default)]
    pub tool_symlinks: Vec<ToolEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Metadata {
    /// When this workspace was installed/updated
    pub installed_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymlinkEntry {
    /// Source path (in workspace)
    pub source: PathBuf,
    /// Target path (in XDG_CONFIG_HOME)
    pub target: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolEntry {
    /// Tool name
    pub name: String,
    /// Version installed
    pub version: String,
    /// Source path (in cache)
    pub source: PathBuf,
    /// Target path (in state/bin)
    pub target: PathBuf,
}

impl Lockfile {
    /// Create a new lockfile
    pub fn new() -> Self {
        let now = chrono::Utc::now().to_rfc3339();
        Self {
            version: 1,
            metadata: Metadata {
                installed_at: now,
            },
            config_symlinks: Vec::new(),
            tool_symlinks: Vec::new(),
        }
    }

    /// Load lockfile from disk
    pub fn load(path: &Path) -> Result<Self> {
        let contents = fs::read_to_string(path)
            .with_context(|| format!("Failed to read lockfile from {:?}", path))?;

        toml::from_str(&contents)
            .with_context(|| format!("Failed to parse lockfile from {:?}", path))
    }

    /// Save lockfile to disk
    pub fn save(&self, path: &Path) -> Result<()> {
        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create lockfile directory {:?}", parent))?;
        }

        let contents =
            toml::to_string_pretty(self).context("Failed to serialize lockfile")?;

        fs::write(path, contents)
            .with_context(|| format!("Failed to write lockfile to {:?}", path))?;

        Ok(())
    }

    /// Add a config symlink entry
    pub fn add_config_symlink(&mut self, source: PathBuf, target: PathBuf) {
        self.config_symlinks.push(SymlinkEntry { source, target });
    }

    /// Add a tool symlink entry
    pub fn add_tool_symlink(&mut self, name: String, version: String, source: PathBuf, target: PathBuf) {
        self.tool_symlinks.push(ToolEntry {
            name,
            version,
            source,
            target,
        });
    }

    /// Iterate over all config symlink entries
    pub fn config_symlinks(&self) -> impl Iterator<Item = &SymlinkEntry> {
        self.config_symlinks.iter()
    }

    /// Iterate over all tool symlink entries
    pub fn tool_symlinks(&self) -> impl Iterator<Item = &ToolEntry> {
        self.tool_symlinks.iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_lockfile_new() {
        let lockfile = Lockfile::new();
        assert_eq!(lockfile.version, 1);
        assert!(!lockfile.metadata.installed_at.is_empty());
        assert!(lockfile.config_symlinks.is_empty());
        assert!(lockfile.tool_symlinks.is_empty());
    }

    #[test]
    fn test_lockfile_save_load() {
        let temp = TempDir::new().unwrap();
        let lockfile_path = temp.path().join("devws.lock");

        let mut lockfile = Lockfile::new();
        lockfile.add_config_symlink(
            PathBuf::from("/source/.zshrc"),
            PathBuf::from("/target/.zshrc"),
        );
        lockfile.add_tool_symlink(
            "rg".to_string(),
            "14.0.0".to_string(),
            PathBuf::from("/cache/rg"),
            PathBuf::from("/bin/rg"),
        );

        lockfile.save(&lockfile_path).unwrap();
        assert!(lockfile_path.exists());

        let loaded = Lockfile::load(&lockfile_path).unwrap();
        assert_eq!(loaded.config_symlinks.len(), 1);
        assert_eq!(loaded.tool_symlinks.len(), 1);
        assert_eq!(loaded.tool_symlinks[0].name, "rg");
        assert_eq!(loaded.tool_symlinks[0].version, "14.0.0");
    }

    #[test]
    fn test_lockfile_add_entries() {
        let mut lockfile = Lockfile::new();

        lockfile.add_config_symlink(
            PathBuf::from("/a"),
            PathBuf::from("/b"),
        );
        lockfile.add_tool_symlink(
            "tool".to_string(),
            "1.0".to_string(),
            PathBuf::from("/c"),
            PathBuf::from("/d"),
        );

        assert_eq!(lockfile.config_symlinks.len(), 1);
        assert_eq!(lockfile.tool_symlinks.len(), 1);
    }

    #[test]
    fn test_lockfile_iterators() {
        let mut lockfile = Lockfile::new();

        lockfile.add_config_symlink(
            PathBuf::from("/config/zsh/.zshrc"),
            PathBuf::from("/home/.config/zsh/.zshrc"),
        );
        lockfile.add_config_symlink(
            PathBuf::from("/config/nvim/init.lua"),
            PathBuf::from("/home/.config/nvim/init.lua"),
        );
        lockfile.add_tool_symlink(
            "rg".to_string(),
            "14.0.0".to_string(),
            PathBuf::from("/cache/rg"),
            PathBuf::from("/bin/rg"),
        );
        lockfile.add_tool_symlink(
            "fd".to_string(),
            "9.0.0".to_string(),
            PathBuf::from("/cache/fd"),
            PathBuf::from("/bin/fd"),
        );

        // Test config symlinks iterator
        let config_entries: Vec<_> = lockfile.config_symlinks().collect();
        assert_eq!(config_entries.len(), 2);
        assert_eq!(config_entries[0].source, PathBuf::from("/config/zsh/.zshrc"));
        assert_eq!(config_entries[1].source, PathBuf::from("/config/nvim/init.lua"));

        // Test tool symlinks iterator
        let tool_entries: Vec<_> = lockfile.tool_symlinks().collect();
        assert_eq!(tool_entries.len(), 2);
        assert_eq!(tool_entries[0].name, "rg");
        assert_eq!(tool_entries[0].version, "14.0.0");
        assert_eq!(tool_entries[1].name, "fd");
        assert_eq!(tool_entries[1].version, "9.0.0");
    }
}
