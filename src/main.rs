use anyhow::Result;
use clap::Parser;
use dws::cli::Cli;
use dws::commands;

fn main() -> Result<()> {
    // Initialize tracing
    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("dws=info,ubi=warn,info"));

    tracing_subscriber::fmt().with_env_filter(env_filter).init();

    // Parse CLI arguments
    let cli = Cli::parse();

    // Execute command
    commands::execute(cli)
}
