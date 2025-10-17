use crate::lockfile::{AssetRecord, BinaryLink, ExtraLink, Lockfile};
use crate::toolset::{InstallerKind, ToolBinary, ToolDefinition, ToolExtra};
use anyhow::{bail, Context, Result};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::runtime::Runtime;

mod github;
pub(crate) use self::github::GithubApi;

pub(crate) fn default_github_api() -> Result<Arc<dyn GithubApi>> {
    github::default_api()
}

// Phase 0 refactor: removed external `ubi` installer backend.
// Placeholder: future github/gitlab/script modules will be added here.

#[derive(Clone)]
#[allow(dead_code)]
pub(crate) struct InstallContext {
    pub cache_tools_dir: PathBuf,
    pub bin_dir: PathBuf,
    pub share_dir: PathBuf,
    pub github_api: Arc<dyn GithubApi>,
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
    extras: Vec<ToolExtra>,
    asset_filters: Vec<String>,
    checksum: [u8; 32],
    context: InstallContext,
}

impl GithubInstaller {
    fn new(def: &ToolDefinition, context: InstallContext) -> Result<Self> {
        let project = def
            .project
            .clone()
            .context("GitHub installer requires a `project` field")?;

        let checksum_value = def
            .checksum
            .clone()
            .context("GitHub installer requires a `checksum` field")?;

        let checksum = github::parse_sha256(&checksum_value)?;

        if def.asset_filter.is_empty() {
            bail!(
                "GitHub installer requires at least one asset_filter pattern (tool '{}')",
                def.name
            );
        }

        Ok(Self {
            name: def.name.clone(),
            project,
            version: def.version.clone(),
            bins: def.bin.clone(),
            extras: def.extras.clone(),
            asset_filters: def.asset_filter.clone(),
            checksum,
            context,
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
            .context
            .github_api
            .fetch_release(&self.project, self.version.as_deref())?;

        let selected = release.select_asset(&self.asset_filters).with_context(|| {
            format!(
                "Failed to select asset for tool '{}' using patterns {:?}",
                self.name, self.asset_filters
            )
        })?;

        let manifest_version = self.version.clone().unwrap_or_else(|| "latest".to_string());
        let resolved_version = release.tag_name.clone();

        let tool_slug = sanitize_component(&self.name);
        let version_slug = sanitize_component(&resolved_version);
        let version_dir = self
            .context
            .cache_tools_dir
            .join(&tool_slug)
            .join(&version_slug);
        fs::create_dir_all(&version_dir).with_context(|| {
            format!(
                "Failed to create cache directory for tool '{}' at {:?}",
                self.name, version_dir
            )
        })?;

        let asset_path = version_dir.join(&selected.asset.name);

        let mut digest = if asset_path.exists() {
            github::compute_sha256(&asset_path)?
        } else {
            self.context
                .github_api
                .download_asset(&selected.asset.browser_download_url, &asset_path)?
        };

        if digest != self.checksum {
            if asset_path.exists() {
                fs::remove_file(&asset_path).with_context(|| {
                    format!(
                        "Failed to remove asset with invalid checksum at {:?}",
                        asset_path
                    )
                })?;
            }

            digest = self
                .context
                .github_api
                .download_asset(&selected.asset.browser_download_url, &asset_path)?;

            if digest != self.checksum {
                bail!(
                    "Checksum mismatch for asset '{}': expected {}, got {}",
                    selected.asset.name,
                    github::format_digest(&self.checksum),
                    github::format_digest(&digest)
                );
            }
        }

        let extract_dir = version_dir.join("contents");
        if extract_dir.exists() {
            fs::remove_dir_all(&extract_dir).with_context(|| {
                format!(
                    "Failed to clear previous extraction directory for tool '{}' at {:?}",
                    self.name, extract_dir
                )
            })?;
        }

        fs::create_dir_all(&extract_dir).with_context(|| {
            format!(
                "Failed to create extraction directory for tool '{}' at {:?}",
                self.name, extract_dir
            )
        })?;

        github::extract_archive(&asset_path, &extract_dir).with_context(|| {
            format!(
                "Failed to extract asset '{}' for tool '{}'",
                selected.asset.name, self.name
            )
        })?;

        let mut binary_links = Vec::new();
        for bin in &self.bins {
            let source_path =
                github::resolve_binary_path(&extract_dir, &bin.source).with_context(|| {
                    format!(
                        "Failed to locate binary '{}' within archive for tool '{}'",
                        bin.source, self.name
                    )
                })?;

            let link_name = bin
                .link
                .clone()
                .or_else(|| {
                    source_path
                        .file_name()
                        .map(|value| value.to_string_lossy().to_string())
                })
                .with_context(|| {
                    format!(
                        "Unable to determine link name for binary '{}' in tool '{}'",
                        bin.source, self.name
                    )
                })?;

            let target_path = self.context.bin_dir.join(&link_name);

            if target_path.exists() || target_path.symlink_metadata().is_ok() {
                fs::remove_file(&target_path).with_context(|| {
                    format!("Failed to remove existing binary at {:?}", target_path)
                })?;
            }

            create_symlink(&source_path, &target_path)?;

            binary_links.push(BinaryLink {
                link: link_name,
                source: source_path,
                target: target_path,
            });
        }

        let mut extra_links = Vec::new();
        for extra in &self.extras {
            let resolved_source =
                github::resolve_extra_path(&extract_dir, extra).with_context(|| {
                    format!(
                        "Failed to locate extra '{}' (kind='{}') for tool '{}'",
                        extra.source, extra.kind, self.name
                    )
                })?;

            let target_path =
                github::resolve_extra_target(&self.context, &tool_slug, extra, &resolved_source)?;

            if target_path.exists() || target_path.symlink_metadata().is_ok() {
                fs::remove_file(&target_path).with_context(|| {
                    format!("Failed to remove existing extra at {:?}", target_path)
                })?;
            }

            create_symlink(&resolved_source, &target_path)?;

            extra_links.push(ExtraLink {
                kind: extra.kind.to_string(),
                source: resolved_source,
                target: target_path,
            });
        }

        lockfile.record_tool_install(
            &self.name,
            &manifest_version,
            &resolved_version,
            "github",
            binary_links,
            extra_links,
            Some(AssetRecord {
                name: selected.asset.name.clone(),
                url: selected.asset.browser_download_url.clone(),
                checksum: github::format_digest(&digest),
                archive_path: asset_path,
                extract_dir,
                pattern_index: Some(selected.pattern_index),
                pattern: Some(selected.pattern.to_string()),
            }),
        );
        Ok(())
    }
}

pub(crate) fn create_installer(
    definition: &ToolDefinition,
    context: InstallContext,
) -> Result<Option<InstallerDispatch>> {
    match definition.installer {
        InstallerKind::Github => {
            let installer = GithubInstaller::new(definition, context)?;
            Ok(Some(InstallerDispatch {
                resolved_version: installer.version.clone(),
                installer: Box::new(installer),
            }))
        }
        _ => Ok(None),
    }
}

fn create_symlink(source: &Path, target: &Path) -> Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::symlink;
        symlink(source, target).with_context(|| {
            format!("Failed to create symlink from {:?} to {:?}", target, source)
        })?;
    }

    #[cfg(windows)]
    {
        use std::os::windows::fs::symlink_file;
        if let Err(err) = symlink_file(source, target) {
            // Fall back to copying when symlinks are not permitted.
            std::fs::copy(source, target).with_context(|| {
                format!(
                    "Failed to create symlink (and copy fallback) from {:?} to {:?}: {}",
                    target, source, err
                )
            })?;
        }
    }

    Ok(())
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
    use super::github::{GithubAsset, GithubRelease};
    use super::{create_installer, sanitize_component, GithubApi, InstallContext};
    use crate::lockfile::Lockfile;
    use crate::toolset::{InstallerKind, ToolBinary, ToolDefinition};
    use anyhow::{Context as AnyhowContext, Result as TestResult};
    use flate2::write::GzEncoder;
    use flate2::Compression;
    use sha2::{Digest as ShaDigestTrait, Sha256};
    use std::fs;
    use std::io::Cursor;
    use std::path::{Path, PathBuf};
    use std::sync::Arc;
    use tar::{Builder, Header};
    use tempfile::TempDir;

    #[test]
    fn test_sanitize_component() {
        assert_eq!(sanitize_component("hello-world"), "hello-world");
        assert_eq!(sanitize_component("Hello World!"), "Hello-World-");
        assert_eq!(sanitize_component(""), "default");
        assert_eq!(sanitize_component("///"), "default");
        assert_eq!(sanitize_component("v1.2.3"), "v1.2.3");
    }

    fn sample_definition(installer: InstallerKind, bins: Vec<String>) -> ToolDefinition {
        let checksum = if installer == InstallerKind::Github {
            Some(
                "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
                    .to_string(),
            )
        } else {
            None
        };

        let asset_filter = if installer == InstallerKind::Github {
            vec!["tool".to_string()]
        } else {
            Vec::new()
        };

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
            asset_filter,
            checksum,
            app: None,
            team_id: None,
            self_update: false,
            platforms: Vec::new(),
            hosts: Vec::new(),
        }
    }

    struct NoopGithubApi;

    impl GithubApi for NoopGithubApi {
        fn fetch_release(
            &self,
            _project: &str,
            _tag: Option<&str>,
        ) -> anyhow::Result<GithubRelease> {
            unreachable!("fetch_release should not be called in this test")
        }

        fn download_asset(&self, _url: &str, _dest: &Path) -> anyhow::Result<[u8; 32]> {
            unreachable!("download_asset should not be called in this test")
        }
    }

    fn default_context() -> InstallContext {
        InstallContext {
            cache_tools_dir: PathBuf::from("/tmp/cache/tools"),
            bin_dir: PathBuf::from("/tmp/state/bin"),
            share_dir: PathBuf::from("/tmp/state/share"),
            github_api: Arc::new(NoopGithubApi),
        }
    }

    #[derive(Clone)]
    struct MockGithubApi {
        release: GithubRelease,
        asset_bytes: Vec<u8>,
        digest: [u8; 32],
    }

    impl GithubApi for MockGithubApi {
        fn fetch_release(
            &self,
            _project: &str,
            _tag: Option<&str>,
        ) -> anyhow::Result<GithubRelease> {
            Ok(self.release.clone())
        }

        fn download_asset(&self, _url: &str, dest: &Path) -> anyhow::Result<[u8; 32]> {
            if let Some(parent) = dest.parent() {
                fs::create_dir_all(parent).with_context(|| {
                    format!("Failed to create asset parent directory {:?}", parent)
                })?;
            }
            fs::write(dest, &self.asset_bytes)
                .with_context(|| format!("Failed to write mock asset to {:?}", dest))?;
            Ok(self.digest)
        }
    }

    fn build_tar_gz(entries: &[(&str, &[u8])]) -> TestResult<Vec<u8>> {
        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        {
            let mut builder = Builder::new(&mut encoder);
            for (path, data) in entries {
                let mut header = Header::new_gnu();
                header.set_path(path)?;
                header.set_size(data.len() as u64);
                header.set_mode(0o755);
                header.set_cksum();
                builder.append_data(&mut header, *path, &mut Cursor::new(*data))?;
            }
            builder.finish()?;
        }
        Ok(encoder.finish()?)
    }

    fn mock_release(asset_name: &str, download_url: &str, size: u64) -> GithubRelease {
        GithubRelease {
            id: 42,
            tag_name: "v1.0.0".to_string(),
            name: Some("Mock Release".to_string()),
            draft: false,
            prerelease: false,
            assets: vec![GithubAsset {
                id: 99,
                name: asset_name.to_string(),
                content_type: Some("application/gzip".to_string()),
                browser_download_url: download_url.to_string(),
                size,
                state: Some("uploaded".to_string()),
            }],
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

    #[test]
    fn github_installer_installs_and_records_asset() -> TestResult<()> {
        let temp = TempDir::new()?;
        let cache_tools_dir = temp.path().join("cache/tools");
        let bin_dir = temp.path().join("state/bin");
        let share_dir = temp.path().join("state/share");
        fs::create_dir_all(&cache_tools_dir)?;
        fs::create_dir_all(&bin_dir)?;
        fs::create_dir_all(&share_dir)?;

        let asset_bytes = build_tar_gz(&[("tool", b"#!/bin/sh\necho hi\n")])?;
        let digest_array = {
            let mut hasher = Sha256::new();
            hasher.update(&asset_bytes);
            hasher.finalize().into()
        };
        let checksum = format!("sha256:{}", hex::encode(digest_array));

        let release = mock_release(
            "tool.tar.gz",
            "https://example.com/tool.tar.gz",
            asset_bytes.len() as u64,
        );
        let github_api = MockGithubApi {
            release,
            asset_bytes,
            digest: digest_array,
        };

        let context = InstallContext {
            cache_tools_dir: cache_tools_dir.clone(),
            bin_dir: bin_dir.clone(),
            share_dir: share_dir.clone(),
            github_api: Arc::new(github_api),
        };

        let definition = ToolDefinition {
            name: "mock".to_string(),
            installer: InstallerKind::Github,
            project: Some("owner/mock".to_string()),
            version: Some("v1.0.0".to_string()),
            url: None,
            shell: None,
            bin: vec![ToolBinary {
                source: "tool".to_string(),
                link: None,
            }],
            extras: Vec::new(),
            asset_filter: vec!["tool".to_string()],
            checksum: Some(checksum),
            app: None,
            team_id: None,
            self_update: false,
            platforms: Vec::new(),
            hosts: Vec::new(),
        };

        let mut lockfile = Lockfile::new();
        let dispatch = create_installer(&definition, context.clone())?
            .expect("github installer should be created");
        dispatch
            .installer
            .install(None, &mut lockfile)
            .expect("install should succeed");

        let receipts: Vec<_> = lockfile.tool_receipts().collect();
        assert_eq!(receipts.len(), 1);
        let receipt = receipts[0];
        assert_eq!(receipt.name, "mock");
        assert_eq!(receipt.binaries.len(), 1);
        assert!(receipt.asset.is_some());
        let asset = receipt.asset.as_ref().unwrap();
        assert_eq!(asset.name, "tool.tar.gz");
        assert_eq!(asset.pattern_index, Some(0));
        assert!(bin_dir.join("tool").exists());

        Ok(())
    }

    #[test]
    fn github_installer_rejects_checksum_mismatch() -> TestResult<()> {
        let temp = TempDir::new()?;
        let cache_tools_dir = temp.path().join("cache/tools");
        let bin_dir = temp.path().join("state/bin");
        let share_dir = temp.path().join("state/share");
        fs::create_dir_all(&cache_tools_dir)?;
        fs::create_dir_all(&bin_dir)?;
        fs::create_dir_all(&share_dir)?;

        let asset_bytes = build_tar_gz(&[("tool", b"#!/bin/sh\necho hi\n")])?;
        let digest_array = {
            let mut hasher = Sha256::new();
            hasher.update(&asset_bytes);
            hasher.finalize().into()
        };

        let release = mock_release(
            "tool.tar.gz",
            "https://example.com/tool.tar.gz",
            asset_bytes.len() as u64,
        );
        let github_api = MockGithubApi {
            release,
            asset_bytes,
            digest: digest_array,
        };

        let context = InstallContext {
            cache_tools_dir,
            bin_dir,
            share_dir,
            github_api: Arc::new(github_api),
        };

        let definition = ToolDefinition {
            name: "mock".to_string(),
            installer: InstallerKind::Github,
            project: Some("owner/mock".to_string()),
            version: Some("v1.0.0".to_string()),
            url: None,
            shell: None,
            bin: vec![ToolBinary {
                source: "tool".to_string(),
                link: None,
            }],
            extras: Vec::new(),
            asset_filter: vec!["tool".to_string()],
            checksum: Some(
                "sha256:0000000000000000000000000000000000000000000000000000000000000000"
                    .to_string(),
            ),
            app: None,
            team_id: None,
            self_update: false,
            platforms: Vec::new(),
            hosts: Vec::new(),
        };

        let mut lockfile = Lockfile::new();
        let dispatch =
            create_installer(&definition, context)?.expect("github installer should be created");
        let err = dispatch
            .installer
            .install(None, &mut lockfile)
            .expect_err("expected checksum mismatch");
        assert!(err.to_string().contains("Checksum mismatch"));
        Ok(())
    }

    #[test]
    fn github_installer_errors_when_binary_missing() -> TestResult<()> {
        let temp = TempDir::new()?;
        let cache_tools_dir = temp.path().join("cache/tools");
        let bin_dir = temp.path().join("state/bin");
        let share_dir = temp.path().join("state/share");
        fs::create_dir_all(&cache_tools_dir)?;
        fs::create_dir_all(&bin_dir)?;
        fs::create_dir_all(&share_dir)?;

        let asset_bytes = build_tar_gz(&[("other", b"data")])?;
        let digest_array = {
            let mut hasher = Sha256::new();
            hasher.update(&asset_bytes);
            hasher.finalize().into()
        };
        let checksum = format!("sha256:{}", hex::encode(digest_array));

        let release = mock_release(
            "tool.tar.gz",
            "https://example.com/tool.tar.gz",
            asset_bytes.len() as u64,
        );
        let github_api = MockGithubApi {
            release,
            asset_bytes,
            digest: digest_array,
        };

        let context = InstallContext {
            cache_tools_dir,
            bin_dir,
            share_dir,
            github_api: Arc::new(github_api),
        };

        let definition = ToolDefinition {
            name: "mock".to_string(),
            installer: InstallerKind::Github,
            project: Some("owner/mock".to_string()),
            version: Some("v1.0.0".to_string()),
            url: None,
            shell: None,
            bin: vec![ToolBinary {
                source: "tool".to_string(),
                link: None,
            }],
            extras: Vec::new(),
            asset_filter: vec!["tool".to_string()],
            checksum: Some(checksum),
            app: None,
            team_id: None,
            self_update: false,
            platforms: Vec::new(),
            hosts: Vec::new(),
        };

        let mut lockfile = Lockfile::new();
        let dispatch =
            create_installer(&definition, context)?.expect("github installer should be created");
        let err = dispatch
            .installer
            .install(None, &mut lockfile)
            .expect_err("expected missing binary error");
        assert!(err.to_string().contains("Failed to locate binary 'tool'"));
        Ok(())
    }
}
