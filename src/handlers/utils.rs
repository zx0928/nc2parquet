use anyhow::{Context, Result};
use log::{info, warn};

use nc2parquet::{
    cli::OutputFormat,
    input::JobConfig,
    storage::{StorageBackend, StorageFactory},
};

/// Check if output file exists and handle overwrite logic
pub async fn check_output_overwrite(output_path: &str) -> Result<()> {
    let storage = StorageFactory::from_path(output_path).await?;

    if storage.exists(output_path).await? {
        return Err(anyhow::anyhow!(
            "Output file already exists: {}. Use --force to overwrite",
            output_path
        ));
    }

    Ok(())
}

/// Check if async processing is needed (for S3 paths)
pub fn needs_async_processing(config: &JobConfig) -> bool {
    config.nc_key.starts_with("s3://") || config.parquet_key.starts_with("s3://")
}

/// Print configuration summary
pub fn print_config_summary(config: &JobConfig, format: &OutputFormat) {
    match format {
        OutputFormat::Human => {
            println!("\nConfiguration Summary:");
            println!("  Input:    {}", config.nc_key);
            println!("  Variable: {}", config.variable_name);
            println!("  Output:   {}", config.parquet_key);
            println!("  Filters:  {}", config.filters.len());

            for (i, filter) in config.filters.iter().enumerate() {
                println!("    {}: {}", i + 1, filter.kind());
            }
        }
        OutputFormat::Json => {
            if let Ok(json) = serde_json::to_string_pretty(config) {
                println!("{}", json);
            }
        }
        _ => print_config_summary(config, &OutputFormat::Human),
    }
}

/// Show output file information
pub async fn show_output_info(output_path: &str, format: &OutputFormat) -> Result<()> {
    let storage = StorageFactory::from_path(output_path).await?;

    if !storage.exists(output_path).await? {
        warn!("Output file was not created: {}", output_path);
        return Ok(());
    }

    match format {
        OutputFormat::Human => {
            info!("Output file created successfully: {}", output_path);
        }
        OutputFormat::Json => {
            let info = serde_json::json!({
                "output_file": output_path,
                "status": "created"
            });
            println!("{}", serde_json::to_string_pretty(&info)?);
        }
        _ => {
            info!("Output: {}", output_path);
        }
    }

    Ok(())
}

/// Get file size for performance metrics
pub async fn get_file_size(file_path: &str) -> Result<u64> {
    if file_path.starts_with("s3://") {
        Ok(0)
    } else {
        let metadata = tokio::fs::metadata(file_path)
            .await
            .context("Failed to get file metadata")?;
        Ok(metadata.len())
    }
}
