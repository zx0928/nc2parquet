use clap::Parser;
use log::{debug, error};
use std::process;

use nc2parquet::cli::{Cli, Commands};

mod handlers;

use handlers::{
    handle_completions_command, handle_convert_command, handle_info_command,
    handle_template_command, handle_validate_command,
};

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    init_logging(&cli);

    debug!("CLI arguments: {:?}", std::env::args().collect::<Vec<_>>());

    let result = match &cli.command {
        Commands::Convert { .. } => handle_convert_command(&cli).await,
        Commands::Validate { .. } => handle_validate_command(&cli).await,
        Commands::Info { .. } => handle_info_command(&cli).await,
        Commands::Template { .. } => handle_template_command(&cli).await,
        Commands::Completions { .. } => handle_completions_command(&cli).await,
    };

    match result {
        Ok(()) => debug!("Command completed successfully"),
        Err(e) => {
            error!("Command failed: {}", e);

            if cli.verbose {
                let mut cause = e.source();
                while let Some(err) = cause {
                    error!("  Caused by: {}", err);
                    cause = err.source();
                }
            }

            process::exit(1);
        }
    }
}

fn init_logging(cli: &Cli) {
    let log_level = if cli.quiet {
        "error"
    } else if cli.verbose {
        "debug"
    } else {
        "info"
    };

    env_logger::Builder::new()
        .filter_module("nc2parquet", log_level.parse().unwrap())
        .init();
}
