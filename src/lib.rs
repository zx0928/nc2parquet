//! # nc2parquet
//!
//! A Rust library for converting NetCDF files to Parquet format with flexible filtering capabilities.
//!
//! ## Features
//!
//! - **Multiple filter types**: Range filters, list filters, 2D point filters, and 3D point filters
//! - **Filter intersection**: Apply multiple filters that intersect properly across dimensions
//! - **Efficient processing**: Only extracts data for coordinates that match all filter criteria
//! - **Post-processing framework**: Transform DataFrames with built-in processors and custom pipelines
//! - **Type safety**: Strong typing with comprehensive error handling

pub mod cli;
pub mod extract;
pub mod filters;
pub mod info;
pub mod input;
pub mod output;
pub mod postprocess;
pub mod storage;

#[cfg(test)]
mod tests;

#[cfg(test)]
pub mod test_helpers;

use crate::extract::extract_data_to_dataframe;
use crate::input::JobConfig;
use crate::output::{write_dataframe_to_parquet, write_dataframe_to_parquet_async};
use crate::storage::{StorageBackend, StorageFactory};

/// Processes a NetCDF file according to the provided job configuration.
///
/// This function orchestrates the entire conversion pipeline:
/// 1. Opens the NetCDF file
/// 2. Validates the specified variable exists
/// 3. Applies all configured filters with intersection logic
/// 4. Extracts the filtered data into a DataFrame
/// 5. Writes the DataFrame to a Parquet file
///
/// # Arguments
///
/// * `config` - The job configuration specifying input file, filters, and output
///
/// # Returns
///
/// Returns `Ok(())` on successful conversion, or an error if any step fails.
///
/// # Errors
///
/// This function will return an error if:
/// - The NetCDF file cannot be opened
/// - The specified variable is not found in the NetCDF file
/// - Any filter fails to apply
/// - The output Parquet file cannot be written
pub fn process_netcdf_job(config: &JobConfig) -> Result<(), Box<dyn std::error::Error>> {
    let file = netcdf::open(&config.nc_key)?;
    let var = file.variable(&config.variable_name).ok_or(format!(
        "Variable '{}' not found in NetCDF file",
        config.variable_name
    ))?;

    let mut filters = Vec::new();
    for filter_config in &config.filters {
        let filter = filter_config.to_filter()?;
        filters.push(filter);
    }

    let mut df = extract_data_to_dataframe(&file, &var, &config.variable_name, &filters)?;

    // Apply post-processing if configured
    if let Some(ref postprocess_config) = config.postprocessing {
        use crate::postprocess::ProcessingPipeline;
        let mut pipeline = ProcessingPipeline::from_config(postprocess_config)?;
        df = pipeline.execute(df)?;
    }

    write_dataframe_to_parquet(&df, &config.parquet_key)?;
    file.close()?;

    Ok(())
}

/// Async version of NetCDF processing that supports both local files and S3.
///
/// This function provides the same functionality as `process_netcdf_job` but with
/// support for S3 input files. When an S3 path is detected, the file is downloaded
/// to a temporary location, processed, and then cleaned up.
///
/// # Arguments
///
/// * `config` - The job configuration specifying input file, filters, and output
///
/// # Returns
///
/// Returns `Ok(())` on successful conversion, or an error if any step fails.
///
/// # Errors
///
/// This function will return an error if:
/// - The input file cannot be accessed (local or S3)
/// - The NetCDF file format is invalid
/// - The specified variable is not found in the NetCDF file
/// - Any filter fails to apply
/// - The output file cannot be written (local or S3)
pub async fn process_netcdf_job_async(
    config: &JobConfig,
) -> Result<(), Box<dyn std::error::Error>> {
    // Check if input is S3 path
    let (file, temp_file_path) = if config.nc_key.starts_with("s3://") {
        // Download from S3 to temporary file
        let storage = StorageFactory::from_path(&config.nc_key).await?;
        let data = storage.read(&config.nc_key).await?;

        // Create temporary file
        let temp_file = tempfile::NamedTempFile::new()?;
        let temp_path = temp_file.path().to_path_buf();

        // Write S3 data to temporary file
        tokio::fs::write(&temp_path, data).await?;

        // Open NetCDF file from temporary location
        let file = netcdf::open(&temp_path)?;
        (file, Some(temp_path))
    } else {
        // Open local file directly
        let file = netcdf::open(&config.nc_key)?;
        (file, None)
    };

    let var = file.variable(&config.variable_name).ok_or(format!(
        "Variable '{}' not found in NetCDF file",
        config.variable_name
    ))?;

    let mut filters = Vec::new();
    for filter_config in &config.filters {
        let filter = filter_config.to_filter()?;
        filters.push(filter);
    }

    let mut df = extract_data_to_dataframe(&file, &var, &config.variable_name, &filters)?;

    // Apply post-processing if configured
    if let Some(ref postprocess_config) = config.postprocessing {
        use crate::postprocess::ProcessingPipeline;
        let mut pipeline = ProcessingPipeline::from_config(postprocess_config)?;
        df = pipeline.execute(df)?;
    }

    // Check if output is S3 path
    if config.parquet_key.starts_with("s3://") {
        write_dataframe_to_parquet_async(&df, &config.parquet_key).await?;
    } else {
        write_dataframe_to_parquet(&df, &config.parquet_key)?;
    }

    file.close()?;

    // Clean up temporary file if it was created
    if let Some(temp_path) = temp_file_path
        && temp_path.exists()
    {
        std::fs::remove_file(temp_path)?;
    }

    Ok(())
}
