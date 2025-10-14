use anyhow::{Context, Result};
use chrono::Utc;
use git2::{
    build::CheckoutBuilder, ErrorCode, Object, ObjectType, Repository, ResetType, Status,
    StatusOptions,
};
use std::collections::{HashMap, HashSet};
use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;

use crate::config::{default_profile_name, Config};
use crate::dotfiles::Dotfiles;
use crate::environment::{Environment, Shell};
use crate::installers::{self, InstallContext, ToolInstaller};
use crate::lockfile::{Lockfile, ToolEntry as LockfileToolEntry};
use crate::profile::Profile;
use crate::toolset::{ToolDefinition, ToolSet};
use crate::ui::{self, Progress};
use tokio::runtime::Runtime;

/// Template file definition for workspace initialization
struct TemplateFile {
    /// Path relative to a profile root (e.g., "config/zsh/.zshrc")
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
        path: "config/.dwsignore",
        content: include_str!("../templates/profile/config/.dwsignore"),
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
        path: "config.toml",
        content: include_str!("../templates/profile/config.toml"),
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
    /// Profiles directory: $XDG_CONFIG_HOME/dws/profiles
    Profiles,
    /// Active profile root: $XDG_CONFIG_HOME/dws/profiles/<profile>
    Profile,
    /// Config directory: active profile config
    Config,
    /// Profile `config.toml` file
    ProfileConfig,
    /// Bin directory: $XDG_STATE_HOME/dws/bin
    Bin,
    /// Share directory: $XDG_STATE_HOME/dws/share
    Share,
    /// Lockfile path: $XDG_STATE_HOME/dws/dws.lock
    Lockfile,
    /// Cache directory: $XDG_CACHE_HOME/dws
    Cache,
    /// Workspace config file path
    ConfigFile,
}

struct ToolInstallTask {
    name: String,
    resolved_version: Option<String>,
    installer: Box<dyn ToolInstaller>,
}

struct UpdatedTool {
    name: String,
    version: Option<String>,
}

#[derive(Debug, Clone)]
pub struct EnvironmentExport {
    pub shell: Shell,
    pub script: String,
    pub defaulted: bool,
}

/// Workspace - represents the dws installation
///
/// The workspace is rooted at $XDG_CONFIG_HOME/dws and represents your dotfiles.
#[derive(Debug)]
pub struct Workspace {
    /// Workspace root: $XDG_CONFIG_HOME/dws (version controlled)
    workspace_dir: PathBuf,
    /// Profiles directory: $XDG_CONFIG_HOME/dws/profiles
    profiles_dir: PathBuf,
    /// State directory: $XDG_STATE_HOME/dws (local execution state)
    state_dir: PathBuf,
    /// Cache directory: $XDG_CACHE_HOME/dws (downloaded artifacts)
    cache_dir: PathBuf,
    /// Path to workspace config file
    config_path: PathBuf,
    /// Loaded workspace configuration
    workspace_config: Config,
    /// Currently active profile
    active_profile: Profile,
}

impl Workspace {
    /// Create a new Workspace
    ///
    /// Initializes workspace with XDG-compliant directories:
    /// - Workspace: $XDG_CONFIG_HOME/dws (default: ~/.config/dws)
    /// - State: $XDG_STATE_HOME/dws (default: ~/.local/state/dws)
    pub fn new() -> Result<Self> {
        let workspace_dir = Self::get_workspace_dir()?;
        let profiles_dir = workspace_dir.join("profiles");
        let state_dir = Self::get_state_dir()?;
        let cache_dir = Self::get_cache_dir()?;
        let config_path = workspace_dir.join("config.toml");

        let workspace_config = Config::load(&config_path)?;
        let active_name = workspace_config.active_profile().to_string();
        let active_profile = Profile::new(active_name.clone(), profiles_dir.join(&active_name));

        Ok(Self {
            workspace_dir,
            profiles_dir,
            state_dir,
            cache_dir,
            config_path,
            workspace_config,
            active_profile,
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

    /// Get the cache directory (XDG_CACHE_HOME/dws)
    fn get_cache_dir() -> Result<PathBuf> {
        let base = env::var("XDG_CACHE_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| {
                directories::BaseDirs::new()
                    .expect("Failed to get home directory")
                    .cache_dir()
                    .to_path_buf()
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
            WorkspacePath::Profiles => self.profiles_dir.clone(),
            WorkspacePath::Profile => self.active_profile.root().to_path_buf(),
            WorkspacePath::Config => self.active_profile.config_dir(),
            WorkspacePath::ProfileConfig => self.active_profile.config_file(),
            WorkspacePath::Bin => self.state_dir.join("bin"),
            WorkspacePath::Share => self.state_dir.join("share"),
            WorkspacePath::Lockfile => self.state_dir.join("dws.lock"),
            WorkspacePath::Cache => self.cache_dir.clone(),
            WorkspacePath::ConfigFile => self.config_path.clone(),
        }
    }

    /// Check if workspace exists (has been initialized)
    pub fn exists(&self) -> bool {
        self.active_profile.root().exists()
    }

    /// Access the active profile
    pub fn active_profile(&self) -> &Profile {
        &self.active_profile
    }

    pub fn active_profile_name(&self) -> &str {
        self.workspace_config.active_profile()
    }

    pub fn init_with_shell(
        &mut self,
        repository: Option<&str>,
        shell: Option<&str>,
        profile: Option<&str>,
    ) -> Result<()> {
        let shell = match shell {
            Some(name) => name.to_string(),
            None => Self::detect_shell_from_env()?,
        };

        self.init(repository, &shell, profile)
    }

    fn detect_shell_from_env() -> Result<String> {
        env::var("SHELL")
            .ok()
            .and_then(|s| s.rsplit('/').next().map(str::to_string))
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "SHELL environment variable not set. Please specify shell with --shell flag."
                )
            })
    }

    pub fn list_profiles(&self) -> Result<Vec<String>> {
        if !self.profiles_dir.exists() {
            return Ok(Vec::new());
        }

        let mut profiles = Vec::new();
        for entry in fs::read_dir(&self.profiles_dir)? {
            let entry = entry?;
            if entry.file_type()?.is_dir() {
                if let Some(name) = entry.file_name().to_str() {
                    profiles.push(name.to_string());
                }
            }
        }
        profiles.sort();
        Ok(profiles)
    }

    pub fn clone_into_profile(
        &self,
        repository: &str,
        profile_name: Option<&str>,
    ) -> Result<String> {
        let name = profile_name
            .map(|s| s.to_string())
            .unwrap_or_else(|| Self::profile_name_from_repository(repository));
        let profile = Profile::new(name.clone(), self.profile_path(&name));
        self.clone_profile(repository, &profile)?;
        Ok(name)
    }

    pub fn use_profile(&mut self, profile_name: &str) -> Result<()> {
        if profile_name == self.active_profile_name() {
            ui::info(format!("Profile '{profile_name}' is already active."));
            return Ok(());
        }

        let profile = Profile::new(profile_name.to_string(), self.profile_path(profile_name));
        if !profile.root().exists() {
            anyhow::bail!("Profile '{}' does not exist", profile_name);
        }

        let lockfile_path = self.path(WorkspacePath::Lockfile);
        if lockfile_path.exists() {
            let lockfile = Lockfile::load(&lockfile_path)?;
            self.remove_tracked_symlinks(&lockfile)?;
        }

        self.set_active_profile(profile_name.to_string())?;
        let progress = ui::Progress::new("Switching", format!("profile '{profile_name}'"));
        match self.install() {
            Ok(()) => progress.success("Finished", Some("active now".to_string())),
            Err(err) => {
                progress.fail("Failed", &err);
                return Err(err);
            }
        }
        Ok(())
    }

    fn profile_path(&self, profile: &str) -> PathBuf {
        self.profiles_dir.join(profile)
    }

    fn set_active_profile(&mut self, profile: impl Into<String>) -> Result<()> {
        let profile = profile.into();
        fs::create_dir_all(&self.profiles_dir).with_context(|| {
            format!(
                "Failed to create profiles directory {:?}",
                self.profiles_dir
            )
        })?;
        self.workspace_config.set_active_profile(profile.clone());
        self.active_profile = Profile::new(profile.clone(), self.profile_path(&profile));
        self.workspace_config.save(&self.config_path)?;
        Ok(())
    }

    /// Initialize workspace and shell integration
    pub fn init(
        &mut self,
        repository: Option<&str>,
        shell: &str,
        profile: Option<&str>,
    ) -> Result<()> {
        let target_name = if let Some(name) = profile {
            name.to_string()
        } else if let Some(repo) = repository {
            Self::profile_name_from_repository(repo)
        } else {
            self.workspace_config.active_profile().to_string()
        };

        let target_profile = Profile::new(target_name.clone(), self.profile_path(&target_name));

        if let Some(repo) = repository {
            if target_profile.root().exists() {
                Self::verify_profile_repo(&target_profile, repo)?;
                ui::status(
                    "Reusing",
                    format!("profile at {}", target_profile.root().display()),
                );
            } else {
                self.clone_profile(repo, &target_profile)?;
            }
        } else {
            self.ensure_profile_template(&target_profile)?;
        }

        self.set_active_profile(target_name)?;

        let install_progress = Progress::new(
            "Installing",
            format!("workspace for profile '{}'", self.active_profile_name()),
        );
        if let Err(err) = self.install() {
            install_progress.fail("Failed", &err);
            return Err(err).context("Failed to install workspace");
        }
        install_progress.success("Finished", Some("ready".to_string()));

        self.setup(shell)
            .with_context(|| format!("Failed to setup shell integration for {}", shell))?;

        ui::success(
            "Finished",
            format!(
                "workspace initialized (profile '{}', shell {})",
                self.active_profile_name(),
                shell
            ),
        );
        ui::info("Run 'exec $SHELL' to reload your shell.");

        Ok(())
    }

    /// Verify that the workspace's git remote URL matches the provided repository
    fn verify_profile_repo(profile: &Profile, expected_repo: &str) -> Result<()> {
        let expected_url = Self::canonical_url(expected_repo);

        let repo = git2::Repository::open(profile.root())
            .context("Workspace exists but profile is not a git repository")?;

        let remote = repo
            .find_remote("origin")
            .context("Workspace git repository has no 'origin' remote")?;

        let actual_url = remote
            .url()
            .ok_or_else(|| anyhow::anyhow!("Origin remote has no URL"))?;

        let actual_normalized = Self::canonical_url(actual_url);

        if expected_url != actual_normalized {
            anyhow::bail!(
                "Workspace repository URL mismatch\n  Expected: {}\n  Actual:   {}\n\nThe workspace at {:?} was cloned from a different repository.",
                expected_url,
                actual_normalized,
                profile.root()
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
        let trimmed = repository.trim();

        let url = if trimmed.contains("://") || trimmed.starts_with("git@") {
            // Full URL (https/ssh/etc.) - use as-is
            trimmed.to_string()
        } else if trimmed.split('/').count() == 2 {
            // GitHub shorthand: user/repo (allow dots in repo name)
            let repo = trimmed.trim_end_matches(".git");
            format!("https://github.com/{}.git", repo)
        } else {
            // Other format - pass through
            trimmed.to_string()
        };

        // Normalize: remove trailing slash, ensure .git suffix for https/http
        let url = url.trim_end_matches('/');

        if (url.starts_with("https://") || url.starts_with("http://")) && !url.ends_with(".git") {
            format!("{}.git", url)
        } else {
            url.to_string()
        }
    }

    fn profile_name_from_repository(repository: &str) -> String {
        let trimmed = repository
            .trim()
            .trim_end_matches('/')
            .trim_end_matches(".git");
        trimmed
            .rsplit(|c| ['/', ':'].contains(&c))
            .next()
            .map(|s| s.to_string())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| default_profile_name().to_string())
    }

    fn ensure_profile_template(&self, profile: &Profile) -> Result<()> {
        let verb = if profile.root().exists() {
            "Updating"
        } else {
            "Creating"
        };
        let progress = Progress::new(
            verb,
            format!("template profile at {}", profile.root().display()),
        );

        fs::create_dir_all(&self.workspace_dir).with_context(|| {
            format!(
                "Failed to create workspace directory {:?}",
                self.workspace_dir
            )
        })?;

        fs::create_dir_all(&self.profiles_dir).with_context(|| {
            format!(
                "Failed to create profiles directory {:?}",
                self.profiles_dir
            )
        })?;

        for template in TEMPLATE_FILES {
            let file_path = profile.root().join(template.path);

            if let Some(parent) = file_path.parent() {
                fs::create_dir_all(parent)
                    .with_context(|| format!("Failed to create directory {:?}", parent))?;
            }

            if !file_path.exists() {
                fs::write(&file_path, template.content)
                    .with_context(|| format!("Failed to write template file {:?}", file_path))?;
            }
        }

        progress.success("Prepared", None);
        Ok(())
    }

    fn clone_profile(&self, repository: &str, profile: &Profile) -> Result<()> {
        let url = Self::canonical_url(repository);

        let progress = Progress::new(
            "Cloning",
            format!("{} -> {}", url, profile.root().display()),
        );

        fs::create_dir_all(&self.profiles_dir).with_context(|| {
            format!(
                "Failed to create profiles directory {:?}",
                self.profiles_dir
            )
        })?;

        if profile.root().exists() {
            anyhow::bail!(
                "Profile '{}' already exists at {:?}",
                profile.name(),
                profile.root()
            );
        }

        match git2::Repository::clone(&url, profile.root()) {
            Ok(_) => {
                progress.success("Cloned", Some(format!("profile '{}'", profile.name())));
                Ok(())
            }
            Err(err) => {
                progress.fail("Failed", &err);
                Err(err).with_context(|| {
                    format!("Failed to clone repository {} to {:?}", url, profile.root())
                })
            }
        }
    }

    /// Setup shell integration by adding dws env to shell rc files
    pub fn setup(&self, shell: &str) -> Result<()> {
        let progress = Progress::new("Configuring", format!("{shell} shell integration"));

        let configure_result = (|| -> Result<bool> {
            let home = directories::BaseDirs::new()
                .ok_or_else(|| anyhow::anyhow!("Could not determine home directory"))?
                .home_dir()
                .to_path_buf();

            match shell {
                "zsh" => {
                    let zshenv = home.join(".zshenv");
                    Self::add_shell_integration(&zshenv, "eval \"$(dws env --shell zsh)\"")
                }
                "bash" => {
                    let bashrc = home.join(".bashrc");
                    Self::add_shell_integration(&bashrc, "eval \"$(dws env --shell bash)\"")
                }
                "fish" => {
                    let config_fish = home.join(".config/fish/config.fish");
                    if let Some(parent) = config_fish.parent() {
                        fs::create_dir_all(parent)?;
                    }
                    Self::add_shell_integration(&config_fish, "dws env --shell fish | source")
                }
                _ => Err(anyhow::anyhow!("Unsupported shell: {shell}")),
            }
        })();

        match configure_result {
            Ok(changed) => {
                let detail = if changed {
                    format!("{shell} rc updated")
                } else {
                    format!("{shell} rc already configured")
                };
                progress.success("Configured", Some(detail));
                Ok(())
            }
            Err(err) => {
                progress.fail("Failed", &err);
                Err(err)
            }
        }
    }

    /// Add integration line to shell rc file (idempotent)
    fn add_shell_integration(rc_file: &PathBuf, integration_line: &str) -> Result<bool> {
        let existing_content = if rc_file.exists() {
            fs::read_to_string(rc_file).with_context(|| format!("Failed to read {:?}", rc_file))?
        } else {
            String::new()
        };

        if existing_content.contains(integration_line) {
            return Ok(false);
        }

        let new_content = if existing_content.is_empty() {
            format!("# dws shell integration\n{}\n", integration_line)
        } else if existing_content.ends_with('\n') {
            format!(
                "{}# dws shell integration\n{}\n",
                existing_content, integration_line
            )
        } else {
            format!(
                "{}\n# dws shell integration\n{}\n",
                existing_content, integration_line
            )
        };

        fs::write(rc_file, new_content)
            .with_context(|| format!("Failed to write {:?}", rc_file))?;

        Ok(true)
    }

    /// Get the dotfile manager for the active profile
    pub fn dotfiles(&self) -> Result<Dotfiles> {
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

        Ok(Dotfiles::new(config_dir, target_dir))
    }

    /// Get the Environment for shell integration
    pub fn environment(&self, shell: Shell) -> Result<Environment> {
        Environment::new_from_workspace(self, shell)
    }

    pub fn environment_export(&self, shell: &str) -> Result<EnvironmentExport> {
        if !self.exists() {
            anyhow::bail!("Workspace not initialized. Run: dws init [repo]");
        }

        let (resolved, defaulted) = match Shell::from_name(shell) {
            Some(shell) => (shell, false),
            None => (Shell::Zsh, true),
        };

        let env = self.environment(resolved)?;
        let script = env.format_for_shell(resolved);

        Ok(EnvironmentExport {
            shell: resolved,
            script,
            defaulted,
        })
    }

    /// Load tool definitions defined for this workspace.
    pub fn tools(&self) -> Result<ToolSet> {
        let profile_root = self.path(WorkspacePath::Profile);
        let workspace_config = self.path(WorkspacePath::ConfigFile);
        ToolSet::load(&profile_root, &workspace_config)
    }

    fn prepare_tool_install_context(&self) -> Result<InstallContext> {
        let cache_dir = self.path(WorkspacePath::Cache);
        fs::create_dir_all(&cache_dir)
            .with_context(|| format!("Failed to create cache directory {:?}", cache_dir))?;
        let cache_tools_dir = cache_dir.join("tools");
        fs::create_dir_all(&cache_tools_dir).with_context(|| {
            format!(
                "Failed to create cache tools directory {:?}",
                cache_tools_dir
            )
        })?;

        let bin_dir = self.path(WorkspacePath::Bin);
        fs::create_dir_all(&bin_dir)
            .with_context(|| format!("Failed to create bin directory {:?}", bin_dir))?;

        let share_dir = self.path(WorkspacePath::Share);
        fs::create_dir_all(&share_dir)
            .with_context(|| format!("Failed to create share directory {:?}", share_dir))?;

        let man_dir = share_dir.join("man");
        fs::create_dir_all(&man_dir)
            .with_context(|| format!("Failed to create man directory {:?}", man_dir))?;

        let zsh_functions = share_dir.join("zsh").join("site-functions");
        fs::create_dir_all(&zsh_functions).with_context(|| {
            format!(
                "Failed to create zsh site-functions directory {:?}",
                zsh_functions
            )
        })?;

        Ok(InstallContext {
            cache_tools_dir,
            bin_dir,
        })
    }

    fn build_tool_tasks(
        &self,
        definitions: Vec<(String, ToolDefinition)>,
        context: &InstallContext,
    ) -> Result<Vec<ToolInstallTask>> {
        let mut tasks = Vec::new();

        for (name, definition) in definitions {
            match installers::create_installer(&definition, context.clone())? {
                Some(dispatch) => tasks.push(ToolInstallTask {
                    name,
                    resolved_version: dispatch.resolved_version,
                    installer: dispatch.installer,
                }),
                None => ui::warn(format!(
                    "Skipping tool '{name}' - installer '{}' is not yet supported",
                    definition.installer
                )),
            }
        }

        Ok(tasks)
    }

    fn execute_tool_tasks(
        &self,
        tasks: Vec<ToolInstallTask>,
        lockfile: &mut Lockfile,
        pending_label: &str,
        success_label: &str,
        fail_action: &str,
    ) -> Result<Vec<UpdatedTool>> {
        if tasks.is_empty() {
            return Ok(Vec::new());
        }

        let needs_runtime = tasks.iter().any(|task| task.installer.requires_runtime());
        let total = tasks.len();

        ui::status(
            "Install",
            format!("{} tool{} queued", total, if total == 1 { "" } else { "s" }),
        );

        let mut runtime = if needs_runtime {
            Some(Runtime::new().context("Failed to create Tokio runtime")?)
        } else {
            None
        };

        let mut updated = Vec::new();

        for (index, task) in tasks.into_iter().enumerate() {
            let ToolInstallTask {
                name,
                resolved_version,
                installer,
            } = task;

            let position = index + 1;
            let mut display = name.clone();
            if let Some(version) = &resolved_version {
                display = format!("{display} {version}");
            }
            let message = format!("{display} ({position}/{total})");
            let progress = Progress::new(pending_label, message);
            if let Err(err) = installer.install(runtime.as_mut(), lockfile) {
                progress.fail("Failed", &err);
                return Err(err).context(format!("Failed to {fail_action} tool '{name}'"));
            }
            let detail = resolved_version
                .as_ref()
                .map(|version| format!("version {version}"));
            progress.success(success_label, detail);

            updated.push(UpdatedTool {
                name,
                version: resolved_version,
            });
        }

        Ok(updated)
    }

    /// Install the workspace (symlink configs, install tools)
    pub fn install(&self) -> Result<()> {
        let tools = self.tools()?;

        // Load or create lockfile
        let lockfile_path = self.path(WorkspacePath::Lockfile);
        let mut lockfile = if lockfile_path.exists() {
            // Cleanup existing installation first
            let old_lockfile = Lockfile::load(&lockfile_path)?;
            self.remove_tracked_symlinks(&old_lockfile)?;
            Lockfile::new()
        } else {
            Lockfile::new()
        };

        // Install config entries and record in lockfile
        let dotfiles = self.dotfiles()?;
        let config_entries = dotfiles.discover_entries()?;

        for entry in &config_entries {
            entry.install()?;
            lockfile.add_config_symlink(entry.source.clone(), entry.target.clone());
        }

        let install_context = self.prepare_tool_install_context()?;

        let definitions: Vec<(String, ToolDefinition)> = tools
            .iter()
            .map(|(name, entry)| (name.clone(), entry.definition.clone()))
            .collect();

        let tasks = self.build_tool_tasks(definitions, &install_context)?;
        if tasks.is_empty() {
            ui::info("No tools defined for the active profile.");
        }
        let installed =
            self.execute_tool_tasks(tasks, &mut lockfile, "Installing", "Installed", "install")?;

        self.prune_unused_bin(&lockfile)?;
        self.prune_unused_cache(&lockfile)?;

        // Save lockfile
        lockfile.metadata.installed_at = Utc::now().to_rfc3339();
        lockfile.save(&lockfile_path)?;

        if !installed.is_empty() {
            let summary = installed
                .iter()
                .map(|update| match &update.version {
                    Some(version) => format!("{} ({})", update.name, version),
                    None => update.name.clone(),
                })
                .collect::<Vec<String>>()
                .join(", ");

            ui::success(
                "Finished",
                format!("installed {} tool(s): {}", installed.len(), summary),
            );
        }

        Ok(())
    }

    /// Update installed tools, respecting version pins and the `self_update` flag.
    pub fn update_tools(&self, requested: Option<&str>) -> Result<()> {
        let tools = self.tools()?;
        if tools.is_empty() {
            ui::info("No tools defined for the active profile.");
            return Ok(());
        }

        let mut selected: Vec<(&str, &crate::toolset::ToolEntry)> = Vec::new();
        if let Some(tool_name) = requested {
            if let Some(entry) = tools.entries().get(tool_name) {
                selected.push((tool_name, entry));
            } else {
                anyhow::bail!(
                    "Tool '{}' is not defined for the active profile or workspace overrides.",
                    tool_name
                );
            }
        } else {
            selected.extend(tools.iter().map(|(name, entry)| (name.as_str(), entry)));
        }

        let mut candidates = Vec::new();
        for (name, entry) in selected {
            if entry.definition.self_update {
                ui::info(format!(
                    "Skipping '{name}' because it maintains itself (self_update = true)."
                ));
                continue;
            }

            if let Some(version) = entry.definition.version.as_deref() {
                ui::info(format!(
                    "Skipping '{name}' because it is pinned to version '{version}'."
                ));
                continue;
            }

            candidates.push((name.to_string(), entry.definition.clone()));
        }

        if candidates.is_empty() {
            ui::info("No tools eligible for update.");
            return Ok(());
        }

        let install_context = self.prepare_tool_install_context()?;

        let tasks = self.build_tool_tasks(candidates, &install_context)?;

        if tasks.is_empty() {
            ui::warn("No installers available for the selected tools.");
            return Ok(());
        }

        let lockfile_path = self.path(WorkspacePath::Lockfile);
        let mut lockfile = if lockfile_path.exists() {
            Lockfile::load(&lockfile_path)?
        } else {
            Lockfile::new()
        };

        let mut current_entries: HashMap<String, Vec<LockfileToolEntry>> = HashMap::new();
        for entry in lockfile.tool_symlinks() {
            current_entries
                .entry(entry.name.clone())
                .or_default()
                .push(entry.clone());
        }

        let mut filtered_tasks = Vec::new();
        for task in tasks {
            if let Some(resolved) = &task.resolved_version {
                if let Some(entries) = current_entries.get(&task.name) {
                    let version_matches = entries.iter().all(|entry| entry.version == *resolved);
                    let missing_paths = entries.iter().any(|entry| {
                        !entry.source.exists()
                            || !entry.target.exists()
                            || entry
                                .target
                                .symlink_metadata()
                                .map(|meta| !meta.file_type().is_symlink())
                                .unwrap_or(true)
                    });

                    if version_matches && !missing_paths {
                        ui::info(format!(
                            "'{}' is already at version '{}'; skipping.",
                            task.name, resolved
                        ));
                        continue;
                    } else if version_matches && missing_paths {
                        ui::warn(format!(
                            "'{}' is already at version '{}' but required files are missing; reinstalling.",
                            task.name, resolved
                        ));
                    }
                }
            }
            filtered_tasks.push(task);
        }

        if filtered_tasks.is_empty() {
            ui::success("Finished", "all tools are up to date.");
            return Ok(());
        }

        let names_to_update: Vec<String> = filtered_tasks
            .iter()
            .map(|task| task.name.clone())
            .collect();

        for name in &names_to_update {
            let prior_entries: Vec<LockfileToolEntry> = lockfile
                .tool_symlinks()
                .filter(|entry| entry.name == *name)
                .cloned()
                .collect();
            self.remove_tool_symlink_entries(&prior_entries)?;
            lockfile.retain_tool_symlinks(|entry| entry.name != *name);
        }

        let updated = self
            .execute_tool_tasks(
                filtered_tasks,
                &mut lockfile,
                "Updating",
                "Updated",
                "update",
            )
            .context("Failed to update selected tools")?;

        self.prune_unused_bin(&lockfile)?;
        self.prune_unused_cache(&lockfile)?;

        lockfile.metadata.installed_at = Utc::now().to_rfc3339();
        lockfile.save(&lockfile_path)?;

        let summary = updated
            .iter()
            .map(|update| {
                if let Some(version) = &update.version {
                    format!("{} ({})", update.name, version)
                } else {
                    update.name.clone()
                }
            })
            .collect::<Vec<String>>()
            .join(", ");

        ui::success(
            "Finished",
            format!("updated {} tool(s): {}", updated.len(), summary),
        );

        Ok(())
    }

    /// Reset workspace state and active profile repository.
    pub fn reset(&self, force: bool) -> Result<()> {
        let profile_path = self.path(WorkspacePath::Profile);

        if !profile_path.exists() {
            anyhow::bail!(
                "Active profile at {:?} does not exist. Run 'dws init' first.",
                profile_path
            );
        }

        let repo = match Repository::open(&profile_path) {
            Ok(repo) => Some(repo),
            Err(err) => match err.code() {
                ErrorCode::NotFound => {
                    ui::info(format!(
                        "Skipping git reset: active profile '{}' is not a git repository.",
                        self.active_profile_name()
                    ));
                    None
                }
                _ => return Err(err.into()),
            },
        };

        if let Some(repo) = repo.as_ref() {
            if !force {
                ensure_clean_worktree(repo)?;
            }
        }

        if !force && !confirm_reset()? {
            ui::info("Reset cancelled.");
            return Ok(());
        }

        let uninstall_progress = Progress::new("Cleaning", "current workspace state");
        if let Err(err) = self.uninstall() {
            uninstall_progress.fail("Failed", &err);
            return Err(err).context("Failed to uninstall existing workspace state");
        }
        uninstall_progress.success("Cleaned", Some("clean".to_string()));

        if let Some(repo) = repo {
            let reset_progress = Progress::new(
                "Resetting",
                format!("profile repository at {}", profile_path.display()),
            );
            if let Err(err) = reset_repository(repo) {
                reset_progress.fail("Failed", &err);
                return Err(err).context("Failed to reset profile repository");
            }
            reset_progress.success("Reset", None);
        }

        let install_progress = Progress::new("Reinstalling", "workspace");
        if let Err(err) = self.install() {
            install_progress.fail("Failed", &err);
            return Err(err).context("Failed to reinstall workspace after reset");
        }
        install_progress.success("Installed", None);

        ui::success("Finished", "workspace reset complete.");
        Ok(())
    }

    /// Uninstall the workspace
    pub fn uninstall(&self) -> Result<()> {
        let lockfile_path = self.path(WorkspacePath::Lockfile);

        if !lockfile_path.exists() {
            return Ok(());
        }

        let lockfile = Lockfile::load(&lockfile_path)?;
        self.remove_tracked_symlinks(&lockfile)?;

        // Remove lockfile
        fs::remove_file(&lockfile_path)
            .with_context(|| format!("Failed to remove lockfile {:?}", lockfile_path))?;

        if self.state_dir.exists() {
            fs::remove_dir_all(&self.state_dir).with_context(|| {
                format!("Failed to remove state directory {:?}", self.state_dir)
            })?;
        }

        if self.cache_dir.exists() {
            fs::remove_dir_all(&self.cache_dir).with_context(|| {
                format!("Failed to remove cache directory {:?}", self.cache_dir)
            })?;
        }

        Ok(())
    }

    fn remove_tool_symlink_entries(&self, entries: &[LockfileToolEntry]) -> Result<()> {
        for entry in entries {
            if entry.target.exists() || entry.target.symlink_metadata().is_ok() {
                fs::remove_file(&entry.target)
                    .with_context(|| format!("Failed to remove tool symlink {:?}", entry.target))?;
            }
        }

        Ok(())
    }

    /// Remove all symlinks tracked in the lockfile
    fn remove_tracked_symlinks(&self, lockfile: &Lockfile) -> Result<()> {
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
                fs::remove_file(&entry.target)
                    .with_context(|| format!("Failed to remove tool symlink {:?}", entry.target))?;
            }
        }

        Ok(())
    }

    /// Remove cached tool versions that no longer have symlinks tracked in the lockfile.
    ///
    /// The cache is organised as $XDG_CACHE_HOME/dws/tools/<tool>/<version>. The lockfile stores the
    /// fully qualified path to the version directory. Anything not referenced gets pruned.
    fn prune_unused_cache(&self, lockfile: &Lockfile) -> Result<()> {
        let cache_dir = self.path(WorkspacePath::Cache);
        let tools_dir = cache_dir.join("tools");
        if !tools_dir.exists() {
            return Ok(());
        }

        let mut in_use: HashSet<PathBuf> = HashSet::new();
        for entry in lockfile.tool_symlinks() {
            let mut current = entry.source.parent();
            while let Some(dir) = current {
                if !dir.starts_with(&tools_dir) {
                    break;
                }

                in_use.insert(dir.to_path_buf());

                if dir == tools_dir {
                    break;
                }

                current = dir.parent();
            }
        }

        for tool_entry in fs::read_dir(&tools_dir)
            .with_context(|| format!("Failed to read cache directory {:?}", tools_dir))?
        {
            let tool_entry = tool_entry?;
            let tool_path = tool_entry.path();
            if !tool_path.is_dir() {
                continue;
            }

            for version_entry in fs::read_dir(&tool_path)? {
                let version_entry = version_entry?;
                let version_path = version_entry.path();
                if !version_path.is_dir() {
                    continue;
                }

                if !in_use.contains(&version_path) {
                    fs::remove_dir_all(&version_path).with_context(|| {
                        format!("Failed to remove cached tool at {:?}", version_path)
                    })?;
                }
            }

            if !tool_path.exists() {
                continue;
            }

            if tool_path.read_dir()?.next().is_none() {
                fs::remove_dir(&tool_path).with_context(|| {
                    format!("Failed to remove empty cache directory {:?}", tool_path)
                })?;
            }
        }

        Ok(())
    }

    /// Remove stale symlinks from $XDG_STATE_HOME/dws/bin when they are no longer listed in the
    /// lockfile. Only symlinks are touched; any user-managed files remain untouched.
    fn prune_unused_bin(&self, lockfile: &Lockfile) -> Result<()> {
        let bin_dir = self.path(WorkspacePath::Bin);
        if !bin_dir.exists() {
            return Ok(());
        }

        let valid: HashSet<PathBuf> = lockfile
            .tool_symlinks()
            .map(|entry| entry.target.clone())
            .collect();

        for entry in fs::read_dir(&bin_dir)
            .with_context(|| format!("Failed to read bin directory {:?}", bin_dir))?
        {
            let entry = entry?;
            let target = entry.path();

            if valid.contains(&target) {
                continue;
            }

            if target
                .symlink_metadata()
                .map(|metadata| metadata.file_type().is_symlink())
                .unwrap_or(false)
            {
                fs::remove_file(&target).with_context(|| {
                    format!("Failed to remove stale binary symlink {:?}", target)
                })?;
            }
        }

        Ok(())
    }
}

fn ensure_clean_worktree(repo: &Repository) -> Result<()> {
    if repo.state() != git2::RepositoryState::Clean {
        anyhow::bail!("Repository has an in-progress operation (merge, rebase, etc.). Finish it first or re-run with --force.");
    }

    let mut opts = StatusOptions::new();
    opts.include_untracked(true)
        .include_ignored(false)
        .recurse_untracked_dirs(true);

    let statuses = repo
        .statuses(Some(&mut opts))
        .context("Failed to inspect repository status")?;

    let mut dirty_paths: Vec<String> = Vec::new();
    for entry in statuses.iter() {
        let status = entry.status();
        if status.intersects(
            Status::INDEX_NEW
                | Status::INDEX_MODIFIED
                | Status::INDEX_DELETED
                | Status::INDEX_RENAMED
                | Status::INDEX_TYPECHANGE
                | Status::WT_NEW
                | Status::WT_MODIFIED
                | Status::WT_DELETED
                | Status::WT_RENAMED
                | Status::WT_TYPECHANGE,
        ) {
            if let Some(path) = entry.path() {
                dirty_paths.push(path.to_string());
            }
        }
    }

    if dirty_paths.is_empty() {
        return Ok(());
    }

    if dirty_paths.len() > 5 {
        dirty_paths.truncate(5);
        dirty_paths.push("…".to_string());
    }

    anyhow::bail!(
        "Profile repository has uncommitted changes: {}.\nRe-run with --force to discard local modifications.",
        dirty_paths.join(", ")
    );
}

fn confirm_reset() -> Result<bool> {
    print!("This will reinstall all tools and dotfiles. Continue? [y/N]: ");
    io::stdout().flush().context("Failed to flush stdout")?;

    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .context("Failed to read confirmation")?;

    let trimmed = input.trim().to_ascii_lowercase();
    Ok(trimmed == "y" || trimmed == "yes")
}

fn reset_repository(repo: Repository) -> Result<()> {
    if let Err(err) = fetch_remote(&repo) {
        ui::warn(format!("Failed to fetch 'origin': {err}"));
    }

    let target = determine_reset_target(&repo)?;
    repo.reset(&target, ResetType::Hard, None)
        .context("Failed to perform hard reset")?;

    let mut checkout = CheckoutBuilder::new();
    checkout.force().remove_untracked(true).remove_ignored(true);

    repo.checkout_head(Some(&mut checkout))
        .context("Failed to clean working tree")?;

    let _ = repo.cleanup_state();
    Ok(())
}

fn fetch_remote(repo: &Repository) -> Result<()> {
    let mut remote = repo
        .find_remote("origin")
        .context("Repository has no 'origin' remote")?;

    remote.fetch::<&str>(&[], None, None)?;
    Ok(())
}

fn determine_reset_target(repo: &Repository) -> Result<Object<'_>> {
    let head = repo.head().context("Repository has no HEAD reference")?;

    if head.is_branch() {
        if let Some(branch) = head.shorthand() {
            let remote_ref = format!("refs/remotes/origin/{}", branch);
            if let Ok(obj) = repo.revparse_single(&remote_ref) {
                return Ok(obj);
            }
        }
    }

    head.peel(ObjectType::Commit)
        .context("Failed to resolve HEAD commit")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::default_profile_name;
    use crate::lockfile::ToolEntry as LockfileToolEntry;
    use crate::toolset::InstallerKind;
    use rstest::rstest;
    use serial_test::serial;
    use tempfile::TempDir;

    fn setup_test_env() -> TempDir {
        let temp = TempDir::new().unwrap();
        env::set_var("XDG_CONFIG_HOME", temp.path());
        env::set_var("XDG_STATE_HOME", temp.path().join("state"));
        env::set_var("XDG_CACHE_HOME", temp.path().join("cache"));
        env::set_var("HOME", temp.path());
        temp
    }

    #[test]
    #[serial]
    fn test_workspace_new() {
        let _temp = setup_test_env();
        let workspace = Workspace::new().unwrap();

        assert!(workspace
            .path(WorkspacePath::Root)
            .to_string_lossy()
            .contains("dws"));
        assert!(workspace
            .path(WorkspacePath::Profile)
            .to_string_lossy()
            .contains("profiles"));
        assert_eq!(workspace.active_profile_name(), default_profile_name());
    }

    #[test]
    #[serial]
    fn test_workspace_paths() {
        let _temp = setup_test_env();
        let workspace = Workspace::new().unwrap();

        assert!(workspace
            .path(WorkspacePath::Profiles)
            .to_string_lossy()
            .contains("profiles"));
        assert!(workspace
            .path(WorkspacePath::Config)
            .to_string_lossy()
            .contains("config"));
        assert!(workspace
            .path(WorkspacePath::ProfileConfig)
            .to_string_lossy()
            .ends_with("config.toml"));
        assert!(workspace
            .path(WorkspacePath::Share)
            .to_string_lossy()
            .contains("share"));
        assert!(workspace
            .path(WorkspacePath::Cache)
            .to_string_lossy()
            .contains("cache"));
        assert!(workspace
            .path(WorkspacePath::Lockfile)
            .to_string_lossy()
            .contains("dws.lock"));
        assert!(workspace
            .path(WorkspacePath::Bin)
            .to_string_lossy()
            .contains("bin"));
    }

    #[test]
    #[serial]
    fn test_workspace_tools() {
        let _temp = setup_test_env();
        let workspace = Workspace::new().unwrap();

        let profile_config = workspace.path(WorkspacePath::ProfileConfig);
        if let Some(parent) = profile_config.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(
            &profile_config,
            r#"
[tools.ripgrep]
installer = "ubi"
project = "BurntSushi/ripgrep"
"#,
        )
        .unwrap();

        let tools = workspace.tools().unwrap();
        assert_eq!(tools.len(), 1);
        let (name, entry) = tools.iter().next().unwrap();
        assert_eq!(name, "ripgrep");
        assert_eq!(entry.definition.installer, InstallerKind::Ubi);
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
        assert!(workspace.path(WorkspacePath::Bin).exists());
        assert!(workspace.path(WorkspacePath::Share).join("man").exists());
        assert!(workspace
            .path(WorkspacePath::Share)
            .join("zsh/site-functions")
            .exists());

        // Verify lockfile contents
        let lockfile = Lockfile::load(&workspace.path(WorkspacePath::Lockfile)).unwrap();
        assert_eq!(lockfile.config_symlinks.len(), 1);
    }

    #[test]
    #[serial]
    fn test_remove_tool_symlink_entries() {
        let _temp = setup_test_env();
        let workspace = Workspace::new().unwrap();

        let bin_dir = workspace.path(WorkspacePath::Bin);
        fs::create_dir_all(&bin_dir).unwrap();
        let target = bin_dir.join("rg");
        fs::write(&target, "dummy").unwrap();

        let entries = vec![LockfileToolEntry {
            name: "rg".to_string(),
            version: "14.0.0".to_string(),
            source: PathBuf::from("/cache/tools/rg/14.0.0/rg"),
            target: target.clone(),
        }];

        workspace.remove_tool_symlink_entries(&entries).unwrap();
        assert!(!target.exists());
    }

    #[test]
    #[serial]
    fn test_update_tools_skips_pinned() {
        let _temp = setup_test_env();
        let workspace = Workspace::new().unwrap();

        let profile_config = workspace.path(WorkspacePath::ProfileConfig);
        if let Some(parent) = profile_config.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(
            &profile_config,
            r#"
[tools.ripgrep]
installer = "ubi"
project = "BurntSushi/ripgrep"
version = "14.0.0"
"#,
        )
        .unwrap();

        workspace.update_tools(None).unwrap();

        assert!(!workspace.path(WorkspacePath::Lockfile).exists());
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
        assert!(!workspace.path(WorkspacePath::Bin).exists());
        assert!(!workspace.path(WorkspacePath::Share).exists());
        assert!(!workspace.path(WorkspacePath::Cache).exists());
    }

    #[test]
    #[serial]
    fn test_init_new_workspace_without_url() {
        let _temp = setup_test_env();
        let mut workspace = Workspace::new().unwrap();

        assert!(!workspace.path(WorkspacePath::Profile).exists());

        workspace.init(None, "zsh", None).unwrap();

        assert!(workspace.path(WorkspacePath::Profile).exists());
        assert!(workspace.path(WorkspacePath::Config).exists());
        assert!(workspace.path(WorkspacePath::ProfileConfig).exists());
        assert!(workspace
            .path(WorkspacePath::Config)
            .join("zsh/.zshrc")
            .exists());
        assert!(workspace
            .path(WorkspacePath::Profile)
            .join("README.md")
            .exists());
    }

    #[test]
    #[serial]
    fn test_init_new_workspace_with_url() {
        let _temp = setup_test_env();
        let mut workspace = Workspace::new().unwrap();

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
        repo.commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[])
            .unwrap();

        assert!(!workspace.path(WorkspacePath::Profile).exists());

        let repo_url = format!("file://{}", source_repo_path.display());
        workspace.init(Some(&repo_url), "bash", None).unwrap();

        assert!(workspace.path(WorkspacePath::Profile).exists());
        assert!(workspace
            .path(WorkspacePath::Profile)
            .join("README.md")
            .exists());
    }

    #[test]
    #[serial]
    fn test_init_existing_workspace_without_url() {
        let _temp = setup_test_env();
        let mut workspace = Workspace::new().unwrap();

        workspace
            .ensure_profile_template(workspace.active_profile())
            .unwrap();
        assert!(workspace.path(WorkspacePath::Profile).exists());

        let result = workspace.init(None, "bash", None);

        assert!(result.is_ok());
        assert!(workspace.path(WorkspacePath::Profile).exists());
    }

    #[test]
    #[serial]
    fn test_init_existing_workspace_with_matching_url() {
        let _temp = setup_test_env();
        let mut workspace = Workspace::new().unwrap();

        let source_temp = TempDir::new().unwrap();
        let source_repo_path = source_temp.path().join("test-repo");
        git2::Repository::init(&source_repo_path).unwrap();

        let repo_url = format!("file://{}", source_repo_path.display());
        let profile_name = workspace.clone_into_profile(&repo_url, None).unwrap();
        let profile_root = workspace.path(WorkspacePath::Profiles).join(&profile_name);
        assert!(profile_root.exists());

        let result = workspace.init(Some(repo_url.as_str()), "zsh", Some(profile_name.as_str()));
        assert!(result.is_ok());
        assert_eq!(workspace.active_profile_name(), profile_name.as_str());
    }

    #[test]
    #[serial]
    fn test_init_existing_workspace_with_mismatched_url() {
        let _temp = setup_test_env();
        let mut workspace = Workspace::new().unwrap();

        let initial_active = workspace.active_profile_name().to_string();
        let source_temp = TempDir::new().unwrap();
        let source_repo_path = source_temp.path().join("test-repo");
        git2::Repository::init(&source_repo_path).unwrap();

        let repo_url = format!("file://{}", source_repo_path.display());
        let profile_name = workspace.clone_into_profile(&repo_url, None).unwrap();
        let profile_root = workspace.path(WorkspacePath::Profiles).join(&profile_name);
        assert!(profile_root.exists());

        let different_url = "file:///different/repo";
        let result = workspace.init(Some(different_url), "zsh", Some(profile_name.as_str()));

        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("mismatch"));
        assert_eq!(workspace.active_profile_name(), initial_active);
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
    #[case("octocat/cli.tool", "https://github.com/octocat/cli.tool.git")]
    #[case("someone/dot.repo.git", "https://github.com/someone/dot.repo.git")]
    #[case("https://github.com/user/repo", "https://github.com/user/repo.git")]
    #[case("https://github.com/user/repo.git", "https://github.com/user/repo.git")]
    #[case("https://github.com/user/repo/", "https://github.com/user/repo.git")]
    #[case("http://github.com/user/repo", "http://github.com/user/repo.git")]
    #[case("https://gitlab.com/user/repo", "https://gitlab.com/user/repo.git")]
    #[case(
        "https://bitbucket.org/user/repo",
        "https://bitbucket.org/user/repo.git"
    )]
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

        assert!(!workspace.path(WorkspacePath::Profile).exists());

        workspace
            .ensure_profile_template(workspace.active_profile())
            .unwrap();

        assert!(workspace.path(WorkspacePath::Profile).exists());
        assert!(workspace
            .path(WorkspacePath::Profile)
            .join("README.md")
            .exists());
        assert!(workspace
            .path(WorkspacePath::Profile)
            .join(".gitignore")
            .exists());
        assert!(workspace
            .path(WorkspacePath::Config)
            .join(".dwsignore")
            .exists());
        assert!(workspace
            .path(WorkspacePath::Config)
            .join("zsh/.zshrc")
            .exists());
        assert!(workspace
            .path(WorkspacePath::Config)
            .join("bash/.bashrc")
            .exists());
        assert!(workspace
            .path(WorkspacePath::Config)
            .join("fish/config.fish")
            .exists());
        assert!(workspace.path(WorkspacePath::ProfileConfig).exists());
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
        repo.commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[])
            .unwrap();

        assert!(!workspace
            .path(WorkspacePath::Profiles)
            .join("test-repo")
            .exists());

        let repo_url = format!("file://{}", source_repo_path.display());
        let name = workspace.clone_into_profile(&repo_url, None).unwrap();
        assert_eq!(name, "test-repo");

        let cloned_root = workspace.path(WorkspacePath::Profiles).join(&name);
        assert!(cloned_root.exists());
        assert!(cloned_root.join("test.txt").exists());
        assert!(cloned_root.join(".git").exists());
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
        let name = workspace.clone_into_profile(&repo_url, None).unwrap();
        let profile = Profile::new(
            name.clone(),
            workspace.path(WorkspacePath::Profiles).join(&name),
        );

        let result = Workspace::verify_profile_repo(&profile, &repo_url);
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
        let name = workspace.clone_into_profile(&repo_url, None).unwrap();
        let profile = Profile::new(
            name.clone(),
            workspace.path(WorkspacePath::Profiles).join(&name),
        );

        let different_url = "file:///different/path";
        let result = Workspace::verify_profile_repo(&profile, different_url);

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

    #[rstest]
    #[case("/bin/zsh", "zsh")]
    #[case("/usr/local/bin/bash", "bash")]
    #[case("fish", "fish")]
    #[serial]
    fn test_detect_shell_from_env(#[case] shell_path: &str, #[case] expected: &str) {
        let previous = env::var("SHELL").ok();
        env::set_var("SHELL", shell_path);
        assert_eq!(Workspace::detect_shell_from_env().unwrap(), expected);
        match previous {
            Some(value) => env::set_var("SHELL", value),
            None => env::remove_var("SHELL"),
        }
    }

    #[test]
    #[serial]
    fn test_detect_shell_from_env_missing() {
        let previous = env::var("SHELL").ok();
        env::remove_var("SHELL");
        let err = Workspace::detect_shell_from_env().unwrap_err();
        assert!(err
            .to_string()
            .contains("SHELL environment variable not set"));
        match previous {
            Some(value) => env::set_var("SHELL", value),
            None => env::remove_var("SHELL"),
        }
    }

    #[test]
    #[serial]
    fn test_environment_export_with_known_shell() {
        let temp = setup_test_env();
        let workspace = Workspace::new().unwrap();
        workspace
            .ensure_profile_template(workspace.active_profile())
            .unwrap();
        let export = workspace.environment_export("bash").unwrap();
        assert!(!export.defaulted);
        assert_eq!(export.shell, Shell::Bash);
        assert!(export.script.contains("export PATH"));
        drop(temp);
    }

    #[test]
    #[serial]
    fn test_environment_export_defaults_unknown_shell() {
        let temp = setup_test_env();
        let workspace = Workspace::new().unwrap();
        workspace
            .ensure_profile_template(workspace.active_profile())
            .unwrap();
        let export = workspace.environment_export("powershell").unwrap();
        assert!(export.defaulted);
        assert_eq!(export.shell, Shell::Zsh);
        assert!(export.script.contains("export PATH"));
        drop(temp);
    }
}
