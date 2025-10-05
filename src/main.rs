use anyhow::Result;
use clap::Parser;
use devspace::cli::Cli;
use devspace::commands;

fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .init();

    // Parse CLI arguments
    let cli = Cli::parse();

    // Execute command
    commands::execute(cli)
}
