use crate::Workspace;
use anyhow::Result;

pub fn execute(workspace: &Workspace, shell: String) -> Result<()> {
    let export = workspace.environment_export(&shell)?;
    if export.defaulted {
        eprintln!(
            "Unknown shell '{}'; defaulting to {}.",
            shell,
            export.shell.as_str()
        );
    }
    println!("{}", export.script);

    Ok(())
}
