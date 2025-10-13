use anyhow::Result;
use std::path::PathBuf;

use crate::workspace::{Workspace, WorkspacePath};

/// Shell type for environment generation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Shell {
    Zsh,
    Bash,
    Fish,
}

impl Shell {
    pub fn from_name(name: &str) -> Option<Self> {
        match name.to_lowercase().as_str() {
            "zsh" => Some(Shell::Zsh),
            "bash" => Some(Shell::Bash),
            "fish" => Some(Shell::Fish),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Shell::Zsh => "zsh",
            Shell::Bash => "bash",
            Shell::Fish => "fish",
        }
    }
}

/// Shell environment configuration
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Environment {
    pub bin_path: PathBuf,
    pub man_path: PathBuf,
    pub completions_path: PathBuf,
}

impl Environment {
    /// Create a new shell environment from workspace
    pub fn new_from_workspace(workspace: &Workspace, _shell: Shell) -> Result<Self> {
        let share_path = workspace.path(WorkspacePath::Share);
        Ok(Self {
            bin_path: workspace.path(WorkspacePath::Bin),
            man_path: share_path.join("man"),
            completions_path: share_path.join("zsh/site-functions"),
        })
    }

    /// Format the environment for the given shell
    pub fn format_for_shell(&self, shell: Shell) -> String {
        match shell {
            Shell::Zsh => self.format_zsh(),
            Shell::Bash => self.format_bash(),
            Shell::Fish => self.format_fish(),
        }
    }

    fn format_zsh(&self) -> String {
        format!(
            "export PATH=\"{}:$PATH\"\nexport MANPATH=\"{}:${{MANPATH:-}}\"\nfpath=({} ${{fpath[@]}})",
            self.bin_path.display(),
            self.man_path.display(),
            self.completions_path.display()
        )
    }

    fn format_bash(&self) -> String {
        format!(
            "export PATH=\"{}:$PATH\"\nexport MANPATH=\"{}:${{MANPATH:-}}\"",
            self.bin_path.display(),
            self.man_path.display()
        )
    }

    fn format_fish(&self) -> String {
        format!(
            "set -gx PATH {} $PATH\nset -gx MANPATH {} $MANPATH",
            self.bin_path.display(),
            self.man_path.display()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::workspace::Workspace;
    use serial_test::serial;
    use std::env;
    use tempfile::TempDir;

    fn setup_test_env() -> TempDir {
        let temp = TempDir::new().unwrap();
        env::set_var("XDG_CONFIG_HOME", temp.path());
        env::set_var("XDG_STATE_HOME", temp.path().join("state"));
        temp
    }

    #[test]
    #[serial]
    fn test_shell_environment_new() {
        let _temp = setup_test_env();
        let workspace = Workspace::new().unwrap();
        let env = Environment::new_from_workspace(&workspace, Shell::Zsh).unwrap();

        assert!(env.bin_path.to_string_lossy().ends_with("/bin"));
        assert!(env.man_path.to_string_lossy().ends_with("/share/man"));
        assert!(env
            .completions_path
            .to_string_lossy()
            .ends_with("/share/zsh/site-functions"));
    }

    #[test]
    #[serial]
    fn test_shell_environment_format_zsh() {
        let _temp = setup_test_env();
        let workspace = Workspace::new().unwrap();
        let env = Environment::new_from_workspace(&workspace, Shell::Zsh).unwrap();
        let output = env.format_for_shell(Shell::Zsh);

        assert!(output.contains("export PATH="));
        assert!(output.contains("export MANPATH="));
        assert!(output.contains("fpath=("));
        assert!(output.contains("/bin:$PATH"));
        assert!(output.contains("/share/man:"));
        assert!(output.contains("/share/zsh/site-functions"));
    }

    #[test]
    #[serial]
    fn test_shell_environment_format_bash() {
        let _temp = setup_test_env();
        let workspace = Workspace::new().unwrap();
        let env = Environment::new_from_workspace(&workspace, Shell::Bash).unwrap();
        let output = env.format_for_shell(Shell::Bash);

        assert!(output.contains("export PATH="));
        assert!(output.contains("export MANPATH="));
        assert!(!output.contains("fpath")); // bash doesn't have fpath
        assert!(output.contains("/bin:$PATH"));
        assert!(output.contains("/share/man:"));
    }

    #[test]
    #[serial]
    fn test_shell_environment_format_fish() {
        let _temp = setup_test_env();
        let workspace = Workspace::new().unwrap();
        let env = Environment::new_from_workspace(&workspace, Shell::Fish).unwrap();
        let output = env.format_for_shell(Shell::Fish);

        assert!(output.contains("set -gx PATH"));
        assert!(output.contains("set -gx MANPATH"));
        assert!(!output.contains("export")); // fish uses set -gx
        assert!(output.contains("/bin $PATH"));
        assert!(output.contains("/share/man $MANPATH"));
    }

    #[test]
    fn test_shell_from_name() {
        assert_eq!(Shell::from_name("zsh"), Some(Shell::Zsh));
        assert_eq!(Shell::from_name("BASH"), Some(Shell::Bash));
        assert_eq!(Shell::from_name("Fish"), Some(Shell::Fish));
        assert_eq!(Shell::from_name("powershell"), None);
    }

    #[test]
    fn test_shell_as_str() {
        assert_eq!(Shell::Zsh.as_str(), "zsh");
        assert_eq!(Shell::Bash.as_str(), "bash");
        assert_eq!(Shell::Fish.as_str(), "fish");
    }
}
