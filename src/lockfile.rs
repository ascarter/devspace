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
    /// Tool installation receipts (schema v2 placeholder)
    #[serde(default)]
    pub tool_receipts: Vec<ToolReceipt>,
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
pub struct BinaryLink {
    /// Symlink name exposed in the workspace bin directory
    pub link: String,
    /// Absolute source path of the binary in the cache/tools directory
    pub source: PathBuf,
    /// Absolute target path of the symlink in the workspace bin directory
    pub target: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolReceipt {
    /// Tool name
    pub name: String,
    /// Version string from manifest (may be "latest")
    pub manifest_version: String,
    /// Resolved concrete version/tag
    pub resolved_version: String,
    /// Installer backend kind (github | gitlab | script)
    pub installer_kind: String,
    /// When this tool/version was installed
    pub installed_at: String,
    /// Linked binaries for this tool
    #[serde(default)]
    pub binaries: Vec<BinaryLink>,
    #[serde(default)]
    pub extras: Vec<ExtraLink>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtraLink {
    pub kind: String,
    pub source: PathBuf,
    pub target: PathBuf,
}

impl Default for Lockfile {
    fn default() -> Self {
        Self::new()
    }
}

impl Lockfile {
    /// Create a new lockfile
    pub fn new() -> Self {
        let now = chrono::Utc::now().to_rfc3339();
        Self {
            version: 2,
            metadata: Metadata { installed_at: now },
            config_symlinks: Vec::new(),
            tool_receipts: Vec::new(),
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

        let contents = toml::to_string_pretty(self).context("Failed to serialize lockfile")?;

        fs::write(path, contents)
            .with_context(|| format!("Failed to write lockfile to {:?}", path))?;

        Ok(())
    }

    /// Add a config symlink entry
    pub fn add_config_symlink(&mut self, source: PathBuf, target: PathBuf) {
        self.config_symlinks.push(SymlinkEntry { source, target });
    }

    /// Add a tool receipt (skeleton placeholder for Phase 0)
    #[allow(clippy::too_many_arguments)]
    pub fn add_tool_receipt(
        &mut self,
        name: String,
        manifest_version: String,
        resolved_version: String,
        installer_kind: String,
        installed_at: String,
        binaries: Vec<BinaryLink>,
        extras: Vec<ExtraLink>,
    ) {
        self.tool_receipts.push(ToolReceipt {
            name,
            manifest_version,
            resolved_version,
            installer_kind,
            installed_at,
            binaries,
            extras,
        });
    }

    /// Convenience helper to record a tool installation event, generating the timestamp automatically.
    ///
    /// This should be preferred over calling `add_tool_receipt` directly in installer backends.
    /// `manifest_version` is the version string as specified in the manifest (may be "latest").
    /// `resolved_version` is the concrete tag/version determined during installation.
    pub fn record_tool_install(
        &mut self,
        name: &str,
        manifest_version: &str,
        resolved_version: &str,
        installer_kind: &str,
        binaries: Vec<BinaryLink>,
        extras: Vec<ExtraLink>,
    ) {
        let installed_at = chrono::Utc::now().to_rfc3339();
        self.add_tool_receipt(
            name.to_string(),
            manifest_version.to_string(),
            resolved_version.to_string(),
            installer_kind.to_string(),
            installed_at,
            binaries,
            extras,
        );
    }

    /// Iterate over all config symlink entries
    pub fn config_symlinks(&self) -> impl Iterator<Item = &SymlinkEntry> {
        self.config_symlinks.iter()
    }

    /// Iterate over all tool receipt entries
    pub fn tool_receipts(&self) -> impl Iterator<Item = &ToolReceipt> {
        self.tool_receipts.iter()
    }

    /// Retain only tool receipt entries that satisfy the provided predicate.
    pub fn retain_tool_receipts<F>(&mut self, mut predicate: F)
    where
        F: FnMut(&ToolReceipt) -> bool,
    {
        self.tool_receipts.retain(|entry| predicate(entry));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_lockfile_new() {
        let lockfile = Lockfile::new();
        assert_eq!(lockfile.version, 2);
        assert!(!lockfile.metadata.installed_at.is_empty());
        assert!(lockfile.config_symlinks.is_empty());
        assert!(lockfile.tool_receipts.is_empty());
    }

    #[test]
    fn test_lockfile_save_load() {
        let temp = TempDir::new().unwrap();
        let lockfile_path = temp.path().join("dws.lock");

        let mut lockfile = Lockfile::new();
        lockfile.add_config_symlink(
            PathBuf::from("/source/.zshrc"),
            PathBuf::from("/target/.zshrc"),
        );
        lockfile.add_tool_receipt(
            "rg".to_string(),
            "14.0.0".to_string(),
            "14.0.0".to_string(),
            "github".to_string(),
            chrono::Utc::now().to_rfc3339(),
            vec![BinaryLink {
                link: "rg".to_string(),
                source: PathBuf::from("/cache/rg"),
                target: PathBuf::from("/bin/rg"),
            }],
            Vec::new(),
        );

        lockfile.save(&lockfile_path).unwrap();
        assert!(lockfile_path.exists());

        let loaded = Lockfile::load(&lockfile_path).unwrap();
        assert_eq!(loaded.config_symlinks.len(), 1);
        assert_eq!(loaded.tool_receipts.len(), 1);
        assert_eq!(loaded.tool_receipts[0].name, "rg");
        assert_eq!(loaded.tool_receipts[0].resolved_version, "14.0.0");
    }

    #[test]
    fn test_lockfile_add_entries() {
        let mut lockfile = Lockfile::new();

        lockfile.add_config_symlink(PathBuf::from("/a"), PathBuf::from("/b"));
        lockfile.add_tool_receipt(
            "tool".to_string(),
            "1.0".to_string(),
            "1.0".to_string(),
            "github".to_string(),
            chrono::Utc::now().to_rfc3339(),
            vec![BinaryLink {
                link: "tool".to_string(),
                source: PathBuf::from("/c"),
                target: PathBuf::from("/bin/tool"),
            }],
            Vec::new(),
        );

        assert_eq!(lockfile.config_symlinks.len(), 1);
        assert_eq!(lockfile.tool_receipts.len(), 1);
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
        lockfile.add_tool_receipt(
            "rg".to_string(),
            "14.0.0".to_string(),
            "14.0.0".to_string(),
            "github".to_string(),
            chrono::Utc::now().to_rfc3339(),
            vec![BinaryLink {
                link: "rg".to_string(),
                source: PathBuf::from("/cache/rg"),
                target: PathBuf::from("/bin/rg"),
            }],
            Vec::new(),
        );
        lockfile.add_tool_receipt(
            "fd".to_string(),
            "9.0.0".to_string(),
            "9.0.0".to_string(),
            "github".to_string(),
            chrono::Utc::now().to_rfc3339(),
            vec![BinaryLink {
                link: "fd".to_string(),
                source: PathBuf::from("/cache/fd"),
                target: PathBuf::from("/bin/fd"),
            }],
            Vec::new(),
        );

        // Test config symlinks iterator
        let config_entries: Vec<_> = lockfile.config_symlinks().collect();
        assert_eq!(config_entries.len(), 2);
        assert_eq!(
            config_entries[0].source,
            PathBuf::from("/config/zsh/.zshrc")
        );
        assert_eq!(
            config_entries[1].source,
            PathBuf::from("/config/nvim/init.lua")
        );

        // Test tool symlinks iterator
        let receipts: Vec<_> = lockfile.tool_receipts().collect();
        assert_eq!(receipts.len(), 2);
        assert_eq!(receipts[0].name, "rg");
        assert_eq!(receipts[0].resolved_version, "14.0.0");
        assert_eq!(receipts[1].name, "fd");
        assert_eq!(receipts[1].resolved_version, "9.0.0");
    }

    #[test]
    fn test_lockfile_retain_tool_symlinks() {
        let mut lockfile = Lockfile::new();

        lockfile.add_tool_receipt(
            "rg".to_string(),
            "14.0.0".to_string(),
            "14.0.0".to_string(),
            "github".to_string(),
            chrono::Utc::now().to_rfc3339(),
            vec![BinaryLink {
                link: "rg".to_string(),
                source: PathBuf::from("/cache/rg"),
                target: PathBuf::from("/bin/rg"),
            }],
            Vec::new(),
        );
        lockfile.add_tool_receipt(
            "fd".to_string(),
            "9.0.0".to_string(),
            "9.0.0".to_string(),
            "github".to_string(),
            chrono::Utc::now().to_rfc3339(),
            vec![BinaryLink {
                link: "fd".to_string(),
                source: PathBuf::from("/cache/fd"),
                target: PathBuf::from("/bin/fd"),
            }],
            Vec::new(),
        );

        lockfile.retain_tool_receipts(|entry| entry.name != "fd");

        let receipts: Vec<_> = lockfile.tool_receipts().collect();
        assert_eq!(receipts.len(), 1);
        assert_eq!(receipts[0].name, "rg");
    }

    #[test]
    fn test_record_tool_install() {
        let mut lockfile = Lockfile::new();
        assert_eq!(lockfile.tool_receipts().count(), 0);

        lockfile.record_tool_install(
            "exa",
            "latest",
            "v1.0.0",
            "github",
            vec![BinaryLink {
                link: "exa".to_string(),
                source: PathBuf::from("/cache/exa"),
                target: PathBuf::from("/bin/exa"),
            }],
            Vec::new(),
        );

        let receipts: Vec<_> = lockfile.tool_receipts().collect();
        assert_eq!(receipts.len(), 1);
        let receipt = receipts[0];
        assert_eq!(receipt.name, "exa");
        assert_eq!(receipt.manifest_version, "latest");
        assert_eq!(receipt.resolved_version, "v1.0.0");
        assert_eq!(receipt.installer_kind, "github");
        assert!(!receipt.installed_at.is_empty());
    }
}
