use crate::errors::Nc2ParquetError;
use crate::filters::{NC2DPointFilter, NC3DPointFilter, NCFilter, NCListFilter, NCRangeFilter};
use crate::postprocess::ProcessingPipelineConfig;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

/// Complete configuration for a NetCDF-to-Parquet conversion job.
///
/// # Examples
///
/// ```rust
/// use nc2parquet::input::JobConfig;
///
/// let config = JobConfig::from_json(r#"{
///     "nc_key": "data/temperature.nc",
///     "variable_name": "t2m",
///     "parquet_key": "output/temperature.parquet",
///     "filters": []
/// }"#).unwrap();
///
/// assert_eq!(config.nc_key, "data/temperature.nc");
/// assert_eq!(config.variable_name, "t2m");
/// assert_eq!(config.parquet_key, "output/temperature.parquet");
/// assert!(config.filters.is_empty());
/// assert!(config.postprocessing.is_none());
/// ```
#[derive(Deserialize, Serialize, Clone)]
pub struct JobConfig {
    /// Path or URI to the input NetCDF file.
    ///
    /// Accepts local filesystem paths (e.g. `"data/input.nc"`) or S3 URIs
    /// in the form `"s3://bucket-name/path/to/file.nc"`.
    pub nc_key: String,
    /// Name of the variable to extract from the NetCDF file.
    ///
    /// This must match exactly the variable name stored inside the NetCDF file.
    pub variable_name: String,
    /// Filters to apply during data extraction.
    ///
    /// Filters are intersected — only data points that satisfy all filters are
    /// included in the output. An empty list means all data is extracted.
    pub filters: Vec<FilterConfig>,
    /// Path or URI where the output Parquet file should be written.
    ///
    /// Accepts local filesystem paths or S3 URIs (same format as `nc_key`).
    pub parquet_key: String,
    /// Optional post-processing pipeline configuration.
    ///
    /// When present, the extracted DataFrame is transformed by the specified
    /// pipeline before being written to Parquet. If `None`, the raw extracted
    /// data is written without modification.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub postprocessing: Option<ProcessingPipelineConfig>,
}

/// All supported filter configurations.
///
/// # Examples
///
/// ```rust
/// use nc2parquet::input::{FilterConfig, RangeParams};
///
/// let filter = FilterConfig::Range {
///     params: RangeParams {
///         dimension_name: "latitude".to_string(),
///         min_value: -10.0,
///         max_value: 10.0,
///     },
/// };
///
/// assert_eq!(filter.kind(), "range");
/// ```
#[derive(Deserialize, Serialize, Clone)]
#[serde(tag = "kind")]
pub enum FilterConfig {
    /// Range filter — selects dimension values within `[min_value, max_value]`.
    #[serde(rename = "range")]
    Range {
        /// Range filter parameters (dimension name and bounds).
        params: RangeParams,
    },
    /// List filter — selects only the discrete dimension values listed in `params`.
    #[serde(rename = "list")]
    List {
        /// List filter parameters (dimension name and allowed values).
        params: ListParams,
    },
    /// 2D spatial point filter — selects grid cells within `tolerance` of the
    /// specified (latitude, longitude) points.
    #[serde(rename = "2d_point")]
    Point2D {
        /// 2D point filter parameters (dimension names, points, tolerance).
        params: Point2DParams,
    },
    /// 3D spatiotemporal point filter — selects grid cells that match both the
    /// specified time steps and the spatial (lat, lon) points within tolerance.
    #[serde(rename = "3d_point")]
    Point3D {
        /// 3D point filter parameters (dimension names, time steps, points, tolerance).
        params: Point3DParams,
    },
}

/// Parameters for range-based filtering.
#[derive(Deserialize, Serialize, Clone)]
pub struct RangeParams {
    /// Name of the NetCDF dimension variable to filter (e.g. `"latitude"`).
    pub dimension_name: String,
    /// Minimum value of the range (inclusive).
    pub min_value: f64,
    /// Maximum value of the range (inclusive).
    pub max_value: f64,
}

/// Parameters for list-based filtering.
#[derive(Deserialize, Serialize, Clone)]
pub struct ListParams {
    /// Name of the NetCDF dimension variable to filter (e.g. `"pressure"`).
    pub dimension_name: String,
    /// The discrete values to include (e.g. `[1000.0, 850.0, 500.0]`).
    pub values: Vec<f64>,
}

/// Parameters for 2D spatial point filtering.
#[derive(Deserialize, Serialize, Clone)]
pub struct Point2DParams {
    /// Name of the latitude dimension variable in the NetCDF file.
    pub lat_dimension_name: String,
    /// Name of the longitude dimension variable in the NetCDF file.
    pub lon_dimension_name: String,
    /// Target (latitude, longitude) coordinate pairs to match against.
    pub points: Vec<(f64, f64)>,
    /// Maximum allowed distance (in the same units as lat/lon) for a grid cell
    /// to be considered a match.
    pub tolerance: f64,
}

/// Parameters for 3D spatiotemporal point filtering.
#[derive(Deserialize, Serialize, Clone)]
pub struct Point3DParams {
    /// Name of the time dimension variable in the NetCDF file.
    pub time_dimension_name: String,
    /// Name of the latitude dimension variable in the NetCDF file.
    pub lat_dimension_name: String,
    /// Name of the longitude dimension variable in the NetCDF file.
    pub lon_dimension_name: String,
    /// Exact time step values to include (matched by equality against the time
    /// dimension coordinate values).
    pub steps: Vec<f64>,
    /// Target (latitude, longitude) coordinate pairs to match against.
    pub points: Vec<(f64, f64)>,
    /// Maximum allowed distance (in the same units as lat/lon) for a grid cell
    /// to be considered a spatial match.
    pub tolerance: f64,
}

impl JobConfig {
    /// Loads a job configuration from a JSON file.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// use nc2parquet::input::JobConfig;
    ///
    /// // Load configuration from a file on disk
    /// let config = JobConfig::from_file("config.json")?;
    /// println!("Processing variable: {}", config.variable_name);
    /// # Ok(())
    /// # }
    /// ```
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, Nc2ParquetError> {
        let content = fs::read_to_string(path)?;
        let config: JobConfig = serde_json::from_str(&content)
            .map_err(|e| Nc2ParquetError::Serialization(e.to_string()))?;
        Ok(config)
    }

    /// Loads a job configuration from a JSON string.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use nc2parquet::input::JobConfig;
    ///
    /// let json = r#"{
    ///     "nc_key": "input.nc",
    ///     "variable_name": "temperature",
    ///     "parquet_key": "output.parquet",
    ///     "filters": []
    /// }"#;
    ///
    /// let config = JobConfig::from_json(json).unwrap();
    /// assert_eq!(config.variable_name, "temperature");
    /// assert_eq!(config.nc_key, "input.nc");
    /// assert_eq!(config.parquet_key, "output.parquet");
    /// assert!(config.filters.is_empty());
    /// ```
    pub fn from_json(json_str: &str) -> Result<Self, Nc2ParquetError> {
        let config: JobConfig = serde_json::from_str(json_str)
            .map_err(|e| Nc2ParquetError::Serialization(e.to_string()))?;
        Ok(config)
    }
}

impl FilterConfig {
    /// Converts this filter configuration into a concrete filter implementation.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use nc2parquet::input::{FilterConfig, RangeParams};
    ///
    /// let config = FilterConfig::Range {
    ///     params: RangeParams {
    ///         dimension_name: "latitude".to_string(),
    ///         min_value: -10.0,
    ///         max_value: 10.0,
    ///     },
    /// };
    ///
    /// // Convert to a concrete filter ready for application
    /// let filter = config.to_filter().unwrap();
    /// ```
    pub fn to_filter(&self) -> Result<Box<dyn NCFilter>, Nc2ParquetError> {
        match self {
            FilterConfig::Range { params } => {
                let filter =
                    NCRangeFilter::new(&params.dimension_name, params.min_value, params.max_value);
                Ok(Box::new(filter))
            }
            FilterConfig::List { params } => {
                let filter = NCListFilter::new(&params.dimension_name, params.values.clone());
                Ok(Box::new(filter))
            }
            FilterConfig::Point2D { params } => {
                let filter = NC2DPointFilter::new(
                    &params.lat_dimension_name,
                    &params.lon_dimension_name,
                    params.points.clone(),
                    params.tolerance,
                );
                Ok(Box::new(filter))
            }
            FilterConfig::Point3D { params } => {
                let filter = NC3DPointFilter::new(
                    &params.time_dimension_name,
                    &params.lat_dimension_name,
                    &params.lon_dimension_name,
                    params.steps.clone(),
                    params.points.clone(),
                    params.tolerance,
                );
                Ok(Box::new(filter))
            }
        }
    }

    /// Returns the string identifier for this filter type.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use nc2parquet::input::{FilterConfig, RangeParams, ListParams};
    ///
    /// let range_filter = FilterConfig::Range {
    ///     params: RangeParams {
    ///         dimension_name: "latitude".to_string(),
    ///         min_value: 0.0,
    ///         max_value: 90.0,
    ///     },
    /// };
    /// assert_eq!(range_filter.kind(), "range");
    ///
    /// let list_filter = FilterConfig::List {
    ///     params: ListParams {
    ///         dimension_name: "pressure".to_string(),
    ///         values: vec![1000.0, 850.0],
    ///     },
    /// };
    /// assert_eq!(list_filter.kind(), "list");
    /// ```
    pub fn kind(&self) -> &'static str {
        match self {
            FilterConfig::Range { .. } => "range",
            FilterConfig::List { .. } => "list",
            FilterConfig::Point2D { .. } => "2d_point",
            FilterConfig::Point3D { .. } => "3d_point",
        }
    }
}
