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

#[cfg(all(test, feature = "dhat-heap"))]
#[global_allocator]
static ALLOC: dhat::Alloc = dhat::Alloc;

use crate::extract::extract_data_to_dataframe;
use crate::input::JobConfig;
use crate::output::{write_dataframe_to_parquet, write_dataframe_to_parquet_async};
use crate::storage::{StorageBackend, StorageFactory};

/// Converts a NetCDF file to Parquet according to the provided job configuration.
///
/// Opens the file, applies filters, extracts the variable into a DataFrame, runs
/// optional post-processing, and writes a Parquet file. The NetCDF file is closed
/// before the Parquet write to minimise peak memory usage.
pub fn process_netcdf_job(config: &JobConfig) -> Result<(), Nc2ParquetError> {
    // Close the NetCDF file before postprocessing and Parquet writing to free
    // file-descriptor and library buffers ahead of the peak-memory write step.
    let mut df = {
        let file = netcdf::open(&config.nc_key)?;
        let var = file
            .variable(&config.variable_name)
            .ok_or_else(|| Nc2ParquetError::VariableNotFound(config.variable_name.clone()))?;

        let mut filters = Vec::new();
        for filter_config in &config.filters {
            let filter = filter_config.to_filter()?;
            filters.push(filter);
        }

        let df = extract_data_to_dataframe(&file, &var, &config.variable_name, &filters)?;
        drop(var);
        file.close()?;
        df
    };

    if let Some(ref postprocess_config) = config.postprocessing {
        use crate::postprocess::ProcessingPipeline;
        let mut pipeline = ProcessingPipeline::from_config(postprocess_config)?;
        df = pipeline.execute(df)?;
    }

    write_dataframe_to_parquet(&mut df, &config.parquet_key)?;

    Ok(())
}

/// Async variant of [`process_netcdf_job`] with support for S3 input/output.
///
/// When an S3 URI is detected, the file is downloaded to a temporary path,
/// processed, and the temporary file is removed on completion.
pub async fn process_netcdf_job_async(config: &JobConfig) -> Result<(), Nc2ParquetError> {
    // `temp_file_path` is kept outside the extraction block so it can be cleaned up afterward.
    let (mut df, temp_file_path) = {
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

        let df = extract_data_to_dataframe(&file, &var, &config.variable_name, &filters)?;
        drop(var);
        file.close()?;
        (df, temp_file_path)
    };

    if let Some(ref postprocess_config) = config.postprocessing {
        use crate::postprocess::ProcessingPipeline;
        let mut pipeline = ProcessingPipeline::from_config(postprocess_config)?;
        df = pipeline.execute(df)?;
    }

    if config.parquet_key.starts_with("s3://") {
        write_dataframe_to_parquet_async(&mut df, &config.parquet_key).await?;
    } else {
        write_dataframe_to_parquet(&mut df, &config.parquet_key)?;
    }

    if let Some(temp_path) = temp_file_path
        && temp_path.exists()
    {
        std::fs::remove_file(temp_path)?;
    }

    Ok(())
}
