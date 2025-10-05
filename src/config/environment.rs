use anyhow::Result;
use std::path::PathBuf;

use super::profile::get_environment_dir;

/// Shell type for environment generation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Shell {
    Zsh,
    Bash,
    Fish,
}

/// Shell environment configuration for a profile
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShellEnvironment {
    pub bin_path: PathBuf,
    pub man_path: PathBuf,
    pub completions_path: PathBuf,
}

impl ShellEnvironment {
    /// Create a new shell environment for the given profile
    pub fn new(profile_name: &str) -> Result<Self> {
        let env_dir = get_environment_dir(profile_name)?;

        Ok(Self {
            bin_path: env_dir.join("bin"),
            man_path: env_dir.join("share/man"),
            completions_path: env_dir.join("share/zsh/site-functions"),
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

    #[test]
    fn test_shell_environment_new() {
        let env = ShellEnvironment::new("test-profile").unwrap();

        assert!(env.bin_path.to_string_lossy().contains("test-profile"));
        assert!(env.bin_path.to_string_lossy().ends_with("/bin"));
        assert!(env.man_path.to_string_lossy().ends_with("/share/man"));
        assert!(env
            .completions_path
            .to_string_lossy()
            .ends_with("/share/zsh/site-functions"));
    }

    #[test]
    fn test_shell_environment_format_zsh() {
        let env = ShellEnvironment::new("default").unwrap();
        let output = env.format_for_shell(Shell::Zsh);

        assert!(output.contains("export PATH="));
        assert!(output.contains("export MANPATH="));
        assert!(output.contains("fpath=("));
        assert!(output.contains("/bin:$PATH"));
        assert!(output.contains("/share/man:"));
        assert!(output.contains("/share/zsh/site-functions"));
    }

    #[test]
    fn test_shell_environment_format_bash() {
        let env = ShellEnvironment::new("default").unwrap();
        let output = env.format_for_shell(Shell::Bash);

        assert!(output.contains("export PATH="));
        assert!(output.contains("export MANPATH="));
        assert!(!output.contains("fpath")); // bash doesn't have fpath
        assert!(output.contains("/bin:$PATH"));
        assert!(output.contains("/share/man:"));
    }

    #[test]
    fn test_shell_environment_format_fish() {
        let env = ShellEnvironment::new("default").unwrap();
        let output = env.format_for_shell(Shell::Fish);

        assert!(output.contains("set -gx PATH"));
        assert!(output.contains("set -gx MANPATH"));
        assert!(!output.contains("export")); // fish uses set -gx
        assert!(output.contains("/bin $PATH"));
        assert!(output.contains("/share/man $MANPATH"));
    }
}
