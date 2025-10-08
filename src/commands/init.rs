use crate::Workspace;
use anyhow::Result;
use std::env;

/// Detect shell from $SHELL environment variable
///
/// The $SHELL variable contains the user's preferred shell (set by the system
/// or user). This is the standard POSIX way to detect the active shell.
/// We extract just the shell name (e.g., "zsh" from "/bin/zsh") to determine
/// which shell integration to set up.
///
/// Returns an error if $SHELL is not set, as this tool requires a full
/// development environment and the user should explicitly specify the shell.
fn detect_shell() -> Result<String> {
    env::var("SHELL")
        .ok()
        .and_then(|s| s.split('/').last().map(String::from))
        .ok_or_else(|| anyhow::anyhow!("SHELL environment variable not set. Please specify shell with --shell flag."))
}

pub fn execute(
    workspace: &Workspace,
    repository: Option<String>,
    shell: Option<String>,
) -> Result<()> {
    let shell = match shell {
        Some(s) => s,
        None => detect_shell()?,
    };

    workspace.init(repository.as_deref(), &shell)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;
    use serial_test::serial;

    /// Test shell detection from $SHELL environment variable
    ///
    /// Tests common shells found in /etc/shells across macOS, Linux, and BSD,
    /// with various installation paths including Homebrew, Nix, and custom locations.
    #[rstest]
    #[case("/bin/zsh", "zsh")]
    #[case("/usr/bin/zsh", "zsh")]
    #[case("/bin/bash", "bash")]
    #[case("/usr/bin/bash", "bash")]
    #[case("/usr/local/bin/bash", "bash")]
    #[case("/bin/fish", "fish")]
    #[case("/usr/bin/fish", "fish")]
    #[case("/usr/local/bin/fish", "fish")]
    #[case("/opt/homebrew/bin/fish", "fish")]
    #[case("/bin/sh", "sh")]
    #[case("/bin/dash", "dash")]
    #[case("/bin/ksh", "ksh")]
    #[case("/bin/tcsh", "tcsh")]
    #[case("/bin/csh", "csh")]
    #[case("zsh", "zsh")] // No path
    #[case("/custom/path/to/zsh", "zsh")] // Custom installation
    #[case("/nix/store/hash-zsh-5.9/bin/zsh", "zsh")] // Nix-style path
    #[serial]
    fn test_detect_shell(#[case] shell_path: &str, #[case] expected: &str) {
        env::set_var("SHELL", shell_path);
        assert_eq!(detect_shell().unwrap(), expected);
        env::remove_var("SHELL");
    }

    #[test]
    #[serial]
    fn test_detect_shell_when_unset() {
        env::remove_var("SHELL");
        let result = detect_shell();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("SHELL environment variable not set"));
    }
}
