use anyhow::{bail, Context, Result};
use regex::Regex;
use reqwest::blocking::{Client, Response};
use reqwest::header::{ACCEPT, USER_AGENT};
use serde::Deserialize;
use std::cmp::{Ordering, Reverse};
use std::env;

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

#[cfg(test)]
mod tests {
    use super::*;

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
}
