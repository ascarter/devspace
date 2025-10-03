use anyhow::{Context, Result};
use std::env;
use std::path::PathBuf;

/// Get the XDG config directory for devspace
///
/// Returns `$XDG_CONFIG_HOME/devspace` or `~/.config/devspace` if not set
pub fn config_dir() -> Result<PathBuf> {
    let base = env::var("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            directories::BaseDirs::new()
                .expect("Failed to get home directory")
                .home_dir()
                .join(".config")
        });

    Ok(base.join("devspace"))
}

/// Get the XDG state directory for devspace
///
/// Returns `$XDG_STATE_HOME/devspace` or `~/.local/state/devspace` if not set
pub fn state_dir() -> Result<PathBuf> {
    let base = env::var("XDG_STATE_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            directories::BaseDirs::new()
                .expect("Failed to get home directory")
                .home_dir()
                .join(".local/state")
        });

    Ok(base.join("devspace"))
}

/// Get the XDG cache directory for devspace
///
/// Returns `$XDG_CACHE_HOME/devspace` or `~/.cache/devspace` if not set
pub fn cache_dir() -> Result<PathBuf> {
    let base = env::var("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            directories::BaseDirs::new()
                .expect("Failed to get home directory")
                .home_dir()
                .join(".cache")
        });

    Ok(base.join("devspace"))
}

/// Get the user's local bin directory
///
/// Returns `$HOME/.local/bin`
pub fn bin_dir() -> Result<PathBuf> {
    let base_dirs = directories::BaseDirs::new().context("Failed to get home directory")?;
    Ok(base_dirs.home_dir().join(".local/bin"))
}

/// Get the home directory
pub fn home_dir() -> Result<PathBuf> {
    directories::BaseDirs::new()
        .context("Failed to get home directory")
        .map(|bd| bd.home_dir().to_path_buf())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_dir() {
        let dir = config_dir().unwrap();
        assert!(dir.to_string_lossy().contains("devspace"));
        assert!(
            dir.to_string_lossy().contains(".config")
                || dir.to_string_lossy().contains("XDG_CONFIG_HOME")
        );
    }

    #[test]
    fn test_state_dir() {
        let dir = state_dir().unwrap();
        assert!(dir.to_string_lossy().contains("devspace"));
        assert!(
            dir.to_string_lossy().contains(".local/state")
                || dir.to_string_lossy().contains("XDG_STATE_HOME")
        );
    }

    #[test]
    fn test_cache_dir() {
        let dir = cache_dir().unwrap();
        assert!(dir.to_string_lossy().contains("devspace"));
        assert!(
            dir.to_string_lossy().contains(".cache")
                || dir.to_string_lossy().contains("XDG_CACHE_HOME")
        );
    }

    #[test]
    fn test_bin_dir() {
        let dir = bin_dir().unwrap();
        assert!(dir.to_string_lossy().contains(".local/bin"));
    }

    #[test]
    fn test_home_dir() {
        let dir = home_dir().unwrap();
        assert!(dir.is_absolute());
    }
}
