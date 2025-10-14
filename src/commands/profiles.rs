use crate::{ui, Workspace};
use anyhow::Result;

pub fn execute(workspace: &Workspace) -> Result<()> {
    let profiles = workspace.list_profiles()?;
    if profiles.is_empty() {
        ui::info("No profiles found. Use 'dws init' or 'dws clone' to create one.");
    } else {
        for name in profiles {
            if name == workspace.active_profile_name() {
                ui::success("Active", &name);
            } else {
                ui::status("Profile", &name);
            }
        }
    }
    Ok(())
}
