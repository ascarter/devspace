use anyhow::{bail, Context, Result};
use serde::Deserialize;
use std::collections::{BTreeMap, HashMap};
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

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
    pub installer: InstallerKind,
    pub project: Option<String>,
    pub version: Option<String>,
    pub url: Option<String>,
    pub shell: Option<String>,
    pub bin: Vec<String>,
    pub symlinks: Vec<String>,
    pub app: Option<String>,
    pub team_id: Option<String>,
    pub self_update: bool,
}

/// Representation of a single manifest file.
#[derive(Debug, Clone)]
pub struct Manifest {
    pub path: PathBuf,
    pub precedence: u8,
    pub tools: BTreeMap<String, ToolDefinition>,
}

impl Manifest {
    /// Load a manifest file from disk.
    pub fn load(path: &Path) -> Result<Self> {
        let precedence = precedence_for(path);

        let contents = fs::read_to_string(path)
            .with_context(|| format!("Failed to read manifest file {:?}", path))?;

        if contents.trim().is_empty() {
            return Ok(Self {
                path: path.to_path_buf(),
                precedence,
                tools: BTreeMap::new(),
            });
        }

        let raw: RawManifest = toml::from_str(&contents)
            .with_context(|| format!("Failed to parse manifest file {:?}", path))?;

        let mut tools = BTreeMap::new();

        for (name, raw_def) in raw.entries {
            let definition = ToolDefinition {
                installer: raw_def.installer,
                project: raw_def.project,
                version: raw_def.version,
                url: raw_def.url,
                shell: raw_def.shell,
                bin: raw_def.bin,
                symlinks: raw_def.symlinks,
                app: raw_def.app,
                team_id: raw_def.team_id,
                self_update: raw_def.self_update,
            };

            tools.insert(name, definition);
        }

        Ok(Self {
            path: path.to_path_buf(),
            precedence,
            tools,
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

        for entry in fs::read_dir(dir)
            .with_context(|| format!("Failed to read manifests directory {:?}", dir))?
        {
            let entry = entry?;
            let path = entry.path();

            if !path.is_file() || path.extension().and_then(|ext| ext.to_str()) != Some("toml") {
                continue;
            }

            let manifest = Manifest::load(&path)?;
            manifests.push(manifest);
        }

        // Sort manifests by precedence (global -> platform -> host) and then by file name
        manifests.sort_by(|a, b| {
            let a_name = a
                .path
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or_default();
            let b_name = b
                .path
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or_default();

            a.precedence
                .cmp(&b.precedence)
                .then_with(|| a_name.cmp(b_name))
        });

        let mut merged: BTreeMap<String, ManifestEntry> = BTreeMap::new();
        let mut precedence_map: HashMap<String, (u8, PathBuf)> = HashMap::new();

        for manifest in &manifests {
            for (name, definition) in &manifest.tools {
                match precedence_map.get(name) {
                    None => {
                        precedence_map
                            .insert(name.clone(), (manifest.precedence, manifest.path.clone()));
                        merged.insert(
                            name.clone(),
                            ManifestEntry {
                                name: name.clone(),
                                source: manifest.path.clone(),
                                precedence: manifest.precedence,
                                definition: definition.clone(),
                            },
                        );
                    }
                    Some((existing_precedence, existing_path)) => {
                        if manifest.precedence == *existing_precedence {
                            bail!(
                                "Tool '{}' defined in both {:?} and {:?} with the same precedence",
                                name,
                                existing_path,
                                manifest.path
                            );
                        }

                        if manifest.precedence > *existing_precedence {
                            precedence_map
                                .insert(name.clone(), (manifest.precedence, manifest.path.clone()));
                            merged.insert(
                                name.clone(),
                                ManifestEntry {
                                    name: name.clone(),
                                    source: manifest.path.clone(),
                                    precedence: manifest.precedence,
                                    definition: definition.clone(),
                                },
                            );
                        }
                    }
                }
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

#[derive(Debug, Deserialize)]
struct RawManifest {
    #[serde(flatten)]
    entries: BTreeMap<String, RawToolDefinition>,
}

#[derive(Debug, Deserialize)]
struct RawToolDefinition {
    installer: InstallerKind,
    #[serde(default)]
    project: Option<String>,
    #[serde(default)]
    version: Option<String>,
    #[serde(default)]
    url: Option<String>,
    #[serde(default)]
    shell: Option<String>,
    #[serde(default)]
    bin: Vec<String>,
    #[serde(default)]
    symlinks: Vec<String>,
    #[serde(default)]
    app: Option<String>,
    #[serde(default, rename = "team_id")]
    team_id: Option<String>,
    #[serde(default)]
    self_update: bool,
}

fn precedence_for(path: &Path) -> u8 {
    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or_default()
        .to_lowercase();

    const GLOBAL_STEMS: &[&str] = &["cli", "global", "base", "shared"];
    const PLATFORM_STEMS: &[&str] = &[
        "macos", "darwin", "linux", "ubuntu", "debian", "fedora", "arch", "windows", "win32",
    ];

    if GLOBAL_STEMS.contains(&stem.as_str()) {
        0
    } else if PLATFORM_STEMS.contains(&stem.as_str()) {
        1
    } else {
        2
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn load_from_dir_parses_multiple_files() {
        let temp = TempDir::new().unwrap();
        let manifest_dir = temp.path();

        let cli_manifest = r#"
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

        let mac_manifest = r#"
[ghostty]
installer = "dmg"
url = "https://ghostty.org/download"
app = "Ghostty.app"
team_id = "24VZTF6M5V"
self_update = true
"#;

        fs::write(manifest_dir.join("cli.toml"), cli_manifest).unwrap();
        fs::write(manifest_dir.join("macos.toml"), mac_manifest).unwrap();

        let manifests = ManifestSet::load_from_dir(manifest_dir).unwrap();
        assert_eq!(manifests.len(), 3);

        let mut names: Vec<_> = manifests.iter().map(|m| m.name.clone()).collect();
        names.sort();
        assert_eq!(names, vec!["fd", "ghostty", "ripgrep"]);

        let rg = manifests.iter().find(|m| m.name == "ripgrep").unwrap();
        assert_eq!(rg.definition.installer, InstallerKind::Ubi);
        assert_eq!(rg.definition.project.as_deref(), Some("BurntSushi/ripgrep"));
        assert_eq!(rg.definition.version.as_deref(), Some("14.0.0"));
        assert_eq!(rg.definition.bin, vec!["rg"]);
        assert!(rg.definition.self_update);

        let ghostty = manifests.iter().find(|m| m.name == "ghostty").unwrap();
        assert_eq!(ghostty.definition.installer, InstallerKind::Dmg);
        assert_eq!(
            ghostty.definition.url.as_deref(),
            Some("https://ghostty.org/download")
        );
        assert_eq!(ghostty.definition.app.as_deref(), Some("Ghostty.app"));
        assert_eq!(ghostty.definition.team_id.as_deref(), Some("24VZTF6M5V"));
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
        let path = temp.path().join("invalid.toml");
        fs::write(&path, "[tool]\nproject = \"example\"\n").unwrap();

        let error = ManifestSet::load_from_dir(temp.path()).unwrap_err();
        let message = error.to_string().to_lowercase();
        assert!(
            message.contains("failed to parse manifest file"),
            "{}",
            error
        );
    }

    #[test]
    fn overrides_follow_precedence() {
        let temp = TempDir::new().unwrap();
        let dir = temp.path();

        fs::write(
            dir.join("cli.toml"),
            r#"
[ripgrep]
installer = "ubi"
version = "13.0.0"
"#,
        )
        .unwrap();

        fs::write(
            dir.join("macos.toml"),
            r#"
[ripgrep]
installer = "ubi"
version = "14.0.0"
"#,
        )
        .unwrap();

        fs::write(
            dir.join("work-laptop.toml"),
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
        assert!(entry
            .source
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap()
            .contains("work-laptop"));
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
        assert_eq!(
            manifest_paths,
            vec!["cli.toml", "macos.toml", "work-laptop.toml"]
        );
    }

    #[test]
    fn same_precedence_conflict_errors() {
        let temp = TempDir::new().unwrap();
        let dir = temp.path();

        fs::write(
            dir.join("work.toml"),
            r#"
[ripgrep]
installer = "ubi"
"#,
        )
        .unwrap();

        fs::write(
            dir.join("laptop.toml"),
            r#"
[ripgrep]
installer = "ubi"
"#,
        )
        .unwrap();

        let error = ManifestSet::load_from_dir(dir).unwrap_err();
        assert!(error.to_string().contains("same precedence"), "{}", error);
    }
}
