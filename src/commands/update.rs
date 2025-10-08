use crate::Workspace;
use anyhow::Result;

pub fn execute(_workspace: &Workspace, name: Option<String>) -> Result<()> {
    if let Some(tool_name) = name {
        println!("TODO: Check for updates: {}", tool_name);
        println!("TODO: Update if not pinned (or show newer version)");
    } else {
        println!("TODO: Check all tools for updates");
        println!("TODO: Update unpinned tools");
    }
    Ok(())
}
