use anyhow::{Context, Result};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use whoami::fallible;

/// Supported installer backends defined in tool specifications.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum InstallerKind {
    Dmg,
    Flatpak,
    Curl,
    Github,
    Gitlab,
    Script,
}

impl fmt::Display for InstallerKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            InstallerKind::Dmg => write!(f, "dmg"),
            InstallerKind::Flatpak => write!(f, "flatpak"),
            InstallerKind::Curl => write!(f, "curl"),
            InstallerKind::Github => write!(f, "github"),
            InstallerKind::Gitlab => write!(f, "gitlab"),
            InstallerKind::Script => write!(f, "script"),
        }
    }
}

/// Executable declared in a manifest entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolBinary {
    pub source: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub link: Option<String>,
}

/// Types of ancillary files exposed alongside binaries.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ExtraKind {
    Man,
    Completion,
    Other,
}

impl fmt::Display for ExtraKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExtraKind::Man => write!(f, "man"),
            ExtraKind::Completion => write!(f, "completion"),
            ExtraKind::Other => write!(f, "other"),
        }
    }
}

/// Additional linkables (man pages, completions, arbitrary files).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolExtra {
    pub source: String,
    pub kind: ExtraKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shell: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<String>,
}

/// Raw representation of a single tool defined in `dws.toml` (profile) or `config.toml` (workspace override).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolSpecToml {
    pub installer: InstallerKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shell: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub bin: Vec<ToolBinary>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub extras: Vec<ToolExtra>,
    #[serde(
        default,
        alias = "asset_filters",
        rename = "asset_filter",
        skip_serializing_if = "Vec::is_empty"
    )]
    pub asset_filter: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub checksum: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub app: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub team_id: Option<String>,
    #[serde(default)]
    pub self_update: bool,
    #[serde(default, rename = "platform", skip_serializing_if = "Vec::is_empty")]
    pub platforms: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub hosts: Vec<String>,
}

impl ToolSpecToml {
    fn applies_to(&self, platform_tags: &HashSet<String>, host_slug: Option<&str>) -> bool {
        let platform_ok = if self.platforms.is_empty() {
            true
        } else {
            self.platforms
                .iter()
                .any(|filter| platform_tags.contains(filter))
        };

        if !platform_ok {
            return false;
        }

        if self.hosts.is_empty() {
            return true;
        }

        let host_slug = match host_slug {
            Some(slug) => slug,
            None => return false,
        };

        self.hosts.iter().any(|candidate| candidate == host_slug)
    }

    fn into_definition(self, name: &str) -> Result<ToolDefinition> {
        Ok(ToolDefinition {
            installer: self.installer,
            project: self.project,
            version: self.version,
            url: self.url,
            shell: self.shell,
            bin: self.bin,
            extras: self.extras,
            asset_filter: self.asset_filter,
            checksum: self.checksum,
            app: self.app,
            team_id: self.team_id,
            self_update: self.self_update,
            platforms: self.platforms,
            hosts: self.hosts,
            name: name.to_string(),
        })
    }

    pub fn validate(&self, name: &str, source: &Path) -> Vec<ManifestIssue> {
        let mut issues = Vec::new();

        match self.installer {
            InstallerKind::Github | InstallerKind::Gitlab => {
                if self
                    .project
                    .as_deref()
                    .map(|value| value.trim().is_empty())
                    .unwrap_or(true)
                {
                    issues.push(ManifestIssue::tool_issue(
                        source,
                        name,
                        "field `project` is required for release installers",
                    ));
                }

                if self.asset_filter.is_empty() {
                    issues.push(ManifestIssue::tool_issue(
                        source,
                        name,
                        "at least one `asset_filter` regex must be defined",
                    ));
                }
            }
            InstallerKind::Script => {
                if self
                    .url
                    .as_deref()
                    .map(|value| value.trim().is_empty())
                    .unwrap_or(true)
                {
                    issues.push(ManifestIssue::tool_issue(
                        source,
                        name,
                        "field `url` is required for script installers",
                    ));
                }

                if self
                    .shell
                    .as_deref()
                    .map(|value| value.trim().is_empty())
                    .unwrap_or(true)
                {
                    issues.push(ManifestIssue::tool_issue(
                        source,
                        name,
                        "field `shell` is required for script installers",
                    ));
                }
            }
            _ => {}
        }

        let checksum_required = matches!(
            self.installer,
            InstallerKind::Github | InstallerKind::Gitlab | InstallerKind::Script
        );

        match self.checksum.as_deref() {
            Some(value) if !value.trim().is_empty() => {
                if !is_valid_checksum(value) {
                    issues.push(ManifestIssue::tool_issue(
                        source,
                        name,
                        "checksum must be formatted as `sha256:<64 hex characters>`",
                    ));
                }
            }
            Some(_) => {
                issues.push(ManifestIssue::tool_issue(
                    source,
                    name,
                    "checksum must not be empty",
                ));
            }
            None if checksum_required => {
                issues.push(ManifestIssue::tool_issue(
                    source,
                    name,
                    "checksum is required for this installer",
                ));
            }
            None => {}
        }

        for pattern in &self.asset_filter {
            if let Err(err) = Regex::new(pattern) {
                issues.push(ManifestIssue::tool_issue(
                    source,
                    name,
                    format!("invalid asset_filter regex `{pattern}`: {err}"),
                ));
            }
        }

        if self.bin.is_empty() {
            issues.push(ManifestIssue::tool_issue(
                source,
                name,
                "declare at least one [[tools.<name>.bin]] entry",
            ));
        }

        for (idx, bin) in self.bin.iter().enumerate() {
            if bin.source.trim().is_empty() {
                issues.push(ManifestIssue::tool_issue(
                    source,
                    name,
                    format!("bin entry #{idx} must specify a non-empty `source`"),
                ));
            }

            if let Some(link) = bin.link.as_deref() {
                if link.trim().is_empty() {
                    issues.push(ManifestIssue::tool_issue(
                        source,
                        name,
                        format!("bin entry #{idx} has an empty `link` value"),
                    ));
                }
            }
        }

        for (idx, extra) in self.extras.iter().enumerate() {
            if extra.source.trim().is_empty() {
                issues.push(ManifestIssue::tool_issue(
                    source,
                    name,
                    format!("extras entry #{idx} must specify a non-empty `source`"),
                ));
            }

            if let Some(target) = extra.target.as_deref() {
                if target.trim().is_empty() {
                    issues.push(ManifestIssue::tool_issue(
                        source,
                        name,
                        format!("extras entry #{idx} has an empty `target` value"),
                    ));
                }
            }

            if matches!(extra.kind, ExtraKind::Completion)
                && extra
                    .shell
                    .as_deref()
                    .map(|value| value.trim().is_empty())
                    .unwrap_or(true)
            {
                issues.push(ManifestIssue::tool_issue(
                    source,
                    name,
                    format!("extras entry #{idx} (kind=\"completion\") requires a `shell` value"),
                ));
            }
        }

        issues
    }
}

/// Complete representation of a tool manifest file (`dws.toml` or `config.toml`).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ToolConfigFile {
    #[serde(default)]
    pub active_profile: Option<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub tools: BTreeMap<String, ToolSpecToml>,
    #[serde(flatten)]
    pub extras: BTreeMap<String, toml::Value>,
}

impl ToolConfigFile {
    pub fn load(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }

        let contents = fs::read_to_string(path)
            .with_context(|| format!("Failed to read config file {:?}", path))?;

        if contents.trim().is_empty() {
            return Ok(Self::default());
        }

        toml::from_str(&contents).with_context(|| format!("Failed to parse config file {:?}", path))
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create directory {:?}", parent))?;
        }

        let contents =
            toml::to_string_pretty(self).context("Failed to serialize dws configuration")?;
        fs::write(path, contents)
            .with_context(|| format!("Failed to write config file {:?}", path))?;
        Ok(())
    }
}

pub fn validate_tool_config(path: &Path, config: &ToolConfigFile) -> Vec<ManifestIssue> {
    let mut issues = Vec::new();
    for (name, spec) in &config.tools {
        issues.extend(spec.validate(name, path));
    }
    issues
}

/// Fully-resolved, filtered tool definition ready for installation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolDefinition {
    pub name: String,
    pub installer: InstallerKind,
    pub project: Option<String>,
    pub version: Option<String>,
    pub url: Option<String>,
    pub shell: Option<String>,
    pub bin: Vec<ToolBinary>,
    pub extras: Vec<ToolExtra>,
    pub asset_filter: Vec<String>,
    pub checksum: Option<String>,
    pub app: Option<String>,
    pub team_id: Option<String>,
    pub self_update: bool,
    pub platforms: Vec<String>,
    pub hosts: Vec<String>,
}

/// Validation issue surfaced while checking manifest files.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManifestIssue {
    pub source: PathBuf,
    pub tool: Option<String>,
    pub message: String,
}

impl ManifestIssue {
    pub fn tool_issue(source: &Path, tool: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            source: source.to_path_buf(),
            tool: Some(tool.into()),
            message: message.into(),
        }
    }

    pub fn general(source: &Path, message: impl Into<String>) -> Self {
        Self {
            source: source.to_path_buf(),
            tool: None,
            message: message.into(),
        }
    }
}

/// An entry combines the tool name, the source file where it was defined, and its definition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolEntry {
    pub source: PathBuf,
    pub definition: ToolDefinition,
}

/// Collection of tools discovered after applying platform/host filters and overrides.
#[derive(Debug, Default)]
pub struct ToolSet {
    entries: BTreeMap<String, ToolEntry>,
}

impl ToolSet {
    /// Load resolved tool definitions from the profile and workspace configuration files.
    pub fn load(profile_root: &Path, workspace_config_path: &Path) -> Result<Self> {
        let profile_file = profile_root.join("dws.toml");
        let profile_config = ToolConfigFile::load(&profile_file)?;
        let workspace_config = ToolConfigFile::load(workspace_config_path)?;

        Self::from_configs(
            profile_file,
            profile_config,
            workspace_config_path,
            workspace_config,
        )
    }

    fn from_configs(
        profile_source: PathBuf,
        mut profile_config: ToolConfigFile,
        workspace_config_path: &Path,
        mut workspace_config: ToolConfigFile,
    ) -> Result<Self> {
        let platform_tags = platform_tags();
        let host_slug = host_slug();

        // Lowercase filters once so we can compare cheaply.
        normalize_tool_filters(&mut profile_config.tools);
        normalize_tool_filters(&mut workspace_config.tools);

        let mut entries = BTreeMap::new();

        for (name, spec) in profile_config.tools {
            if spec.applies_to(&platform_tags, host_slug.as_deref()) {
                let definition = spec.into_definition(&name)?;
                entries.insert(
                    name.clone(),
                    ToolEntry {
                        source: profile_source.clone(),
                        definition,
                    },
                );
            }
        }

        let workspace_source = workspace_config_path.to_path_buf();

        for (name, spec) in workspace_config.tools {
            if spec.applies_to(&platform_tags, host_slug.as_deref()) {
                let definition = spec.into_definition(&name)?;
                entries.insert(
                    name.clone(),
                    ToolEntry {
                        source: workspace_source.clone(),
                        definition,
                    },
                );
            }
        }

        Ok(Self { entries })
    }

    /// Iterate over resolved tool entries.
    pub fn iter(&self) -> impl Iterator<Item = (&String, &ToolEntry)> {
        self.entries.iter()
    }

    /// Returns the total number of resolved tool definitions.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether any tool entries were discovered.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Returns a reference to the internal entry map.
    pub fn entries(&self) -> &BTreeMap<String, ToolEntry> {
        &self.entries
    }
}

fn normalize_tool_filters(map: &mut BTreeMap<String, ToolSpecToml>) {
    for spec in map.values_mut() {
        spec.platforms = spec
            .platforms
            .iter()
            .map(|value| normalize_filter(value))
            .collect();
        spec.hosts = spec
            .hosts
            .iter()
            .map(|value| normalize_filter(value))
            .collect();
    }
}

fn normalize_filter(value: &str) -> String {
    value.trim().to_ascii_lowercase()
}

fn is_valid_checksum(value: &str) -> bool {
    let trimmed = value.trim();
    if let Some(rest) = trimmed.strip_prefix("sha256:") {
        rest.len() == 64 && rest.chars().all(|ch| ch.is_ascii_hexdigit())
    } else {
        false
    }
}

fn platform_tags() -> HashSet<String> {
    use std::env;

    let mut tags = HashSet::new();
    let os = normalize_filter(env::consts::OS);
    tags.insert(os.clone());
    tags.insert(format!("{}-{}", os, normalize_filter(env::consts::ARCH)));

    if os == "linux" {
        for distro in linux_distribution_tags() {
            tags.insert(format!("linux-{}", distro));
        }
    }

    if os == "macos" {
        // Provide convenient alias matching the historical manifest slug.
        tags.insert("darwin".to_string());
    }

    tags
}

fn linux_distribution_tags() -> Vec<String> {
    let mut tags = Vec::new();
    if let Ok(contents) = fs::read_to_string("/etc/os-release") {
        let mut values = HashMap::new();
        for line in contents.lines() {
            if let Some((key, value)) = line.split_once('=') {
                let trimmed = value.trim_matches('"').trim().to_ascii_lowercase();
                values.insert(key.to_ascii_lowercase(), trimmed.to_string());
            }
        }

        if let Some(id) = values.get("id") {
            tags.push(id.clone());
        }

        if let Some(id_like) = values.get("id_like") {
            for item in id_like
                .split(|c: char| c.is_ascii_whitespace() || c == ',')
                .map(|part| part.trim())
                .filter(|part| !part.is_empty())
            {
                tags.push(item.to_ascii_lowercase());
            }
        }
    }

    tags
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
    use std::path::Path;
    use tempfile::TempDir;

    fn setup_profile(temp: &TempDir, contents: &str) -> PathBuf {
        let profile_root = temp.path().join("profiles").join("default");
        fs::create_dir_all(&profile_root).unwrap();
        fs::write(profile_root.join("dws.toml"), contents).unwrap();
        profile_root
    }

    fn write_workspace_config(temp: &TempDir, contents: &str) -> PathBuf {
        let workspace_config = temp.path().join("config.toml");
        fs::write(&workspace_config, contents).unwrap();
        workspace_config
    }

    #[test]
    fn load_profile_tools() {
        let temp = TempDir::new().unwrap();
        let profile_root = setup_profile(
            &temp,
            r#"
[tools.ripgrep]
installer = "curl"
project = "BurntSushi/ripgrep"
self_update = true

[[tools.ripgrep.bin]]
source = "rg"
"#,
        );

        let workspace_config = write_workspace_config(&temp, "");

        let tools = ToolSet::load(&profile_root, &workspace_config).unwrap();
        assert_eq!(tools.len(), 1);
        let entry = tools.iter().next().unwrap().1;
        assert_eq!(entry.definition.name, "ripgrep");
        assert_eq!(entry.definition.installer, InstallerKind::Curl);
        assert_eq!(
            entry.definition.bin,
            vec![ToolBinary {
                source: "rg".to_string(),
                link: None
            }]
        );
        assert!(entry.definition.self_update);
    }

    #[test]
    fn workspace_overrides_profile_tool() {
        let temp = TempDir::new().unwrap();
        let profile_root = setup_profile(
            &temp,
            r#"
[tools.ripgrep]
installer = "curl"
project = "BurntSushi/ripgrep"
version = "14.0.0"

[[tools.ripgrep.bin]]
source = "rg"
"#,
        );

        let workspace_config = write_workspace_config(
            &temp,
            r#"
[tools.ripgrep]
installer = "curl"
project = "BurntSushi/ripgrep"
version = "latest"

[[tools.ripgrep.bin]]
source = "rg"
"#,
        );

        let tools = ToolSet::load(&profile_root, &workspace_config).unwrap();
        assert_eq!(tools.len(), 1);
        let entry = tools.iter().next().unwrap().1;
        assert_eq!(entry.definition.version.as_deref(), Some("latest"));
        assert_eq!(
            entry.definition.bin,
            vec![ToolBinary {
                source: "rg".to_string(),
                link: None
            }]
        );
        assert_eq!(entry.source, workspace_config);
    }

    #[test]
    fn platform_filters_are_respected() {
        let temp = TempDir::new().unwrap();
        let current = normalize_filter(std::env::consts::OS);
        let profile_root = setup_profile(
            &temp,
            &format!(
                r#"
[tools.match]
installer = "curl"
platform = ["{current}"]

[tools.skip]
installer = "curl"
platform = ["totally-different"]
"#
            ),
        );
        let workspace_config = write_workspace_config(&temp, "");

        let tools = ToolSet::load(&profile_root, &workspace_config).unwrap();
        assert_eq!(tools.len(), 1);
        assert_eq!(
            tools.iter().next().unwrap().1.definition.name,
            "match".to_string()
        );
    }

    #[test]
    fn workspace_override_must_match_filters() {
        let temp = TempDir::new().unwrap();
        let profile_root = setup_profile(
            &temp,
            r#"
[tools.ripgrep]
installer = "curl"
project = "BurntSushi/ripgrep"
"#,
        );

        let workspace_config = write_workspace_config(
            &temp,
            r#"
[tools.ripgrep]
installer = "curl"
project = "BurntSushi/ripgrep"
platform = ["does-not-match"]
"#,
        );

        let tools = ToolSet::load(&profile_root, &workspace_config).unwrap();
        assert_eq!(tools.len(), 1);
        let entry = tools.iter().next().unwrap().1;
        assert_eq!(entry.source, profile_root.join("dws.toml"));
        assert_eq!(entry.definition.version, None);
    }

    #[test]
    fn host_filters_are_respected() {
        let temp = TempDir::new().unwrap();
        let host_slug = super::host_slug().unwrap();
        let profile_root = setup_profile(
            &temp,
            &format!(
                r#"
[tools.match]
installer = "curl"
hosts = ["{host_slug}"]

[tools.skip]
installer = "curl"
hosts = ["someone-else"]
"#
            ),
        );
        let workspace_config = write_workspace_config(&temp, "");

        let tools = ToolSet::load(&profile_root, &workspace_config).unwrap();
        assert_eq!(tools.len(), 1);
        assert_eq!(
            tools.iter().next().unwrap().1.definition.name,
            "match".to_string()
        );
    }

    #[test]
    fn extra_keys_are_preserved_on_roundtrip() {
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("config.toml");
        let original = r#"
active_profile = "work"

[tools.ripgrep]
installer = "curl"

[extras]
custom = "value"
"#;
        fs::write(&path, original).unwrap();

        let mut parsed = ToolConfigFile::load(&path).unwrap();
        parsed.active_profile = Some("personal".to_string());
        parsed.save(&path).unwrap();

        let contents = fs::read_to_string(&path).unwrap();
        assert!(contents.contains("[extras]"));
        assert!(contents.contains("custom = \"value\""));
        assert!(contents.contains("active_profile = \"personal\""));
    }

    #[test]
    fn validate_flags_missing_checksum() {
        let manifest = r#"
[tools.sample]
installer = "github"
project = "owner/sample"
asset_filter = ["^sample$"]

[[tools.sample.bin]]
source = "sample"
"#;
        let config: ToolConfigFile = toml::from_str(manifest).unwrap();
        let spec = config.tools.get("sample").unwrap();
        let issues = spec.validate("sample", Path::new("dws.toml"));
        assert!(issues
            .iter()
            .any(|issue| issue.message.contains("checksum is required")));
    }

    #[test]
    fn validate_completion_requires_shell() {
        let manifest = r#"
[tools.sample]
installer = "github"
project = "owner/sample"
asset_filter = ["^sample$"]
checksum = "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"

[[tools.sample.bin]]
source = "sample"

[[tools.sample.extras]]
source = "complete/_sample"
kind = "completion"
"#;
        let config: ToolConfigFile = toml::from_str(manifest).unwrap();
        let spec = config.tools.get("sample").unwrap();
        let issues = spec.validate("sample", Path::new("dws.toml"));
        assert!(issues
            .iter()
            .any(|issue| issue.message.contains("requires a `shell` value")));
    }
}
