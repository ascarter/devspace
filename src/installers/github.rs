use anyhow::{bail, Context, Result};
use reqwest::blocking::{Client, Response};
use reqwest::header::{ACCEPT, USER_AGENT};
use serde::Deserialize;
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
}
