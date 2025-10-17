use crate::lockfile::Lockfile;
use crate::toolset::{InstallerKind, ToolBinary, ToolDefinition};
use anyhow::{Context, Result};
use std::path::PathBuf;
use tokio::runtime::Runtime;

mod github;
use github::GithubClient;

// Phase 0 refactor: removed external `ubi` installer backend.
// Placeholder: future github/gitlab/script modules will be added here.

#[derive(Clone, Debug)]
#[allow(dead_code)]
pub(crate) struct InstallContext {
    pub cache_tools_dir: PathBuf,
    pub bin_dir: PathBuf,
}

pub(crate) trait ToolInstaller {
    fn requires_runtime(&self) -> bool;
    fn install(&self, runtime: Option<&mut Runtime>, lockfile: &mut Lockfile) -> Result<()>;
}

pub(crate) struct InstallerDispatch {
    pub installer: Box<dyn ToolInstaller>,
    pub resolved_version: Option<String>,
}

struct GithubInstaller {
    name: String,
    project: String,
    version: Option<String>,
    bins: Vec<ToolBinary>,
    asset_filters: Vec<String>,
    client: GithubClient,
}

impl GithubInstaller {
    fn new(def: &ToolDefinition) -> Result<Self> {
        let project = def
            .project
            .clone()
            .context("GitHub installer requires a `project` field")?;

        let client = GithubClient::from_env()?;

        Ok(Self {
            name: def.name.clone(),
            project,
            version: def.version.clone(),
            bins: def.bin.clone(),
            asset_filters: def.asset_filter.clone(),
            client,
        })
    }
}

impl ToolInstaller for GithubInstaller {
    fn requires_runtime(&self) -> bool {
        // Future async metadata/download will require a runtime.
        false
    }
    fn install(&self, _runtime: Option<&mut Runtime>, lockfile: &mut Lockfile) -> Result<()> {
        let release = self
            .client
            .fetch_release(&self.project, self.version.as_deref())?;

        let selected = release.select_asset(&self.asset_filters).with_context(|| {
            format!(
                "Failed to select asset for tool '{}' using patterns {:?}",
                self.name, self.asset_filters
            )
        })?;

        let manifest_version = self.version.clone().unwrap_or_else(|| "latest".to_string());
        let resolved_version = release.tag_name.clone();

        // Phase 2 placeholder: future steps will select assets and install binaries.
        let _requested_bins = &self.bins;
        let _asset_name = &selected.asset.name;

        lockfile.record_tool_install(
            &self.name,
            &manifest_version,
            &resolved_version,
            "github",
            vec![],
        );
        Ok(())
    }
}

pub(crate) fn create_installer(
    definition: &ToolDefinition,
    _context: InstallContext,
) -> Result<Option<InstallerDispatch>> {
    match definition.installer {
        InstallerKind::Github => {
            let installer = GithubInstaller::new(definition)?;
            Ok(Some(InstallerDispatch {
                resolved_version: installer.version.clone(),
                installer: Box::new(installer),
            }))
        }
        _ => Ok(None),
    }
}

#[cfg_attr(not(test), allow(dead_code))]
pub(crate) fn sanitize_component(value: &str) -> String {
    let mut result = String::with_capacity(value.len());
    for ch in value.chars() {
        match ch {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '.' | '-' | '_' => result.push(ch),
            _ => result.push('-'),
        }
    }

    if result.trim_matches('-').is_empty() {
        "default".to_string()
    } else {
        result
    }
}

#[cfg(test)]
mod tests {
    use super::{create_installer, sanitize_component, InstallContext};
    use crate::toolset::{InstallerKind, ToolBinary, ToolDefinition};
    use std::path::PathBuf;

    #[test]
    fn test_sanitize_component() {
        assert_eq!(sanitize_component("hello-world"), "hello-world");
        assert_eq!(sanitize_component("Hello World!"), "Hello-World-");
        assert_eq!(sanitize_component(""), "default");
        assert_eq!(sanitize_component("///"), "default");
        assert_eq!(sanitize_component("v1.2.3"), "v1.2.3");
    }

    fn sample_definition(installer: InstallerKind, bins: Vec<String>) -> ToolDefinition {
        ToolDefinition {
            name: "tool".to_string(),
            installer,
            project: Some("owner/project".to_string()),
            version: Some("1.0.0".to_string()),
            url: None,
            shell: None,
            bin: if bins.is_empty() {
                vec![ToolBinary {
                    source: "tool".to_string(),
                    link: None,
                }]
            } else {
                bins.into_iter()
                    .map(|name| ToolBinary {
                        source: name,
                        link: None,
                    })
                    .collect()
            },
            extras: Vec::new(),
            asset_filter: Vec::new(),
            checksum: None,
            app: None,
            team_id: None,
            self_update: false,
            platforms: Vec::new(),
            hosts: Vec::new(),
        }
    }

    fn default_context() -> InstallContext {
        InstallContext {
            cache_tools_dir: PathBuf::from("/tmp/cache/tools"),
            bin_dir: PathBuf::from("/tmp/state/bin"),
        }
    }

    #[test]
    fn test_create_installer_dispatch() {
        let cases = [
            (InstallerKind::Curl, false),
            (InstallerKind::Dmg, false),
            (InstallerKind::Flatpak, false),
            (InstallerKind::Github, true),
            (InstallerKind::Gitlab, false),
            (InstallerKind::Script, false),
        ];
        for (kind, expected_some) in cases {
            let definition = sample_definition(kind, vec!["tool".to_string()]);
            let context = default_context();
            let result = create_installer(&definition, context).unwrap();
            assert_eq!(result.is_some(), expected_some);
            if let Some(dispatch) = result {
                assert_eq!(dispatch.resolved_version.as_deref(), Some("1.0.0"));
            }
        }
    }

    #[test]
    fn test_create_installer_defaults_missing_bin() {
        let mut definition = sample_definition(InstallerKind::Curl, Vec::new());
        definition.name = "precious".to_string();
        definition.project = Some("houseabsolute/precious".to_string());

        let context = default_context();
        let installer = create_installer(&definition, context).unwrap();
        assert!(installer.is_none());
    }
}
