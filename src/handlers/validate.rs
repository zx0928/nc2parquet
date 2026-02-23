use anyhow::Result;
use indicatif::{ProgressBar, ProgressStyle};
use log::{debug, info, warn};
use std::time::Duration;

use nc2parquet::{
    cli::{Cli, Commands, OutputFormat},
    input::{FilterConfig, JobConfig},
    postprocess::ProcessorConfig,
};

use super::config::load_configuration;

/// Handle the validate subcommand
pub async fn handle_validate_command(cli: &Cli) -> Result<()> {
    if let Commands::Validate {
        config_file,
        detailed,
    } = &cli.command
    {
        info!("Validating configuration");

        let progress = if cli.quiet {
            None
        } else {
            let progress = ProgressBar::new_spinner();
            progress.enable_steady_tick(Duration::from_millis(80));
            progress.set_style(
                ProgressStyle::with_template("{spinner:.green} {msg}")
                    .unwrap()
                    .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]),
            );
            progress.set_message("Validating configuration...");
            Some(progress)
        };

        let config = load_configuration(
            cli,
            &config_file
                .as_ref()
                .map(|p| p.to_string_lossy().to_string()),
            &None,
            &None,
        )?;

        if let Some(pb) = &progress {
            pb.set_message("Running configuration checks...");
        }

        validate_config(&config).await?;

        if let Some(pb) = &progress {
            pb.finish_with_message("✓ Configuration valid!");
        }

        if *detailed {
            show_detailed_validation(&config, &cli.output_format).await?;
        } else {
            println!("Configuration validation passed successfully");
        }

        Ok(())
    } else {
        unreachable!("Validate command handler called with wrong command type");
    }
}

/// Validate configuration
pub async fn validate_config(config: &JobConfig) -> Result<()> {
    let mut errors = Vec::new();
    let mut warnings = Vec::new();

    if config.nc_key.is_empty() {
        errors.push("Input NetCDF path cannot be empty".to_string());
    } else {
        if !config.nc_key.starts_with("s3://") {
            let path = std::path::Path::new(&config.nc_key);
            if !path.exists() {
                warnings.push(format!("Input file does not exist: {}", config.nc_key));
            } else if !path.is_file() {
                errors.push(format!("Input path is not a file: {}", config.nc_key));
            }
        }

        if !config.nc_key.ends_with(".nc") && !config.nc_key.ends_with(".nc4") {
            warnings.push(format!(
                "Input file does not have a typical NetCDF extension (.nc or .nc4): {}",
                config.nc_key
            ));
        }
    }

    if config.parquet_key.is_empty() {
        errors.push("Output Parquet path cannot be empty".to_string());
    } else {
        if !config.parquet_key.starts_with("s3://") {
            let output_path = std::path::Path::new(&config.parquet_key);
            if let Some(parent) = output_path.parent()
                && !parent.exists()
            {
                warnings.push(format!(
                    "Output directory does not exist: {}",
                    parent.display()
                ));
            }
        }

        if !config.parquet_key.ends_with(".parquet") && !config.parquet_key.ends_with(".pq") {
            warnings.push(format!(
                "Output file does not have a typical Parquet extension (.parquet or .pq): {}",
                config.parquet_key
            ));
        }
    }

    if config.variable_name.is_empty() {
        errors.push("Variable name cannot be empty".to_string());
    } else if config.variable_name.contains(" ") || config.variable_name.contains("\t") {
        errors.push(format!(
            "Variable name contains whitespace: '{}'",
            config.variable_name
        ));
    }

    for (i, filter) in config.filters.iter().enumerate() {
        match filter.to_filter() {
            Ok(_) => match filter {
                nc2parquet::input::FilterConfig::Range { params } => {
                    if params.min_value >= params.max_value {
                        errors.push(format!(
                            "Filter {}: Range min_value ({}) must be less than max_value ({})",
                            i + 1,
                            params.min_value,
                            params.max_value
                        ));
                    }
                    if params.dimension_name.is_empty() {
                        errors.push(format!(
                            "Filter {}: Range dimension_name cannot be empty",
                            i + 1
                        ));
                    }
                }
                nc2parquet::input::FilterConfig::List { params } => {
                    if params.values.is_empty() {
                        warnings.push(format!(
                            "Filter {}: List filter has no values (will match nothing)",
                            i + 1
                        ));
                    }
                    if params.dimension_name.is_empty() {
                        errors.push(format!(
                            "Filter {}: List dimension_name cannot be empty",
                            i + 1
                        ));
                    }
                }
                nc2parquet::input::FilterConfig::Point2D { params } => {
                    if params.points.is_empty() {
                        warnings.push(format!(
                            "Filter {}: 2D point filter has no points (will match nothing)",
                            i + 1
                        ));
                    }
                    if params.tolerance < 0.0 {
                        errors.push(format!(
                            "Filter {}: 2D point tolerance cannot be negative: {}",
                            i + 1,
                            params.tolerance
                        ));
                    }
                    if params.lat_dimension_name.is_empty() || params.lon_dimension_name.is_empty()
                    {
                        errors.push(format!("Filter {}: 2D point latitude and longitude dimension names cannot be empty", i + 1));
                    }
                }
                nc2parquet::input::FilterConfig::Point3D { params } => {
                    if params.points.is_empty() || params.steps.is_empty() {
                        warnings.push(format!("Filter {}: 3D point filter has no points or steps (will match nothing)", i + 1));
                    }
                    if params.tolerance < 0.0 {
                        errors.push(format!(
                            "Filter {}: 3D point tolerance cannot be negative: {}",
                            i + 1,
                            params.tolerance
                        ));
                    }
                    if params.time_dimension_name.is_empty()
                        || params.lat_dimension_name.is_empty()
                        || params.lon_dimension_name.is_empty()
                    {
                        errors.push(format!("Filter {}: 3D point time, latitude, and longitude dimension names cannot be empty", i + 1));
                    }
                }
            },
            Err(e) => {
                errors.push(format!("Invalid filter at index {}: {}", i + 1, e));
            }
        }
    }

    if std::env::var("NC2PARQUET_CONFIG").is_ok()
        && std::env::var("NC2PARQUET_CONFIG").unwrap().is_empty()
    {
        warnings.push("NC2PARQUET_CONFIG environment variable is set but empty".to_string());
    }

    for warning in &warnings {
        warn!("Configuration warning: {}", warning);
    }

    if !errors.is_empty() {
        let error_msg = format!(
            "Configuration validation failed with {} error(s):\n{}",
            errors.len(),
            errors
                .iter()
                .enumerate()
                .map(|(i, e)| format!("  {}. {}", i + 1, e))
                .collect::<Vec<_>>()
                .join("\n")
        );
        return Err(anyhow::anyhow!(error_msg));
    }

    if warnings.is_empty() {
        info!("Configuration validation passed");
    } else {
        info!(
            "Configuration validation passed with {} warning(s)",
            warnings.len()
        );
    }

    debug!("Configuration validation completed successfully");
    Ok(())
}

pub async fn show_detailed_validation(config: &JobConfig, format: &OutputFormat) -> Result<()> {
    println!("\n=== Detailed Validation Report ===");

    println!("\n1. Configuration Summary:");
    println!("   Input:        {}", config.nc_key);
    println!("   Variable:     {}", config.variable_name);
    println!("   Output:       {}", config.parquet_key);
    println!("   Format:       {:?}", format);

    println!("\n2. Storage Information:");
    let input_storage = if config.nc_key.starts_with("s3://") {
        "S3"
    } else {
        "Local"
    };
    let output_storage = if config.parquet_key.starts_with("s3://") {
        "S3"
    } else {
        "Local"
    };
    println!("   Input Storage:  {}", input_storage);
    println!("   Output Storage: {}", output_storage);

    if !config.filters.is_empty() {
        println!("\n3. Filters Applied:");
        println!("   Total Filters: {}", config.filters.len());

        for (i, filter) in config.filters.iter().enumerate() {
            match filter {
                FilterConfig::Range { params } => {
                    println!(
                        "     {}. Range Filter: {} ({} to {})",
                        i + 1,
                        params.dimension_name,
                        params.min_value,
                        params.max_value
                    );
                }
                FilterConfig::List { params } => {
                    println!(
                        "     {}. List Filter: {} {:?}",
                        i + 1,
                        params.dimension_name,
                        params.values
                    );
                }
                FilterConfig::Point2D { params } => {
                    println!(
                        "     {}. Point2D Filter: {},{} {} points ±{}",
                        i + 1,
                        params.lat_dimension_name,
                        params.lon_dimension_name,
                        params.points.len(),
                        params.tolerance
                    );
                    for (j, (lat, lon)) in params.points.iter().enumerate() {
                        if j < 3 {
                            println!("         Point {}: ({}, {})", j + 1, lat, lon);
                        } else if j == 3 {
                            println!("         ... and {} more", params.points.len() - 3);
                            break;
                        }
                    }
                }
                FilterConfig::Point3D { params } => {
                    println!(
                        "     {}. Point3D Filter: {},{},{} {} points, {} steps ±{}",
                        i + 1,
                        params.time_dimension_name,
                        params.lat_dimension_name,
                        params.lon_dimension_name,
                        params.points.len(),
                        params.steps.len(),
                        params.tolerance
                    );
                    for (j, (lat, lon)) in params.points.iter().enumerate() {
                        if j < 2 {
                            println!("         Point {}: ({}, {})", j + 1, lat, lon);
                        } else if j == 2 {
                            println!("         ... and {} more", params.points.len() - 2);
                            break;
                        }
                    }
                }
            }
        }
    } else {
        println!("\n3. Filters Applied: None");
    }

    if let Some(postprocessing) = &config.postprocessing {
        println!("\n4. Post-Processing:");
        println!(
            "   Pipeline: {} processors defined",
            postprocessing.processors.len()
        );
        for (i, processor) in postprocessing.processors.iter().enumerate() {
            let processor_type = match processor {
                ProcessorConfig::RenameColumns { .. } => "Rename Columns",
                ProcessorConfig::DatetimeConvert { .. } => "Datetime Convert",
                ProcessorConfig::UnitConvert { .. } => "Unit Convert",
                ProcessorConfig::Aggregate { .. } => "Aggregate",
                ProcessorConfig::ApplyFormula { .. } => "Apply Formula",
            };
            println!("     {}. {}", i + 1, processor_type);
        }
    } else {
        println!("\n4. Post-Processing: None");
    }

    println!("\n✓ All validation checks passed");
    Ok(())
}
