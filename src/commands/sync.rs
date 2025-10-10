use crate::Workspace;
use anyhow::Result;

pub fn execute(_workspace: &Workspace) -> Result<()> {
    println!("TODO: Git pull active profile");
    println!("TODO: Install new tools from config.toml");
    println!("TODO: Update symlinks (respect version pins)");
    Ok(())
}
