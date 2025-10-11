use super::{sanitize_component, InstallContext, ToolInstaller};
use crate::lockfile::Lockfile;
use crate::toolset::ToolDefinition;
use anyhow::{anyhow, bail, Context, Result};
use reqwest::blocking::Client as BlockingClient;
use reqwest::header::{ACCEPT, AUTHORIZATION};
use serde::Deserialize;
use std::env;
use std::fs;
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};
use tokio::runtime::Runtime;
use ubi::UbiBuilder;
use url::{form_urlencoded, Url};
use walkdir::WalkDir;

pub(crate) struct UbiInstaller {
    definition: ToolDefinition,
    install_root: PathBuf,
    version_label: String,
    bin_dir: PathBuf,
    binary_specs: Vec<BinarySpec>,
}

#[derive(Debug, Clone)]
struct BinarySpec {
    source: BinarySource,
    link_name: String,
}

#[derive(Debug, Clone)]
enum BinarySource {
    Name(String),
    Explicit(PathBuf),
}

#[derive(Debug)]
struct ResolvedBinary {
    link_name: String,
    source: PathBuf,
}

impl UbiInstaller {
    pub(crate) fn new(mut definition: ToolDefinition, context: InstallContext) -> Result<Self> {
        if definition.project.is_none() {
            bail!(
                "Tool '{}' must specify `project` when using the ubi installer",
                definition.name
            );
        }

        if definition.bin.is_empty() {
            let project = definition.project.as_ref().expect("validated above");
            let default_bin = default_bin_name(project);
            definition.bin = vec![default_bin];
        }

        let binary_specs = parse_binary_specs(&definition.bin)?;

        let version_label = resolve_version(&definition)?;

        let install_root = context
            .cache_tools_dir
            .join(sanitize_component(&definition.name))
            .join(sanitize_component(&version_label));

        Ok(Self {
            bin_dir: context.bin_dir,
            definition,
            install_root,
            version_label,
            binary_specs,
        })
    }

    fn install_release(&self, runtime: &mut Runtime) -> Result<()> {
        fs::create_dir_all(&self.install_root).with_context(|| {
            format!(
                "Failed to create ubi install directory {:?} for '{}'",
                self.install_root, self.definition.name
            )
        })?;

        let project = self.definition.project.as_ref().expect("validated above");

        let mut builder = UbiBuilder::new()
            .project(project)
            .install_dir(&self.install_root)
            .extract_all();

        if let Some(tag) = self.definition.version.as_deref() {
            builder = builder.tag(tag);
        }

        let mut ubi = builder
            .build()
            .with_context(|| format!("Failed to configure ubi for '{}'", self.definition.name))?;

        // Despite the name, `install_binary` unpacks the entire release when `extract_all` is set.
        // We rely on that to preserve additional assets (man pages, completions, etc.) alongside
        // the primary binary and then wire up symlinks ourselves.
        runtime.block_on(async {
            ubi.install_binary()
                .await
                .with_context(|| format!("Failed to install '{}' via ubi", self.definition.name))
        })
    }

    fn binaries(&self) -> Result<Vec<ResolvedBinary>> {
        let mut results = Vec::new();
        for spec in &self.binary_specs {
            let source = match &spec.source {
                BinarySource::Explicit(relative) => {
                    let candidate = self.install_root.join(relative);
                    if candidate.exists() {
                        candidate
                    } else {
                        bail!(
                            "Could not locate binary '{}' for '{}' at {:?}",
                            relative.display(),
                            self.definition.name,
                            candidate
                        );
                    }
                }
                BinarySource::Name(name) => self.find_binary_path(name)?,
            };

            results.push(ResolvedBinary {
                link_name: spec.link_name.clone(),
                source,
            });
        }
        Ok(results)
    }

    fn find_binary_path(&self, bin_name: &str) -> Result<PathBuf> {
        let direct = self.install_root.join(bin_name);
        if direct.exists() {
            return Ok(direct);
        }

        #[cfg(target_os = "windows")]
        let alt = self.install_root.join(format!("{bin_name}.exe"));
        #[cfg(not(target_os = "windows"))]
        let alt = self.install_root.join(format!("{bin_name}.exe"));
        if alt.exists() {
            return Ok(alt);
        }

        let mut matches = Vec::new();
        for entry in WalkDir::new(&self.install_root)
            .into_iter()
            .filter_map(Result::ok)
            .filter(|e| e.file_type().is_file())
        {
            let name = entry.file_name().to_string_lossy();
            if name == bin_name || name == format!("{bin_name}.exe") {
                matches.push(entry.into_path());
            }
        }

        matches.into_iter().next().ok_or_else(|| {
            anyhow!(
                "Could not locate binary '{}' within {:?}",
                bin_name,
                self.install_root
            )
        })
    }

    fn link_binaries(&self, binaries: Vec<ResolvedBinary>, lockfile: &mut Lockfile) -> Result<()> {
        fs::create_dir_all(&self.bin_dir)
            .with_context(|| format!("Failed to create bin directory {:?}", self.bin_dir))?;

        for binary in binaries {
            let target = self.bin_dir.join(&binary.link_name);
            if target.exists() || target.symlink_metadata().is_ok() {
                fs::remove_file(&target).with_context(|| {
                    format!("Failed to remove existing binary symlink {:?}", target)
                })?;
            }

            symlink(&binary.source, &target).with_context(|| {
                format!(
                    "Failed to create symlink {:?} -> {:?}",
                    target, binary.source
                )
            })?;

            lockfile.add_tool_symlink(
                self.definition.name.clone(),
                self.version_label.clone(),
                binary.source,
                target,
            );
        }

        Ok(())
    }
}

impl ToolInstaller for UbiInstaller {
    fn requires_runtime(&self) -> bool {
        true
    }

    fn install(&self, runtime: Option<&mut Runtime>, lockfile: &mut Lockfile) -> Result<()> {
        let runtime = runtime.ok_or_else(|| anyhow!("UBI installer requires a Tokio runtime"))?;
        self.install_release(runtime)?;
        let binaries = self.binaries()?;
        self.link_binaries(binaries, lockfile)
    }
}

impl UbiInstaller {
    pub(crate) fn resolved_version(&self) -> &str {
        &self.version_label
    }
}

// Tests: there isn't currently an automated test for this installer because the upstream `ubi`
// crate always talks to live forge endpoints. Once we introduce a mock downloader (for example via
// a local HTTP server that serves a tiny archive), we can exercise this logic end-to-end without
// hitting the network.

fn default_bin_name(project: &str) -> String {
    let mut candidate = project.trim_end_matches('/');
    if let Some(pos) = candidate.rfind('/') {
        candidate = &candidate[pos + 1..];
    }
    let candidate = candidate.trim_end_matches(".git");
    if candidate.is_empty() {
        "tool".to_string()
    } else {
        candidate.to_string()
    }
}

fn parse_binary_specs(entries: &[String]) -> Result<Vec<BinarySpec>> {
    let mut specs = Vec::new();

    for entry in entries {
        let (raw_source, alias) = if let Some((left, right)) = entry.split_once(':') {
            (left.trim(), Some(right.trim()))
        } else {
            (entry.trim(), None)
        };

        if raw_source.is_empty() {
            bail!("Binary entry must not be empty");
        }

        let has_sep = raw_source.contains('/') || raw_source.contains('\\');
        let link_name = alias
            .filter(|s| !s.is_empty())
            .map(ToString::to_string)
            .or_else(|| {
                Path::new(raw_source)
                    .file_name()
                    .map(|os| os.to_string_lossy().into_owned())
            })
            .unwrap_or_else(|| raw_source.to_string());

        let source = if has_sep {
            BinarySource::Explicit(PathBuf::from(raw_source))
        } else {
            BinarySource::Name(raw_source.to_string())
        };

        specs.push(BinarySpec { source, link_name });
    }

    Ok(specs)
}

fn resolve_version(definition: &ToolDefinition) -> Result<String> {
    if let Some(ref version) = definition.version {
        if !version.trim().is_empty() && !version.eq_ignore_ascii_case("latest") {
            return Ok(version.clone());
        }
    }

    let project = definition
        .project
        .as_ref()
        .ok_or_else(|| anyhow!("Tool '{}' must define a project", definition.name))?;

    fetch_latest_tag(project)
        .with_context(|| format!("Failed to resolve latest release for '{}'", project))
}

#[derive(Debug)]
enum ForgeSpec {
    GitHub {
        api_base: String,
        owner: String,
        repo: String,
    },
    GitLab {
        api_base: String,
        project: String,
    },
}

fn fetch_latest_tag(project: &str) -> Result<String> {
    let spec = parse_project_spec(project)?;
    match spec {
        ForgeSpec::GitHub {
            api_base,
            owner,
            repo,
        } => github_latest(&api_base, &owner, &repo),
        ForgeSpec::GitLab { api_base, project } => gitlab_latest(&api_base, &project),
    }
}

fn parse_project_spec(project: &str) -> Result<ForgeSpec> {
    if project.starts_with("http://") || project.starts_with("https://") {
        let url = Url::parse(project)?;
        build_spec_from_url(&url)
    } else {
        let trimmed = project.trim_matches('/');
        let mut parts = trimmed.split('/');
        let owner = parts
            .next()
            .ok_or_else(|| anyhow!("Could not parse project from '{project}'"))?;
        let repo = parts
            .next()
            .ok_or_else(|| anyhow!("Could not parse project from '{project}'"))?;
        Ok(ForgeSpec::GitHub {
            api_base: "https://api.github.com".to_string(),
            owner: owner.trim_end_matches(".git").to_string(),
            repo: repo.trim_end_matches(".git").to_string(),
        })
    }
}

fn build_spec_from_url(url: &Url) -> Result<ForgeSpec> {
    let host = url
        .host_str()
        .ok_or_else(|| anyhow!("Project URL '{}' is missing a host", url))?;

    let mut segments: Vec<&str> = url
        .path_segments()
        .map(|segments| segments.filter(|s| !s.is_empty()).collect())
        .unwrap_or_default();

    if host.contains("gitlab") {
        if let Some(pos) = segments.iter().position(|s| *s == "-") {
            segments.truncate(pos);
        }
        if segments.is_empty() {
            bail!("Could not parse GitLab project from '{}'.", url);
        }
        let encoded = encode_gitlab_path(&segments);
        let api_base = if host == "gitlab.com" {
            "https://gitlab.com/api/v4".to_string()
        } else {
            format!("https://{host}/api/v4")
        };
        Ok(ForgeSpec::GitLab {
            api_base,
            project: encoded,
        })
    } else {
        if segments.len() < 2 {
            bail!("Could not parse GitHub project from '{}'.", url);
        }
        let owner = segments[0].trim_end_matches(".git").to_string();
        let repo = segments[1].trim_end_matches(".git").to_string();
        let api_base = if host == "github.com" {
            "https://api.github.com".to_string()
        } else {
            format!("https://{host}/api/v3")
        };
        Ok(ForgeSpec::GitHub {
            api_base,
            owner,
            repo,
        })
    }
}

fn encode_gitlab_path(segments: &[&str]) -> String {
    segments
        .iter()
        .map(|segment| form_urlencoded::byte_serialize(segment.as_bytes()).collect::<String>())
        .collect::<Vec<String>>()
        .join("%2F")
}

fn github_latest(api_base: &str, owner: &str, repo: &str) -> Result<String> {
    #[derive(Deserialize)]
    struct GitHubRelease {
        tag_name: String,
    }

    let url = format!("{api_base}/repos/{owner}/{repo}/releases/latest");
    let client = blocking_client()?;

    let mut request = client
        .get(url)
        .header(ACCEPT, "application/vnd.github+json");

    if let Some(token) = github_token() {
        request = request.bearer_auth(token);
    }

    let release: GitHubRelease = request.send()?.error_for_status()?.json()?;
    Ok(release.tag_name)
}

fn gitlab_latest(api_base: &str, project: &str) -> Result<String> {
    #[derive(Deserialize)]
    struct GitLabRelease {
        tag_name: String,
    }

    let url = format!("{api_base}/projects/{project}/releases/permalink/latest");
    let client = blocking_client()?;

    let mut request = client.get(url);
    if let Some(token) = gitlab_token() {
        request = request.header(AUTHORIZATION, format!("Bearer {token}"));
    }

    let release: GitLabRelease = request.send()?.error_for_status()?.json()?;
    Ok(release.tag_name)
}

fn blocking_client() -> Result<BlockingClient> {
    BlockingClient::builder()
        .user_agent(default_user_agent())
        .build()
        .map_err(Into::into)
}

fn default_user_agent() -> String {
    format!("dws/{}", env!("CARGO_PKG_VERSION"))
}

fn github_token() -> Option<String> {
    env::var("GITHUB_TOKEN")
        .ok()
        .or_else(|| env::var("GH_TOKEN").ok())
}

fn gitlab_token() -> Option<String> {
    env::var("CI_TOKEN")
        .ok()
        .or_else(|| env::var("CI_JOB_TOKEN").ok())
        .or_else(|| env::var("GITLAB_TOKEN").ok())
}
