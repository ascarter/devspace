use crate::Workspace;
use anyhow::Result;

pub fn execute(_workspace: &Workspace, _force: bool) -> Result<()> {
    // TODO: Implement reset command
    // 1. Check git status - error if dirty (unless force)
    // 2. Confirm with user (unless force)
    // 3. workspace.uninstall()
    // 4. git clean -fdx or git reset --hard + git pull
    // 5. workspace.install()

    println!("TODO: dws reset - not yet implemented");
    Ok(())
}
