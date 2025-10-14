use crate::cli::SelfAction;
use crate::{ui, Workspace};
use anyhow::Result;

pub fn execute(_workspace: &Workspace, action: SelfAction) -> Result<()> {
    match action {
        SelfAction::Info => {
            ui::info("TODO: Show dws version");
            ui::info("TODO: Show disk usage");
            ui::info("TODO: Show profile count");
            Ok(())
        }
        SelfAction::Update => {
            ui::info("TODO: Check for dws updates");
            ui::info("TODO: Download and install new version");
            Ok(())
        }
        SelfAction::Uninstall => {
            ui::info("TODO: Confirm uninstall (like rustup)");
            ui::info("TODO: Remove binary");
            ui::info("TODO: Remove $XDG_CONFIG_HOME/dws");
            ui::info("TODO: Remove $XDG_STATE_HOME/dws");
            ui::info("TODO: Remove $XDG_CACHE_HOME/dws");
            ui::info("TODO: Remove shell integration");
            Ok(())
        }
    }
}
