use crate::Workspace;
use anyhow::Result;

pub fn execute(workspace: &mut Workspace, profile: String) -> Result<()> {
    workspace.use_profile(&profile)?;
    println!("Profile '{}' activated.", profile);
    Ok(())
}
