use crate::Workspace;
use anyhow::Result;

pub fn execute(workspace: &Workspace, name: Option<String>) -> Result<()> {
    workspace.update_tools(name.as_deref())
}
