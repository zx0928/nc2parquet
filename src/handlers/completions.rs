use anyhow::{Context, Result};
use clap::CommandFactory;
use clap_complete::{Shell, generate};
use log::info;

use nc2duckdb::cli::{Cli, Commands};

/// Handle the completions subcommand
pub async fn handle_completions_command(cli: &Cli) -> Result<()> {
    if let Commands::Completions { shell, output } = &cli.command {
        info!("Generating shell completions for: {:?}", shell);

        let mut cmd = Cli::command();
        let name = cmd.get_name().to_string();

        let completions = match shell {
            Shell::Bash | Shell::Zsh | Shell::Fish | Shell::PowerShell => {
                let mut buf = Vec::new();
                generate(*shell, &mut cmd, name, &mut buf);
                String::from_utf8(buf)
                    .with_context(|| format!("Failed to generate {:?} completions", shell))?
            }
            _ => return Err(anyhow::anyhow!("Unsupported shell: {:?}", shell)),
        };

        match output {
            Some(path) => {
                std::fs::write(path, &completions).context("Failed to write completions file")?;
                info!("Completions written to: {}", path.display());
            }
            None => print!("{}", completions),
        }
    } else {
        unreachable!("Completions command handler called with wrong command type");
    }

    Ok(())
}
