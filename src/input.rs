use crate::errors::Nc2ParquetError;
use crate::filters::{NC2DPointFilter, NC3DPointFilter, NCFilter, NCListFilter, NCRangeFilter};
use crate::postprocess::ProcessingPipelineConfig;
use polars::prelude::{GzipLevel, ParquetCompression, ZstdLevel};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

/// Compression codec to use when writing Parquet output files.
///
/// # Examples
///
/// ```rust
/// use nc2parquet::input::CompressionCodec;
/// use serde_json;
///
/// let codec: CompressionCodec = serde_json::from_str(r#""zstd""#).unwrap();
/// assert!(matches!(codec, CompressionCodec::Zstd));
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CompressionCodec {
    /// No compression (largest file, fastest write).
    Uncompressed,
    /// Snappy compression — fast, good compression ratio.
    Snappy,
    /// Gzip compression — good ratio, slower than Snappy.
    Gzip,
    /// LZ4 raw compression — very fast, lower ratio than Snappy.
    Lz4,
    /// Zstandard compression — best ratio, tunable speed/compression trade-off.
    Zstd,
}

fn default_compression() -> CompressionCodec {
    CompressionCodec::Snappy
}

fn default_statistics() -> bool {
    true
}

/// Output configuration for the Parquet writer.
///
/// Controls compression, row group size, data page size, and statistics.
/// When absent from a [`JobConfig`], the Polars default settings are used
/// (Zstd compression, min/max/null-count statistics, no row-group size limit).
///
/// # Examples
///
/// ```rust
/// use nc2parquet::input::{CompressionCodec, OutputConfig};
///
/// let config = OutputConfig::default();
/// assert!(matches!(config.compression, CompressionCodec::Snappy));
/// assert!(config.statistics);
/// assert!(config.compression_level.is_none());
/// assert!(config.row_group_size.is_none());
/// assert!(config.data_page_size.is_none());
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OutputConfig {
    /// Compression codec to apply to all column pages.
    #[serde(default = "default_compression")]
    pub compression: CompressionCodec,
    /// Compression level for codecs that support it.
    ///
    /// - `Zstd`: 1–22 (default: 3)
    /// - `Gzip`: 0–9 (default: 6)
    /// - `Snappy`, `Lz4`, `Uncompressed`: must be `None`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compression_level: Option<u32>,
    /// Maximum number of rows per row group.
    ///
    /// When `None`, Polars writes all rows in a single row group. A smaller
    /// value reduces peak memory usage during reads at the cost of slightly
    /// larger file footprint.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub row_group_size: Option<usize>,
    /// Maximum byte size of a data page.
    ///
    /// When `None`, defaults to 1 MiB (1 048 576 bytes).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data_page_size: Option<usize>,
    /// Whether to write column statistics (min, max, null count) into the file.
    ///
    /// Defaults to `true`. Statistics enable predicate pushdown in query engines
    /// but add a small amount of write overhead.
    #[serde(default = "default_statistics")]
    pub statistics: bool,
}

impl Default for OutputConfig {
    fn default() -> Self {
        OutputConfig {
            compression: default_compression(),
            compression_level: None,
            row_group_size: None,
            data_page_size: None,
            statistics: default_statistics(),
        }
    }
}

impl OutputConfig {
    /// Maps this config's `compression` + `compression_level` to a Polars
    /// [`ParquetCompression`] value ready to pass to [`ParquetWriter::with_compression`].
    ///
    /// # Examples
    ///
    /// ```rust
    /// use nc2parquet::input::{CompressionCodec, OutputConfig};
    /// use polars::prelude::ParquetCompression;
    ///
    /// let config = OutputConfig { compression: CompressionCodec::Snappy, ..Default::default() };
    /// assert!(matches!(config.to_polars_compression(), ParquetCompression::Snappy));
    ///
    /// let zstd = OutputConfig { compression: CompressionCodec::Zstd, compression_level: Some(3), ..Default::default() };
    /// assert!(matches!(zstd.to_polars_compression(), ParquetCompression::Zstd(Some(_))));
    /// ```
    pub fn to_polars_compression(&self) -> ParquetCompression {
        match self.compression {
            CompressionCodec::Uncompressed => ParquetCompression::Uncompressed,
            CompressionCodec::Snappy => ParquetCompression::Snappy,
            CompressionCodec::Gzip => {
                let level = self
                    .compression_level
                    .and_then(|l| GzipLevel::try_new(l as u8).ok());
                ParquetCompression::Gzip(level)
            }
            CompressionCodec::Lz4 => ParquetCompression::Lz4Raw,
            CompressionCodec::Zstd => {
                let level = self
                    .compression_level
                    .and_then(|l| ZstdLevel::try_new(l as i32).ok());
                ParquetCompression::Zstd(level)
            }
        }
    }

    /// Validates compression level values and row group sizes.
    ///
    /// Returns `Err` when:
    /// - Zstd level is outside 1–22
    /// - Gzip level is outside 0–9
    /// - A compression level is provided for Snappy, Lz4, or Uncompressed
    /// - `row_group_size` is `Some(0)`
    ///
    /// # Examples
    ///
    /// ```rust
    /// use nc2parquet::input::{CompressionCodec, OutputConfig};
    ///
    /// // Valid: Zstd level 3
    /// let ok = OutputConfig { compression: CompressionCodec::Zstd, compression_level: Some(3), ..Default::default() };
    /// assert!(ok.validate().is_ok());
    ///
    /// // Invalid: Zstd level 25
    /// let bad = OutputConfig { compression: CompressionCodec::Zstd, compression_level: Some(25), ..Default::default() };
    /// assert!(bad.validate().is_err());
    ///
    /// // Invalid: Snappy with a level
    /// let bad2 = OutputConfig { compression: CompressionCodec::Snappy, compression_level: Some(1), ..Default::default() };
    /// assert!(bad2.validate().is_err());
    ///
    /// // Invalid: row_group_size of 0
    /// let bad3 = OutputConfig { row_group_size: Some(0), ..Default::default() };
    /// assert!(bad3.validate().is_err());
    /// ```
    pub fn validate(&self) -> Result<(), Nc2ParquetError> {
        if let Some(level) = self.compression_level {
            match self.compression {
                CompressionCodec::Zstd => {
                    let l = level as i32;
                    if !(1..=22).contains(&l) {
                        return Err(Nc2ParquetError::Config(format!(
                            "Zstd compression level must be between 1 and 22, got {}",
                            level
                        )));
                    }
                }
                CompressionCodec::Gzip => {
                    if level > 9 {
                        return Err(Nc2ParquetError::Config(format!(
                            "Gzip compression level must be between 0 and 9, got {}",
                            level
                        )));
                    }
                }
                CompressionCodec::Snappy
                | CompressionCodec::Lz4
                | CompressionCodec::Uncompressed => {
                    return Err(Nc2ParquetError::Config(format!(
                        "{:?} compression does not accept a compression level",
                        self.compression
                    )));
                }
            }
        }

        if let Some(rg) = self.row_group_size
            && rg == 0
        {
            return Err(Nc2ParquetError::Config(
                "row_group_size must be greater than 0".to_string(),
            ));
        }

        Ok(())
    }
}

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
/// assert!(config.variable_names.is_none());
/// assert!(config.output.is_none());
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
    /// When `variable_names` is also set, `variable_names` takes precedence.
    pub variable_name: String,
    /// Optional list of variable names to extract in a single pass.
    ///
    /// When set, all listed variables are extracted into one Parquet file with
    /// shared dimension columns and one value column per variable. All variables
    /// must have identical dimensions (same names and sizes).
    ///
    /// When `None`, the single `variable_name` field is used instead.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub variable_names: Option<Vec<String>>,

    /// Optional list of variable names with different dimensions to merge.
    ///
    /// When set, all listed variables are extracted into one Parquet file,
    /// broadcasting lower-dimensional variables to match the highest-dimensional
    /// variable's shape. Unlike `variable_names`, dimensions need not match.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub merge_variable_names: Option<Vec<String>>,
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
    /// Optional Parquet output configuration (compression, row group size, etc.).
    ///
    /// When `None`, the Polars defaults are used: Zstd compression with no
    /// compression-level override, no row-group-size limit, and min/max/null-count
    /// statistics enabled.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub output: Option<OutputConfig>,
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
#[derive(Debug, Deserialize, Serialize, Clone)]
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
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct RangeParams {
    /// Name of the NetCDF dimension variable to filter (e.g. `"latitude"`).
    pub dimension_name: String,
    /// Minimum value of the range (inclusive).
    pub min_value: f64,
    /// Maximum value of the range (inclusive).
    pub max_value: f64,
}

/// Parameters for list-based filtering.
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ListParams {
    /// Name of the NetCDF dimension variable to filter (e.g. `"pressure"`).
    pub dimension_name: String,
    /// The discrete values to include (e.g. `[1000.0, 850.0, 500.0]`).
    pub values: Vec<f64>,
}

/// Parameters for 2D spatial point filtering.
#[derive(Debug, Deserialize, Serialize, Clone)]
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
#[derive(Debug, Deserialize, Serialize, Clone)]
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
    /// Returns the effective list of variable names to extract.
    ///
    /// When `variable_names` is `Some`, that list is returned. Otherwise the
    /// single `variable_name` is wrapped in a one-element `Vec`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use nc2parquet::input::JobConfig;
    ///
    /// // Without variable_names: falls back to variable_name
    /// let config = JobConfig::from_json(r#"{
    ///     "nc_key": "input.nc",
    ///     "variable_name": "temperature",
    ///     "parquet_key": "output.parquet",
    ///     "filters": []
    /// }"#).unwrap();
    /// assert_eq!(config.effective_variable_names(), vec!["temperature"]);
    ///
    /// // With variable_names set: returns the list
    /// let config2 = JobConfig::from_json(r#"{
    ///     "nc_key": "input.nc",
    ///     "variable_name": "temperature",
    ///     "variable_names": ["temperature", "pressure"],
    ///     "parquet_key": "output.parquet",
    ///     "filters": []
    /// }"#).unwrap();
    /// assert_eq!(config2.effective_variable_names(), vec!["temperature", "pressure"]);
    /// ```
    pub fn effective_variable_names(&self) -> Vec<String> {
        if let Some(ref names) = self.variable_names {
            names.clone()
        } else {
            vec![self.variable_name.clone()]
        }
    }

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

/// Configuration for batch processing of multiple NetCDF files via a glob pattern.
///
/// # Examples
///
/// ```rust
/// use nc2parquet::input::BatchConfig;
///
/// let config = BatchConfig {
///     pattern: "data/**/*.nc".to_string(),
///     output_dir: "/tmp/output".to_string(),
///     variable_name: "temperature".to_string(),
///     filters: vec![],
///     postprocessing: None,
///     output_template: None,
///     output: None,
///     fail_fast: false,
/// };
///
/// assert_eq!(config.pattern, "data/**/*.nc");
/// assert_eq!(config.variable_name, "temperature");
/// assert!(!config.fail_fast);
/// ```
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BatchConfig {
    /// Glob pattern to match input NetCDF files (e.g. `"data/**/*.nc"`).
    pub pattern: String,
    /// Output directory where Parquet files will be written.
    pub output_dir: String,
    /// Name of the variable to extract from each NetCDF file.
    pub variable_name: String,
    /// Filters to apply during data extraction (same set for all files).
    pub filters: Vec<FilterConfig>,
    /// Optional post-processing pipeline applied to every converted file.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub postprocessing: Option<ProcessingPipelineConfig>,
    /// Output filename template. Supports `{stem}` (filename without extension)
    /// and `{name}` (full filename). Defaults to `"{stem}.parquet"`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_template: Option<String>,
    /// Optional Parquet output configuration applied to every file in the batch.
    ///
    /// When `None`, Polars defaults are used (same as [`JobConfig::output`]).
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub output: Option<OutputConfig>,
    /// When `true`, stop processing on the first file error and return that error.
    /// When `false`, collect all per-file errors in [`BatchResult::failed`].
    pub fail_fast: bool,
}

/// Result of a batch conversion operation.
///
/// Tracks which files succeeded and which failed, allowing callers to report
/// partial success when `fail_fast` is `false`.
#[derive(Debug)]
pub struct BatchResult {
    /// Paths of files that were converted successfully.
    pub succeeded: Vec<String>,
    /// Files that failed, together with the error that caused the failure.
    pub failed: Vec<(String, Nc2ParquetError)>,
    /// Total number of files that matched the glob pattern.
    pub total_files: usize,
}
