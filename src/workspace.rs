use anyhow::{Context, Result};
use std::env;
use std::fs;
use std::path::PathBuf;

use crate::config::Config;
use crate::environment::{Environment, Shell};
use crate::lockfile::Lockfile;

/// Template file definition for workspace initialization
struct TemplateFile {
    /// Path relative to workspace root (e.g., "config/zsh/.zshrc")
    path: &'static str,
    /// File content embedded at compile time
    content: &'static str,
}

/// All template files embedded at compile time
///
/// Templates are loaded from templates/profile/ directory and embedded
/// into the binary. To add a new template file, add it to this array.
const TEMPLATE_FILES: &[TemplateFile] = &[
    TemplateFile {
        path: ".dwsignore",
        content: include_str!("../templates/profile/.dwsignore"),
    },
    TemplateFile {
        path: ".gitignore",
        content: include_str!("../templates/profile/.gitignore"),
    },
    TemplateFile {
        path: "README.md",
        content: include_str!("../templates/profile/README.md"),
    },
    TemplateFile {
        path: "manifests/cli.toml",
        content: include_str!("../templates/profile/cli.toml"),
    },
    TemplateFile {
        path: "manifests/macos.toml",
        content: include_str!("../templates/profile/macos.toml"),
    },
    TemplateFile {
        path: "config/zsh/.zshrc",
        content: include_str!("../templates/profile/config/zsh/.zshrc"),
    },
    TemplateFile {
        path: "config/bash/.bashrc",
        content: include_str!("../templates/profile/config/bash/.bashrc"),
    },
    TemplateFile {
        path: "config/fish/config.fish",
        content: include_str!("../templates/profile/config/fish/config.fish"),
    },
];

/// Workspace path types
#[derive(Debug, Clone, Copy)]
pub enum WorkspacePath {
    /// Workspace root: $XDG_CONFIG_HOME/dws
    Root,
    /// Config directory: workspace/config
    Config,
    /// Manifests directory: workspace/manifests
    Manifests,
    /// Bin directory: $XDG_STATE_HOME/dws/bin
    Bin,
    /// Share directory: $XDG_STATE_HOME/dws/share
    Share,
    /// Lockfile path: $XDG_STATE_HOME/dws/dws.lock
    Lockfile,
}

/// Workspace - represents the dws installation
///
/// The workspace is rooted at $XDG_CONFIG_HOME/dws and represents your dotfiles.
/// No profiles, no switching - this IS your environment.
#[derive(Debug)]
pub struct Workspace {
    /// Workspace root: $XDG_CONFIG_HOME/dws (version controlled)
    workspace_dir: PathBuf,
    /// State directory: $XDG_STATE_HOME/dws (local execution state)
    state_dir: PathBuf,
}

impl Workspace {
    /// Create a new Workspace
    ///
    /// Initializes workspace with XDG-compliant directories:
    /// - Workspace: $XDG_CONFIG_HOME/dws (default: ~/.config/dws)
    /// - State: $XDG_STATE_HOME/dws (default: ~/.local/state/dws)
    pub fn new() -> Result<Self> {
        let workspace_dir = Self::get_workspace_dir()?;
        let state_dir = Self::get_state_dir()?;

        Ok(Self {
            workspace_dir,
            state_dir,
        })
    }

    /// Get the workspace directory (XDG_CONFIG_HOME/dws)
    fn get_workspace_dir() -> Result<PathBuf> {
        let base = env::var("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| {
                directories::BaseDirs::new()
                    .expect("Failed to get home directory")
                    .home_dir()
                    .join(".config")
            });

        Ok(base.join("dws"))
    }

    /// Get the state directory (XDG_STATE_HOME/dws)
    fn get_state_dir() -> Result<PathBuf> {
        let base = env::var("XDG_STATE_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| {
                directories::BaseDirs::new()
                    .expect("Failed to get home directory")
                    .home_dir()
                    .join(".local/state")
            });

        Ok(base.join("dws"))
    }

    /// Get path for a specific workspace location
    pub fn path(&self, path_type: WorkspacePath) -> PathBuf {
        match path_type {
            WorkspacePath::Root => self.workspace_dir.clone(),
            WorkspacePath::Config => self.workspace_dir.join("config"),
            WorkspacePath::Manifests => self.workspace_dir.join("manifests"),
            WorkspacePath::Bin => self.state_dir.join("bin"),
            WorkspacePath::Share => self.state_dir.join("share"),
            WorkspacePath::Lockfile => self.state_dir.join("dws.lock"),
        }
    }

    /// Check if workspace exists (has been initialized)
    pub fn exists(&self) -> bool {
        self.workspace_dir.exists()
    }

    /// Initialize workspace and shell integration
    pub fn init(&self, repository: Option<&str>, shell: &str) -> Result<()> {
        let exists = self.workspace_dir.exists();

        match (exists, repository) {
            (false, None) => {
                self.create()?;
            }

            (false, Some(repo)) => {
                self.clone(repo)?;
            }

            (true, None) => {
                println!("Using existing workspace at {:?}", self.workspace_dir);
            }

            (true, Some(repo)) => {
                self.verify_url(repo)?;
                println!("Using existing workspace at {:?}", self.workspace_dir);
            }
        }

        println!("Installing workspace...");
        self.install()
            .context("Failed to install workspace")?;

        self.setup(shell)
            .with_context(|| format!("Failed to setup shell integration for {}", shell))?;

        println!("\n✓ Workspace initialized successfully!");
        println!("  Shell: {}", shell);
        println!("  Config: {:?}", self.workspace_dir);
        println!("\nRun 'exec $SHELL' to reload your shell.");

        Ok(())
    }

    /// Verify that the workspace's git remote URL matches the provided repository
    fn verify_url(&self, expected_repo: &str) -> Result<()> {
        let expected_url = Self::canonical_url(expected_repo);

        let repo = git2::Repository::open(&self.workspace_dir)
            .context("Workspace exists but is not a git repository")?;

        let remote = repo.find_remote("origin")
            .context("Workspace git repository has no 'origin' remote")?;

        let actual_url = remote.url()
            .ok_or_else(|| anyhow::anyhow!("Origin remote has no URL"))?;

        let actual_normalized = Self::canonical_url(actual_url);

        if expected_url != actual_normalized {
            anyhow::bail!(
                "Workspace repository URL mismatch\n  Expected: {}\n  Actual:   {}\n\nThe workspace at {:?} was cloned from a different repository.",
                expected_url,
                actual_normalized,
                self.workspace_dir
            );
        }

        Ok(())
    }

    /// Convert repository identifier to canonical URL
    ///
    /// Handles multiple input formats and normalizes to a canonical form:
    /// - GitHub shorthand: "user/repo" -> "https://github.com/user/repo.git"
    /// - Full URLs: normalized (removes trailing slashes, ensures .git suffix)
    /// - Other formats: passed through as-is
    fn canonical_url(repository: &str) -> String {
        let url = if repository.contains("://") {
            // Full URL - use as-is
            repository.to_string()
        } else if repository.contains('/') && !repository.contains('.') {
            // GitHub shorthand: user/repo
            format!("https://github.com/{}.git", repository)
        } else {
            // Other format - pass through
            repository.to_string()
        };

        // Normalize: remove trailing slash, ensure .git suffix for https/http
        let url = url.trim_end_matches('/');

        if (url.starts_with("https://") || url.starts_with("http://")) && !url.ends_with(".git") {
            format!("{}.git", url)
        } else {
            url.to_string()
        }
    }

    /// Create workspace from template
    fn create(&self) -> Result<()> {
        println!("Creating template workspace at {:?}...", self.workspace_dir);

        fs::create_dir_all(&self.workspace_dir)
            .with_context(|| format!("Failed to create workspace directory {:?}", self.workspace_dir))?;

        for template in TEMPLATE_FILES {
            let file_path = self.workspace_dir.join(template.path);

            if let Some(parent) = file_path.parent() {
                fs::create_dir_all(parent)
                    .with_context(|| format!("Failed to create directory {:?}", parent))?;
            }

            fs::write(&file_path, template.content)
                .with_context(|| format!("Failed to write template file {:?}", file_path))?;
        }

        println!("✓ Template created");
        Ok(())
    }

    /// Clone workspace from git repository
    fn clone(&self, repository: &str) -> Result<()> {
        let url = Self::canonical_url(repository);

        println!("Cloning repository: {}", url);

        git2::Repository::clone(&url, &self.workspace_dir)
            .with_context(|| format!("Failed to clone repository {} to {:?}", url, self.workspace_dir))?;

        println!("✓ Repository cloned");
        Ok(())
    }

    /// Setup shell integration by adding dws env to shell rc files
    pub fn setup(&self, shell: &str) -> Result<()> {
        println!("Setting up {} integration...", shell);

        let home = directories::BaseDirs::new()
            .ok_or_else(|| anyhow::anyhow!("Could not determine home directory"))?
            .home_dir()
            .to_path_buf();

        match shell {
            "zsh" => {
                let zshenv = home.join(".zshenv");
                Self::add_shell_integration(&zshenv, "eval \"$(dws env --shell zsh)\"")?;
            }
            "bash" => {
                let bashrc = home.join(".bashrc");
                Self::add_shell_integration(&bashrc, "eval \"$(dws env --shell bash)\"")?;
            }
            "fish" => {
                let config_fish = home.join(".config/fish/config.fish");
                // Create parent directory if it doesn't exist
                if let Some(parent) = config_fish.parent() {
                    fs::create_dir_all(parent)?;
                }
                Self::add_shell_integration(&config_fish, "dws env --shell fish | source")?;
            }
            _ => {
                anyhow::bail!("Unsupported shell: {}", shell);
            }
        }

        println!("✓ Shell integration added to {}", shell);
        Ok(())
    }

    /// Add integration line to shell rc file (idempotent)
    fn add_shell_integration(rc_file: &PathBuf, integration_line: &str) -> Result<()> {
        // Read existing content (or empty if file doesn't exist)
        let existing_content = if rc_file.exists() {
            fs::read_to_string(rc_file)
                .with_context(|| format!("Failed to read {:?}", rc_file))?
        } else {
            String::new()
        };

        // Check if integration is already present
        if existing_content.contains(integration_line) {
            println!("  Shell integration already present in {:?}", rc_file);
            return Ok(());
        }

        // Append integration line
        let new_content = if existing_content.is_empty() {
            format!("# dws shell integration\n{}\n", integration_line)
        } else if existing_content.ends_with('\n') {
            format!("{}# dws shell integration\n{}\n", existing_content, integration_line)
        } else {
            format!("{}\n# dws shell integration\n{}\n", existing_content, integration_line)
        };

        fs::write(rc_file, new_content)
            .with_context(|| format!("Failed to write {:?}", rc_file))?;

        Ok(())
    }

    /// Get the Config for managing dotfiles
    pub fn config(&self) -> Result<Config> {
        let config_dir = self.path(WorkspacePath::Config);

        // Target is $XDG_CONFIG_HOME (default: ~/.config)
        let target_dir = env::var("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| {
                directories::BaseDirs::new()
                    .expect("Failed to get home directory")
                    .home_dir()
                    .join(".config")
            });

        Ok(Config::new(config_dir, target_dir))
    }

    /// Get the Environment for shell integration
    pub fn environment(&self, shell: Shell) -> Result<Environment> {
        Environment::new_from_workspace(self, shell)
    }

    /// Install the workspace (symlink configs, install tools)
    pub fn install(&self) -> Result<()> {
        // Load or create lockfile
        let lockfile_path = self.path(WorkspacePath::Lockfile);
        let mut lockfile = if lockfile_path.exists() {
            // Cleanup existing installation first
            let old_lockfile = Lockfile::load(&lockfile_path)?;
            self.cleanup(&old_lockfile)?;
            Lockfile::new()
        } else {
            Lockfile::new()
        };

        // Install config entries and record in lockfile
        let config = self.config()?;
        let config_entries = config.discover_entries()?;

        for entry in &config_entries {
            entry.install()?;
            lockfile.add_config_symlink(entry.source.clone(), entry.target.clone());
        }

        // TODO: Install tools from manifests and add to lockfile

        // Save lockfile
        lockfile.save(&lockfile_path)?;

        Ok(())
    }

    /// Uninstall the workspace (remove all symlinks)
    pub fn uninstall(&self) -> Result<()> {
        let lockfile_path = self.path(WorkspacePath::Lockfile);

        if !lockfile_path.exists() {
            return Ok(());
        }

        let lockfile = Lockfile::load(&lockfile_path)?;
        self.cleanup(&lockfile)?;

        // Remove lockfile
        fs::remove_file(&lockfile_path)
            .with_context(|| format!("Failed to remove lockfile {:?}", lockfile_path))?;

        Ok(())
    }

    /// Remove all symlinks tracked in the lockfile
    fn cleanup(&self, lockfile: &Lockfile) -> Result<()> {
        // Remove config symlinks
        for entry in lockfile.config_symlinks() {
            if entry.target.exists() || entry.target.symlink_metadata().is_ok() {
                fs::remove_file(&entry.target).with_context(|| {
                    format!("Failed to remove config symlink {:?}", entry.target)
                })?;
            }
        }

        // Remove tool symlinks
        for entry in lockfile.tool_symlinks() {
            if entry.target.exists() || entry.target.symlink_metadata().is_ok() {
                fs::remove_file(&entry.target).with_context(|| {
                    format!("Failed to remove tool symlink {:?}", entry.target)
                })?;
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;
    use serial_test::serial;
    use tempfile::TempDir;

    fn setup_test_env() -> TempDir {
        let temp = TempDir::new().unwrap();
        env::set_var("XDG_CONFIG_HOME", temp.path());
        env::set_var("XDG_STATE_HOME", temp.path().join("state"));
        temp
    }

    #[test]
    #[serial]
    fn test_workspace_new() {
        let _temp = setup_test_env();
        let workspace = Workspace::new().unwrap();

        assert!(workspace.workspace_dir.to_string_lossy().contains("dws"));
        assert!(workspace.state_dir.to_string_lossy().contains("dws"));
    }

    #[test]
    #[serial]
    fn test_workspace_paths() {
        let _temp = setup_test_env();
        let workspace = Workspace::new().unwrap();

        assert!(workspace.path(WorkspacePath::Config).to_string_lossy().contains("config"));
        assert!(workspace.path(WorkspacePath::Manifests).to_string_lossy().contains("manifests"));
        assert!(workspace.path(WorkspacePath::Lockfile).to_string_lossy().contains("dws.lock"));
        assert!(workspace.path(WorkspacePath::Bin).to_string_lossy().contains("bin"));
    }

    #[test]
    #[serial]
    fn test_workspace_install() {
        let _temp = setup_test_env();
        let workspace = Workspace::new().unwrap();

        // Create workspace structure
        let config_dir = workspace.path(WorkspacePath::Config);
        fs::create_dir_all(&config_dir).unwrap();
        fs::create_dir_all(config_dir.join("zsh")).unwrap();
        fs::write(config_dir.join("zsh/.zshrc"), "test").unwrap();

        // Install
        workspace.install().unwrap();

        // Verify lockfile was created
        assert!(workspace.path(WorkspacePath::Lockfile).exists());

        // Verify lockfile contents
        let lockfile = Lockfile::load(&workspace.path(WorkspacePath::Lockfile)).unwrap();
        assert_eq!(lockfile.config_symlinks.len(), 1);
    }

    #[test]
    #[serial]
    fn test_workspace_uninstall() {
        let _temp = setup_test_env();
        let workspace = Workspace::new().unwrap();

        // Create and install workspace
        let config_dir = workspace.path(WorkspacePath::Config);
        fs::create_dir_all(&config_dir).unwrap();
        fs::create_dir_all(config_dir.join("zsh")).unwrap();
        fs::write(config_dir.join("zsh/.zshrc"), "test").unwrap();

        workspace.install().unwrap();

        // Get XDG_CONFIG_HOME
        let xdg_config = env::var("XDG_CONFIG_HOME").unwrap();
        let config_home = PathBuf::from(xdg_config);

        // Verify symlink exists
        assert!(config_home.join("zsh").exists());

        // Uninstall
        workspace.uninstall().unwrap();

        // Verify symlink removed
        assert!(!config_home.join("zsh").exists());

        // Verify lockfile removed
        assert!(!workspace.path(WorkspacePath::Lockfile).exists());
    }

    #[test]
    #[serial]
    fn test_init_new_workspace_without_url() {
        let _temp = setup_test_env();
        let workspace = Workspace::new().unwrap();

        assert!(!workspace.workspace_dir.exists());

        workspace.init(None, "zsh").unwrap();

        assert!(workspace.workspace_dir.exists());
        assert!(workspace.path(WorkspacePath::Config).exists());
        assert!(workspace.path(WorkspacePath::Manifests).exists());
        assert!(workspace.path(WorkspacePath::Config).join("zsh/.zshrc").exists());
        assert!(workspace.workspace_dir.join("README.md").exists());
    }

    #[test]
    #[serial]
    fn test_init_new_workspace_with_url() {
        let _temp = setup_test_env();
        let workspace = Workspace::new().unwrap();

        let source_temp = TempDir::new().unwrap();
        let source_repo_path = source_temp.path().join("test-repo");
        let repo = git2::Repository::init(&source_repo_path).unwrap();

        let readme_path = source_repo_path.join("README.md");
        fs::write(&readme_path, "test content").unwrap();

        let mut index = repo.index().unwrap();
        index.add_path(std::path::Path::new("README.md")).unwrap();
        let tree_id = index.write_tree().unwrap();

        let tree = repo.find_tree(tree_id).unwrap();
        let sig = git2::Signature::now("Test", "test@example.com").unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[]).unwrap();

        assert!(!workspace.workspace_dir.exists());

        let repo_url = format!("file://{}", source_repo_path.display());
        workspace.init(Some(&repo_url), "bash").unwrap();

        assert!(workspace.workspace_dir.exists());
        assert!(workspace.workspace_dir.join("README.md").exists());
    }

    #[test]
    #[serial]
    fn test_init_existing_workspace_without_url() {
        let _temp = setup_test_env();
        let workspace = Workspace::new().unwrap();

        workspace.create().unwrap();
        assert!(workspace.workspace_dir.exists());

        let result = workspace.init(None, "bash");

        assert!(result.is_ok());
        assert!(workspace.workspace_dir.exists());
    }

    #[test]
    #[serial]
    fn test_init_existing_workspace_with_matching_url() {
        let _temp = setup_test_env();
        let workspace = Workspace::new().unwrap();

        let source_temp = TempDir::new().unwrap();
        let source_repo_path = source_temp.path().join("test-repo");
        git2::Repository::init(&source_repo_path).unwrap();

        let repo_url = format!("file://{}", source_repo_path.display());
        workspace.clone(&repo_url).unwrap();
        assert!(workspace.workspace_dir.exists());

        let result = workspace.init(Some(&repo_url), "zsh");
        assert!(result.is_ok());
    }

    #[test]
    #[serial]
    fn test_init_existing_workspace_with_mismatched_url() {
        let _temp = setup_test_env();
        let workspace = Workspace::new().unwrap();

        let source_temp = TempDir::new().unwrap();
        let source_repo_path = source_temp.path().join("test-repo");
        git2::Repository::init(&source_repo_path).unwrap();

        let repo_url = format!("file://{}", source_repo_path.display());
        workspace.clone(&repo_url).unwrap();

        let different_url = "file:///different/repo";
        let result = workspace.init(Some(different_url), "zsh");

        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("mismatch"));
    }

    /// Test canonical URL normalization
    ///
    /// Verifies that various URL formats are normalized correctly:
    /// - GitHub shorthand (user/repo)
    /// - Full URLs with and without .git suffix
    /// - URLs with trailing slashes
    /// - SSH URLs (passed through)
    /// - File URLs (passed through)
    #[rstest]
    #[case("user/repo", "https://github.com/user/repo.git")]
    #[case("octocat/Hello-World", "https://github.com/octocat/Hello-World.git")]
    #[case("https://github.com/user/repo", "https://github.com/user/repo.git")]
    #[case("https://github.com/user/repo.git", "https://github.com/user/repo.git")]
    #[case("https://github.com/user/repo/", "https://github.com/user/repo.git")]
    #[case("http://github.com/user/repo", "http://github.com/user/repo.git")]
    #[case("https://gitlab.com/user/repo", "https://gitlab.com/user/repo.git")]
    #[case("https://bitbucket.org/user/repo", "https://bitbucket.org/user/repo.git")]
    #[case("git@github.com:user/repo.git", "git@github.com:user/repo.git")]
    #[case("git@gitlab.com:user/repo.git", "git@gitlab.com:user/repo.git")]
    #[case("file:///path/to/repo", "file:///path/to/repo")]
    #[case("file:///path/to/repo.git", "file:///path/to/repo.git")]
    fn test_canonical_url(#[case] input: &str, #[case] expected: &str) {
        assert_eq!(Workspace::canonical_url(input), expected);
    }

    #[test]
    #[serial]
    fn test_create() {
        let _temp = setup_test_env();
        let workspace = Workspace::new().unwrap();

        assert!(!workspace.workspace_dir.exists());

        workspace.create().unwrap();

        assert!(workspace.workspace_dir.exists());
        assert!(workspace.workspace_dir.join("README.md").exists());
        assert!(workspace.workspace_dir.join(".gitignore").exists());
        assert!(workspace.workspace_dir.join(".dwsignore").exists());
        assert!(workspace.path(WorkspacePath::Config).join("zsh/.zshrc").exists());
        assert!(workspace.path(WorkspacePath::Config).join("bash/.bashrc").exists());
        assert!(workspace.path(WorkspacePath::Config).join("fish/config.fish").exists());
        assert!(workspace.path(WorkspacePath::Manifests).join("cli.toml").exists());
        assert!(workspace.path(WorkspacePath::Manifests).join("macos.toml").exists());
    }

    #[test]
    #[serial]
    fn test_clone() {
        let _temp = setup_test_env();
        let workspace = Workspace::new().unwrap();

        let source_temp = TempDir::new().unwrap();
        let source_repo_path = source_temp.path().join("test-repo");
        let repo = git2::Repository::init(&source_repo_path).unwrap();

        let test_file = source_repo_path.join("test.txt");
        fs::write(&test_file, "test content").unwrap();

        let mut index = repo.index().unwrap();
        index.add_path(std::path::Path::new("test.txt")).unwrap();
        let tree_id = index.write_tree().unwrap();

        let tree = repo.find_tree(tree_id).unwrap();
        let sig = git2::Signature::now("Test", "test@example.com").unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[]).unwrap();

        assert!(!workspace.workspace_dir.exists());

        let repo_url = format!("file://{}", source_repo_path.display());
        workspace.clone(&repo_url).unwrap();

        assert!(workspace.workspace_dir.exists());
        assert!(workspace.workspace_dir.join("test.txt").exists());
        assert!(workspace.workspace_dir.join(".git").exists());
    }

    #[test]
    #[serial]
    fn test_verify_url_matches() {
        let _temp = setup_test_env();
        let workspace = Workspace::new().unwrap();

        let source_temp = TempDir::new().unwrap();
        let source_repo_path = source_temp.path().join("test-repo");
        git2::Repository::init(&source_repo_path).unwrap();

        let repo_url = format!("file://{}", source_repo_path.display());
        workspace.clone(&repo_url).unwrap();

        let result = workspace.verify_url(&repo_url);
        assert!(result.is_ok());
    }

    #[test]
    #[serial]
    fn test_verify_url_mismatch() {
        let _temp = setup_test_env();
        let workspace = Workspace::new().unwrap();

        let source_temp = TempDir::new().unwrap();
        let source_repo_path = source_temp.path().join("test-repo");
        git2::Repository::init(&source_repo_path).unwrap();

        let repo_url = format!("file://{}", source_repo_path.display());
        workspace.clone(&repo_url).unwrap();

        let different_url = "file:///different/path";
        let result = workspace.verify_url(different_url);

        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("mismatch"));
    }

    #[test]
    #[serial]
    fn test_workspace_reinstall() {
        let _temp = setup_test_env();
        let workspace = Workspace::new().unwrap();

        // Create workspace with first config
        let config_dir = workspace.path(WorkspacePath::Config);
        fs::create_dir_all(&config_dir).unwrap();
        fs::create_dir_all(config_dir.join("zsh")).unwrap();
        fs::write(config_dir.join("zsh/.zshrc"), "test1").unwrap();

        workspace.install().unwrap();

        // Change config
        fs::remove_dir_all(config_dir.join("zsh")).unwrap();
        fs::create_dir_all(config_dir.join("fish")).unwrap();
        fs::write(config_dir.join("fish/config.fish"), "test2").unwrap();

        // Reinstall
        workspace.install().unwrap();

        // Get XDG_CONFIG_HOME
        let xdg_config = env::var("XDG_CONFIG_HOME").unwrap();
        let config_home = PathBuf::from(xdg_config);

        // Verify old symlink removed, new one created
        assert!(!config_home.join("zsh").exists());
        assert!(config_home.join("fish").exists());
    }
}
