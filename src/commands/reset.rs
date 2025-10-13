use crate::Workspace;
use anyhow::Result;

pub fn execute(workspace: &Workspace, force: bool) -> Result<()> {
    workspace.reset(force)
}
