//! Post-processing pipeline for transforming DataFrames after NetCDF extraction.
//!
//! # Example
//! ```rust
//! use nc2parquet::postprocess::{PostProcessor, ProcessingPipeline, ColumnRenamer};
//! use polars::prelude::*;
//! use std::collections::HashMap;
//!
//! // Create a pipeline
//! let mut pipeline = ProcessingPipeline::new();
//!
//! // Add processors
//! let mut mappings = HashMap::new();
//! mappings.insert("t2".to_string(), "temperature_2m".to_string());
//! pipeline.add_processor(Box::new(ColumnRenamer::new(mappings)));
//!
//! // Create sample DataFrame
//! let sample_df = df! {
//!     "t2" => [20.5, 21.0, 19.8],
//!     "humidity" => [65, 70, 60]
//! }.unwrap();
//!
//! // Execute pipeline
//! let processed_df = pipeline.execute(sample_df).unwrap();
//! ```

use chrono::{DateTime, Utc};
use log::{debug, warn};
use polars::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub type PostProcessResult<T> = Result<T, PostProcessError>;

/// Errors that can occur during post-processing.
///
/// This error type is returned by all [`PostProcessor`] implementations and by
/// [`ProcessingPipeline::execute`].  It is also automatically converted into
/// [`crate::errors::Nc2ParquetError::PostProcess`] via `#[from]`.
#[derive(thiserror::Error, Debug)]
pub enum PostProcessError {
    /// A column referenced by the processor was not found in the DataFrame.
    ///
    /// The wrapped `String` contains the missing column name.
    #[error("Column '{0}' not found in DataFrame")]
    ColumnNotFound(String),
    /// A value could not be converted to the target type.
    ///
    /// The wrapped `String` describes what conversion failed and why.
    #[error("Conversion error: {0}")]
    ConversionError(String),
    /// A processor configuration parameter is invalid.
    ///
    /// The wrapped `String` describes which parameter is invalid and why.
    #[error("Configuration error: {0}")]
    ConfigurationError(String),
    /// An underlying Polars operation failed.
    #[error("Polars error: {0}")]
    PolarsError(#[from] PolarsError),
    /// A generic processing error not covered by the other variants.
    ///
    /// The wrapped `String` contains a human-readable description of the failure.
    #[error("Processing error: {0}")]
    ProcessingError(String),
}

/// Core trait for post-processing operations on DataFrames.
///
/// Implement this trait to create a custom transformation step that can be added
/// to a [`ProcessingPipeline`].  All implementations must be `Send + Sync` so
/// that pipelines can be used in multi-threaded contexts.
///
/// # Examples
///
/// ```rust
/// use nc2parquet::postprocess::{PostProcessor, PostProcessResult};
/// use polars::prelude::*;
///
/// struct DoubleValues {
///     column: String,
/// }
///
/// impl PostProcessor for DoubleValues {
///     fn process(&self, df: DataFrame) -> PostProcessResult<DataFrame> {
///         Ok(df
///             .lazy()
///             .with_columns([(col(&self.column) * lit(2.0)).alias(&self.column)])
///             .collect()?)
///     }
///     fn name(&self) -> &str { "DoubleValues" }
///     fn description(&self) -> &str { "Multiplies a column's values by 2" }
/// }
/// ```
pub trait PostProcessor: Send + Sync {
    /// Apply the transformation and return the modified DataFrame.
    ///
    /// Implementations receive ownership of `df` and must return the
    /// transformed result (or an error).
    fn process(&self, df: DataFrame) -> PostProcessResult<DataFrame>;

    /// Return the short name/identifier of this processor.
    ///
    /// Used in log messages and pipeline descriptions.
    fn name(&self) -> &str;

    /// Return a human-readable description of what this processor does.
    fn description(&self) -> &str;

    /// Validate that this processor can operate on the given DataFrame schema.
    ///
    /// The default implementation performs no validation and always returns `Ok(())`.
    /// Override this method to enforce schema pre-conditions before `process` is called.
    fn validate_schema(&self, schema: &Schema) -> PostProcessResult<()> {
        let _ = schema;
        Ok(())
    }

    /// Return the expected output schema after this processor runs.
    ///
    /// The default implementation returns the input schema unchanged.
    /// Override this method when the processor adds, removes, or renames columns.
    fn output_schema(&self, input_schema: &Schema) -> PostProcessResult<Schema> {
        Ok(input_schema.clone())
    }
}

/// Configuration for the entire post-processing pipeline.
///
/// Serializes to and deserializes from JSON/YAML.  Use [`ProcessingPipeline::from_config`]
/// to create a runnable pipeline from this configuration.
///
/// # Examples
///
/// ```rust
/// use nc2parquet::postprocess::{ProcessingPipelineConfig, ProcessorConfig};
/// use std::collections::HashMap;
///
/// let config = ProcessingPipelineConfig {
///     name: Some("rename-pipeline".to_string()),
///     processors: vec![
///         ProcessorConfig::RenameColumns {
///             mappings: HashMap::from([("t2".to_string(), "temperature_2m".to_string())]),
///         },
///     ],
/// };
///
/// assert_eq!(config.name.as_deref(), Some("rename-pipeline"));
/// assert_eq!(config.processors.len(), 1);
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessingPipelineConfig {
    /// Optional human-readable name for the pipeline.
    ///
    /// Appears in log output.  When `None`, the pipeline is labelled
    /// `"Configured Pipeline"`.
    pub name: Option<String>,
    /// Ordered list of processors to execute.
    ///
    /// Processors are applied sequentially — the output of each step becomes
    /// the input of the next.
    pub processors: Vec<ProcessorConfig>,
}

/// Configuration for a single post-processing step.
///
/// Each variant corresponds to one of the built-in processor types. Use
/// [`ProcessingPipelineConfig`] to combine multiple steps into a pipeline and
/// then [`ProcessingPipeline::from_config`] to instantiate the processors.
///
/// # Examples
///
/// ```rust
/// use nc2parquet::postprocess::{ProcessorConfig, TimeUnit};
/// use std::collections::HashMap;
///
/// // Rename a column
/// let rename = ProcessorConfig::RenameColumns {
///     mappings: HashMap::from([("t2".to_string(), "temperature_2m".to_string())]),
/// };
///
/// // Convert a numeric offset column to datetime
/// let datetime = ProcessorConfig::DatetimeConvert {
///     column: "time".to_string(),
///     base: "1900-01-01T00:00:00Z".to_string(),
///     unit: TimeUnit::Hours,
/// };
///
/// // Convert temperature from Kelvin to Celsius
/// let convert = ProcessorConfig::UnitConvert {
///     column: "temperature".to_string(),
///     from_unit: "kelvin".to_string(),
///     to_unit: "celsius".to_string(),
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ProcessorConfig {
    /// Rename one or more columns using a name-to-name mapping.
    RenameColumns {
        /// Map of `old_column_name -> new_column_name` pairs.
        mappings: HashMap<String, String>,
    },
    /// Convert a numeric column of time offsets to a datetime column.
    DatetimeConvert {
        /// Name of the column containing numeric time offsets.
        column: String,
        /// ISO 8601 base datetime string (e.g. `"1900-01-01T00:00:00Z"`).
        base: String,
        /// Unit of the numeric offsets stored in `column`.
        unit: TimeUnit,
    },
    /// Convert column values from one physical unit to another.
    UnitConvert {
        /// Name of the column to convert.
        column: String,
        /// Source unit (e.g. `"kelvin"`, `"celsius"`, `"fahrenheit"`).
        from_unit: String,
        /// Target unit (e.g. `"celsius"`, `"kelvin"`, `"fahrenheit"`).
        to_unit: String,
    },
    /// Aggregate rows using group-by and statistical operations.
    Aggregate {
        /// Column names to group by.  Use an empty list for a global aggregation.
        group_by: Vec<String>,
        /// Map of `column_name -> aggregation_operation`.
        aggregations: HashMap<String, AggregationOp>,
    },
    /// Create or overwrite a column by applying a mathematical formula.
    ApplyFormula {
        /// Name of the output column to write the formula result into.
        target_column: String,
        /// Formula string (e.g. `"a + b * 2.0"`, `"sqrt(a)"`, `"a < 5.0"`).
        formula: String,
        /// Column names referenced by the formula.
        source_columns: Vec<String>,
    },
}

/// Time units used when converting numeric offsets to datetime values.
///
/// Used by [`ProcessorConfig::DatetimeConvert`] to specify how the raw numeric
/// values in the source column should be interpreted before adding them to the
/// base datetime.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TimeUnit {
    /// Each unit represents one second.
    Seconds,
    /// Each unit represents one minute (60 seconds).
    Minutes,
    /// Each unit represents one hour (3 600 seconds).
    Hours,
    /// Each unit represents one day (86 400 seconds).
    Days,
    /// Each unit represents one millisecond (0.001 seconds).
    Milliseconds,
    /// Each unit represents one microsecond (1 × 10⁻⁶ seconds).
    Microseconds,
    /// Each unit represents one nanosecond (1 × 10⁻⁹ seconds).
    Nanoseconds,
}

/// Statistical aggregation operations supported by [`ProcessorConfig::Aggregate`].
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AggregationOp {
    /// Arithmetic mean of the group.
    Mean,
    /// Sum of all values in the group.
    Sum,
    /// Minimum value in the group.
    Min,
    /// Maximum value in the group.
    Max,
    /// Number of rows in the group.
    Count,
    /// Sample standard deviation (ddof = 1).
    Std,
    /// Sample variance (ddof = 1).
    Var,
    /// First value in the group (in original row order).
    First,
    /// Last value in the group (in original row order).
    Last,
}

impl TimeUnit {
    /// Convert the time unit to a multiplier for seconds
    pub(crate) fn to_seconds_multiplier(&self) -> f64 {
        match self {
            TimeUnit::Nanoseconds => 1e-9,
            TimeUnit::Microseconds => 1e-6,
            TimeUnit::Milliseconds => 1e-3,
            TimeUnit::Seconds => 1.0,
            TimeUnit::Minutes => 60.0,
            TimeUnit::Hours => 3600.0,
            TimeUnit::Days => 86400.0,
        }
    }
}

/// Pipeline that chains multiple post-processors together.
///
/// Processors are executed in the order they were added.  The output DataFrame
/// of each step becomes the input of the next step.
///
/// # Examples
///
/// ```rust
/// use nc2parquet::postprocess::{ProcessingPipeline, ColumnRenamer, PostProcessor};
/// use polars::prelude::*;
/// use std::collections::HashMap;
///
/// let mut pipeline = ProcessingPipeline::new();
///
/// let mut mappings = HashMap::new();
/// mappings.insert("t2".to_string(), "temperature_2m".to_string());
/// pipeline.add_processor(Box::new(ColumnRenamer::new(mappings)));
///
/// let df = df! {
///     "t2" => [20.5f64, 21.0, 19.8],
/// }.unwrap();
///
/// let result = pipeline.execute(df).unwrap();
/// assert!(result.column("temperature_2m").is_ok());
/// ```
pub struct ProcessingPipeline {
    processors: Vec<Box<dyn PostProcessor>>,
    name: String,
}

impl ProcessingPipeline {
    /// Creates a new empty processing pipeline with a default name.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use nc2parquet::postprocess::ProcessingPipeline;
    ///
    /// let pipeline = ProcessingPipeline::new();
    /// assert_eq!(pipeline.name(), "Unnamed Pipeline");
    /// ```
    pub fn new() -> Self {
        Self {
            name: "Unnamed Pipeline".to_string(),
            processors: Vec::new(),
        }
    }

    /// Creates a new empty processing pipeline with the given name.
    ///
    /// # Arguments
    ///
    /// * `name` - Human-readable label used in log messages
    ///
    /// # Examples
    ///
    /// ```rust
    /// use nc2parquet::postprocess::ProcessingPipeline;
    ///
    /// let pipeline = ProcessingPipeline::with_name("temperature-pipeline".to_string());
    /// assert_eq!(pipeline.name(), "temperature-pipeline");
    /// ```
    pub fn with_name(name: String) -> Self {
        Self {
            name,
            processors: Vec::new(),
        }
    }

    /// Returns the name of this pipeline.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use nc2parquet::postprocess::ProcessingPipeline;
    ///
    /// let pipeline = ProcessingPipeline::with_name("my-pipeline".to_string());
    /// assert_eq!(pipeline.name(), "my-pipeline");
    /// ```
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Creates a processing pipeline from a [`ProcessingPipelineConfig`].
    ///
    /// Each [`ProcessorConfig`] entry in `config.processors` is instantiated
    /// into a concrete [`PostProcessor`] implementation and appended to the
    /// pipeline in order.
    ///
    /// # Errors
    ///
    /// Returns [`PostProcessError::ConfigurationError`] if any processor config
    /// contains invalid parameters (e.g. a malformed base datetime string).
    ///
    /// # Examples
    ///
    /// ```rust
    /// use nc2parquet::postprocess::{ProcessingPipeline, ProcessingPipelineConfig, ProcessorConfig};
    /// use std::collections::HashMap;
    ///
    /// let config = ProcessingPipelineConfig {
    ///     name: Some("rename-pipeline".to_string()),
    ///     processors: vec![
    ///         ProcessorConfig::RenameColumns {
    ///             mappings: HashMap::from([("t2".to_string(), "temperature_2m".to_string())]),
    ///         },
    ///     ],
    /// };
    ///
    /// let pipeline = ProcessingPipeline::from_config(&config).unwrap();
    /// assert_eq!(pipeline.name(), "rename-pipeline");
    /// ```
    pub fn from_config(config: &ProcessingPipelineConfig) -> PostProcessResult<Self> {
        let mut pipeline = Self {
            name: config
                .name
                .clone()
                .unwrap_or_else(|| "Configured Pipeline".to_string()),
            processors: Vec::new(),
        };

        for processor_config in &config.processors {
            let processor = create_processor(processor_config)?;
            pipeline.add_processor(processor);
        }

        Ok(pipeline)
    }

    /// Appends a processor to the end of this pipeline.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use nc2parquet::postprocess::{ProcessingPipeline, ColumnRenamer};
    /// use std::collections::HashMap;
    ///
    /// let mut pipeline = ProcessingPipeline::new();
    /// pipeline.add_processor(Box::new(ColumnRenamer::new(HashMap::new())));
    /// ```
    pub fn add_processor(&mut self, processor: Box<dyn PostProcessor>) {
        self.processors.push(processor);
    }

    /// Executes all processors in order and returns the final DataFrame.
    ///
    /// If the pipeline is empty, the original DataFrame is returned unchanged.
    ///
    /// # Errors
    ///
    /// Returns the first [`PostProcessError`] encountered while running a processor.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use nc2parquet::postprocess::{ProcessingPipeline, ColumnRenamer};
    /// use polars::prelude::*;
    /// use std::collections::HashMap;
    ///
    /// let mut mappings = HashMap::new();
    /// mappings.insert("old_col".to_string(), "new_col".to_string());
    ///
    /// let mut pipeline = ProcessingPipeline::new();
    /// pipeline.add_processor(Box::new(ColumnRenamer::new(mappings)));
    ///
    /// let df = df! { "old_col" => [1i32, 2, 3] }.unwrap();
    /// let result = pipeline.execute(df).unwrap();
    /// assert!(result.column("new_col").is_ok());
    /// ```
    pub fn execute(&mut self, mut df: DataFrame) -> PostProcessResult<DataFrame> {
        debug!(
            "Executing pipeline '{}' with {} processors",
            self.name,
            self.processors.len()
        );

        if self.processors.is_empty() {
            return Ok(df);
        }

        debug!("Initial DataFrame shape: {:?}", df.shape());

        for (i, processor) in self.processors.iter().enumerate() {
            let processor_name = processor.name();
            debug!(
                "Executing processor {} '{}' - input shape: {:?}",
                i + 1,
                processor_name,
                df.shape()
            );

            df = processor.process(df)?;

            debug!(
                "Processor '{}' completed - output shape: {:?}",
                processor_name,
                df.shape()
            );
        }

        debug!("Pipeline '{}' completed successfully", self.name);
        Ok(df)
    }
}

impl Default for ProcessingPipeline {
    fn default() -> Self {
        Self::new()
    }
}

pub(crate) fn create_processor(
    config: &ProcessorConfig,
) -> PostProcessResult<Box<dyn PostProcessor>> {
    match config {
        ProcessorConfig::RenameColumns { mappings } => {
            Ok(Box::new(ColumnRenamer::new(mappings.clone())))
        }
        ProcessorConfig::DatetimeConvert { column, base, unit } => {
            let base_dt = DateTime::parse_from_rfc3339(base)
                .map_err(|e| {
                    PostProcessError::ConfigurationError(format!(
                        "Invalid base datetime '{}': {}",
                        base, e
                    ))
                })?
                .with_timezone(&Utc);
            Ok(Box::new(DateTimeConverter::new(
                column.clone(),
                base_dt,
                unit.clone(),
            )))
        }
        ProcessorConfig::UnitConvert {
            column,
            from_unit,
            to_unit,
        } => Ok(Box::new(UnitConverter::new(
            column.clone(),
            from_unit.clone(),
            to_unit.clone(),
        ))),
        ProcessorConfig::Aggregate {
            group_by,
            aggregations,
        } => Ok(Box::new(Aggregator::new(
            group_by.clone(),
            aggregations.clone(),
        ))),
        ProcessorConfig::ApplyFormula {
            target_column,
            formula,
            source_columns,
        } => Ok(Box::new(FormulaApplier::new(
            target_column.clone(),
            formula.clone(),
            source_columns.clone(),
        ))),
    }
}

/// Processor that renames DataFrame columns according to a mapping.
///
/// Columns not present in the mapping are left unchanged. If a key in the
/// mapping does not match any column in the DataFrame, a warning is logged and
/// the entry is silently skipped.
pub struct ColumnRenamer {
    mappings: HashMap<String, String>,
}

/// Processor that converts a numeric column of time offsets into a Polars
/// [`DataType::Datetime`] column.
///
/// The output values are computed as `base_datetime + value * unit_multiplier`.
pub struct DateTimeConverter {
    column: String,
    base_datetime: DateTime<Utc>,
    unit: TimeUnit,
}

/// Processor that converts the values of a column from one physical unit to
/// another.
///
/// Built-in conversions: Kelvin ↔ Celsius, Celsius ↔ Fahrenheit, Fahrenheit ↔
/// Celsius. All other unit pairs are treated as a multiplicative scaling by
/// `conversion_factor`.
pub struct UnitConverter {
    column: String,
    from_unit: String,
    to_unit: String,
    conversion_factor: f64,
}

/// Processor that aggregates rows using group-by and statistical operations.
///
/// When `group_by` is empty, a global (whole-DataFrame) aggregation is
/// performed.  Output column names are `{original_column}_{operation}`.
pub struct Aggregator {
    group_by: Vec<String>,
    aggregations: HashMap<String, AggregationOp>,
}

/// Processor that creates or overwrites a column by evaluating a mathematical
/// formula string.
///
/// Supported formula syntax:
/// - Arithmetic: `a + b`, `a - b`, `a * b`, `a / b`
/// - Functions: `sqrt(a)`
/// - Comparisons: `a < b`, `a > b`, `a == b`, `a != b`, `a <= b`, `a >= b`
/// - Constants: any valid `f64` literal (e.g. `273.15`)
/// - Operator precedence: `*` and `/` bind tighter than `+` and `-`
pub struct FormulaApplier {
    target_column: String,
    formula: String,
    source_columns: Vec<String>,
}

impl ColumnRenamer {
    /// Creates a new column renamer with the given name mappings.
    ///
    /// # Arguments
    ///
    /// * `mappings` - Map of `old_column_name -> new_column_name` pairs.
    ///   Columns absent from the map are left unchanged.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use nc2parquet::postprocess::ColumnRenamer;
    /// use std::collections::HashMap;
    ///
    /// let mut mappings = HashMap::new();
    /// mappings.insert("t2".to_string(), "temperature_2m".to_string());
    ///
    /// let renamer = ColumnRenamer::new(mappings);
    /// ```
    pub fn new(mappings: HashMap<String, String>) -> Self {
        Self { mappings }
    }
}

impl DateTimeConverter {
    /// Creates a new datetime converter.
    ///
    /// # Arguments
    ///
    /// * `column` - Name of the numeric column containing time offsets
    /// * `base_datetime` - The datetime that corresponds to offset `0`
    /// * `unit` - The time unit of the numeric offsets
    ///
    /// # Examples
    ///
    /// ```rust
    /// use nc2parquet::postprocess::{DateTimeConverter, TimeUnit};
    /// use chrono::{DateTime, Utc};
    ///
    /// let base: DateTime<Utc> = "1900-01-01T00:00:00Z".parse().unwrap();
    /// let converter = DateTimeConverter::new("time".to_string(), base, TimeUnit::Hours);
    /// ```
    pub fn new(column: String, base_datetime: DateTime<Utc>, unit: TimeUnit) -> Self {
        Self {
            column,
            base_datetime,
            unit,
        }
    }
}

impl UnitConverter {
    /// Creates a new unit converter with automatic conversion factor calculation.
    ///
    /// The conversion factor is derived from the `from_unit`/`to_unit` pair.
    /// Built-in supported pairs: `kelvin`↔`celsius`, `celsius`↔`fahrenheit`.
    /// All other pairs use a factor of `1.0` (no-op multiplication).
    ///
    /// # Arguments
    ///
    /// * `column` - Name of the column to convert
    /// * `from_unit` - Source unit string (case-insensitive, e.g. `"kelvin"`)
    /// * `to_unit` - Target unit string (case-insensitive, e.g. `"celsius"`)
    ///
    /// # Examples
    ///
    /// ```rust
    /// use nc2parquet::postprocess::UnitConverter;
    ///
    /// // Convert temperature column from Kelvin to Celsius
    /// let converter = UnitConverter::new(
    ///     "temperature".to_string(),
    ///     "kelvin".to_string(),
    ///     "celsius".to_string(),
    /// );
    /// ```
    pub fn new(column: String, from_unit: String, to_unit: String) -> Self {
        // Calculate conversion factor based on units
        let conversion_factor = Self::calculate_conversion_factor(&from_unit, &to_unit);
        Self {
            column,
            from_unit,
            to_unit,
            conversion_factor,
        }
    }

    /// Creates a new unit converter with an explicit conversion factor.
    ///
    /// Use this constructor when the automatic factor derivation is insufficient
    /// or when you need to apply an arbitrary scaling factor.
    ///
    /// # Arguments
    ///
    /// * `column` - Name of the column to convert
    /// * `from_unit` - Source unit string (used for logging only)
    /// * `to_unit` - Target unit string (used for logging only)
    /// * `factor` - Multiplicative scaling factor applied to every value
    ///
    /// # Examples
    ///
    /// ```rust
    /// use nc2parquet::postprocess::UnitConverter;
    ///
    /// // Convert meters to feet (1 m = 3.28084 ft)
    /// let converter = UnitConverter::with_conversion_factor(
    ///     "elevation".to_string(),
    ///     "meters".to_string(),
    ///     "feet".to_string(),
    ///     3.28084,
    /// );
    /// ```
    pub fn with_conversion_factor(
        column: String,
        from_unit: String,
        to_unit: String,
        factor: f64,
    ) -> Self {
        Self {
            column,
            from_unit,
            to_unit,
            conversion_factor: factor,
        }
    }

    fn calculate_conversion_factor(from_unit: &str, to_unit: &str) -> f64 {
        match (
            from_unit.to_lowercase().as_str(),
            to_unit.to_lowercase().as_str(),
        ) {
            ("kelvin", "celsius") | ("k", "c") => 1.0,
            ("celsius", "kelvin") | ("c", "k") => 1.0,
            ("celsius", "fahrenheit") | ("c", "f") => 9.0 / 5.0,
            ("fahrenheit", "celsius") | ("f", "c") => 5.0 / 9.0,
            _ => 1.0,
        }
    }
}

impl Aggregator {
    /// Creates a new aggregator.
    ///
    /// # Arguments
    ///
    /// * `group_by` - Columns to group by.  Pass an empty `Vec` for a global aggregation.
    /// * `aggregations` - Map of `column_name -> operation`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use nc2parquet::postprocess::{Aggregator, AggregationOp};
    /// use std::collections::HashMap;
    ///
    /// let mut aggs = HashMap::new();
    /// aggs.insert("temperature".to_string(), AggregationOp::Mean);
    ///
    /// // Group by "station", compute the mean temperature per station
    /// let aggregator = Aggregator::new(vec!["station".to_string()], aggs);
    /// ```
    pub fn new(group_by: Vec<String>, aggregations: HashMap<String, AggregationOp>) -> Self {
        Self {
            group_by,
            aggregations,
        }
    }
}

impl PostProcessor for ColumnRenamer {
    fn process(&self, mut df: DataFrame) -> PostProcessResult<DataFrame> {
        debug!("Renaming columns with {} mappings", self.mappings.len());

        for (old_name, new_name) in &self.mappings {
            let column_names: Vec<&str> =
                df.get_column_names().iter().map(|s| s.as_str()).collect();
            if !column_names.contains(&old_name.as_str()) {
                warn!(
                    "Column '{}' not found in DataFrame, skipping rename",
                    old_name
                );
                continue;
            }

            debug!("Renaming column '{}' to '{}'", old_name, new_name);
            df.rename(old_name, new_name.into())?;
        }

        Ok(df)
    }

    fn name(&self) -> &str {
        "ColumnRenamer"
    }

    fn description(&self) -> &str {
        "Renames columns based on provided mappings"
    }

    fn output_schema(&self, input_schema: &Schema) -> PostProcessResult<Schema> {
        let mut new_fields = Vec::new();
        for (name, dtype) in input_schema.iter() {
            let name_str = name.as_str();
            let new_name = if let Some(mapped) = self.mappings.get(name_str) {
                mapped.clone()
            } else {
                name_str.to_string()
            };
            new_fields.push(Field::new(new_name.into(), dtype.clone()));
        }

        Ok(Schema::from_iter(new_fields))
    }
}

impl PostProcessor for DateTimeConverter {
    fn process(&self, df: DataFrame) -> PostProcessResult<DataFrame> {
        debug!(
            "Converting column '{}' to datetime using base {} and unit {:?}",
            self.column,
            self.base_datetime.to_rfc3339(),
            self.unit
        );

        let column_names: Vec<&str> = df.get_column_names().iter().map(|s| s.as_str()).collect();
        if !column_names.contains(&self.column.as_str()) {
            return Err(PostProcessError::ColumnNotFound(self.column.clone()));
        }

        let base_timestamp_ms = self.base_datetime.timestamp_millis();
        let unit_multiplier_ms = self.unit.to_seconds_multiplier() * 1000.0;

        let result =
            df.lazy()
                .with_columns([(col(&self.column) * lit(unit_multiplier_ms)
                    + lit(base_timestamp_ms))
                .cast(DataType::Datetime(
                    polars::prelude::TimeUnit::Milliseconds,
                    None,
                ))
                .alias(&self.column)])
                .collect()?;

        Ok(result)
    }

    fn name(&self) -> &str {
        "DateTimeConverter"
    }

    fn description(&self) -> &str {
        "Converts numeric column values to datetime based on a base datetime and time unit"
    }

    fn output_schema(&self, input_schema: &Schema) -> PostProcessResult<Schema> {
        let mut new_schema = input_schema.clone();

        // Replace the column with datetime type
        new_schema.with_column(
            self.column.clone().into(),
            DataType::Datetime(polars::prelude::TimeUnit::Milliseconds, None),
        );

        Ok(new_schema)
    }
}

impl PostProcessor for UnitConverter {
    fn process(&self, df: DataFrame) -> PostProcessResult<DataFrame> {
        debug!(
            "Converting column '{}' from {} to {} (factor: {})",
            self.column, self.from_unit, self.to_unit, self.conversion_factor
        );

        let column_names: Vec<&str> = df.get_column_names().iter().map(|s| s.as_str()).collect();
        if !column_names.contains(&self.column.as_str()) {
            return Err(PostProcessError::ColumnNotFound(self.column.clone()));
        }

        let result = if (self.from_unit.to_lowercase() == "kelvin"
            || self.from_unit.to_lowercase() == "k")
            && (self.to_unit.to_lowercase() == "celsius" || self.to_unit.to_lowercase() == "c")
        {
            // Special case: Kelvin to Celsius (K - 273.15)
            df.lazy()
                .with_columns([(col(&self.column) - lit(273.15)).alias(&self.column)])
                .collect()?
        } else if (self.from_unit.to_lowercase() == "celsius"
            || self.from_unit.to_lowercase() == "c")
            && (self.to_unit.to_lowercase() == "kelvin" || self.to_unit.to_lowercase() == "k")
        {
            df.lazy()
                .with_columns([(col(&self.column) + lit(273.15)).alias(&self.column)])
                .collect()?
        } else if (self.from_unit.to_lowercase() == "celsius"
            || self.from_unit.to_lowercase() == "c")
            && (self.to_unit.to_lowercase() == "fahrenheit" || self.to_unit.to_lowercase() == "f")
        {
            df.lazy()
                .with_columns(
                    [(col(&self.column) * lit(9.0 / 5.0) + lit(32.0)).alias(&self.column)],
                )
                .collect()?
        } else if (self.from_unit.to_lowercase() == "fahrenheit"
            || self.from_unit.to_lowercase() == "f")
            && (self.to_unit.to_lowercase() == "celsius" || self.to_unit.to_lowercase() == "c")
        {
            df.lazy()
                .with_columns([
                    ((col(&self.column) - lit(32.0)) * lit(5.0 / 9.0)).alias(&self.column)
                ])
                .collect()?
        } else {
            df.lazy()
                .with_columns([
                    (col(&self.column) * lit(self.conversion_factor)).alias(&self.column)
                ])
                .collect()?
        };

        Ok(result)
    }

    fn name(&self) -> &str {
        "UnitConverter"
    }

    fn description(&self) -> &str {
        "Converts values in a column from one unit to another"
    }
}

impl PostProcessor for Aggregator {
    fn process(&self, df: DataFrame) -> PostProcessResult<DataFrame> {
        debug!(
            "Aggregating data with group_by: {:?}, aggregations: {:?}",
            self.group_by, self.aggregations
        );

        let column_names: Vec<&str> = df.get_column_names().iter().map(|s| s.as_str()).collect();
        for col_name in &self.group_by {
            if !column_names.contains(&col_name.as_str()) {
                return Err(PostProcessError::ColumnNotFound(col_name.clone()));
            }
        }

        for col_name in self.aggregations.keys() {
            if !column_names.contains(&col_name.as_str()) {
                return Err(PostProcessError::ColumnNotFound(col_name.clone()));
            }
        }

        let mut agg_exprs = Vec::new();

        for (col_name, agg_op) in &self.aggregations {
            let (expr, suffix) = match agg_op {
                AggregationOp::Mean => (col(col_name).mean(), "mean"),
                AggregationOp::Sum => (col(col_name).sum(), "sum"),
                AggregationOp::Min => (col(col_name).min(), "min"),
                AggregationOp::Max => (col(col_name).max(), "max"),
                AggregationOp::Count => (col(col_name).count(), "count"),
                AggregationOp::Std => (col(col_name).std(1), "std"),
                AggregationOp::Var => (col(col_name).var(1), "var"),
                AggregationOp::First => (col(col_name).first(), "first"),
                AggregationOp::Last => (col(col_name).last(), "last"),
            };
            agg_exprs.push(expr.alias(format!("{}_{}", col_name, suffix)));
        }

        let result = if !self.group_by.is_empty() {
            df.lazy()
                .group_by(self.group_by.iter().map(col).collect::<Vec<_>>())
                .agg(agg_exprs)
                .collect()?
        } else {
            df.lazy().select(agg_exprs).collect()?
        };

        Ok(result)
    }

    fn name(&self) -> &str {
        "Aggregator"
    }

    fn description(&self) -> &str {
        "Aggregates data using group by operations and statistical functions"
    }
}

impl PostProcessor for FormulaApplier {
    fn process(&self, df: DataFrame) -> PostProcessResult<DataFrame> {
        debug!(
            "Applying formula '{}' to create column '{}'",
            self.formula, self.target_column
        );

        let column_names: Vec<&str> = df.get_column_names().iter().map(|s| s.as_str()).collect();
        for col_name in &self.source_columns {
            if !column_names.contains(&col_name.as_str()) {
                return Err(PostProcessError::ColumnNotFound(col_name.clone()));
            }
        }

        let result = self.apply_formula(df)?;

        Ok(result)
    }

    fn name(&self) -> &str {
        "FormulaApplier"
    }

    fn description(&self) -> &str {
        "Applies mathematical formulas to create new columns or transform existing ones"
    }

    fn output_schema(&self, input_schema: &Schema) -> PostProcessResult<Schema> {
        let mut new_schema = input_schema.clone();
        if !new_schema.contains(&self.target_column) {
            new_schema.with_column(self.target_column.as_str().into(), DataType::Float64);
        }

        Ok(new_schema)
    }
}

impl FormulaApplier {
    /// Creates a new formula applier.
    ///
    /// # Arguments
    ///
    /// * `target_column` - Name of the column to create or overwrite with the
    ///   formula result
    /// * `formula` - Formula string (see [`FormulaApplier`] for syntax)
    /// * `source_columns` - Column names referenced inside the formula
    ///
    /// # Examples
    ///
    /// ```rust
    /// use nc2parquet::postprocess::FormulaApplier;
    ///
    /// // Create a "wind_speed" column as sqrt(u^2 + v^2)
    /// let applier = FormulaApplier::new(
    ///     "wind_speed".to_string(),
    ///     "sqrt(u * u + v * v)".to_string(),
    ///     vec!["u".to_string(), "v".to_string()],
    /// );
    /// ```
    pub fn new(target_column: String, formula: String, source_columns: Vec<String>) -> Self {
        Self {
            target_column,
            formula,
            source_columns,
        }
    }

    fn apply_formula(&self, df: DataFrame) -> PostProcessResult<DataFrame> {
        let formula = self.formula.trim();

        let result = if formula.contains('<')
            || formula.contains('>')
            || formula.contains("==")
            || formula.contains("!=")
        {
            self.parse_comparison_formula(df, formula)?
        } else if formula.contains('+')
            || formula.contains('-')
            || formula.contains('*')
            || formula.contains('/')
        {
            self.parse_arithmetic_formula(df, formula)?
        } else if formula.starts_with("sqrt(") {
            self.parse_function_formula(df, formula)?
        } else {
            let operand_expr = self.parse_operand_with_validation(&df, formula)?;
            df.lazy()
                .with_columns([operand_expr.alias(&self.target_column)])
                .collect()?
        };

        Ok(result)
    }

    fn parse_comparison_formula(
        &self,
        df: DataFrame,
        formula: &str,
    ) -> PostProcessResult<DataFrame> {
        let comparison_ops = ["==", "!=", "<=", ">=", "<", ">"];

        for op in comparison_ops {
            if formula.contains(op) {
                let parts: Vec<&str> = formula.split(op).collect();
                if parts.len() == 2 {
                    let left = parts[0].trim();
                    let right = parts[1].trim();

                    let left_expr = self.parse_operand_with_validation(&df, left)?;
                    let right_expr = self.parse_operand_with_validation(&df, right)?;

                    let result_expr = match op {
                        "==" => left_expr.eq(right_expr),
                        "!=" => left_expr.neq(right_expr),
                        "<" => left_expr.lt(right_expr),
                        "<=" => left_expr.lt_eq(right_expr),
                        ">" => left_expr.gt(right_expr),
                        ">=" => left_expr.gt_eq(right_expr),
                        _ => unreachable!(),
                    };

                    return Ok(df
                        .lazy()
                        .with_columns([result_expr.alias(&self.target_column)])
                        .collect()?);
                }
            }
        }

        Err(PostProcessError::ProcessingError(format!(
            "Unable to parse comparison formula: {}",
            formula
        )))
    }

    fn parse_arithmetic_formula(
        &self,
        df: DataFrame,
        formula: &str,
    ) -> PostProcessResult<DataFrame> {
        let expr = self.parse_expression(&df, formula)?;

        Ok(df
            .lazy()
            .with_columns([expr.alias(&self.target_column)])
            .collect()?)
    }

    /// Recursive expression parser with operator precedence (+ and - lowest, * and / higher).
    fn parse_expression(&self, df: &DataFrame, expr: &str) -> PostProcessResult<Expr> {
        let expr = expr.trim();
        let mut depth = 0;
        for (i, c) in expr.char_indices() {
            match c {
                '(' => depth += 1,
                ')' => depth -= 1,
                '+' | '-' if depth == 0 => {
                    let left = &expr[..i];
                    let right = &expr[i + 1..];
                    let left_expr = self.parse_expression(df, left)?;
                    let right_expr = self.parse_expression(df, right)?;

                    return Ok(match c {
                        '+' => left_expr + right_expr,
                        '-' => left_expr - right_expr,
                        _ => unreachable!(),
                    });
                }
                _ => {}
            }
        }

        self.parse_term(df, expr)
    }

    fn parse_term(&self, df: &DataFrame, expr: &str) -> PostProcessResult<Expr> {
        let expr = expr.trim();
        let mut depth = 0;
        for (i, c) in expr.char_indices() {
            match c {
                '(' => depth += 1,
                ')' => depth -= 1,
                '*' | '/' if depth == 0 => {
                    let left = &expr[..i];
                    let right = &expr[i + 1..];
                    let left_expr = self.parse_term(df, left)?;
                    let right_expr = self.parse_term(df, right)?;

                    return Ok(match c {
                        '*' => left_expr * right_expr,
                        '/' => left_expr / right_expr,
                        _ => unreachable!(),
                    });
                }
                _ => {}
            }
        }

        self.parse_factor(df, expr)
    }

    fn parse_factor(&self, df: &DataFrame, expr: &str) -> PostProcessResult<Expr> {
        let expr = expr.trim();

        if expr.starts_with('(') && expr.ends_with(')') {
            return self.parse_expression(df, &expr[1..expr.len() - 1]);
        }

        self.parse_operand_with_validation(df, expr)
    }

    fn parse_function_formula(&self, df: DataFrame, formula: &str) -> PostProcessResult<DataFrame> {
        if formula.starts_with("sqrt(") && formula.ends_with(")") {
            let inner = &formula[5..formula.len() - 1];
            let operand = self.parse_operand_with_validation(&df, inner)?;

            Ok(df
                .lazy()
                .with_columns([operand.sqrt().alias(&self.target_column)])
                .collect()?)
        } else {
            Err(PostProcessError::ProcessingError(format!(
                "Unsupported function in formula: {}",
                formula
            )))
        }
    }

    fn parse_operand_with_validation(
        &self,
        df: &DataFrame,
        operand: &str,
    ) -> PostProcessResult<Expr> {
        let operand = operand.trim();

        if let Ok(constant) = operand.parse::<f64>() {
            return Ok(lit(constant));
        }

        let column_names: Vec<String> = df
            .get_column_names()
            .iter()
            .map(|s| s.to_string())
            .collect();
        if column_names.contains(&operand.to_string()) {
            Ok(col(operand))
        } else {
            Err(PostProcessError::ProcessingError(format!(
                "Invalid operand '{}': not a valid number or existing column. Available columns: [{}]",
                operand,
                column_names.join(", ")
            )))
        }
    }
}
