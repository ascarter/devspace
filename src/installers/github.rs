use anyhow::{bail, Context, Result};
use regex::Regex;
use reqwest::blocking::{Client, Response};
use reqwest::header::{ACCEPT, USER_AGENT};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::cmp::{Ordering, Reverse};
use std::env;
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use tar::Archive;
use walkdir::WalkDir;
use xz2::read::XzDecoder;
use zip::ZipArchive;

use flate2::read::GzDecoder;

use super::InstallContext;
use crate::toolset::{ExtraKind, ToolExtra};

const API_ROOT: &str = "https://api.github.com";
const DEFAULT_USER_AGENT: &str = "dws/0.1";

#[derive(Clone)]
pub struct GithubClient {
    http: Client,
    token: Option<String>,
    user_agent: String,
}

impl GithubClient {
    pub fn new(token: Option<String>) -> Result<Self> {
        let http = Client::builder()
            .build()
            .context("Failed to build GitHub client")?;
        let user_agent = env::var("DWS_GITHUB_USER_AGENT")
            .ok()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| DEFAULT_USER_AGENT.to_string());
        Ok(Self {
            http,
            token,
            user_agent,
        })
    }

    pub fn from_env() -> Result<Self> {
        let token = env::var("DWS_GITHUB_TOKEN")
            .ok()
            .filter(|value| !value.trim().is_empty())
            .or_else(|| {
                env::var("GITHUB_TOKEN")
                    .ok()
                    .filter(|value| !value.trim().is_empty())
            });
        Self::new(token)
    }

    pub fn fetch_release(&self, project: &str, tag: Option<&str>) -> Result<GithubRelease> {
        let url = release_endpoint(project, tag);
        let mut request = self
            .http
            .get(&url)
            .header(ACCEPT, "application/vnd.github+json")
            .header(USER_AGENT, &self.user_agent);

        if let Some(token) = &self.token {
            request = request.bearer_auth(token);
        }

        let response = request
            .send()
            .with_context(|| format!("Failed to request GitHub release metadata from {url}"))?;
        let response = handle_errors(response, project, tag)?;

        response
            .json::<GithubRelease>()
            .with_context(|| format!("Failed to decode GitHub release response from {url}"))
    }

    pub fn download_asset(&self, url: &str, dest: &Path) -> Result<[u8; 32]> {
        let mut request = self.http.get(url).header(USER_AGENT, &self.user_agent);

        if let Some(token) = &self.token {
            request = request.bearer_auth(token);
        }

        let mut response = request
            .send()
            .with_context(|| format!("Failed to download GitHub asset from {url}"))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response
                .text()
                .unwrap_or_else(|_| "<unavailable>".to_string());
            bail!("GitHub asset download returned {status}: {body}");
        }

        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent).with_context(|| {
                format!("Failed to create parent directory for asset at {:?}", dest)
            })?;
        }

        let temp_path = dest.with_extension("download");
        let mut file = File::create(&temp_path)
            .with_context(|| format!("Failed to create temporary asset file at {:?}", temp_path))?;

        let mut hasher = Sha256::new();
        let mut buffer = [0u8; 8192];

        loop {
            let read = response
                .read(&mut buffer)
                .with_context(|| format!("Failed while reading asset stream from {url}"))?;
            if read == 0 {
                break;
            }
            file.write_all(&buffer[..read])
                .with_context(|| format!("Failed while writing asset to {:?}", temp_path))?;
            hasher.update(&buffer[..read]);
        }

        file.flush()
            .with_context(|| format!("Failed to flush downloaded asset to {:?}", temp_path))?;

        fs::rename(&temp_path, dest).with_context(|| {
            format!(
                "Failed to move downloaded asset from {:?} to {:?}",
                temp_path, dest
            )
        })?;

        Ok(hasher.finalize().into())
    }
}

fn handle_errors(response: Response, project: &str, tag: Option<&str>) -> Result<Response> {
    let status = response.status();

    if status.is_success() {
        return Ok(response);
    }

    if status.as_u16() == 404 {
        let suffix = tag.map(|t| format!(" for tag '{t}'")).unwrap_or_default();
        bail!("GitHub release not found for repository '{project}'{suffix}");
    }

    let body = response
        .text()
        .unwrap_or_else(|_| "<unavailable>".to_string());
    bail!("GitHub API returned {status} for repository '{project}': {body}");
}

pub(crate) fn release_endpoint(project: &str, tag: Option<&str>) -> String {
    let trimmed = project.trim();
    let normalized = trimmed.trim_matches('/');
    if let Some(tag) = tag {
        format!("{API_ROOT}/repos/{normalized}/releases/tags/{tag}")
    } else {
        format!("{API_ROOT}/repos/{normalized}/releases/latest")
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
pub struct GithubRelease {
    pub id: u64,
    pub tag_name: String,
    pub name: Option<String>,
    pub draft: bool,
    pub prerelease: bool,
    pub assets: Vec<GithubAsset>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
pub struct GithubAsset {
    pub id: u64,
    pub name: String,
    pub content_type: Option<String>,
    pub browser_download_url: String,
    pub size: u64,
    pub state: Option<String>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
pub struct SelectedAsset<'a> {
    pub asset: &'a GithubAsset,
    pub pattern_index: usize,
    pub pattern: &'a str,
}

impl GithubRelease {
    pub fn select_asset<'a>(&'a self, filters: &'a [String]) -> Result<SelectedAsset<'a>> {
        if filters.is_empty() {
            bail!("GitHub installer requires at least one asset_filter pattern");
        }

        if self.assets.is_empty() {
            bail!(
                "GitHub release '{}' does not expose any assets",
                self.tag_name
            );
        }

        for (index, pattern) in filters.iter().enumerate() {
            let regex = Regex::new(pattern)
                .with_context(|| format!("Invalid asset_filter regex '{pattern}'"))?;

            let mut best: Option<(&GithubAsset, AssetRank<'_>)> = None;
            let mut ambiguous = false;

            for asset in &self.assets {
                if !regex.is_match(&asset.name) {
                    continue;
                }

                let rank = AssetRank::new(asset);
                match &mut best {
                    None => best = Some((asset, rank)),
                    Some((_, current_rank)) => match rank.cmp(current_rank) {
                        Ordering::Greater => {
                            best = Some((asset, rank));
                            ambiguous = false;
                        }
                        Ordering::Equal => {
                            ambiguous = true;
                        }
                        Ordering::Less => {}
                    },
                }
            }

            if let Some((asset, _)) = best {
                if ambiguous {
                    bail!("Asset filter '{pattern}' matched multiple assets with equal scoring");
                }

                return Ok(SelectedAsset {
                    asset,
                    pattern_index: index,
                    pattern,
                });
            }
        }

        bail!(
            "No release asset matched the provided asset_filter patterns: {:?}",
            filters
        );
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct AssetRank<'a> {
    score: i32,
    size: Reverse<u64>,
    name: Reverse<&'a str>,
}

impl<'a> AssetRank<'a> {
    fn new(asset: &'a GithubAsset) -> Self {
        let mut score = 0;
        if matches!(asset.state.as_deref(), Some("uploaded" | "available")) {
            score += 100;
        }

        score += extension_score(&asset.name);
        score += architecture_score(&asset.name);

        Self {
            score,
            size: Reverse(asset.size),
            name: Reverse(asset.name.as_str()),
        }
    }
}

impl<'a> Ord for AssetRank<'a> {
    fn cmp(&self, other: &Self) -> Ordering {
        (self.score, &self.size, &self.name).cmp(&(other.score, &other.size, &other.name))
    }
}

impl<'a> PartialOrd for AssetRank<'a> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

fn extension_score(name: &str) -> i32 {
    if name.ends_with(".tar.gz") || name.ends_with(".tar.xz") {
        30
    } else if name.ends_with(".tgz") {
        25
    } else if name.ends_with(".zip") {
        20
    } else if name.ends_with(".tar") {
        15
    } else {
        5
    }
}

fn architecture_score(name: &str) -> i32 {
    let lowered = name.to_ascii_lowercase();
    let mut score = 0;
    if lowered.contains("x86_64") || lowered.contains("amd64") {
        score += 10;
    }
    if lowered.contains("arm64") || lowered.contains("aarch64") {
        score += 7;
    }
    if lowered.contains("linux") {
        score += 5;
    }
    if lowered.contains("apple") || lowered.contains("darwin") || lowered.contains("macos") {
        score += 5;
    }
    score
}

pub fn parse_sha256(value: &str) -> Result<[u8; 32]> {
    let trimmed = value.trim();
    let digest = trimmed
        .strip_prefix("sha256:")
        .context("Checksum must use `sha256:<hex>` format")?;

    if digest.len() != 64 {
        bail!("SHA256 checksum must be exactly 64 hex characters");
    }

    let bytes = hex::decode(digest).with_context(|| "Failed to decode SHA256 checksum")?;
    let mut array = [0u8; 32];
    array.copy_from_slice(&bytes);
    Ok(array)
}

pub fn format_digest(bytes: &[u8; 32]) -> String {
    hex::encode(bytes)
}

pub fn compute_sha256(path: &Path) -> Result<[u8; 32]> {
    let mut file = File::open(path)
        .with_context(|| format!("Failed to open file for checksum calculation at {:?}", path))?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 8192];

    loop {
        let read = file
            .read(&mut buffer)
            .with_context(|| format!("Failed to read file {:?} while hashing", path))?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }

    Ok(hasher.finalize().into())
}

pub fn extract_archive(archive_path: &Path, dest: &Path) -> Result<()> {
    let filename = archive_path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();

    if filename.ends_with(".tar.gz") || filename.ends_with(".tgz") {
        let file = File::open(archive_path)
            .with_context(|| format!("Failed to open archive {:?}", archive_path))?;
        let decoder = GzDecoder::new(file);
        let mut archive = Archive::new(decoder);
        archive
            .unpack(dest)
            .with_context(|| format!("Failed to unpack tar.gz archive {:?}", archive_path))?;
    } else if filename.ends_with(".tar.xz") || filename.ends_with(".txz") {
        let file = File::open(archive_path)
            .with_context(|| format!("Failed to open archive {:?}", archive_path))?;
        let decoder = XzDecoder::new(file);
        let mut archive = Archive::new(decoder);
        archive
            .unpack(dest)
            .with_context(|| format!("Failed to unpack tar.xz archive {:?}", archive_path))?;
    } else if filename.ends_with(".tar") {
        let file = File::open(archive_path)
            .with_context(|| format!("Failed to open archive {:?}", archive_path))?;
        let mut archive = Archive::new(file);
        archive
            .unpack(dest)
            .with_context(|| format!("Failed to unpack tar archive {:?}", archive_path))?;
    } else if filename.ends_with(".zip") {
        let file = File::open(archive_path)
            .with_context(|| format!("Failed to open zip archive {:?}", archive_path))?;
        let mut archive = ZipArchive::new(file)
            .with_context(|| format!("Failed to read zip archive {:?}", archive_path))?;

        for index in 0..archive.len() {
            let mut entry = archive.by_index(index).with_context(|| {
                format!("Failed to read zip entry #{index} from {:?}", archive_path)
            })?;

            let Some(enclosed) = entry.enclosed_name().map(|path| dest.join(path)) else {
                continue;
            };

            if entry.name().ends_with('/') {
                fs::create_dir_all(&enclosed)
                    .with_context(|| format!("Failed to create directory {:?}", enclosed))?;
            } else {
                if let Some(parent) = enclosed.parent() {
                    fs::create_dir_all(parent).with_context(|| {
                        format!("Failed to create parent directory {:?}", parent)
                    })?;
                }

                let mut outfile = File::create(&enclosed)
                    .with_context(|| format!("Failed to create file {:?}", enclosed))?;
                io::copy(&mut entry, &mut outfile)
                    .with_context(|| format!("Failed to extract zip entry {:?}", enclosed))?;

                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    if let Some(mode) = entry.unix_mode() {
                        fs::set_permissions(&enclosed, fs::Permissions::from_mode(mode))
                            .with_context(|| {
                                format!("Failed to set permissions on {:?}", enclosed)
                            })?;
                    }
                }
            }
        }
    } else {
        let target = dest.join(
            archive_path
                .file_name()
                .context("Archive path is missing a filename")?,
        );
        fs::copy(archive_path, &target)
            .with_context(|| format!("Failed to copy asset {:?} to {:?}", archive_path, target))?;
    }

    Ok(())
}

pub fn resolve_binary_path(extract_root: &Path, source: &str) -> Result<PathBuf> {
    let relative = Path::new(source);
    if relative.is_absolute() {
        bail!("Binary source path '{}' must be relative", source);
    }

    let direct = extract_root.join(relative);
    if direct.exists() {
        return Ok(direct);
    }

    if relative.components().count() == 1 {
        let needle = relative.as_os_str();
        let mut matches = Vec::new();
        for entry in WalkDir::new(extract_root).into_iter() {
            let entry = entry?;
            if entry.file_type().is_file() && entry.file_name() == needle {
                matches.push(entry.into_path());
                if matches.len() > 1 {
                    break;
                }
            }
        }

        return match matches.len() {
            1 => Ok(matches.remove(0)),
            0 => bail!(
                "Binary '{}' not found in extracted contents under {:?}",
                source,
                extract_root
            ),
            _ => bail!(
                "Binary '{}' matched multiple files in extracted contents under {:?}",
                source,
                extract_root
            ),
        };
    }

    bail!(
        "Binary '{}' not found at relative path {:?} inside extracted contents",
        source,
        relative
    );
}

pub fn resolve_extra_path(extract_root: &Path, extra: &ToolExtra) -> Result<PathBuf> {
    resolve_binary_path(extract_root, &extra.source)
}

pub fn resolve_extra_target(
    context: &InstallContext,
    tool_slug: &str,
    extra: &ToolExtra,
    resolved_source: &Path,
) -> Result<PathBuf> {
    if let Some(target) = &extra.target {
        let expanded = shellexpand::env(target)
            .map_err(|err| anyhow::anyhow!("Failed to expand target '{}': {}", target, err))?;
        return Ok(PathBuf::from(expanded.as_ref()));
    }

    match extra.kind {
        ExtraKind::Man => manpage_target(&context.share_dir, resolved_source),
        ExtraKind::Completion => {
            let shell = extra
                .shell
                .as_deref()
                .context("Completion extra requires a shell field")?
                .to_ascii_lowercase();
            completion_target(&context.share_dir, &shell, resolved_source)
        }
        ExtraKind::Other => other_extra_target(&context.share_dir, tool_slug, &extra.source),
    }
}

fn manpage_target(share_dir: &Path, resolved_source: &Path) -> Result<PathBuf> {
    let section = resolved_source
        .extension()
        .and_then(|ext| ext.to_str())
        .filter(|ext| !ext.is_empty())
        .unwrap_or("1");
    let man_dir = share_dir.join("man").join(format!("man{section}"));
    fs::create_dir_all(&man_dir)
        .with_context(|| format!("Failed to create man directory {:?}", man_dir))?;
    let filename = resolved_source
        .file_name()
        .context("Man page missing file name")?;
    Ok(man_dir.join(filename))
}

fn completion_target(share_dir: &Path, shell: &str, resolved_source: &Path) -> Result<PathBuf> {
    let dir = match shell {
        "zsh" => share_dir.join("zsh").join("site-functions"),
        "bash" => share_dir.join("bash-completions"),
        "fish" => share_dir.join("fish").join("vendor_completions.d"),
        other => bail!("Unsupported completion shell '{}'.", other),
    };

    fs::create_dir_all(&dir)
        .with_context(|| format!("Failed to create completion directory {:?}", dir))?;

    let filename = resolved_source
        .file_name()
        .context("Completion file missing file name")?;

    Ok(dir.join(filename))
}

fn other_extra_target(share_dir: &Path, tool_slug: &str, source: &str) -> Result<PathBuf> {
    let relative = Path::new(source);
    if relative.is_absolute() {
        return Ok(relative.to_path_buf());
    }

    let target = share_dir.join("extras").join(tool_slug).join(relative);
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create extras directory {:?}", parent))?;
    }

    Ok(target)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn endpoint_latest() {
        let endpoint = release_endpoint("owner/repo", None);
        assert_eq!(
            endpoint,
            "https://api.github.com/repos/owner/repo/releases/latest"
        );
    }

    #[test]
    fn endpoint_tag() {
        let endpoint = release_endpoint("/owner/repo/", Some("v1.2.3"));
        assert_eq!(
            endpoint,
            "https://api.github.com/repos/owner/repo/releases/tags/v1.2.3"
        );
    }

    #[test]
    fn parse_sha256_validates_length() {
        let err = parse_sha256("sha256:deadbeef").unwrap_err();
        assert!(err.to_string().contains("64"));
    }

    #[test]
    fn parse_sha256_requires_prefix() {
        let err = parse_sha256("deadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeef")
            .unwrap_err();
        assert!(err.to_string().contains("sha256"));
    }

    #[test]
    fn parse_release_payload() {
        let payload = r#"
        {
            "id": 1,
            "tag_name": "v1.0.0",
            "name": "Release",
            "draft": false,
            "prerelease": false,
            "assets": [
                {
                    "id": 10,
                    "name": "tool.tar.gz",
                    "content_type": "application/gzip",
                    "browser_download_url": "https://example.com/tool.tar.gz",
                    "size": 1024,
                    "state": "uploaded"
                }
            ]
        }
        "#;
        let release: GithubRelease = serde_json::from_str(payload).unwrap();
        assert_eq!(release.tag_name, "v1.0.0");
        assert_eq!(release.assets.len(), 1);
        assert_eq!(release.assets[0].name, "tool.tar.gz");
    }

    fn sample_release() -> GithubRelease {
        GithubRelease {
            id: 1,
            tag_name: "v1.0.0".to_string(),
            name: Some("Release".to_string()),
            draft: false,
            prerelease: false,
            assets: vec![
                GithubAsset {
                    id: 10,
                    name: "tool-macos-x86_64.tar.gz".to_string(),
                    content_type: Some("application/gzip".to_string()),
                    browser_download_url: "https://example.com/mac.tar.gz".to_string(),
                    size: 1024,
                    state: Some("uploaded".to_string()),
                },
                GithubAsset {
                    id: 11,
                    name: "tool-macos-arm64.tar.gz".to_string(),
                    content_type: Some("application/gzip".to_string()),
                    browser_download_url: "https://example.com/arm.tar.gz".to_string(),
                    size: 2048,
                    state: Some("uploaded".to_string()),
                },
                GithubAsset {
                    id: 12,
                    name: "tool-windows.zip".to_string(),
                    content_type: Some("application/zip".to_string()),
                    browser_download_url: "https://example.com/win.zip".to_string(),
                    size: 4096,
                    state: Some("uploaded".to_string()),
                },
            ],
        }
    }

    #[test]
    fn select_asset_with_matching_pattern() {
        let release = sample_release();
        let filters = vec!["macos".to_string(), "windows".to_string()];
        let selected = release.select_asset(&filters).unwrap();
        assert_eq!(selected.pattern_index, 0);
        assert_eq!(selected.asset.name, "tool-macos-x86_64.tar.gz");
    }

    #[test]
    fn select_asset_uses_subsequent_pattern() {
        let release = sample_release();
        let filters = vec!["linux".to_string(), "windows".to_string()];
        let selected = release.select_asset(&filters).unwrap();
        assert_eq!(selected.pattern_index, 1);
        assert_eq!(selected.asset.name, "tool-windows.zip");
    }

    #[test]
    fn select_asset_reports_missing_match() {
        let release = sample_release();
        let filters = vec!["linux".to_string()];
        let err = release.select_asset(&filters).unwrap_err();
        assert!(err.to_string().contains("No release asset matched"));
    }

    #[test]
    fn select_asset_detects_invalid_regex() {
        let release = sample_release();
        let filters = vec!["[".to_string()];
        let err = release.select_asset(&filters).unwrap_err();
        assert!(err.to_string().contains("Invalid asset_filter regex"));
    }

    #[test]
    fn resolve_extra_target_man_defaults_section() {
        let temp = TempDir::new().unwrap();
        let context = InstallContext {
            cache_tools_dir: temp.path().join("cache"),
            bin_dir: temp.path().join("bin"),
            share_dir: temp.path().join("share"),
        };
        fs::create_dir_all(&context.share_dir).unwrap();

        let extra = ToolExtra {
            source: "docs/rg.1".to_string(),
            kind: ExtraKind::Man,
            shell: None,
            target: None,
        };

        let resolved = context.cache_tools_dir.join("extract/docs/rg.1");
        fs::create_dir_all(resolved.parent().unwrap()).unwrap();
        fs::write(&resolved, "man").unwrap();

        let target = resolve_extra_target(&context, "rg", &extra, &resolved).unwrap();
        assert!(target.to_string_lossy().contains("man1/rg.1"));
    }

    #[test]
    fn resolve_extra_target_completion_shell_dir() {
        let temp = TempDir::new().unwrap();
        let context = InstallContext {
            cache_tools_dir: temp.path().join("cache"),
            bin_dir: temp.path().join("bin"),
            share_dir: temp.path().join("share"),
        };
        fs::create_dir_all(&context.share_dir).unwrap();

        let extra = ToolExtra {
            source: "completions/_rg".to_string(),
            kind: ExtraKind::Completion,
            shell: Some("zsh".to_string()),
            target: None,
        };

        let resolved = context.cache_tools_dir.join("extract/completions/_rg");
        fs::create_dir_all(resolved.parent().unwrap()).unwrap();
        fs::write(&resolved, "comp").unwrap();

        let target = resolve_extra_target(&context, "rg", &extra, &resolved).unwrap();
        assert!(target
            .to_string_lossy()
            .contains("share/zsh/site-functions/_rg"));
    }

    #[test]
    fn resolve_extra_target_absolute_other() {
        let temp = TempDir::new().unwrap();
        let context = InstallContext {
            cache_tools_dir: temp.path().join("cache"),
            bin_dir: temp.path().join("bin"),
            share_dir: temp.path().join("share"),
        };
        fs::create_dir_all(&context.share_dir).unwrap();

        let extra = ToolExtra {
            source: "/absolute/path".to_string(),
            kind: ExtraKind::Other,
            shell: None,
            target: None,
        };

        let resolved = context.cache_tools_dir.join("extract/absolute/path");
        fs::create_dir_all(resolved.parent().unwrap()).unwrap();
        fs::write(&resolved, "content").unwrap();

        let target = resolve_extra_target(&context, "rg", &extra, &resolved).unwrap();
        assert_eq!(target, PathBuf::from("/absolute/path"));
    }

    #[test]
    fn resolve_binary_path_missing_errors() {
        let temp = TempDir::new().unwrap();
        let root = temp.path().join("contents");
        fs::create_dir_all(&root).unwrap();
        let err = resolve_binary_path(&root, "missing").unwrap_err();
        assert!(err.to_string().contains("not found"));
    }

    #[test]
    fn compute_sha256_matches_known_digest() {
        let temp = TempDir::new().unwrap();
        let file = temp.path().join("file");
        fs::write(&file, b"hello world").unwrap();
        let digest = compute_sha256(&file).unwrap();
        assert_eq!(
            format_digest(&digest),
            "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9"
        );
    }
}
