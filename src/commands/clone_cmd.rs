use crate::{ui, Workspace};
use anyhow::Result;

pub fn execute(
    workspace: &mut Workspace,
    repository: String,
    profile: Option<String>,
) -> Result<()> {
    let name = workspace.clone_into_profile(&repository, profile.as_deref())?;
    ui::success(
        "Cloned",
        format!("profile '{name}'. Run 'dws use {name}' to activate it."),
    );
    Ok(())
}
