use anyhow::{Context, Result};
use indicatif::{ProgressBar, ProgressStyle};
use log::info;
use nc2parquet::{
    cli::{Cli, Commands},
    input::{BatchConfig, CompressionCodec, OutputConfig},
    postprocess::{ProcessingPipelineConfig, ProcessorConfig},
    process_netcdf_batch, process_netcdf_job, process_netcdf_job_async,
};

use super::config::load_configuration;
use super::utils::{
    check_output_overwrite, get_file_size, needs_async_processing, print_config_summary,
    show_output_info,
};
use super::validate::validate_config;

/// Handle the convert subcommand
pub async fn handle_convert_command(cli: &Cli) -> Result<()> {
    if let Commands::Convert {
        input,
        output,
        variable,
        input_override,
        output_override,
        range_filters,
        list_filters,
        point2d_filters,
        point3d_filters,
        force,
        dry_run,
        rename_columns,
        unit_conversions,
        kelvin_to_celsius,
        formulas,
        variables,
        glob,
        compression,
        compression_level,
        row_group_size,
        no_statistics,
    } = &cli.command
    {
        info!("Starting NetCDF to Parquet conversion");

        if let Some(pattern) = glob {
            info!("Batch mode: glob pattern '{}'", pattern);

            // The output positional argument is treated as the output directory in glob mode.
            let output_dir = output
                .as_deref()
                .ok_or_else(|| {
                    anyhow::anyhow!(
                        "OUTPUT argument is required as the output directory in glob mode"
                    )
                })?
                .to_string();

            let variable_name = variable
                .as_deref()
                .ok_or_else(|| {
                    anyhow::anyhow!("Variable name (-n / --variable) is required in glob mode")
                })?
                .to_string();

            let batch_config = BatchConfig {
                pattern: pattern.clone(),
                output_dir,
                variable_name,
                filters: vec![],
                postprocessing: None,
                output_template: None,
                output: None,
                fail_fast: false,
            };

            let result = process_netcdf_batch(&batch_config).context("Batch processing failed")?;

            if !cli.quiet {
                println!(
                    "Batch complete: {}/{} files succeeded, {} failed",
                    result.succeeded.len(),
                    result.total_files,
                    result.failed.len()
                );
                for path in &result.succeeded {
                    println!("  OK  {}", path);
                }
                for (path, err) in &result.failed {
                    println!("  ERR {} — {}", path, err);
                }
            }

            return Ok(());
        }

        let mut config = load_configuration(cli, input, output, variable)?;

        if let Some(input_path) = input_override {
            config.nc_key = input_path.clone();
        }

        if let Some(output_path) = output_override {
            config.parquet_key = output_path.clone();
        }

        if !variables.is_empty() {
            config.variable_names = Some(variables.clone());
        }

        let (
            merged_range_filters,
            merged_list_filters,
            merged_point2d_filters,
            merged_point3d_filters,
        ) = nc2parquet::cli::merge_filters(
            range_filters.clone(),
            list_filters.clone(),
            point2d_filters.clone(),
            point3d_filters.clone(),
        )
        .map_err(|e| anyhow::anyhow!("Filter parsing error: {}", e))?;

        for filter in &merged_range_filters {
            config.filters.push(filter.clone().into());
        }
        for filter in &merged_list_filters {
            config.filters.push(filter.clone().into());
        }
        for filter in &merged_point2d_filters {
            config.filters.push(filter.clone().into());
        }
        for filter in &merged_point3d_filters {
            config.filters.push(filter.clone().into());
        }

        if !rename_columns.is_empty()
            || !unit_conversions.is_empty()
            || !kelvin_to_celsius.is_empty()
            || !formulas.is_empty()
        {
            let mut processors = Vec::new();

            if !rename_columns.is_empty() {
                let mappings = rename_columns
                    .iter()
                    .map(|r| (r.old_name.clone(), r.new_name.clone()))
                    .collect();
                processors.push(ProcessorConfig::RenameColumns { mappings });
            }

            for unit_conversion in unit_conversions.iter() {
                processors.push(ProcessorConfig::UnitConvert {
                    column: unit_conversion.column.clone(),
                    from_unit: unit_conversion.from_unit.clone(),
                    to_unit: unit_conversion.to_unit.clone(),
                });
            }

            for column in kelvin_to_celsius {
                processors.push(ProcessorConfig::UnitConvert {
                    column: column.clone(),
                    from_unit: "kelvin".to_string(),
                    to_unit: "celsius".to_string(),
                });
            }

            for formula in formulas.iter() {
                processors.push(ProcessorConfig::ApplyFormula {
                    target_column: formula.target_column.clone(),
                    formula: formula.formula.clone(),
                    source_columns: formula.source_columns.clone(),
                });
            }

            if !processors.is_empty() {
                config.postprocessing = Some(ProcessingPipelineConfig {
                    name: Some("CLI Pipeline".to_string()),
                    processors,
                });
            }
        }

        let any_compression_flag = compression.is_some()
            || compression_level.is_some()
            || row_group_size.is_some()
            || *no_statistics;
        if any_compression_flag {
            let codec = match compression.as_deref().unwrap_or("snappy") {
                "snappy" => CompressionCodec::Snappy,
                "zstd" => CompressionCodec::Zstd,
                "gzip" => CompressionCodec::Gzip,
                "lz4" => CompressionCodec::Lz4,
                "uncompressed" => CompressionCodec::Uncompressed,
                other => {
                    return Err(anyhow::anyhow!(
                        "Unknown compression codec '{}'. Valid values: snappy, zstd, gzip, lz4, uncompressed",
                        other
                    ));
                }
            };
            let output_cfg = OutputConfig {
                compression: codec,
                compression_level: *compression_level,
                row_group_size: *row_group_size,
                data_page_size: None,
                statistics: !no_statistics,
            };
            output_cfg
                .validate()
                .map_err(|e| anyhow::anyhow!("Output config error: {}", e))?;
            config.output = Some(output_cfg);
        }

        validate_config(&config).await?;

        if !force && !*dry_run {
            check_output_overwrite(&config.parquet_key).await?;
        }

        if *dry_run {
            info!("Dry run mode - configuration validated successfully");
            print_config_summary(&config, &cli.output_format);
            return Ok(());
        }

        info!("Processing: {} -> {}", config.nc_key, config.parquet_key);
        let eff_vars = config.effective_variable_names();
        if eff_vars.len() == 1 {
            info!("Variable: {}", eff_vars[0]);
        } else {
            info!("Variables: {:?}", eff_vars);
        }
        info!("Filters: {} configured", config.filters.len());

        let progress = if cli.quiet {
            None
        } else {
            let pb = ProgressBar::new_spinner();
            pb.set_style(
                ProgressStyle::default_spinner()
                    .template("{spinner:.green} {msg}")
                    .unwrap()
                    .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]),
            );
            pb.set_message("Initializing conversion...");
            Some(pb)
        };

        let start_time = std::time::Instant::now();

        if let Some(ref pb) = progress {
            pb.set_message("Reading NetCDF file...");
        }

        if needs_async_processing(&config) {
            if let Some(ref pb) = progress {
                pb.set_message("Processing with async pipeline...");
            }
            process_netcdf_job_async(&config)
                .await
                .context("Failed to process NetCDF file with async pipeline")?;
        } else {
            if let Some(ref pb) = progress {
                pb.set_message("Processing with sync pipeline...");
            }
            process_netcdf_job(&config).context("Failed to process NetCDF file")?;
        }

        let duration = start_time.elapsed();

        if let Some(pb) = progress {
            pb.finish_with_message(format!(
                "✅ Conversion completed in {:.2}s",
                duration.as_secs_f64()
            ));
        }

        if duration.as_secs() > 1 {
            info!(
                "Conversion completed in {:.2} seconds",
                duration.as_secs_f64()
            );
        } else {
            info!(
                "Conversion completed in {:.0} milliseconds",
                duration.as_millis()
            );
        }

        if cli.verbose
            && let Ok(file_size) = get_file_size(&config.nc_key).await
        {
            let throughput = file_size as f64 / duration.as_secs_f64() / 1_048_576.0; // MB/s
            info!("Input file size: {:.2} MB", file_size as f64 / 1_048_576.0);
            info!("Processing throughput: {:.2} MB/s", throughput);
        }

        show_output_info(&config.parquet_key, &cli.output_format).await?;
    } else {
        unreachable!("Convert command handler called with wrong command type");
    }

    Ok(())
}
