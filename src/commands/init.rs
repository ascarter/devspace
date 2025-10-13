use crate::Workspace;
use anyhow::Result;

pub fn execute(
    workspace: &mut Workspace,
    repository: Option<String>,
    shell: Option<String>,
    profile: Option<String>,
) -> Result<()> {
    workspace.init_with_shell(repository.as_deref(), shell.as_deref(), profile.as_deref())
}
