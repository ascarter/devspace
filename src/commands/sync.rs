use crate::{ui, Workspace};
use anyhow::Result;

pub fn execute(_workspace: &Workspace) -> Result<()> {
    ui::info("TODO: Git pull active profile");
    ui::info("TODO: Install new tools from config.toml");
    ui::info("TODO: Update symlinks (respect version pins)");
    Ok(())
}
