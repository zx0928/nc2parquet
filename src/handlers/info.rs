use anyhow::Result;
use indicatif::{ProgressBar, ProgressStyle};
use log::info;

use nc2duckdb::{
    cli::{Cli, Commands, OutputFormat},
    info::{
        get_netcdf_info, print_file_info_csv, print_file_info_human, print_file_info_json,
        print_file_info_yaml,
    },
};

/// Handle the info subcommand
pub async fn handle_info_command(cli: &Cli) -> Result<()> {
    if let Commands::Info {
        file,
        detailed,
        variable,
        format,
    } = &cli.command
    {
        info!("Gathering file information: {}", file);

        let progress = if cli.quiet {
            None
        } else {
            let pb = ProgressBar::new_spinner();
            pb.set_style(
                ProgressStyle::default_spinner()
                    .template("{spinner:.cyan} {msg}")
                    .unwrap()
                    .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]),
            );
            pb.set_message("Analyzing NetCDF file...");
            Some(pb)
        };

        let output_format = format.as_ref().unwrap_or(&cli.output_format);

        let file_info = get_netcdf_info(file, variable.as_deref(), *detailed).await?;

        if let Some(pb) = progress {
            pb.finish_with_message("✅ File analysis completed");
        }

        match output_format {
            OutputFormat::Human => print_file_info_human(&file_info),
            OutputFormat::Json => print_file_info_json(&file_info)?,
            OutputFormat::Yaml => print_file_info_yaml(&file_info)?,
            OutputFormat::Csv => print_file_info_csv(&file_info)?,
        }
    } else {
        unreachable!("Info command handler called with wrong command type");
    }

    Ok(())
}
