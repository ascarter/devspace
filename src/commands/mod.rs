use crate::cli::{Cli, Commands};
use crate::Workspace;
use anyhow::Result;

mod check;
mod cleanup;
mod clone_cmd;
mod env;
mod init;
mod profiles;
mod reset;
mod self_cmd;
mod status;
mod sync;
mod update;
mod use_profile;

pub fn execute(cli: Cli) -> Result<()> {
    // Create workspace - this is the root entry point
    let mut workspace = Workspace::new()?;

    match cli.command {
        Commands::Init {
            repository,
            shell,
            profile,
        } => init::execute(&mut workspace, repository, shell, profile),

        Commands::Clone {
            repository,
            profile,
        } => clone_cmd::execute(&mut workspace, repository, profile),

        Commands::Use { profile } => use_profile::execute(&mut workspace, profile),

        Commands::Profiles => profiles::execute(&workspace),

        Commands::Sync => sync::execute(&workspace),

        Commands::Reset { force } => reset::execute(&workspace, force),

        Commands::Update { name } => update::execute(&workspace, name),

        Commands::Status => status::execute(&workspace),

        Commands::Cleanup => cleanup::execute(&workspace),

        Commands::Check => check::execute(&workspace),

        Commands::Env { shell } => env::execute(&workspace, shell),

        Commands::Self_(action) => self_cmd::execute(&workspace, action),
    }
}
