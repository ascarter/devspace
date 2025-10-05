use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::path::PathBuf;

use super::dotfiles::ProfileConfiguration;

// Template files embedded at compile time
const CLI_TEMPLATE: &str = include_str!("../../templates/profile/cli.toml");
const MACOS_TEMPLATE: &str = include_str!("../../templates/profile/macos.toml");
const README_TEMPLATE: &str = include_str!("../../templates/profile/README.md");
const DEVSPACEIGNORE_TEMPLATE: &str = include_str!("../../templates/profile/.devspaceignore");

/// devws configuration stored in ~/.config/devws/config.toml
#[derive(Debug, Serialize, Deserialize)]
pub struct DevspaceConfig {
    pub active_profile: String,
}

impl Default for DevspaceConfig {
    fn default() -> Self {
        Self {
            active_profile: "default".to_string(),
        }
    }
}

/// Represents a devws profile
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Profile {
    pub name: String,
    pub path: PathBuf,
}

// Private XDG helpers - only used within config module

/// Get the XDG config directory for devws
///
/// Returns `$XDG_CONFIG_HOME/devws` or `~/.config/devws` if not set
fn devspace_config_dir() -> Result<PathBuf> {
    let base = env::var("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            directories::BaseDirs::new()
                .expect("Failed to get home directory")
                .home_dir()
                .join(".config")
        });

    Ok(base.join("devws"))
}

/// Get the path to the devws config file
fn config_file() -> Result<PathBuf> {
    Ok(devspace_config_dir()?.join("config.toml"))
}

/// Get the path to the profiles directory
fn profiles_dir() -> Result<PathBuf> {
    Ok(devspace_config_dir()?.join("profiles"))
}

/// Read the devws configuration, creating it if it doesn't exist
pub fn read_config() -> Result<DevspaceConfig> {
    let config_path = config_file()?;

    if !config_path.exists() {
        // Create default config
        let default_config = DevspaceConfig::default();
        write_config(&default_config)?;
        return Ok(default_config);
    }

    let contents = fs::read_to_string(&config_path)
        .with_context(|| format!("Failed to read config from {:?}", config_path))?;

    toml::from_str(&contents)
        .with_context(|| format!("Failed to parse config from {:?}", config_path))
}

/// Write the devws configuration
pub fn write_config(config: &DevspaceConfig) -> Result<()> {
    let config_path = config_file()?;

    // Ensure parent directory exists
    if let Some(parent) = config_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create config directory {:?}", parent))?;
    }

    let contents = toml::to_string_pretty(config).context("Failed to serialize config")?;

    fs::write(&config_path, contents)
        .with_context(|| format!("Failed to write config to {:?}", config_path))?;

    Ok(())
}

/// Get the currently active profile name
pub fn get_active_profile() -> Result<String> {
    // Check environment variable first
    if let Ok(profile) = std::env::var("DEVSPACE_PROFILE") {
        return Ok(profile);
    }

    // Read from config
    let config = read_config()?;
    Ok(config.active_profile)
}

/// Set the active profile
pub fn set_active_profile(name: &str) -> Result<()> {
    let mut config = read_config()?;
    config.active_profile = name.to_string();
    write_config(&config)?;
    Ok(())
}

/// List all available profiles
pub fn list_profiles() -> Result<Vec<Profile>> {
    let profiles_path = profiles_dir()?;

    if !profiles_path.exists() {
        return Ok(Vec::new());
    }

    let mut profiles = Vec::new();

    for entry in fs::read_dir(&profiles_path)
        .with_context(|| format!("Failed to read profiles directory {:?}", profiles_path))?
    {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                profiles.push(Profile {
                    name: name.to_string(),
                    path: path.clone(),
                });
            }
        }
    }

    // Sort by name for consistent ordering
    profiles.sort_by(|a, b| a.name.cmp(&b.name));

    Ok(profiles)
}

/// Get a specific profile by name
pub fn get_profile(name: &str) -> Result<Option<Profile>> {
    let profiles_path = profiles_dir()?;
    let profile_path = profiles_path.join(name);

    if profile_path.exists() && profile_path.is_dir() {
        Ok(Some(Profile {
            name: name.to_string(),
            path: profile_path,
        }))
    } else {
        Ok(None)
    }
}

/// Create a new profile with template structure
pub fn create_profile(name: &str) -> Result<Profile> {
    let profiles_path = profiles_dir()?;
    let profile_path = profiles_path.join(name);

    // Check if profile already exists
    if profile_path.exists() {
        anyhow::bail!("Profile '{}' already exists", name);
    }

    // Create directory structure
    fs::create_dir_all(&profile_path)
        .with_context(|| format!("Failed to create profile directory {:?}", profile_path))?;

    let config_dir = profile_path.join("config");
    fs::create_dir_all(&config_dir)
        .with_context(|| format!("Failed to create config directory {:?}", config_dir))?;

    let manifests_dir = profile_path.join("manifests");
    fs::create_dir_all(&manifests_dir)
        .with_context(|| format!("Failed to create manifests directory {:?}", manifests_dir))?;

    // Create .gitkeep in config/ so git tracks empty directory
    fs::write(config_dir.join(".gitkeep"), "").context("Failed to create .gitkeep in config/")?;

    // Create .devspaceignore in config/ directory
    fs::write(config_dir.join(".devspaceignore"), DEVSPACEIGNORE_TEMPLATE)
        .context("Failed to create config/.devspaceignore")?;

    // Create manifests from embedded templates
    fs::write(manifests_dir.join("cli.toml"), CLI_TEMPLATE)
        .context("Failed to create manifests/cli.toml")?;

    fs::write(manifests_dir.join("macos.toml"), MACOS_TEMPLATE)
        .context("Failed to create manifests/macos.toml")?;

    // Create README.md with replacements
    let readme = README_TEMPLATE
        .replace("{PROFILE_NAME}", name)
        .replace("{PROFILE_PATH}", &profile_path.display().to_string());

    fs::write(profile_path.join("README.md"), readme).context("Failed to create README.md")?;

    // Initialize git repository
    std::process::Command::new("git")
        .args(["init"])
        .current_dir(&profile_path)
        .output()
        .context("Failed to initialize git repository")?;

    Ok(Profile {
        name: name.to_string(),
        path: profile_path,
    })
}

/// Switch to a different profile (atomic symlink management)
pub fn switch_profile(name: &str) -> Result<()> {
    // Verify the profile exists
    let new_profile = get_profile(name)?
        .ok_or_else(|| anyhow::anyhow!("Profile '{}' does not exist", name))?;

    // Get home directory
    let home_dir = directories::BaseDirs::new()
        .context("Failed to get home directory")?
        .home_dir()
        .to_path_buf();

    // Get current active profile (if any) and uninstall its config entries
    if let Ok(current_profile_name) = get_active_profile() {
        if let Some(current_profile) = get_profile(&current_profile_name)? {
            let config_dir = current_profile.path.join("config");
            let old_config = ProfileConfiguration::new(config_dir, home_dir.clone());
            old_config
                .uninstall()
                .context("Failed to uninstall old profile")?;
        }
    }

    // Install new profile's config entries
    let new_config_dir = new_profile.path.join("config");
    let new_config = ProfileConfiguration::new(new_config_dir, home_dir);
    new_config
        .install()
        .context("Failed to install new profile")?;

    // Update active profile in config
    set_active_profile(name)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;
    use tempfile::TempDir;

    // Lock to ensure tests don't interfere with each other's env vars
    static TEST_LOCK: Mutex<()> = Mutex::new(());

    struct TestEnv {
        _temp_dir: TempDir,
        _lock: std::sync::MutexGuard<'static, ()>,
    }

    fn setup_test_env() -> TestEnv {
        // Lock ensures only one test modifies XDG_CONFIG_HOME at a time
        let lock = TEST_LOCK.lock().unwrap();

        let temp_dir = TempDir::new().unwrap();
        std::env::set_var("XDG_CONFIG_HOME", temp_dir.path());

        TestEnv {
            _temp_dir: temp_dir,
            _lock: lock,
        }
    }

    impl Drop for TestEnv {
        fn drop(&mut self) {
            // Clean up env var when test completes
            std::env::remove_var("XDG_CONFIG_HOME");
        }
    }

    #[test]
    fn test_config_default() {
        let config = DevspaceConfig::default();
        assert_eq!(config.active_profile, "default");
    }

    #[test]
    fn test_read_write_config() {
        let _temp = setup_test_env();

        let config = DevspaceConfig {
            active_profile: "test-profile".to_string(),
        };

        write_config(&config).unwrap();
        let read = read_config().unwrap();

        assert_eq!(read.active_profile, "test-profile");
    }

    #[test]
    fn test_read_config_creates_default() {
        let _temp = setup_test_env();

        let config = read_config().unwrap();
        assert_eq!(config.active_profile, "default");

        // Verify file was created
        let config_path = config_file().unwrap();
        assert!(config_path.exists());
    }

    #[test]
    fn test_get_active_profile_from_config() {
        let _temp = setup_test_env();

        let config = DevspaceConfig {
            active_profile: "my-profile".to_string(),
        };
        write_config(&config).unwrap();

        let active = get_active_profile().unwrap();
        assert_eq!(active, "my-profile");
    }

    #[test]
    fn test_get_active_profile_from_env() {
        let _temp = setup_test_env();

        // Set env var
        std::env::set_var("DEVSPACE_PROFILE", "env-profile");

        let active = get_active_profile().unwrap();
        assert_eq!(active, "env-profile");

        // Clean up
        std::env::remove_var("DEVSPACE_PROFILE");
    }

    #[test]
    fn test_set_active_profile() {
        let _temp = setup_test_env();

        set_active_profile("new-profile").unwrap();

        let config = read_config().unwrap();
        assert_eq!(config.active_profile, "new-profile");
    }

    #[test]
    fn test_list_profiles_empty() {
        let _temp = setup_test_env();

        let profiles = list_profiles().unwrap();
        assert!(profiles.is_empty());
    }

    #[test]
    fn test_list_profiles() {
        let _temp = setup_test_env();

        // Create some test profiles
        let profiles_path = profiles_dir().unwrap();
        fs::create_dir_all(profiles_path.join("profile1")).unwrap();
        fs::create_dir_all(profiles_path.join("profile2")).unwrap();
        fs::create_dir_all(profiles_path.join("profile3")).unwrap();

        let profiles = list_profiles().unwrap();
        assert_eq!(profiles.len(), 3);
        assert_eq!(profiles[0].name, "profile1");
        assert_eq!(profiles[1].name, "profile2");
        assert_eq!(profiles[2].name, "profile3");
    }

    #[test]
    fn test_get_profile_exists() {
        let _temp = setup_test_env();

        let profiles_path = profiles_dir().unwrap();
        fs::create_dir_all(profiles_path.join("test-profile")).unwrap();

        let profile = get_profile("test-profile").unwrap();
        assert!(profile.is_some());
        assert_eq!(profile.unwrap().name, "test-profile");
    }

    #[test]
    fn test_get_profile_not_exists() {
        let _temp = setup_test_env();

        let profile = get_profile("nonexistent").unwrap();
        assert!(profile.is_none());
    }

    #[test]
    fn test_create_profile() {
        let _temp = setup_test_env();

        let profile = create_profile("test-profile").unwrap();
        assert_eq!(profile.name, "test-profile");
        assert!(profile.path.exists());

        // Verify directory structure
        assert!(profile.path.join("config").exists());
        assert!(profile.path.join("config/.gitkeep").exists());
        assert!(profile.path.join("config/.devspaceignore").exists());
        assert!(profile.path.join("manifests").exists());
        assert!(profile.path.join("manifests/cli.toml").exists());
        assert!(profile.path.join("manifests/macos.toml").exists());
        assert!(profile.path.join("README.md").exists());
        assert!(profile.path.join(".git").exists());
    }

    #[test]
    fn test_create_profile_already_exists() {
        let _temp = setup_test_env();

        create_profile("test-profile").unwrap();

        // Try to create again
        let result = create_profile("test-profile");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("already exists"));
    }

    #[test]
    fn test_create_profile_templates_have_content() {
        let _temp = setup_test_env();

        let profile = create_profile("test-profile").unwrap();

        // Check cli.toml has example content
        let cli_toml = fs::read_to_string(profile.path.join("manifests/cli.toml")).unwrap();
        assert!(cli_toml.contains("ripgrep"));
        assert!(cli_toml.contains("rustup"));

        // Check macos.toml has example content
        let macos_toml = fs::read_to_string(profile.path.join("manifests/macos.toml")).unwrap();
        assert!(macos_toml.contains("ghostty"));

        // Check README has profile name
        let readme = fs::read_to_string(profile.path.join("README.md")).unwrap();
        assert!(readme.contains("test-profile"));
    }

    #[test]
    fn test_switch_profile() {
        let _temp = setup_test_env();

        // Create two profiles
        create_profile("profile1").unwrap();
        create_profile("profile2").unwrap();

        // Create a dotfile in profile1
        let profile1 = get_profile("profile1").unwrap().unwrap();
        fs::write(profile1.path.join("config/.zshrc"), "profile1 config").unwrap();

        // Create a dotfile in profile2
        let profile2 = get_profile("profile2").unwrap().unwrap();
        fs::write(profile2.path.join("config/.zshrc"), "profile2 config").unwrap();

        // Switch to profile1
        switch_profile("profile1").unwrap();

        // Verify active profile
        assert_eq!(get_active_profile().unwrap(), "profile1");

        // Note: We can't test actual symlink creation to ~ without mocking
        // because tests use a temp directory for XDG_CONFIG_HOME
        // The symlink logic is tested in the symlinks module
    }

    #[test]
    fn test_switch_profile_nonexistent() {
        let _temp = setup_test_env();

        let result = switch_profile("nonexistent");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("does not exist"));
    }
}
