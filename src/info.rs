use crate::storage::{StorageBackend, StorageFactory};
use anyhow::{Context, Result};
use log::debug;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Metadata about a single NetCDF dimension.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetCdfDimensionInfo {
    /// Name of the dimension as stored in the NetCDF file.
    pub name: String,
    /// Number of coordinate values along this dimension.
    pub length: usize,
    /// Whether this is an unlimited (record) dimension.
    pub is_unlimited: bool,
}

/// Metadata about a single NetCDF variable.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetCdfVariableInfo {
    /// Name of the variable as stored in the NetCDF file.
    pub name: String,
    /// Human-readable string representation of the variable's data type.
    pub data_type: String,
    /// Names of the dimensions that index this variable, in order.
    pub dimensions: Vec<String>,
    /// Map of attribute name to its formatted value string.
    pub attributes: HashMap<String, String>,
    /// Size of each dimension, in the same order as `dimensions`.
    pub shape: Vec<usize>,
}

/// Complete structural metadata about a NetCDF file.
///
/// Produced by [`get_netcdf_info`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetCdfInfo {
    /// Filesystem path or S3 URI of the NetCDF file.
    pub path: String,
    /// All dimensions present in the file.
    pub dimensions: Vec<NetCdfDimensionInfo>,
    /// All variables present in the file (or just the requested variable when
    /// [`get_netcdf_info`] is called with `variable = Some(...)`).
    pub variables: Vec<NetCdfVariableInfo>,
    /// Global (file-level) attributes.  Populated only when `detailed = true`
    /// is passed to [`get_netcdf_info`].
    pub global_attributes: HashMap<String, String>,
    /// File size in bytes for local files; `None` for S3 paths.
    pub file_size: Option<u64>,
    /// Total number of variables in the file.
    pub total_variables: usize,
    /// Total number of dimensions in the file.
    pub total_dimensions: usize,
}

/// Extracts comprehensive structural metadata from a NetCDF file.
///
/// Supports both local filesystem paths and S3 URIs.  When an S3 path is
/// provided, the file is downloaded to a temporary location, inspected, and
/// then cleaned up.
///
/// # Arguments
///
/// * `file_path` - Local path or `s3://bucket/key` URI to the NetCDF file
/// * `variable` - When `Some(name)`, only that variable's metadata is returned;
///   `None` returns metadata for all variables
/// * `detailed` - When `true`, global (file-level) attributes are populated in
///   the returned [`NetCdfInfo`]
///
/// # Errors
///
/// Returns an error if:
/// - The file cannot be opened (locally or from S3)
/// - The NetCDF file format is invalid
///
/// # Examples
///
/// ```rust,no_run
/// # #[tokio::main]
/// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
/// use nc2parquet::info::get_netcdf_info;
///
/// // Inspect all variables in a local file
/// let info = get_netcdf_info("data/temperature.nc", None, false).await?;
/// println!("Variables: {}", info.total_variables);
/// println!("Dimensions: {}", info.total_dimensions);
///
/// // Inspect only the "t2m" variable with global attributes
/// let info = get_netcdf_info("data/temperature.nc", Some("t2m"), true).await?;
/// println!("Shape: {:?}", info.variables[0].shape);
/// # Ok(())
/// # }
/// ```
pub async fn get_netcdf_info(
    file_path: &str,
    variable: Option<&str>,
    detailed: bool,
) -> Result<NetCdfInfo> {
    let (temp_file, local_path) = if file_path.starts_with("s3://") {
        let storage = StorageFactory::from_path(file_path).await?;
        let data = storage
            .read(file_path)
            .await
            .context("Failed to read S3 file for analysis")?;

        let temp_file =
            tempfile::NamedTempFile::new().context("Failed to create temporary file")?;
        let temp_path = temp_file.path().to_path_buf();

        debug!("Writing S3 data to temporary path: {:?}", temp_path);
        tokio::fs::write(&temp_path, data)
            .await
            .context("Failed to write temporary file")?;

        (Some(temp_file), temp_path.to_string_lossy().to_string())
    } else {
        (None, file_path.to_string())
    };

    debug!("Opening NetCDF file: {}", local_path);
    let file = netcdf::open(&local_path)
        .with_context(|| format!("Failed to open NetCDF file: {}", file_path))?;

    let file_size = if file_path.starts_with("s3://") {
        None
    } else {
        tokio::fs::metadata(&local_path)
            .await
            .ok()
            .map(|metadata| metadata.len())
    };

    let mut dimensions = Vec::new();
    for dim in file.dimensions() {
        dimensions.push(NetCdfDimensionInfo {
            name: dim.name().to_string(),
            length: dim.len(),
            is_unlimited: dim.is_unlimited(),
        });
    }

    let mut variables = Vec::new();
    for var in file.variables() {
        if let Some(var_name) = variable
            && var.name() != var_name
        {
            continue;
        }

        let mut attributes = HashMap::new();
        for attr in var.attributes() {
            if let Ok(value) = attr.value() {
                let value_str = format_attribute_value(&value);
                attributes.insert(attr.name().to_string(), value_str);
            }
        }

        let shape: Vec<usize> = var.dimensions().iter().map(|d| d.len()).collect();

        variables.push(NetCdfVariableInfo {
            name: var.name().to_string(),
            data_type: format_variable_type(&var.vartype()),
            dimensions: var
                .dimensions()
                .iter()
                .map(|d| d.name().to_string())
                .collect(),
            attributes,
            shape,
        });
    }

    let mut global_attributes = HashMap::new();
    if detailed {
        for attr in file.attributes() {
            if let Ok(value) = attr.value() {
                let value_str = format_attribute_value(&value);
                global_attributes.insert(attr.name().to_string(), value_str);
            }
        }
    }

    file.close().context("Failed to close NetCDF file")?;
    drop(temp_file); // Must outlive the netcdf file handle

    Ok(NetCdfInfo {
        path: file_path.to_string(),
        total_dimensions: dimensions.len(),
        total_variables: variables.len(),
        dimensions,
        variables,
        global_attributes,
        file_size,
    })
}

fn format_attribute_value(value: &netcdf::AttributeValue) -> String {
    format!("{:?}", value)
}

fn format_variable_type(var_type: &netcdf::types::NcVariableType) -> String {
    format!("{:?}", var_type)
}

/// Prints [`NetCdfInfo`] to stdout in a human-readable format.
///
/// # Examples
///
/// ```rust,no_run
/// # #[tokio::main]
/// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
/// use nc2parquet::info::{get_netcdf_info, print_file_info_human};
///
/// let info = get_netcdf_info("data/temperature.nc", None, false).await?;
/// print_file_info_human(&info);
/// # Ok(())
/// # }
/// ```
pub fn print_file_info_human(info: &NetCdfInfo) {
    println!("NetCDF File Information:");
    println!("  Path: {}", info.path);
    if let Some(size) = info.file_size {
        println!("  File Size: {:.2} MB", size as f64 / 1_048_576.0);
    }
    println!("  Dimensions: {} total", info.total_dimensions);
    for dim in &info.dimensions {
        println!(
            "    {} ({}{})",
            dim.name,
            dim.length,
            if dim.is_unlimited { ", unlimited" } else { "" }
        );
    }
    println!("  Variables: {} total", info.total_variables);
    for var in &info.variables {
        println!(
            "    {} ({}) - dimensions: [{}]",
            var.name,
            var.data_type,
            var.dimensions.join(", ")
        );
        if !var.attributes.is_empty() {
            for (name, value) in &var.attributes {
                println!("      @{}: {}", name, value);
            }
        }
    }
    if !info.global_attributes.is_empty() {
        println!("  Global Attributes:");
        for (name, value) in &info.global_attributes {
            println!("    @{}: {}", name, value);
        }
    }
}

/// Prints [`NetCdfInfo`] to stdout as a pretty-printed JSON object.
///
/// # Errors
///
/// Returns an error if serialization to JSON fails.
///
/// # Examples
///
/// ```rust,no_run
/// # #[tokio::main]
/// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
/// use nc2parquet::info::{get_netcdf_info, print_file_info_json};
///
/// let info = get_netcdf_info("data/temperature.nc", None, false).await?;
/// print_file_info_json(&info)?;
/// # Ok(())
/// # }
/// ```
pub fn print_file_info_json(info: &NetCdfInfo) -> Result<()> {
    let json = serde_json::json!({
        "path": info.path,
        "dimensions": info.dimensions,
        "variables": info.variables,
        "global_attributes": info.global_attributes,
        "file_size": info.file_size,
        "total_variables": info.total_variables,
        "total_dimensions": info.total_dimensions
    });
    println!("{}", serde_json::to_string_pretty(&json)?);
    Ok(())
}

/// Prints [`NetCdfInfo`] to stdout in YAML format.
///
/// # Errors
///
/// Returns an error if serialization to YAML fails.
///
/// # Examples
///
/// ```rust,no_run
/// # #[tokio::main]
/// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
/// use nc2parquet::info::{get_netcdf_info, print_file_info_yaml};
///
/// let info = get_netcdf_info("data/temperature.nc", None, false).await?;
/// print_file_info_yaml(&info)?;
/// # Ok(())
/// # }
/// ```
pub fn print_file_info_yaml(info: &NetCdfInfo) -> Result<()> {
    let yaml = serde_yaml::to_string(info).context("Failed to serialize NetCDF info to YAML")?;
    println!("{}", yaml);
    Ok(())
}

/// Prints [`NetCdfInfo`] variable metadata to stdout as CSV.
///
/// Each row represents one variable with the columns:
/// `variable_name`, `data_type`, `dimensions`, `shape`, `attributes_count`.
///
/// # Errors
///
/// Returns an error if writing to stdout fails.
///
/// # Examples
///
/// ```rust,no_run
/// # #[tokio::main]
/// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
/// use nc2parquet::info::{get_netcdf_info, print_file_info_csv};
///
/// let info = get_netcdf_info("data/temperature.nc", None, false).await?;
/// print_file_info_csv(&info)?;
/// # Ok(())
/// # }
/// ```
pub fn print_file_info_csv(info: &NetCdfInfo) -> Result<()> {
    println!("variable_name,data_type,dimensions,shape,attributes_count");
    for var in &info.variables {
        let dimensions = format!("\"{}\"", var.dimensions.join(";"));
        let shape = format!(
            "\"{}\"",
            var.shape
                .iter()
                .map(|s| s.to_string())
                .collect::<Vec<_>>()
                .join(";")
        );
        println!(
            "{},{},{},{},{}",
            var.name,
            var.data_type,
            dimensions,
            shape,
            var.attributes.len()
        );
    }
    Ok(())
}
