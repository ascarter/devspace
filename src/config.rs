use crate::toolset::{ToolConfigFile, ToolSpecToml};
use anyhow::{bail, Result};
use std::collections::BTreeMap;
use std::path::Path;
use toml::Value;

const DEFAULT_PROFILE: &str = "default";

#[derive(Debug, Clone)]
pub struct Config {
    inner: ToolConfigFile,
}

impl Default for Config {
    fn default() -> Self {
        let inner = ToolConfigFile {
            active_profile: Some(default_profile()),
            ..ToolConfigFile::default()
        };
        Self { inner }
    }
}

impl Config {
    pub fn load(path: &Path) -> Result<Self> {
        let mut inner = ToolConfigFile::load(path)?;
        if inner.active_profile.is_none() {
            inner.active_profile = Some(default_profile());
        }
        Ok(Self { inner })
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        if self.inner.active_profile.is_none() {
            bail!("Active profile must be set before saving configuration");
        }

        self.inner.save(path)
    }

    pub fn active_profile(&self) -> &str {
        self.inner
            .active_profile
            .as_deref()
            .unwrap_or(DEFAULT_PROFILE)
    }

    pub fn set_active_profile(&mut self, profile: impl Into<String>) {
        self.inner.active_profile = Some(profile.into());
    }

    pub fn tools(&self) -> &BTreeMap<String, ToolSpecToml> {
        &self.inner.tools
    }

    pub fn tools_mut(&mut self) -> &mut BTreeMap<String, ToolSpecToml> {
        &mut self.inner.tools
    }

    pub fn extras(&self) -> &BTreeMap<String, Value> {
        &self.inner.extras
    }
}

fn default_profile() -> String {
    DEFAULT_PROFILE.to_string()
}

pub fn default_profile_name() -> &'static str {
    DEFAULT_PROFILE
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn load_returns_default_when_file_missing() {
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("config.toml");

        let config = Config::load(&path).unwrap();
        assert_eq!(config.active_profile(), DEFAULT_PROFILE);
        assert!(config.tools().is_empty());
    }

    #[test]
    fn roundtrip_preserves_tools_and_extras() {
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("config.toml");
        fs::write(
            &path,
            r#"
active_profile = "work"

[tools.ripgrep]
installer = "ubi"

[extras]
value = "keep"
"#,
        )
        .unwrap();

        let mut config = Config::load(&path).unwrap();
        assert_eq!(config.active_profile(), "work");
        assert_eq!(config.tools().len(), 1);
        assert!(config.extras().contains_key("extras"));

        config.set_active_profile("personal");
        config.save(&path).unwrap();

        let contents = fs::read_to_string(&path).unwrap();
        assert!(contents.contains("active_profile = \"personal\""));
        assert!(contents.contains("[tools.ripgrep]"));
        assert!(contents.contains("[extras]"));
    }
}
