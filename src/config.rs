use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

const DEFAULT_PROFILE: &str = "default";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default = "default_profile")]
    pub active_profile: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            active_profile: DEFAULT_PROFILE.to_string(),
        }
    }
}

impl Config {
    pub fn load(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }

        let contents = fs::read_to_string(path)
            .with_context(|| format!("Failed to read config file {:?}", path))?;
        toml::from_str(&contents).with_context(|| format!("Failed to parse config file {:?}", path))
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create config directory {:?}", parent))?;
        }

        let contents =
            toml::to_string_pretty(self).context("Failed to serialize dws config file")?;
        fs::write(path, contents)
            .with_context(|| format!("Failed to write config file {:?}", path))?;
        Ok(())
    }
}

fn default_profile() -> String {
    DEFAULT_PROFILE.to_string()
}

pub fn default_profile_name() -> &'static str {
    DEFAULT_PROFILE
}
