use crate::{ui, Workspace};
use anyhow::Result;

pub fn execute(_workspace: &Workspace) -> Result<()> {
    ui::info("TODO: Show active profile");
    ui::info("TODO: List installed tools + versions");
    ui::info("TODO: Show available updates");
    Ok(())
}
