use crate::lockfile::Lockfile;
use crate::manifest::{InstallerKind, ManifestEntry};
use anyhow::Result;
use std::path::PathBuf;
use tokio::runtime::Runtime;

mod ubi;

pub(crate) use ubi::UbiInstaller;

#[derive(Clone, Debug)]
pub(crate) struct InstallContext {
    pub cache_apps_dir: PathBuf,
    pub bin_dir: PathBuf,
}

pub(crate) trait ToolInstaller {
    fn requires_runtime(&self) -> bool;
    fn install(&self, runtime: Option<&mut Runtime>, lockfile: &mut Lockfile) -> Result<()>;
}

pub(crate) fn create_installer(
    entry: &ManifestEntry,
    context: InstallContext,
) -> Result<Option<Box<dyn ToolInstaller>>> {
    match entry.definition.installer {
        InstallerKind::Ubi => Ok(Some(Box::new(UbiInstaller::new(entry.clone(), context)?))),
        _ => Ok(None),
    }
}

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
    use crate::manifest::{InstallerKind, ManifestEntry, ToolDefinition};
    use rstest::rstest;

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
            installer,
            project: Some("owner/project".to_string()),
            version: Some("1.0.0".to_string()),
            url: None,
            shell: None,
            bin: if bins.is_empty() {
                vec!["tool".to_string()]
            } else {
                bins
            },
            symlinks: Vec::new(),
            app: None,
            team_id: None,
            self_update: false,
        }
    }

    fn sample_entry(installer: InstallerKind) -> ManifestEntry {
        ManifestEntry {
            name: "tool".to_string(),
            source: "manifests/tools.toml".into(),
            precedence: 0,
            definition: sample_definition(installer, vec!["tool".to_string()]),
        }
    }

    fn default_context() -> InstallContext {
        InstallContext {
            cache_apps_dir: PathBuf::from("/tmp/cache/apps"),
            bin_dir: PathBuf::from("/tmp/state/bin"),
        }
    }

    use std::path::PathBuf;

    #[rstest]
    #[case(InstallerKind::Ubi, true)]
    #[case(InstallerKind::Curl, false)]
    #[case(InstallerKind::Dmg, false)]
    #[case(InstallerKind::Flatpak, false)]
    fn test_create_installer_dispatch(#[case] kind: InstallerKind, #[case] expected_some: bool) {
        let entry = sample_entry(kind);
        let context = default_context();

        let result = create_installer(&entry, context).unwrap();
        assert_eq!(result.is_some(), expected_some);
    }

    #[test]
    fn test_create_installer_defaults_missing_bin() {
        let mut entry = ManifestEntry {
            name: "precious".to_string(),
            source: "manifests/tools.toml".into(),
            precedence: 0,
            definition: sample_definition(InstallerKind::Ubi, Vec::new()),
        };
        entry.definition.project = Some("houseabsolute/precious".to_string());

        let context = default_context();
        let installer = create_installer(&entry, context).unwrap();
        assert!(installer.is_some());
    }
}
