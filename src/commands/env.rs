use crate::{Shell, Workspace};
use anyhow::Result;

pub fn execute(workspace: &Workspace, shell: String) -> Result<()> {
    // Check if workspace exists
    if !workspace.exists() {
        anyhow::bail!("Workspace not initialized. Run: dws init [repo]");
    }

    // Parse shell type
    let shell_type = match shell.to_lowercase().as_str() {
        "zsh" => Shell::Zsh,
        "bash" => Shell::Bash,
        "fish" => Shell::Fish,
        _ => {
            eprintln!("Unknown shell: {}. Defaulting to zsh.", shell);
            Shell::Zsh
        }
    };

    // Generate and output shell environment
    let env = workspace.environment(shell_type)?;
    println!("{}", env.format_for_shell(shell_type));

    Ok(())
}
