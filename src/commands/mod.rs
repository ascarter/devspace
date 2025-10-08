use crate::cli::{Cli, Commands};
use crate::Workspace;
use anyhow::Result;

mod cleanup;
mod env;
mod init;
mod reset;
mod self_cmd;
mod status;
mod sync;
mod update;

pub fn execute(cli: Cli) -> Result<()> {
    // Create workspace - this is the root entry point
    let workspace = Workspace::new()?;

    match cli.command {
        Commands::Init {
            repository,
            shell,
            force,
        } => init::execute(&workspace, repository, shell, force),

        Commands::Sync => sync::execute(&workspace),

        Commands::Reset { force } => reset::execute(&workspace, force),

        Commands::Update { name } => update::execute(&workspace, name),

        Commands::Status => status::execute(&workspace),

        Commands::Cleanup => cleanup::execute(&workspace),

        Commands::Env { shell } => env::execute(&workspace, shell),

        Commands::Self_(action) => self_cmd::execute(&workspace, action),
    }
}
