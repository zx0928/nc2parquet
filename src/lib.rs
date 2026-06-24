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
pub use input::{BatchConfig, BatchResult};

#[cfg(test)]
mod tests;

#[cfg(test)]
pub mod test_helpers;

#[cfg(all(test, feature = "dhat-heap"))]
#[global_allocator]
static ALLOC: dhat::Alloc = dhat::Alloc;

use crate::extract::{
    extract_data_to_dataframe, extract_merge_variable_dataframe, extract_multi_variable_dataframe,
};
use crate::input::JobConfig;
use crate::output::{write_dataframe_to_parquet, write_dataframe_to_parquet_async};
use crate::storage::{StorageBackend, StorageFactory};
use std::path::{Path, PathBuf};

/// Converts a NetCDF file to Parquet according to the provided job configuration.
///
/// Opens the file, applies filters, extracts the variable into a DataFrame, runs
/// optional post-processing, and writes a Parquet file. The NetCDF file is closed
/// before the Parquet write to minimise peak memory usage.
///
/// When `config.variable_names` is set with more than one entry, all listed
/// variables are extracted into a single DataFrame with shared dimension columns.
pub fn process_netcdf_job(config: &JobConfig) -> Result<(), Nc2ParquetError> {
    let var_names = config.effective_variable_names();

    // Close the NetCDF file before postprocessing and Parquet writing to free
    // file-descriptor and library buffers ahead of the peak-memory write step.
    let mut df = {
        let file = netcdf::open(&config.nc_key)?;

        let mut filters = Vec::new();
        for filter_config in &config.filters {
            let filter = filter_config.to_filter()?;
            filters.push(filter);
        }

        let df = if let Some(ref merge_vars) = config.merge_variable_names {
            extract_merge_variable_dataframe(&file, merge_vars, &filters)?
        } else if var_names.len() == 1 {
            let var = file
                .variable(&var_names[0])
                .ok_or_else(|| Nc2ParquetError::VariableNotFound(var_names[0].clone()))?;
            let result = extract_data_to_dataframe(&file, &var, &var_names[0], &filters)?;
            drop(var);
            result
        } else {
            extract_multi_variable_dataframe(&file, &var_names, &filters)?
        };

        file.close()?;
        df
    };

    if let Some(ref postprocess_config) = config.postprocessing {
        use crate::postprocess::ProcessingPipeline;
        let mut pipeline = ProcessingPipeline::from_config(postprocess_config)?;
        df = pipeline.execute(df)?;
    }

    write_dataframe_to_parquet(&mut df, &config.parquet_key, config.output.as_ref())?;

    Ok(())
}

/// Async variant of [`process_netcdf_job`] with support for S3 input/output.
///
/// When an S3 URI is detected, the file is downloaded to a temporary path,
/// processed, and the temporary file is removed on completion.
///
/// When `config.variable_names` is set with more than one entry, all listed
/// variables are extracted into a single DataFrame with shared dimension columns.
pub async fn process_netcdf_job_async(config: &JobConfig) -> Result<(), Nc2ParquetError> {
    let var_names = config.effective_variable_names();

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

        let mut filters = Vec::new();
        for filter_config in &config.filters {
            let filter = filter_config.to_filter()?;
            filters.push(filter);
        }

        let df = if let Some(ref merge_vars) = config.merge_variable_names {
            extract_merge_variable_dataframe(&file, merge_vars, &filters)?
        } else if var_names.len() == 1 {
            let var = file
                .variable(&var_names[0])
                .ok_or_else(|| Nc2ParquetError::VariableNotFound(var_names[0].clone()))?;
            let result = extract_data_to_dataframe(&file, &var, &var_names[0], &filters)?;
            drop(var);
            result
        } else {
            extract_multi_variable_dataframe(&file, &var_names, &filters)?
        };

        file.close()?;
        (df, temp_file_path)
    };

    if let Some(ref postprocess_config) = config.postprocessing {
        use crate::postprocess::ProcessingPipeline;
        let mut pipeline = ProcessingPipeline::from_config(postprocess_config)?;
        df = pipeline.execute(df)?;
    }

    if config.parquet_key.starts_with("s3://") {
        write_dataframe_to_parquet_async(&mut df, &config.parquet_key, config.output.as_ref())
            .await?;
    } else {
        write_dataframe_to_parquet(&mut df, &config.parquet_key, config.output.as_ref())?;
    }

    if let Some(temp_path) = temp_file_path
        && temp_path.exists()
    {
        std::fs::remove_file(temp_path)?;
    }

    Ok(())
}

/// Resolves the output path for a single file within a batch conversion.
///
/// The `template` string may contain:
/// - `{stem}` — replaced with the input filename without its extension
/// - `{name}` — replaced with the full input filename (including extension)
///
/// The resolved filename is joined to `output_dir`.
///
/// # Examples
///
/// ```rust
/// use std::path::{Path, PathBuf};
/// use nc2parquet::resolve_output_path;
///
/// let result = resolve_output_path(
///     Path::new("/data/temperature.nc"),
///     "/tmp/out",
///     "{stem}.parquet",
/// );
/// assert_eq!(result, PathBuf::from("/tmp/out/temperature.parquet"));
///
/// let result = resolve_output_path(
///     Path::new("/data/temperature.nc"),
///     "/tmp/out",
///     "{stem}_converted.parquet",
/// );
/// assert_eq!(result, PathBuf::from("/tmp/out/temperature_converted.parquet"));
///
/// let result = resolve_output_path(
///     Path::new("/data/temperature.nc"),
///     "/tmp/out",
///     "{name}.parquet",
/// );
/// assert_eq!(result, PathBuf::from("/tmp/out/temperature.nc.parquet"));
/// ```
pub fn resolve_output_path(input_path: &Path, output_dir: &str, template: &str) -> PathBuf {
    let stem = input_path
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();

    let name = input_path
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();

    let filename = template.replace("{stem}", &stem).replace("{name}", &name);

    Path::new(output_dir).join(filename)
}

/// Processes all NetCDF files matching a glob pattern and writes Parquet output files.
///
/// Files are processed sequentially. When `config.fail_fast` is `true`, the function
/// returns immediately on the first per-file error. When `false`, all errors are
/// collected in [`BatchResult::failed`] and processing continues.
///
/// S3 glob patterns are rejected with a [`Nc2ParquetError::Config`] error because
/// glob expansion is only supported for local filesystem paths.
///
/// # Errors
///
/// Returns an error if:
/// - The pattern starts with `s3://`
/// - The glob pattern is syntactically invalid
/// - No files match the pattern
/// - Creating the output directory fails
/// - A per-file error occurs and `fail_fast` is `true`
pub fn process_netcdf_batch(config: &BatchConfig) -> Result<BatchResult, Nc2ParquetError> {
    // Reject S3 glob patterns — glob expansion requires local filesystem access.
    if config.pattern.starts_with("s3://") {
        return Err(Nc2ParquetError::Config(
            "Glob patterns are not supported for S3 paths. \
             Use a local filesystem pattern instead."
                .to_string(),
        ));
    }

    let template = config
        .output_template
        .as_deref()
        .unwrap_or("{stem}.parquet");

    let paths = glob::glob(&config.pattern).map_err(|e| {
        Nc2ParquetError::Config(format!("Invalid glob pattern '{}': {}", config.pattern, e))
    })?;

    let mut matched_paths: Vec<PathBuf> = Vec::new();
    for entry in paths {
        match entry {
            Ok(path) => matched_paths.push(path),
            Err(e) => {
                log::warn!("Skipping unreadable glob entry: {}", e);
            }
        }
    }

    if matched_paths.is_empty() {
        return Err(Nc2ParquetError::Config(format!(
            "No files matched pattern '{}'",
            config.pattern
        )));
    }

    std::fs::create_dir_all(&config.output_dir)?;

    let total_files = matched_paths.len();
    let mut succeeded: Vec<String> = Vec::with_capacity(total_files);
    let mut failed: Vec<(String, Nc2ParquetError)> = Vec::new();

    for input_path in &matched_paths {
        let input_str = input_path.to_string_lossy().into_owned();
        let output_path = resolve_output_path(input_path, &config.output_dir, template);
        let output_str = output_path.to_string_lossy().into_owned();

        let job = JobConfig {
            nc_key: input_str.clone(),
            variable_name: config.variable_name.clone(),
            variable_names: None,
            merge_variable_names: None,
            parquet_key: output_str,
            filters: config.filters.clone(),
            postprocessing: config.postprocessing.clone(),
            output: config.output.clone(),
        };

        match process_netcdf_job(&job) {
            Ok(()) => {
                succeeded.push(input_str);
            }
            Err(e) => {
                if config.fail_fast {
                    return Err(e);
                }
                failed.push((input_str, e));
            }
        }
    }

    Ok(BatchResult {
        succeeded,
        failed,
        total_files,
    })
}
