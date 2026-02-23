use anyhow::{Context, Result};
use indicatif::{ProgressBar, ProgressStyle};
use log::{debug, info};
use std::collections::HashMap;

use nc2parquet::{
    cli::{Cli, Commands},
    postprocess::{ProcessingPipelineConfig, ProcessorConfig},
    process_netcdf_job, process_netcdf_job_async,
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
    } = &cli.command
    {
        info!("Starting NetCDF to Parquet conversion");

        let mut config = load_configuration(cli, input, output, variable)?;

        if let Some(input_path) = input_override {
            config.nc_key = input_path.clone();
            debug!("Overriding input path: {}", input_path);
        }

        if let Some(output_path) = output_override {
            config.parquet_key = output_path.clone();
            debug!("Overriding output path: {}", output_path);
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

        for range_filter in &merged_range_filters {
            let filter_config = range_filter.clone().into();
            config.filters.push(filter_config);
            debug!(
                "Added range filter: {}:{}-{}",
                range_filter.dimension, range_filter.min_value, range_filter.max_value
            );
        }

        for list_filter in &merged_list_filters {
            let filter_config = list_filter.clone().into();
            config.filters.push(filter_config);
            debug!(
                "Added list filter: {}:{:?}",
                list_filter.dimension, list_filter.values
            );
        }

        for point2d_filter in &merged_point2d_filters {
            let filter_config = point2d_filter.clone().into();
            config.filters.push(filter_config);
            debug!(
                "Added 2D point filter: {},{} at ({},{}) tolerance={}",
                point2d_filter.lat_dimension,
                point2d_filter.lon_dimension,
                point2d_filter.lat,
                point2d_filter.lon,
                point2d_filter.tolerance
            );
        }

        for point3d_filter in &merged_point3d_filters {
            let filter_config = point3d_filter.clone().into();
            config.filters.push(filter_config);
            debug!(
                "Added 3D point filter: {},{},{} at ({},{},{}) tolerance={}",
                point3d_filter.time_dimension,
                point3d_filter.lat_dimension,
                point3d_filter.lon_dimension,
                point3d_filter.time,
                point3d_filter.lat,
                point3d_filter.lon,
                point3d_filter.tolerance
            );
        }

        if !rename_columns.is_empty()
            || !unit_conversions.is_empty()
            || !kelvin_to_celsius.is_empty()
            || !formulas.is_empty()
        {
            let mut processors = Vec::new();

            if !rename_columns.is_empty() {
                let mut mappings = HashMap::new();
                for rename in rename_columns.iter() {
                    mappings.insert(rename.old_name.clone(), rename.new_name.clone());
                    debug!(
                        "Added column rename: {} -> {}",
                        rename.old_name, rename.new_name
                    );
                }
                processors.push(ProcessorConfig::RenameColumns { mappings });
            }

            for unit_conversion in unit_conversions.iter() {
                processors.push(ProcessorConfig::UnitConvert {
                    column: unit_conversion.column.clone(),
                    from_unit: unit_conversion.from_unit.clone(),
                    to_unit: unit_conversion.to_unit.clone(),
                });
                debug!(
                    "Added unit conversion: {} from {} to {}",
                    unit_conversion.column, unit_conversion.from_unit, unit_conversion.to_unit
                );
            }

            for column in kelvin_to_celsius {
                processors.push(ProcessorConfig::UnitConvert {
                    column: column.clone(),
                    from_unit: "kelvin".to_string(),
                    to_unit: "celsius".to_string(),
                });
                debug!("Added Kelvin to Celsius conversion for column: {}", column);
            }

            for formula in formulas.iter() {
                processors.push(ProcessorConfig::ApplyFormula {
                    target_column: formula.target_column.clone(),
                    formula: formula.formula.clone(),
                    source_columns: formula.source_columns.clone(),
                });
                debug!(
                    "Added formula: {} = {} (sources: {:?})",
                    formula.target_column, formula.formula, formula.source_columns
                );
            }

            if !processors.is_empty() {
                let pipeline_config = ProcessingPipelineConfig {
                    name: Some("CLI Pipeline".to_string()),
                    processors,
                };
                config.postprocessing = Some(pipeline_config);
                info!(
                    "Created post-processing pipeline with {} processors",
                    config.postprocessing.as_ref().unwrap().processors.len()
                );
            }
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
        info!("Variable: {}", config.variable_name);
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
