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
use polars::lazy::dsl::{max_horizontal, min_horizontal};
use polars::prelude::*;
use polars_ops::prelude::RoundMode;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub type PostProcessResult<T> = Result<T, PostProcessError>;

/// Errors that can occur during post-processing.
#[derive(thiserror::Error, Debug)]
pub enum PostProcessError {
    #[error("Column '{0}' not found in DataFrame")]
    ColumnNotFound(String),
    #[error("Conversion error: {0}")]
    ConversionError(String),
    #[error("Configuration error: {0}")]
    ConfigurationError(String),
    #[error("Polars error: {0}")]
    PolarsError(#[from] PolarsError),
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
    fn process(&self, df: DataFrame) -> PostProcessResult<DataFrame>;
    fn name(&self) -> &str;
    fn description(&self) -> &str;

    /// Validate that this processor can operate on the given DataFrame schema.
    /// The default implementation performs no validation.
    fn validate_schema(&self, schema: &Schema) -> PostProcessResult<()> {
        let _ = schema;
        Ok(())
    }

    /// Return the expected output schema after this processor runs.
    /// Override when the processor adds, removes, or renames columns.
    fn output_schema(&self, input_schema: &Schema) -> PostProcessResult<Schema> {
        Ok(input_schema.clone())
    }

    /// Return column names this processor reads from or writes to.
    ///
    /// Used by [`ProcessingPipeline`] to detect independent processors that can
    /// be batched into a single `.collect()` call. An empty `Vec` disables batching.
    fn target_columns(&self) -> Vec<String> {
        vec![]
    }

    /// Express this processor as Polars lazy [`Expr`]s for pipeline batching.
    ///
    /// When `Some` is returned, consecutive independent processors are fused into
    /// a single `.with_columns(…).collect()` call. Return `None` for schema-level
    /// renames, aggregations, or any transform that cannot be a column expression.
    fn to_lazy_expr(&self, schema: &Schema) -> Option<Vec<Expr>> {
        let _ = schema;
        None
    }
}

/// Configuration for the entire post-processing pipeline.
///
/// Use [`ProcessingPipeline::from_config`] to instantiate a runnable pipeline.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessingPipelineConfig {
    /// Human-readable name used in log output. Defaults to `"Configured Pipeline"`.
    pub name: Option<String>,
    /// Ordered list of processors; each step receives the output of the previous.
    pub processors: Vec<ProcessorConfig>,
}

/// Configuration for a single post-processing step.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ProcessorConfig {
    /// Rename one or more columns using an `old_name -> new_name` mapping.
    RenameColumns { mappings: HashMap<String, String> },
    /// Convert a numeric time-offset column to a [`DataType::Datetime`] column.
    DatetimeConvert {
        column: String,
        /// ISO 8601 base datetime (e.g. `"1900-01-01T00:00:00Z"`).
        base: String,
        unit: TimeUnit,
    },
    /// Convert column values from one physical unit to another.
    UnitConvert {
        column: String,
        from_unit: String,
        to_unit: String,
    },
    /// Aggregate rows via group-by and statistical operations.
    Aggregate {
        /// Columns to group by; use an empty list for a global aggregation.
        group_by: Vec<String>,
        aggregations: HashMap<String, AggregationOp>,
    },
    /// Create or overwrite a column by evaluating a mathematical formula.
    ApplyFormula {
        target_column: String,
        /// Formula string (e.g. `"a + b * 2.0"`, `"sqrt(a)"`, `"a < 5.0"`).
        formula: String,
        source_columns: Vec<String>,
    },
}

/// Time units for [`ProcessorConfig::DatetimeConvert`].
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TimeUnit {
    Seconds,
    Minutes,
    Hours,
    Days,
    Milliseconds,
    Microseconds,
    Nanoseconds,
}

/// Statistical aggregation operations for [`ProcessorConfig::Aggregate`].
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AggregationOp {
    Mean,
    Sum,
    Min,
    Max,
    Count,
    /// Sample standard deviation (ddof = 1).
    Std,
    /// Sample variance (ddof = 1).
    Var,
    First,
    Last,
}

impl TimeUnit {
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
    pub fn new() -> Self {
        Self {
            name: "Unnamed Pipeline".to_string(),
            processors: Vec::new(),
        }
    }

    pub fn with_name(name: String) -> Self {
        Self {
            name,
            processors: Vec::new(),
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    /// Creates a pipeline from a [`ProcessingPipelineConfig`].
    ///
    /// Returns [`PostProcessError::ConfigurationError`] if any processor config
    /// contains invalid parameters (e.g. a malformed base datetime string).
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

    pub fn add_processor(&mut self, processor: Box<dyn PostProcessor>) {
        self.processors.push(processor);
    }

    /// Executes all processors in order, returning the transformed DataFrame.
    ///
    /// Consecutive processors with disjoint [`target_columns`](PostProcessor::target_columns)
    /// that implement [`to_lazy_expr`](PostProcessor::to_lazy_expr) are batched into a
    /// single `.collect()` call to eliminate redundant materialisation.
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

        let mut i = 0;
        while i < self.processors.len() {
            let schema = df.schema();

            if let Some(exprs) = self.processors[i].to_lazy_expr(schema) {
                let mut batch_exprs = exprs;
                let mut batch_columns: std::collections::HashSet<String> =
                    self.processors[i].target_columns().into_iter().collect();
                let mut batch_end = i + 1;

                while batch_end < self.processors.len() {
                    let next_cols: std::collections::HashSet<String> = self.processors[batch_end]
                        .target_columns()
                        .into_iter()
                        .collect();

                    if batch_columns.is_disjoint(&next_cols)
                        && let Some(next_exprs) = self.processors[batch_end].to_lazy_expr(schema)
                    {
                        batch_exprs.extend(next_exprs);
                        batch_columns.extend(next_cols);
                        batch_end += 1;
                        continue;
                    }
                    break;
                }

                if batch_end > i + 1 {
                    debug!(
                        "Batching {} processors ({} to {}) into a single collect()",
                        batch_end - i,
                        i + 1,
                        batch_end
                    );
                    df = df
                        .lazy()
                        .with_columns(batch_exprs)
                        .collect()
                        .map_err(PostProcessError::PolarsError)?;
                } else {
                    let processor_name = self.processors[i].name();
                    debug!(
                        "Executing processor {} '{}' - input shape: {:?}",
                        i + 1,
                        processor_name,
                        df.shape()
                    );
                    df = self.processors[i].process(df)?;
                    debug!(
                        "Processor '{}' completed - output shape: {:?}",
                        processor_name,
                        df.shape()
                    );
                }

                i = batch_end;
            } else {
                let processor_name = self.processors[i].name();
                debug!(
                    "Executing processor {} '{}' - input shape: {:?}",
                    i + 1,
                    processor_name,
                    df.shape()
                );
                df = self.processors[i].process(df)?;
                debug!(
                    "Processor '{}' completed - output shape: {:?}",
                    processor_name,
                    df.shape()
                );
                i += 1;
            }
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

/// Renames DataFrame columns according to an `old_name -> new_name` mapping.
/// Keys absent from the DataFrame are logged as warnings and skipped.
pub struct ColumnRenamer {
    mappings: HashMap<String, String>,
}

/// Converts a numeric time-offset column into a [`DataType::Datetime`] column.
/// Output values are `base_datetime + value * unit_multiplier`.
pub struct DateTimeConverter {
    column: String,
    base_datetime: DateTime<Utc>,
    unit: TimeUnit,
}

/// Converts column values from one physical unit to another.
///
/// Built-in conversions: Kelvin ↔ Celsius, Celsius ↔ Fahrenheit.
/// All other pairs apply a multiplicative `conversion_factor`.
pub struct UnitConverter {
    column: String,
    from_unit: String,
    to_unit: String,
    conversion_factor: f64,
}

/// Aggregates rows via group-by and statistical operations.
/// An empty `group_by` produces a global aggregation.
/// Output column names are `{original_column}_{operation}`.
pub struct Aggregator {
    group_by: Vec<String>,
    aggregations: HashMap<String, AggregationOp>,
}

/// Creates or overwrites a column by evaluating a formula string.
///
/// Supported syntax:
/// - Arithmetic: `+`, `-`, `*`, `/` with standard precedence (`*`/`/` bind tighter than `+`/`-`).
/// - Parenthesised sub-expressions: `(a + b) * c`.
/// - Comparisons: `<`, `>`, `==`, `!=`, `<=`, `>=`.
/// - Numeric literals and column names.
/// - **Unary functions** (1 argument): `abs`, `sqrt`, `exp`, `ln`, `log10`,
///   `sin`, `cos`, `tan`, `ceil`, `floor`, `round`.
/// - **Binary functions** (2 arguments): `pow(base, exp)`, `min(a, b)`,
///   `max(a, b)`, `log(value, base)`.
///
/// Function names are case-insensitive. Functions may be nested arbitrarily
/// and combined with arithmetic, e.g. `abs(a - b) + pow(c, 2.0)`.
pub struct FormulaApplier {
    target_column: String,
    formula: String,
    source_columns: Vec<String>,
}

impl ColumnRenamer {
    pub fn new(mappings: HashMap<String, String>) -> Self {
        Self { mappings }
    }
}

impl DateTimeConverter {
    pub fn new(column: String, base_datetime: DateTime<Utc>, unit: TimeUnit) -> Self {
        Self {
            column,
            base_datetime,
            unit,
        }
    }
}

/// Physical unit families used for automatic conversion factor calculation.
///
/// Each family has a designated base unit.  Conversion factors between any two
/// members of the same family are derived from their individual base-unit factors,
/// so adding a new unit requires only a single entry in [`unit_to_base_factor`].
///
/// Temperature is handled separately because it requires an offset (not just a
/// scale), so its conversions remain as explicit match arms in
/// [`UnitConverter::build_conversion_expr`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum UnitFamily {
    /// Temperature (base: K).  Offset conversions handled separately.
    Temperature,
    /// Pressure (base: Pa).
    Pressure,
    /// Speed (base: m/s).
    Speed,
    /// Length (base: m).
    Length,
}

/// Maps a lowercased unit alias to its [`UnitFamily`] and the multiplicative
/// factor that converts **one unit of `alias`** into the family's base unit.
///
/// Examples
/// - `"hpa"` → `(Pressure, 100.0)`  because 1 hPa = 100 Pa
/// - `"km"` → `(Length, 1000.0)`    because 1 km = 1 000 m
/// - `"kt"` → `(Speed, 1.0/1.943844)` because 1 kt ≈ 0.5144 m/s
fn unit_to_base_factor(unit: &str) -> Option<(UnitFamily, f64)> {
    match unit {
        // ── Temperature (base: K) ──────────────────────────────────────────
        "kelvin" | "k" => Some((UnitFamily::Temperature, 1.0)),
        "celsius" | "c" => Some((UnitFamily::Temperature, 1.0)),
        "fahrenheit" | "f" => Some((UnitFamily::Temperature, 1.0)),

        // ── Pressure (base: Pa) ────────────────────────────────────────────
        "pa" | "pascal" => Some((UnitFamily::Pressure, 1.0)),
        "hpa" | "hectopascal" => Some((UnitFamily::Pressure, 100.0)),
        "mbar" | "millibar" => Some((UnitFamily::Pressure, 100.0)),
        "kpa" | "kilopascal" => Some((UnitFamily::Pressure, 1_000.0)),
        "atm" | "atmosphere" => Some((UnitFamily::Pressure, 101_325.0)),
        "inhg" => Some((UnitFamily::Pressure, 3_386.389)),
        "mmhg" => Some((UnitFamily::Pressure, 133.322)),

        // ── Speed (base: m/s) ──────────────────────────────────────────────
        "m/s" | "ms" => Some((UnitFamily::Speed, 1.0)),
        "km/h" | "kmh" => Some((UnitFamily::Speed, 1.0 / 3.6)),
        "kt" | "knot" | "knots" => Some((UnitFamily::Speed, 1.0 / 1.943_844)),
        "mph" => Some((UnitFamily::Speed, 1.0 / 2.236_936)),
        "ft/s" | "fts" => Some((UnitFamily::Speed, 1.0 / 3.280_84)),

        // ── Length (base: m) ───────────────────────────────────────────────
        "m" | "meter" | "metre" => Some((UnitFamily::Length, 1.0)),
        "km" | "kilometer" => Some((UnitFamily::Length, 1_000.0)),
        "ft" | "foot" | "feet" => Some((UnitFamily::Length, 0.304_8)),
        "mi" | "mile" => Some((UnitFamily::Length, 1_609.344)),
        "nm" | "nautical_mile" => Some((UnitFamily::Length, 1_852.0)),
        "cm" | "centimeter" => Some((UnitFamily::Length, 0.01)),
        "mm" | "millimeter" => Some((UnitFamily::Length, 0.001)),

        _ => None,
    }
}

impl UnitConverter {
    pub fn new(column: String, from_unit: String, to_unit: String) -> Self {
        let conversion_factor = Self::calculate_conversion_factor(&from_unit, &to_unit);
        Self {
            column,
            from_unit,
            to_unit,
            conversion_factor,
        }
    }

    /// Creates a unit converter with an explicit multiplicative scaling factor.
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

    /// Computes the multiplicative factor needed to convert `from_unit` values
    /// into `to_unit` values.
    ///
    /// For non-temperature families the factor is derived from the base-unit
    /// lookup: `factor = from_to_base / to_to_base`.
    ///
    /// Temperature conversions involve an additive offset and therefore cannot
    /// be expressed as a pure scale factor; this function returns `1.0` for
    /// temperature pairs, and [`build_conversion_expr`] handles them via
    /// explicit match arms.
    ///
    /// Unknown or cross-family pairs also return `1.0` for backward compatibility.
    fn calculate_conversion_factor(from_unit: &str, to_unit: &str) -> f64 {
        let from_lower = from_unit.to_lowercase();
        let to_lower = to_unit.to_lowercase();

        match (
            unit_to_base_factor(from_lower.as_str()),
            unit_to_base_factor(to_lower.as_str()),
        ) {
            (Some((from_family, from_base)), Some((to_family, to_base)))
                if from_family == to_family =>
            {
                if from_family == UnitFamily::Temperature {
                    // Offset conversions: scale is handled in build_conversion_expr.
                    1.0
                } else {
                    from_base / to_base
                }
            }
            _ => 1.0,
        }
    }

    /// Returns the Polars lazy [`Expr`] for this conversion, shared by `process` and `to_lazy_expr`.
    fn build_conversion_expr(&self) -> Expr {
        let from = self.from_unit.to_lowercase();
        let to = self.to_unit.to_lowercase();

        match (from.as_str(), to.as_str()) {
            ("kelvin", "celsius") | ("k", "c") => col(&self.column) - lit(273.15),
            ("celsius", "kelvin") | ("c", "k") => col(&self.column) + lit(273.15),
            ("celsius", "fahrenheit") | ("c", "f") => {
                col(&self.column) * lit(9.0_f64 / 5.0) + lit(32.0_f64)
            }
            ("fahrenheit", "celsius") | ("f", "c") => {
                (col(&self.column) - lit(32.0_f64)) * lit(5.0_f64 / 9.0)
            }
            _ => col(&self.column) * lit(self.conversion_factor),
        }
    }
}

impl Aggregator {
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

    fn target_columns(&self) -> Vec<String> {
        let mut cols: Vec<String> = self.mappings.keys().cloned().collect();
        cols.extend(self.mappings.values().cloned());
        cols
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

    fn target_columns(&self) -> Vec<String> {
        vec![self.column.clone()]
    }

    fn output_schema(&self, input_schema: &Schema) -> PostProcessResult<Schema> {
        let mut new_schema = input_schema.clone();
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

        let expr = self.build_conversion_expr();
        let result = df
            .lazy()
            .with_columns([expr.alias(&self.column)])
            .collect()?;

        Ok(result)
    }

    fn name(&self) -> &str {
        "UnitConverter"
    }

    fn description(&self) -> &str {
        "Converts values in a column from one unit to another"
    }

    fn target_columns(&self) -> Vec<String> {
        vec![self.column.clone()]
    }

    fn to_lazy_expr(&self, schema: &Schema) -> Option<Vec<Expr>> {
        if !schema.contains(&self.column) {
            return None;
        }

        let expr = self.build_conversion_expr();
        Some(vec![expr.alias(&self.column)])
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

    fn target_columns(&self) -> Vec<String> {
        let mut cols = self.group_by.clone();
        cols.extend(self.aggregations.keys().cloned());
        cols
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

    fn target_columns(&self) -> Vec<String> {
        let mut cols = self.source_columns.clone();
        cols.push(self.target_column.clone());
        cols
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
        } else {
            let expr = self.parse_expression(&df, formula)?;
            df.lazy()
                .with_columns([expr.alias(&self.target_column)])
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

        /// Parse a factor: a function call, a parenthesised sub-expression, or a
    /// bare operand (column name or numeric literal).
    ///
    /// Function calls are detected by `identifier(`. The matching closing
    /// parenthesis is found with depth-counting to handle nested parens.
    fn parse_factor(&self, df: &DataFrame, expr: &str) -> PostProcessResult<Expr> {
        let expr = expr.trim();

        // An identifier immediately followed by `(` is treated as a function call.
        // If there is any operator or whitespace before the `(`, it is a
        // parenthesised sub-expression, not a function call.
        if let Some(paren_pos) = expr.find('(') {
            let name = expr[..paren_pos].trim();
            let is_identifier =
                !name.is_empty() && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_');
            if is_identifier {
                let mut depth: i32 = 0;
                let mut close_pos = None;
                for (i, c) in expr[paren_pos..].char_indices() {
                    match c {
                        '(' => depth += 1,
                        ')' => {
                            depth -= 1;
                            if depth == 0 {
                                close_pos = Some(paren_pos + i);
                                break;
                            }
                        }
                        _ => {}
                    }
                }
                let close = close_pos.ok_or_else(|| {
                    PostProcessError::ProcessingError(format!(
                        "Unmatched '(' in function call '{}'",
                        expr
                    ))
                })?;
                let args_str = &expr[paren_pos + 1..close];
                return self.parse_function_call(df, name, args_str);
            }
        }

        // Parenthesised sub-expression: verify the parens match each other
        // (guard against e.g. `(a) + (b)` reaching parse_factor).
        if expr.starts_with('(') && expr.ends_with(')') {
            let mut depth: i32 = 0;
            let mut matched_at_end = false;
            let last = expr.len() - 1;
            for (i, c) in expr.char_indices() {
                match c {
                    '(' => depth += 1,
                    ')' => {
                        depth -= 1;
                        if depth == 0 && i == last {
                            matched_at_end = true;
                        }
                    }
                    _ => {}
                }
            }
            if matched_at_end {
                return self.parse_expression(df, &expr[1..expr.len() - 1]);
            }
        }

        self.parse_operand_with_validation(df, expr)
    }

    fn parse_function_call(
        &self,
        df: &DataFrame,
        name: &str,
        args_str: &str,
    ) -> PostProcessResult<Expr> {
        let func_name = name.to_lowercase();
        let args = Self::split_function_args(args_str);

        match func_name.as_str() {
            // ── Unary functions ──────────────────────────────────────────────
            "abs" => {
                check_arity("abs", &args, 1)?;
                Ok(self.parse_expression(df, &args[0])?.abs())
            }
            "sqrt" => {
                check_arity("sqrt", &args, 1)?;
                Ok(self.parse_expression(df, &args[0])?.sqrt())
            }
            "exp" => {
                check_arity("exp", &args, 1)?;
                Ok(self.parse_expression(df, &args[0])?.exp())
            }
            "ln" => {
                // ln(x) = log base e.  Polars .log(base: Expr) so we pass
                // lit(std::f64::consts::E).
                check_arity("ln", &args, 1)?;
                Ok(self
                    .parse_expression(df, &args[0])?
                    .log(lit(std::f64::consts::E)))
            }
            "log10" => {
                check_arity("log10", &args, 1)?;
                Ok(self.parse_expression(df, &args[0])?.log(lit(10.0_f64)))
            }
            "sin" => {
                check_arity("sin", &args, 1)?;
                Ok(self.parse_expression(df, &args[0])?.sin())
            }
            "cos" => {
                check_arity("cos", &args, 1)?;
                Ok(self.parse_expression(df, &args[0])?.cos())
            }
            "tan" => {
                check_arity("tan", &args, 1)?;
                Ok(self.parse_expression(df, &args[0])?.tan())
            }
            "ceil" => {
                check_arity("ceil", &args, 1)?;
                Ok(self.parse_expression(df, &args[0])?.ceil())
            }
            "floor" => {
                check_arity("floor", &args, 1)?;
                Ok(self.parse_expression(df, &args[0])?.floor())
            }
            "round" => {
                check_arity("round", &args, 1)?;
                // round(x) rounds to 0 decimal places (nearest integer).
                Ok(self
                    .parse_expression(df, &args[0])?
                    .round(0, RoundMode::HalfAwayFromZero))
            }

            // ── Binary functions ─────────────────────────────────────────────
            "pow" => {
                check_arity("pow", &args, 2)?;
                let base = self.parse_expression(df, &args[0])?;
                let exp = self.parse_expression(df, &args[1])?;
                Ok(base.pow(exp))
            }
            "min" => {
                // Element-wise minimum of two expressions.
                check_arity("min", &args, 2)?;
                let a = self.parse_expression(df, &args[0])?;
                let b = self.parse_expression(df, &args[1])?;
                min_horizontal([a, b]).map_err(PostProcessError::PolarsError)
            }
            "max" => {
                // Element-wise maximum of two expressions.
                check_arity("max", &args, 2)?;
                let a = self.parse_expression(df, &args[0])?;
                let b = self.parse_expression(df, &args[1])?;
                max_horizontal([a, b]).map_err(PostProcessError::PolarsError)
            }
            "log" => {
                // log(value, base) — change-of-base via Polars .log(base: Expr).
                check_arity("log", &args, 2)?;
                let value = self.parse_expression(df, &args[0])?;
                let base = self.parse_expression(df, &args[1])?;
                Ok(value.log(base))
            }

            _ => Err(PostProcessError::ProcessingError(format!(
                "Unknown function: {}",
                name
            ))),
        }
    }

    /// Split comma-separated args respecting parenthesis nesting, so that
    /// `f(a, g(b, c))` splits into `["a", "g(b, c)"]`.
    fn split_function_args(args_str: &str) -> Vec<String> {
        let mut args = Vec::new();
        let mut depth: i32 = 0;
        let mut current = String::new();

        for c in args_str.chars() {
            match c {
                '(' => {
                    depth += 1;
                    current.push(c);
                }
                ')' => {
                    depth -= 1;
                    current.push(c);
                }
                ',' if depth == 0 => {
                    args.push(current.trim().to_string());
                    current.clear();
                }
                _ => current.push(c),
            }
        }

        let tail = current.trim().to_string();
        if !tail.is_empty() {
            args.push(tail);
        }

        args
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

fn check_arity(name: &str, args: &[String], expected: usize) -> PostProcessResult<()> {
    if args.len() == expected {
        Ok(())
    } else {
        Err(PostProcessError::ProcessingError(format!(
            "Function '{}' expects {} argument(s), got {}",
            name,
            expected,
            args.len()
        )))
    }
}
