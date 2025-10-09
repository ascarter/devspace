use anyhow::{anyhow, Context, Result};
use serde::Deserialize;
use std::collections::{BTreeMap, HashMap};
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use whoami::fallible;

/// Supported installer backends defined in manifests.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum InstallerKind {
    Ubi,
    Dmg,
    Flatpak,
    Curl,
}

impl fmt::Display for InstallerKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            InstallerKind::Ubi => write!(f, "ubi"),
            InstallerKind::Dmg => write!(f, "dmg"),
            InstallerKind::Flatpak => write!(f, "flatpak"),
            InstallerKind::Curl => write!(f, "curl"),
        }
    }
}

/// A tool definition loaded from a manifest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolDefinition {
    /// Installer backend used to provision the tool (e.g. `ubi`, `curl`).
    pub installer: InstallerKind,
    /// GitHub `owner/repo` when using backends that download releases.
    pub project: Option<String>,
    /// Explicit version pin; `None` means use the backend default/latest.
    pub version: Option<String>,
    /// Direct download URL for installers that fetch a script or disk image.
    pub url: Option<String>,
    /// Shell interpreter to execute the installer script (e.g. `sh`, `bash`).
    pub shell: Option<String>,
    /// Executables that should be linked into the workspace `bin/` directory.
    pub bin: Vec<String>,
    /// Additional files to symlink; supports `source:target` syntax.
    pub symlinks: Vec<String>,
    /// Name of the `.app` bundle for macOS DMG installations.
    pub app: Option<String>,
    /// Apple Developer team ID used to validate signed macOS apps.
    pub team_id: Option<String>,
    /// Whether the tool has its own update mechanism and should be skipped by `dws update`.
    pub self_update: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum ManifestScope {
    Base = 0,
    Platform = 1,
    Host = 2,
}

/// Representation of a single manifest file.
#[derive(Debug, Clone)]
pub struct Manifest {
    pub path: PathBuf,
    pub precedence: u8,
    /// Partially specified tool definitions as they were parsed from the manifest.
    /// Kept in raw form so higher-precedence manifests can override individual fields.
    entries: BTreeMap<String, ToolDefinitionOverlay>,
}

impl Manifest {
    /// Load a manifest file from disk with a known scope.
    fn load(path: &Path, scope: ManifestScope) -> Result<Self> {
        let contents = fs::read_to_string(path)
            .with_context(|| format!("Failed to read manifest file {:?}", path))?;

        if contents.trim().is_empty() {
            return Ok(Self {
                path: path.to_path_buf(),
                precedence: scope as u8,
                entries: BTreeMap::new(),
            });
        }

        let raw: RawManifest = toml::from_str(&contents)
            .with_context(|| format!("Failed to parse manifest file {:?}", path))?;

        let mut entries = BTreeMap::new();

        for (name, raw_def) in raw.entries {
            entries.insert(name, raw_def);
        }

        Ok(Self {
            path: path.to_path_buf(),
            precedence: scope as u8,
            entries,
        })
    }
}

/// An entry combines the tool name, the manifest it came from, and its definition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManifestEntry {
    pub name: String,
    pub source: PathBuf,
    pub precedence: u8,
    pub definition: ToolDefinition,
}

/// All manifests discovered in the workspace.
#[derive(Debug, Default)]
pub struct ManifestSet {
    manifests: Vec<Manifest>,
    merged: BTreeMap<String, ManifestEntry>,
}

impl ManifestSet {
    /// Load all `*.toml` manifests from the directory.
    pub fn load_from_dir(dir: &Path) -> Result<Self> {
        if !dir.exists() {
            return Ok(Self::default());
        }

        let mut manifests = Vec::new();

        let base_path = dir.join("tools.toml");
        if base_path.exists() {
            manifests.push(Manifest::load(&base_path, ManifestScope::Base)?);
        } else {
            return Ok(Self::default());
        }

        if let Some(platform_path) = platform_manifest_path(dir) {
            manifests.push(Manifest::load(&platform_path, ManifestScope::Platform)?);
        }

        if let Some(host_path) = host_manifest_path(dir) {
            manifests.push(Manifest::load(&host_path, ManifestScope::Host)?);
        }

        let mut overlays: BTreeMap<String, ToolDefinitionOverlay> = BTreeMap::new();
        let mut sources: HashMap<String, (PathBuf, u8)> = HashMap::new();

        for manifest in &manifests {
            for (name, overlay) in &manifest.entries {
                overlays.entry(name.clone()).or_default().apply(overlay);
                sources.insert(name.clone(), (manifest.path.clone(), manifest.precedence));
            }
        }

        let mut merged: BTreeMap<String, ManifestEntry> = BTreeMap::new();

        for (name, overlay) in overlays {
            let definition = overlay
                .into_definition(&name)
                .with_context(|| format!("While finalizing tool '{}'", name))?;
            if let Some((source, precedence)) = sources.remove(&name) {
                merged.insert(
                    name.clone(),
                    ManifestEntry {
                        name,
                        source,
                        precedence,
                        definition,
                    },
                );
            }
        }

        Ok(Self { manifests, merged })
    }

    /// Iterate over resolved manifest entries.
    pub fn iter(&self) -> impl Iterator<Item = &ManifestEntry> {
        self.merged.values()
    }

    /// Returns the total number of resolved tool definitions.
    pub fn len(&self) -> usize {
        self.merged.len()
    }

    /// Whether any manifest entries were discovered.
    pub fn is_empty(&self) -> bool {
        self.merged.is_empty()
    }

    /// Access the underlying manifest files in precedence order.
    pub fn manifests(&self) -> &[Manifest] {
        &self.manifests
    }
}

/// Helper used while merging layered manifest definitions into a concrete [`ToolDefinition`].
#[derive(Debug, Default, Clone, Deserialize)]
struct ToolDefinitionOverlay {
    installer: Option<InstallerKind>,
    project: Option<String>,
    version: Option<String>,
    url: Option<String>,
    shell: Option<String>,
    bin: Option<Vec<String>>,
    symlinks: Option<Vec<String>>,
    app: Option<String>,
    team_id: Option<String>,
    self_update: Option<bool>,
}

impl ToolDefinitionOverlay {
    /// Merge raw values from a single manifest layer.
    fn apply(&mut self, other: &ToolDefinitionOverlay) {
        if let Some(installer) = other.installer {
            self.installer = Some(installer);
        }

        if let Some(project) = &other.project {
            self.project = Some(project.clone());
        }

        if let Some(version) = &other.version {
            self.version = Some(version.clone());
        }

        if let Some(url) = &other.url {
            self.url = Some(url.clone());
        }

        if let Some(shell) = &other.shell {
            self.shell = Some(shell.clone());
        }

        if let Some(bin) = &other.bin {
            self.bin = Some(bin.clone());
        }

        if let Some(symlinks) = &other.symlinks {
            self.symlinks = Some(symlinks.clone());
        }

        if let Some(app) = &other.app {
            self.app = Some(app.clone());
        }

        if let Some(team_id) = &other.team_id {
            self.team_id = Some(team_id.clone());
        }

        if let Some(self_update) = other.self_update {
            self.self_update = Some(self_update);
        }
    }

    /// Finish merging and validate that required fields are present.
    fn into_definition(self, name: &str) -> Result<ToolDefinition> {
        let installer = self
            .installer
            .ok_or_else(|| anyhow!("Tool '{}' is missing required field 'installer'", name))?;

        Ok(ToolDefinition {
            installer,
            project: self.project,
            version: self.version,
            url: self.url,
            shell: self.shell,
            bin: self.bin.unwrap_or_default(),
            symlinks: self.symlinks.unwrap_or_default(),
            app: self.app,
            team_id: self.team_id,
            self_update: self.self_update.unwrap_or(false),
        })
    }
}

#[derive(Debug, Deserialize)]
struct RawManifest {
    #[serde(flatten)]
    entries: BTreeMap<String, ToolDefinitionOverlay>,
}

fn platform_manifest_path(dir: &Path) -> Option<PathBuf> {
    let platform = platform_slug();
    let path = dir.join(format!("tools-{}.toml", platform));
    path.exists().then_some(path)
}

fn host_manifest_path(dir: &Path) -> Option<PathBuf> {
    host_slug().and_then(|slug| {
        let path = dir.join(format!("tools-{}.toml", slug));
        path.exists().then_some(path)
    })
}

fn platform_slug() -> &'static str {
    match std::env::consts::OS {
        "macos" => "macos",
        "linux" => "linux",
        "windows" => "windows",
        other => other,
    }
}

fn host_slug() -> Option<String> {
    let raw = fallible::hostname()
        .ok()
        .filter(|value| !value.trim().is_empty())
        .or_else(|| {
            std::env::var("HOSTNAME")
                .ok()
                .filter(|value| !value.trim().is_empty())
        })
        .or_else(|| {
            std::env::var("COMPUTERNAME")
                .ok()
                .filter(|value| !value.trim().is_empty())
        })
        .or_else(|| {
            std::env::var("HOST")
                .ok()
                .filter(|value| !value.trim().is_empty())
        })
        .unwrap_or_else(|| "local".to_string());

    let mut slug = String::new();
    let mut previous_dash = false;

    for ch in raw.chars() {
        let mapped = if ch.is_ascii_alphanumeric() {
            previous_dash = false;
            ch.to_ascii_lowercase()
        } else {
            if previous_dash {
                continue;
            }
            previous_dash = true;
            '-'
        };

        slug.push(mapped);
    }

    let slug = slug.trim_matches('-').to_string();
    if slug.is_empty() {
        Some("local".to_string())
    } else {
        Some(slug)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn host_slug_for_tests() -> String {
        super::host_slug()
            .expect("hostname slug should be derivable (contains alphanumeric characters)")
    }

    #[test]
    fn load_from_dir_parses_multiple_files() {
        let temp = TempDir::new().unwrap();
        let manifest_dir = temp.path();

        let base_manifest = r#"
[ripgrep]
installer = "ubi"
project = "BurntSushi/ripgrep"
version = "14.0.0"
bin = ["rg"]
symlinks = ["doc/rg.1:${STATE_DIR}/share/man/man1/rg.1"]
self_update = true

[fd]
installer = "ubi"
project = "sharkdp/fd"
"#;

        let platform_manifest = r#"
[ghostty]
installer = "dmg"
url = "https://ghostty.org/download"
app = "Ghostty.app"
team_id = "24VZTF6M5V"
self_update = true
"#;

        fs::write(manifest_dir.join("tools.toml"), base_manifest).unwrap();
        let platform_file = format!("tools-{}.toml", super::platform_slug());
        fs::write(manifest_dir.join(&platform_file), platform_manifest).unwrap();

        let manifests = ManifestSet::load_from_dir(manifest_dir).unwrap();
        assert_eq!(manifests.len(), 3);

        let mut names: Vec<_> = manifests.iter().map(|m| m.name.clone()).collect();
        names.sort();
        assert_eq!(names, vec!["fd", "ghostty", "ripgrep"]);

        let ghostty = manifests.iter().find(|m| m.name == "ghostty").unwrap();
        assert_eq!(ghostty.definition.installer, InstallerKind::Dmg);
        assert_eq!(
            ghostty.definition.url.as_deref(),
            Some("https://ghostty.org/download")
        );
        assert_eq!(ghostty.definition.app.as_deref(), Some("Ghostty.app"));
        assert_eq!(ghostty.definition.team_id.as_deref(), Some("24VZTF6M5V"));
        assert_eq!(
            ghostty.source.file_name().and_then(|s| s.to_str()).unwrap(),
            platform_file
        );
    }

    #[test]
    fn empty_directory_returns_empty_set() {
        let temp = TempDir::new().unwrap();
        let manifests = ManifestSet::load_from_dir(temp.path()).unwrap();
        assert!(manifests.is_empty());
    }

    #[test]
    fn missing_installer_errors() {
        let temp = TempDir::new().unwrap();
        let dir = temp.path();
        fs::write(dir.join("tools.toml"), "[tool]\nproject = \"example\"\n").unwrap();

        let error = ManifestSet::load_from_dir(dir).unwrap_err();
        let message = format!("{error:#}").to_lowercase();
        assert!(
            message.contains("missing required field 'installer'"),
            "{}",
            error
        );
    }

    #[test]
    fn host_overrides_follow_precedence() {
        let temp = TempDir::new().unwrap();
        let dir = temp.path();

        fs::write(
            dir.join("tools.toml"),
            r#"
[ripgrep]
installer = "ubi"
project = "BurntSushi/ripgrep"
version = "13.0.0"
"#,
        )
        .unwrap();

        fs::write(
            dir.join(format!("tools-{}.toml", super::platform_slug())),
            r#"
[ripgrep]
installer = "ubi"
version = "14.0.0"
"#,
        )
        .unwrap();

        let host_slug = host_slug_for_tests();
        fs::write(
            dir.join(format!("tools-{}.toml", host_slug)),
            r#"
[ripgrep]
installer = "ubi"
version = "15.0.0"
bin = ["rg"]
"#,
        )
        .unwrap();

        let manifests = ManifestSet::load_from_dir(dir).unwrap();
        assert_eq!(manifests.len(), 1);

        let entry = manifests.iter().next().unwrap();
        assert_eq!(entry.definition.version.as_deref(), Some("15.0.0"));
        assert_eq!(entry.definition.bin, vec!["rg"]);
        assert_eq!(
            entry.definition.project.as_deref(),
            Some("BurntSushi/ripgrep")
        );
        assert!(entry
            .source
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap()
            .contains(&host_slug));
        assert_eq!(entry.precedence, 2);

        let manifest_paths: Vec<_> = manifests
            .manifests()
            .iter()
            .map(|m| {
                m.path
                    .file_name()
                    .and_then(|s| s.to_str())
                    .unwrap()
                    .to_string()
            })
            .collect();
        assert_eq!(manifest_paths.len(), 3);
        assert_eq!(manifest_paths[0], "tools.toml");
        assert_eq!(
            manifest_paths[1],
            format!("tools-{}.toml", super::platform_slug())
        );
        assert_eq!(manifest_paths[2], format!("tools-{}.toml", host_slug));
    }

    #[test]
    fn inherits_lower_precedence_fields_when_not_overridden() {
        let temp = TempDir::new().unwrap();
        let dir = temp.path();

        fs::write(
            dir.join("tools.toml"),
            r#"
[ripgrep]
installer = "ubi"
project = "BurntSushi/ripgrep"
bin = ["rg"]
symlinks = ["doc/rg.1:${STATE_DIR}/share/man/man1/rg.1"]
"#,
        )
        .unwrap();

        fs::write(
            dir.join(format!("tools-{}.toml", super::platform_slug())),
            r#"
[ripgrep]
version = "14.0.0"
"#,
        )
        .unwrap();

        let manifests = ManifestSet::load_from_dir(dir).unwrap();
        assert_eq!(manifests.len(), 1);

        let entry = manifests.iter().next().unwrap();
        assert_eq!(entry.definition.version.as_deref(), Some("14.0.0"));
        assert_eq!(entry.definition.bin, vec!["rg"]);
        assert_eq!(
            entry.definition.symlinks,
            vec!["doc/rg.1:${STATE_DIR}/share/man/man1/rg.1"]
        );
        assert_eq!(
            entry.definition.project.as_deref(),
            Some("BurntSushi/ripgrep")
        );
    }

    #[test]
    fn explicit_empty_list_overrides_previous_values() {
        let temp = TempDir::new().unwrap();
        let dir = temp.path();

        fs::write(
            dir.join("tools.toml"),
            r#"
[ripgrep]
installer = "ubi"
bin = ["rg"]
"#,
        )
        .unwrap();

        let host_slug = host_slug_for_tests();
        fs::write(
            dir.join(format!("tools-{}.toml", host_slug)),
            r#"
[ripgrep]
bin = []
"#,
        )
        .unwrap();

        let manifests = ManifestSet::load_from_dir(dir).unwrap();
        let entry = manifests.iter().next().unwrap();
        assert!(entry.definition.bin.is_empty());
        assert_eq!(entry.precedence, 2);
    }

    #[test]
    fn bool_fields_can_be_overridden() {
        let temp = TempDir::new().unwrap();
        let dir = temp.path();

        fs::write(
            dir.join("tools.toml"),
            r#"
[rustup]
installer = "curl"
self_update = true
"#,
        )
        .unwrap();

        fs::write(
            dir.join(format!("tools-{}.toml", super::platform_slug())),
            r#"
[rustup]
self_update = false
"#,
        )
        .unwrap();

        let manifests = ManifestSet::load_from_dir(dir).unwrap();
        let entry = manifests.iter().next().unwrap();
        assert!(!entry.definition.self_update);
    }

    #[test]
    fn host_only_tools_are_loaded() {
        let temp = TempDir::new().unwrap();
        let dir = temp.path();

        fs::write(
            dir.join("tools.toml"),
            r#"
[ripgrep]
installer = "ubi"
"#,
        )
        .unwrap();

        let host_slug = host_slug_for_tests();
        fs::write(
            dir.join(format!("tools-{}.toml", host_slug)),
            r#"
[fd]
installer = "ubi"
"#,
        )
        .unwrap();

        let manifests = ManifestSet::load_from_dir(dir).unwrap();
        assert_eq!(manifests.len(), 2);

        let mut names: Vec<_> = manifests.iter().map(|entry| entry.name.clone()).collect();
        names.sort();
        assert_eq!(names, vec!["fd", "ripgrep"]);
    }
}
