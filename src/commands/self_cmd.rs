use crate::cli::SelfAction;
use crate::Workspace;
use anyhow::Result;

pub fn execute(_workspace: &Workspace, action: SelfAction) -> Result<()> {
    match action {
        SelfAction::Info => {
            println!("TODO: Show devws version");
            println!("TODO: Show disk usage");
            println!("TODO: Show profile count");
            Ok(())
        }
        SelfAction::Update => {
            println!("TODO: Check for devws updates");
            println!("TODO: Download and install new version");
            Ok(())
        }
        SelfAction::Uninstall => {
            println!("TODO: Confirm uninstall (like rustup)");
            println!("TODO: Remove binary");
            println!("TODO: Remove $XDG_CONFIG_HOME/devws");
            println!("TODO: Remove $XDG_STATE_HOME/devws");
            println!("TODO: Remove $XDG_CACHE_HOME/devws");
            println!("TODO: Remove shell integration");
            Ok(())
        }
    }
}
