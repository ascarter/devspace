use crate::{ui, Workspace};
use anyhow::Result;

pub fn execute(workspace: &mut Workspace, profile: String) -> Result<()> {
    workspace.use_profile(&profile)?;
    ui::success("Activated", format!("profile '{profile}'"));
    Ok(())
}
