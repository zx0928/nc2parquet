pub mod cli;
pub mod errors;
pub(crate) mod extract;
pub mod filters;
pub mod info;
pub mod input;
pub(crate) mod output;
pub mod postprocess;
pub mod storage;

pub use errors::Nc2ParquetError;

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
///
/// # Examples
///
/// ```rust,no_run
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// use nc2parquet::process_netcdf_job;
/// use nc2parquet::input::JobConfig;
///
/// let config = JobConfig::from_json(r#"{
///     "nc_key": "data/temperature.nc",
///     "variable_name": "t2m",
///     "parquet_key": "output/temperature.parquet",
///     "filters": []
/// }"#)?;
///
/// process_netcdf_job(&config)?;
/// # Ok(())
/// # }
/// ```
pub fn process_netcdf_job(config: &JobConfig) -> Result<(), Nc2ParquetError> {
    let file = netcdf::open(&config.nc_key)?;
    let var = file
        .variable(&config.variable_name)
        .ok_or_else(|| Nc2ParquetError::VariableNotFound(config.variable_name.clone()))?;

    let mut filters = Vec::new();
    for filter_config in &config.filters {
        let filter = filter_config.to_filter()?;
        filters.push(filter);
    }

    let mut df = extract_data_to_dataframe(&file, &var, &config.variable_name, &filters)?;

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
///
/// # Examples
///
/// ```rust,no_run
/// # #[tokio::main]
/// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
/// use nc2parquet::process_netcdf_job_async;
/// use nc2parquet::input::JobConfig;
///
/// // Process a local file asynchronously
/// let config = JobConfig::from_json(r#"{
///     "nc_key": "data/temperature.nc",
///     "variable_name": "t2m",
///     "parquet_key": "output/temperature.parquet",
///     "filters": []
/// }"#)?;
///
/// process_netcdf_job_async(&config).await?;
///
/// // Process an S3 file (requires valid AWS credentials)
/// let s3_config = JobConfig::from_json(r#"{
///     "nc_key": "s3://my-bucket/data/temperature.nc",
///     "variable_name": "t2m",
///     "parquet_key": "s3://my-bucket/output/temperature.parquet",
///     "filters": []
/// }"#)?;
///
/// process_netcdf_job_async(&s3_config).await?;
/// # Ok(())
/// # }
/// ```
pub async fn process_netcdf_job_async(config: &JobConfig) -> Result<(), Nc2ParquetError> {
    let (file, temp_file_path) = if config.nc_key.starts_with("s3://") {
        let storage = StorageFactory::from_path(&config.nc_key).await?;
        let data = storage.read(&config.nc_key).await?;
        let temp_file = tempfile::NamedTempFile::new()?;
        let temp_path = temp_file.path().to_path_buf();
        tokio::fs::write(&temp_path, data).await?;
        let file = netcdf::open(&temp_path)?;
        (file, Some(temp_path))
    } else {
        let file = netcdf::open(&config.nc_key)?;
        (file, None)
    };

    let var = file
        .variable(&config.variable_name)
        .ok_or_else(|| Nc2ParquetError::VariableNotFound(config.variable_name.clone()))?;

    let mut filters = Vec::new();
    for filter_config in &config.filters {
        let filter = filter_config.to_filter()?;
        filters.push(filter);
    }

    let mut df = extract_data_to_dataframe(&file, &var, &config.variable_name, &filters)?;

    if let Some(ref postprocess_config) = config.postprocessing {
        use crate::postprocess::ProcessingPipeline;
        let mut pipeline = ProcessingPipeline::from_config(postprocess_config)?;
        df = pipeline.execute(df)?;
    }

    if config.parquet_key.starts_with("s3://") {
        write_dataframe_to_parquet_async(&df, &config.parquet_key).await?;
    } else {
        write_dataframe_to_parquet(&df, &config.parquet_key)?;
    }

    file.close()?;

    if let Some(temp_path) = temp_file_path
        && temp_path.exists()
    {
        std::fs::remove_file(temp_path)?;
    }

    Ok(())
}
