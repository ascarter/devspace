use crate::Workspace;
use anyhow::Result;

pub fn execute(
    workspace: &mut Workspace,
    repository: String,
    profile: Option<String>,
) -> Result<()> {
    let name = workspace.clone_into_profile(&repository, profile.as_deref())?;
    println!(
        "Profile '{}' cloned. Run 'dws use {}' to activate it.",
        name, name
    );
    Ok(())
}
