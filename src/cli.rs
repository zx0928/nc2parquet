use crate::input::FilterConfig;
use clap::{Parser, Subcommand, ValueEnum};
use clap_complete::Shell;
use serde::{Deserialize, Serialize};
use std::env;
use std::path::PathBuf;

/// High-performance NetCDF to Parquet converter with cloud storage support
#[derive(Parser, Debug)]
#[command(name = "nc2parquet")]
#[command(about = "Convert NetCDF files to Parquet format with advanced filtering")]
#[command(version)]
#[command(author = "Rogerio Alves <rjmalves@users.noreply.github.com>")]
#[command(long_about = "
nc2parquet is a high-performance command-line tool for converting NetCDF files to Parquet format.
It supports advanced filtering capabilities, cloud storage (S3), and provides comprehensive 
configuration management.

FEATURES:
  • Multiple filter types: Range, list, 2D point, and 3D point filters
  • Cloud storage support: Direct S3 input/output with authentication
  • Configuration files: JSON and YAML format support with templates
  • Progress indicators: Real-time progress bars and performance metrics
  • Validation: Comprehensive configuration and data validation
  • Shell completions: Auto-completion for bash, zsh, fish, and PowerShell

EXAMPLES:
  # Basic conversion
  nc2parquet convert input.nc output.parquet -n temperature

  # With filters  
  nc2parquet convert data.nc filtered.parquet -n temp \\
    --range 'latitude:30:60' --list 'level:1000,850,500'

  # S3 support
  nc2parquet convert s3://bucket/input.nc s3://bucket/output.parquet -n sst

  # Using config file
  nc2parquet convert --config weather.json

  # Generate templates
  nc2parquet template multi-filter --format yaml > config.yaml

  # File inspection
  nc2parquet info data.nc --detailed

  # Generate completions
  nc2parquet completions bash > ~/.bash_completion.d/nc2parquet

For more information and examples, see: https://github.com/rjmalves/nc2parquet
")]
pub struct Cli {
    /// Enable verbose logging
    #[arg(short, long, global = true)]
    pub verbose: bool,

    /// Quiet mode - suppress all output except errors
    #[arg(short, long, global = true, conflicts_with = "verbose")]
    pub quiet: bool,

    /// Output format for structured data
    #[arg(long, global = true, value_enum, default_value_t = OutputFormat::Human)]
    pub output_format: OutputFormat,

    /// Configuration file path (JSON or YAML)
    #[arg(short, long, global = true, env = "NC2PARQUET_CONFIG")]
    pub config: Option<PathBuf>,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
#[allow(clippy::large_enum_variant)] // Reason: Convert variant is intentionally large; boxing would add indirection cost
pub enum Commands {
    /// Convert NetCDF files to Parquet format
    #[command(long_about = "
Convert NetCDF files to Parquet format with optional filtering.

This command supports both local files and S3 objects as input/output.
Filters can be specified via command-line arguments or configuration files.

EXAMPLES:
  # Basic conversion
  nc2parquet convert input.nc output.parquet -n temperature

  # With multiple filters
  nc2parquet convert weather.nc filtered.parquet -n temp \\
    --range 'time:0:365' --range 'latitude:30:60' \\
    --list 'pressure:1000,850,500'

  # S3 to S3 conversion
  nc2parquet convert s3://data/input.nc s3://results/output.parquet -n sst

  # Dry run for validation
  nc2parquet convert input.nc output.parquet -n temp --dry-run

  # Using config file with overrides
  nc2parquet convert --config base.json \\
    --input-override new_input.nc --output-override new_output.parquet
")]
    Convert {
        /// Input NetCDF file path (local or S3)
        #[arg(value_name = "INPUT", env = "NC2PARQUET_INPUT")]
        input: Option<String>,

        /// Output Parquet file path (local or S3)
        #[arg(value_name = "OUTPUT", env = "NC2PARQUET_OUTPUT")]
        output: Option<String>,

        /// NetCDF variable name to extract
        #[arg(short = 'n', long, env = "NC2PARQUET_VARIABLE")]
        variable: Option<String>,

        /// Override input path from config
        #[arg(long, env = "NC2PARQUET_INPUT_OVERRIDE")]
        input_override: Option<String>,

        /// Override output path from config
        #[arg(long, env = "NC2PARQUET_OUTPUT_OVERRIDE")]
        output_override: Option<String>,

        /// Apply range filter: dimension:min:max
        #[arg(long = "range", value_parser = parse_range_filter)]
        range_filters: Vec<RangeFilterArg>,

        /// Apply list filter: dimension:val1,val2,val3
        #[arg(long = "list", value_parser = parse_list_filter)]
        list_filters: Vec<ListFilterArg>,

        /// Apply 2D point filter: lat_dim,lon_dim:lat,lon:tolerance
        #[arg(long = "point2d", value_parser = parse_point2d_filter)]
        point2d_filters: Vec<Point2DFilterArg>,

        /// Apply 3D point filter: time_dim,lat_dim,lon_dim:time,lat,lon:tolerance
        #[arg(long = "point3d", value_parser = parse_point3d_filter)]
        point3d_filters: Vec<Point3DFilterArg>,

        /// Force overwrite existing output files
        #[arg(long, env = "NC2PARQUET_FORCE")]
        force: bool,

        /// Dry run - validate configuration without processing
        #[arg(long, env = "NC2PARQUET_DRY_RUN")]
        dry_run: bool,

        /// Rename column: old_name:new_name (can be used multiple times)
        #[arg(long = "rename", value_parser = parse_rename_column)]
        rename_columns: Vec<RenameColumnArg>,

        /// Convert column units: column:from_unit:to_unit
        #[arg(long = "unit-convert", value_parser = parse_unit_conversion)]
        unit_conversions: Vec<UnitConversionArg>,

        /// Convert temperature from Kelvin to Celsius for given column
        #[arg(long = "kelvin-to-celsius")]
        kelvin_to_celsius: Vec<String>,

        /// Apply mathematical formula: target_column:formula:source1,source2,...
        #[arg(long = "formula", value_parser = parse_formula)]
        formulas: Vec<FormulaArg>,
    },

    /// Validate configuration file or arguments
    #[command(long_about = "
Validate configuration files and command-line arguments without processing.

This command performs comprehensive validation including:
• Configuration file syntax and structure
• Filter parameter validation  
• File existence checks (for local files)
• S3 path format validation
• Environment variable validation

EXAMPLES:
  # Validate a configuration file
  nc2parquet validate config.json

  # Validate with detailed output
  nc2parquet validate config.yaml --detailed

  # Validate using global config
  nc2parquet validate --config ~/.nc2parquet.json
")]
    Validate {
        /// Configuration file to validate
        config_file: Option<PathBuf>,

        /// Show detailed validation report
        #[arg(long)]
        detailed: bool,
    },

    /// Show information about NetCDF file
    #[command(long_about = "
Inspect NetCDF files and display structure information.

This command analyzes NetCDF files (local or S3) and displays:
• File dimensions and their sizes
• Available variables and their attributes
• Variable-specific information (when specified)
• Coordinate information and metadata

EXAMPLES:
  # Basic file info
  nc2parquet info data.nc

  # Detailed information
  nc2parquet info weather.nc --detailed

  # Info about specific variable
  nc2parquet info ocean.nc -n sea_surface_temperature

  # JSON output for scripting
  nc2parquet info data.nc --format json

  # S3 file inspection
  nc2parquet info s3://bucket/data.nc --detailed
")]
    Info {
        /// NetCDF file path (local or S3)
        file: String,

        /// Show detailed variable information
        #[arg(long)]
        detailed: bool,

        /// Show only specific variable info
        #[arg(short = 'n', long)]
        variable: Option<String>,

        /// Output format for file information
        #[arg(long, value_enum)]
        format: Option<OutputFormat>,
    },

    /// Generate configuration templates
    #[command(long_about = "
Generate configuration file templates for common use cases.

Available templates:
• basic: Simple conversion template
• s3: S3 storage template with authentication
• multi-filter: Complex filtering examples
• weather: Weather data processing template  
• ocean: Ocean/marine data template

Templates can be generated in JSON or YAML format and saved to files
or printed to stdout for piping to other commands.

EXAMPLES:
  # Generate basic JSON template
  nc2parquet template basic

  # Generate YAML template to file
  nc2parquet template s3 --format yaml -o s3_config.yaml

  # Generate multi-filter example
  nc2parquet template multi-filter --format yaml

  # Generate and edit template
  nc2parquet template weather > weather.json
  # ... edit weather.json ...
  nc2parquet convert --config weather.json
")]
    Template {
        /// Template type to generate
        #[arg(value_enum)]
        template_type: TemplateType,

        /// Output file path (default: stdout)
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Configuration format
        #[arg(long, value_enum, default_value_t = ConfigFormat::Json)]
        format: ConfigFormat,
    },

    /// Generate shell completions
    #[command(long_about = "
Generate shell completion scripts for various shells.

Supports bash, zsh, fish, and PowerShell completion generation.
Completions provide auto-completion for all commands, options, and values.

INSTALLATION:
  # Bash (add to ~/.bashrc or /etc/bash_completion.d/)
  nc2parquet completions bash > ~/.bash_completion.d/nc2parquet
  source ~/.bashrc

  # Zsh (add to ~/.zshrc or fpath)
  nc2parquet completions zsh > ~/.zsh/completions/_nc2parquet
  # Add to ~/.zshrc: fpath=(~/.zsh/completions $fpath)

  # Fish (save to completions directory)
  nc2parquet completions fish > ~/.config/fish/completions/nc2parquet.fish

  # PowerShell (add to profile)
  nc2parquet completions powershell > nc2parquet.ps1

EXAMPLES:
  # Generate bash completions
  nc2parquet completions bash

  # Save zsh completions to file
  nc2parquet completions zsh -o _nc2parquet

  # Test completions work
  nc2parquet <TAB><TAB>
")]
    Completions {
        /// Shell to generate completions for
        #[arg(value_enum)]
        shell: Shell,

        /// Output file path (default: stdout)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
}

#[derive(ValueEnum, Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum OutputFormat {
    /// Human-readable output
    Human,
    /// JSON structured output
    Json,
    /// YAML structured output  
    Yaml,
    /// CSV output (where applicable)
    Csv,
}

#[derive(ValueEnum, Clone, Debug, PartialEq, Eq)]
pub enum TemplateType {
    /// Basic conversion template
    Basic,
    /// S3 storage template
    S3,
    /// Multi-filter template
    MultiFilter,
    /// Weather data template
    Weather,
    /// Ocean data template
    Ocean,
}

#[derive(ValueEnum, Clone, Debug, PartialEq, Eq)]
pub enum ConfigFormat {
    /// JSON configuration format
    Json,
    /// YAML configuration format
    Yaml,
}

/// Parsed range filter argument from the command line (`--range dimension:min:max`).
#[derive(Clone, Debug, PartialEq)]
pub struct RangeFilterArg {
    /// Name of the NetCDF dimension to filter.
    pub dimension: String,
    /// Minimum value of the range (inclusive).
    pub min_value: f64,
    /// Maximum value of the range (inclusive).
    pub max_value: f64,
}

/// Parsed list filter argument from the command line (`--list dimension:v1,v2,...`).
#[derive(Clone, Debug, PartialEq)]
pub struct ListFilterArg {
    /// Name of the NetCDF dimension to filter.
    pub dimension: String,
    /// Discrete values to include.
    pub values: Vec<f64>,
}

/// Parsed 2D spatial point filter argument from the command line
/// (`--point2d lat_dim,lon_dim:lat,lon:tolerance`).
#[derive(Debug, Clone)]
pub struct Point2DFilterArg {
    /// Name of the latitude dimension variable.
    pub lat_dimension: String,
    /// Name of the longitude dimension variable.
    pub lon_dimension: String,
    /// Target latitude coordinate.
    pub lat: f64,
    /// Target longitude coordinate.
    pub lon: f64,
    /// Maximum coordinate distance for a cell to be considered a match.
    pub tolerance: f64,
}

/// Parsed 3D spatiotemporal point filter argument from the command line
/// (`--point3d time_dim,lat_dim,lon_dim:time,lat,lon:tolerance`).
#[derive(Debug, Clone)]
pub struct Point3DFilterArg {
    /// Name of the time dimension variable.
    pub time_dimension: String,
    /// Name of the latitude dimension variable.
    pub lat_dimension: String,
    /// Name of the longitude dimension variable.
    pub lon_dimension: String,
    /// Exact time step value to include.
    pub time: f64,
    /// Target latitude coordinate.
    pub lat: f64,
    /// Target longitude coordinate.
    pub lon: f64,
    /// Maximum coordinate distance for a cell to be considered a spatial match.
    pub tolerance: f64,
}

/// Parsed column rename argument from the command line (`--rename old_name:new_name`).
#[derive(Debug, Clone)]
pub struct RenameColumnArg {
    /// Original column name in the DataFrame.
    pub old_name: String,
    /// Replacement column name.
    pub new_name: String,
}

/// Parsed unit conversion argument from the command line
/// (`--unit-convert column:from_unit:to_unit`).
#[derive(Debug, Clone)]
pub struct UnitConversionArg {
    /// Name of the column to convert.
    pub column: String,
    /// Source unit (e.g. `"kelvin"`).
    pub from_unit: String,
    /// Target unit (e.g. `"celsius"`).
    pub to_unit: String,
}

/// Parsed mathematical formula argument from the command line
/// (`--formula target:formula:source1,source2,...`).
#[derive(Debug, Clone)]
pub struct FormulaArg {
    /// Name of the column to create or overwrite with the formula result.
    pub target_column: String,
    /// Formula string (e.g. `"a + b * 2.0"`).
    pub formula: String,
    /// Column names referenced by the formula.
    pub source_columns: Vec<String>,
}

fn parse_range_filter(s: &str) -> Result<RangeFilterArg, String> {
    let parts: Vec<&str> = s.split(':').collect();
    if parts.len() != 3 {
        return Err("Range filter must be in format 'dimension:min:max'".to_string());
    }

    let dimension = parts[0].to_string();
    let min_value = parts[1]
        .parse::<f64>()
        .map_err(|_| "Invalid minimum value in range filter")?;
    let max_value = parts[2]
        .parse::<f64>()
        .map_err(|_| "Invalid maximum value in range filter")?;

    if min_value >= max_value {
        return Err("Minimum value must be less than maximum value".to_string());
    }

    Ok(RangeFilterArg {
        dimension,
        min_value,
        max_value,
    })
}

fn parse_list_filter(s: &str) -> Result<ListFilterArg, String> {
    let parts: Vec<&str> = s.split(':').collect();
    if parts.len() != 2 {
        return Err("List filter must be in format 'dimension:val1,val2,val3'".to_string());
    }

    let dimension = parts[0].to_string();
    let values: Result<Vec<f64>, _> = parts[1]
        .split(',')
        .map(|v| v.trim().parse::<f64>())
        .collect();

    let values = values.map_err(|_| "Invalid numeric values in list filter")?;

    if values.is_empty() {
        return Err("List filter must contain at least one value".to_string());
    }

    Ok(ListFilterArg { dimension, values })
}

fn parse_point2d_filter(s: &str) -> Result<Point2DFilterArg, String> {
    let parts: Vec<&str> = s.split(':').collect();
    if parts.len() != 3 {
        return Err(
            "2D point filter must be in format 'lat_dim,lon_dim:lat,lon:tolerance'".to_string(),
        );
    }

    let dimensions: Vec<&str> = parts[0].split(',').collect();
    if dimensions.len() != 2 {
        return Err("2D point filter dimensions must be 'lat_dim,lon_dim'".to_string());
    }

    let coords: Vec<&str> = parts[1].split(',').collect();
    if coords.len() != 2 {
        return Err("2D point filter coordinates must be 'lat,lon'".to_string());
    }

    let lat_dimension = dimensions[0].to_string();
    let lon_dimension = dimensions[1].to_string();
    let lat = coords[0]
        .parse::<f64>()
        .map_err(|_| "Invalid latitude value")?;
    let lon = coords[1]
        .parse::<f64>()
        .map_err(|_| "Invalid longitude value")?;
    let tolerance = parts[2]
        .parse::<f64>()
        .map_err(|_| "Invalid tolerance value")?;

    if tolerance <= 0.0 {
        return Err("Tolerance must be positive".to_string());
    }

    Ok(Point2DFilterArg {
        lat_dimension,
        lon_dimension,
        lat,
        lon,
        tolerance,
    })
}

fn parse_point3d_filter(s: &str) -> Result<Point3DFilterArg, String> {
    let parts: Vec<&str> = s.split(':').collect();
    if parts.len() != 3 {
        return Err(
            "3D point filter must be in format 'time_dim,lat_dim,lon_dim:time,lat,lon:tolerance'"
                .to_string(),
        );
    }

    let dimensions: Vec<&str> = parts[0].split(',').collect();
    if dimensions.len() != 3 {
        return Err("3D point filter dimensions must be 'time_dim,lat_dim,lon_dim'".to_string());
    }

    let coords: Vec<&str> = parts[1].split(',').collect();
    if coords.len() != 3 {
        return Err("3D point filter coordinates must be 'time,lat,lon'".to_string());
    }

    let time_dimension = dimensions[0].to_string();
    let lat_dimension = dimensions[1].to_string();
    let lon_dimension = dimensions[2].to_string();
    let time = coords[0].parse::<f64>().map_err(|_| "Invalid time value")?;
    let lat = coords[1]
        .parse::<f64>()
        .map_err(|_| "Invalid latitude value")?;
    let lon = coords[2]
        .parse::<f64>()
        .map_err(|_| "Invalid longitude value")?;
    let tolerance = parts[2]
        .parse::<f64>()
        .map_err(|_| "Invalid tolerance value")?;

    if tolerance <= 0.0 {
        return Err("Tolerance must be positive".to_string());
    }

    Ok(Point3DFilterArg {
        time_dimension,
        lat_dimension,
        lon_dimension,
        time,
        lat,
        lon,
        tolerance,
    })
}

fn parse_rename_column(s: &str) -> Result<RenameColumnArg, String> {
    let parts: Vec<&str> = s.split(':').collect();
    if parts.len() != 2 {
        return Err("Column rename must be in format 'old_name:new_name'".to_string());
    }

    let old_name = parts[0].trim().to_string();
    let new_name = parts[1].trim().to_string();

    if old_name.is_empty() || new_name.is_empty() {
        return Err("Column names cannot be empty".to_string());
    }

    Ok(RenameColumnArg { old_name, new_name })
}

fn parse_unit_conversion(s: &str) -> Result<UnitConversionArg, String> {
    let parts: Vec<&str> = s.split(':').collect();
    if parts.len() != 3 {
        return Err("Unit conversion must be in format 'column:from_unit:to_unit'".to_string());
    }

    let column = parts[0].trim().to_string();
    let from_unit = parts[1].trim().to_string();
    let to_unit = parts[2].trim().to_string();

    if column.is_empty() || from_unit.is_empty() || to_unit.is_empty() {
        return Err("Column and unit names cannot be empty".to_string());
    }

    Ok(UnitConversionArg {
        column,
        from_unit,
        to_unit,
    })
}

fn parse_formula(s: &str) -> Result<FormulaArg, String> {
    let parts: Vec<&str> = s.splitn(3, ':').collect();
    if parts.len() != 3 {
        return Err(
            "Formula must be in format 'target_column:formula:source1,source2,...'".to_string(),
        );
    }

    let target_column = parts[0].trim().to_string();
    let formula = parts[1].trim().to_string();
    let source_columns: Vec<String> = parts[2]
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    if target_column.is_empty() || formula.is_empty() || source_columns.is_empty() {
        return Err("Target column, formula, and source columns cannot be empty".to_string());
    }

    Ok(FormulaArg {
        target_column,
        formula,
        source_columns,
    })
}

impl From<RangeFilterArg> for FilterConfig {
    fn from(arg: RangeFilterArg) -> Self {
        FilterConfig::Range {
            params: crate::input::RangeParams {
                dimension_name: arg.dimension,
                min_value: arg.min_value,
                max_value: arg.max_value,
            },
        }
    }
}

impl From<ListFilterArg> for FilterConfig {
    fn from(arg: ListFilterArg) -> Self {
        FilterConfig::List {
            params: crate::input::ListParams {
                dimension_name: arg.dimension,
                values: arg.values,
            },
        }
    }
}

impl From<Point2DFilterArg> for FilterConfig {
    fn from(arg: Point2DFilterArg) -> Self {
        FilterConfig::Point2D {
            params: crate::input::Point2DParams {
                lat_dimension_name: arg.lat_dimension,
                lon_dimension_name: arg.lon_dimension,
                points: vec![(arg.lat, arg.lon)],
                tolerance: arg.tolerance,
            },
        }
    }
}

impl From<Point3DFilterArg> for FilterConfig {
    fn from(arg: Point3DFilterArg) -> Self {
        FilterConfig::Point3D {
            params: crate::input::Point3DParams {
                time_dimension_name: arg.time_dimension,
                lat_dimension_name: arg.lat_dimension,
                lon_dimension_name: arg.lon_dimension,
                steps: vec![arg.time],
                points: vec![(arg.lat, arg.lon)],
                tolerance: arg.tolerance,
            },
        }
    }
}

type FilterResult = Result<
    (
        Vec<RangeFilterArg>,
        Vec<ListFilterArg>,
        Vec<Point2DFilterArg>,
        Vec<Point3DFilterArg>,
    ),
    String,
>;

/// Parse filters from environment variables.
///
/// - `NC2PARQUET_RANGE_FILTERS`: `"dim1:min1:max1,dim2:min2:max2"`
/// - `NC2PARQUET_LIST_FILTERS`: `"dim1:val1,val2,val3;dim2:val4,val5"`
/// - `NC2PARQUET_POINT2D_FILTERS`: `"lat,lon:30.0,-120.0:0.1;lat2,lon2:40.0,-100.0:0.2"`
/// - `NC2PARQUET_POINT3D_FILTERS`: `"time,lat,lon:0.0,30.0,-120.0:0.1"`
pub(crate) fn parse_filters_from_env() -> FilterResult {
    let mut range_filters = Vec::new();
    let mut list_filters = Vec::new();
    let mut point2d_filters = Vec::new();
    let mut point3d_filters = Vec::new();

    if let Ok(range_env) = env::var("NC2PARQUET_RANGE_FILTERS")
        && !range_env.trim().is_empty()
    {
        for filter_str in range_env.split(',') {
            let filter_str = filter_str.trim();
            if !filter_str.is_empty() {
                range_filters.push(parse_range_filter(filter_str).map_err(|e| {
                    format!("Invalid range filter in NC2PARQUET_RANGE_FILTERS: {}", e)
                })?);
            }
        }
    }

    if let Ok(list_env) = env::var("NC2PARQUET_LIST_FILTERS")
        && !list_env.trim().is_empty()
    {
        for filter_str in list_env.split(';') {
            let filter_str = filter_str.trim();
            if !filter_str.is_empty() {
                list_filters.push(parse_list_filter(filter_str).map_err(|e| {
                    format!("Invalid list filter in NC2PARQUET_LIST_FILTERS: {}", e)
                })?);
            }
        }
    }

    if let Ok(point2d_env) = env::var("NC2PARQUET_POINT2D_FILTERS")
        && !point2d_env.trim().is_empty()
    {
        for filter_str in point2d_env.split(';') {
            let filter_str = filter_str.trim();
            if !filter_str.is_empty() {
                point2d_filters.push(parse_point2d_filter(filter_str).map_err(|e| {
                    format!(
                        "Invalid 2D point filter in NC2PARQUET_POINT2D_FILTERS: {}",
                        e
                    )
                })?);
            }
        }
    }

    if let Ok(point3d_env) = env::var("NC2PARQUET_POINT3D_FILTERS")
        && !point3d_env.trim().is_empty()
    {
        for filter_str in point3d_env.split(';') {
            let filter_str = filter_str.trim();
            if !filter_str.is_empty() {
                point3d_filters.push(parse_point3d_filter(filter_str).map_err(|e| {
                    format!(
                        "Invalid 3D point filter in NC2PARQUET_POINT3D_FILTERS: {}",
                        e
                    )
                })?);
            }
        }
    }

    Ok((
        range_filters,
        list_filters,
        point2d_filters,
        point3d_filters,
    ))
}

/// Merges CLI filter arguments with filters parsed from environment variables.
///
/// When a CLI argument list is non-empty it takes priority and the corresponding
/// environment variable filters are ignored.  When a CLI argument list is empty,
/// the environment variable filters are used instead.
///
/// # Returns
///
/// Returns a tuple of `(range_filters, list_filters, point2d_filters, point3d_filters)`.
///
/// # Errors
///
/// Returns an error string if any environment variable contains a malformed
/// filter specification.
///
/// # Examples
///
/// ```rust
/// use nc2parquet::cli::{merge_filters, RangeFilterArg};
///
/// // With CLI arguments present, they take priority
/// let cli_range = vec![RangeFilterArg {
///     dimension: "latitude".to_string(),
///     min_value: 30.0,
///     max_value: 60.0,
/// }];
///
/// let (range, _list, _pt2d, _pt3d) = merge_filters(
///     cli_range,
///     vec![],
///     vec![],
///     vec![],
/// ).unwrap();
///
/// assert_eq!(range.len(), 1);
/// assert_eq!(range[0].dimension, "latitude");
/// ```
pub fn merge_filters(
    cli_range: Vec<RangeFilterArg>,
    cli_list: Vec<ListFilterArg>,
    cli_point2d: Vec<Point2DFilterArg>,
    cli_point3d: Vec<Point3DFilterArg>,
) -> FilterResult {
    let (env_range, env_list, env_point2d, env_point3d) = parse_filters_from_env()?;

    let merged_range = if cli_range.is_empty() {
        env_range
    } else {
        cli_range
    };
    let merged_list = if cli_list.is_empty() {
        env_list
    } else {
        cli_list
    };
    let merged_point2d = if cli_point2d.is_empty() {
        env_point2d
    } else {
        cli_point2d
    };
    let merged_point3d = if cli_point3d.is_empty() {
        env_point3d
    } else {
        cli_point3d
    };

    Ok((merged_range, merged_list, merged_point2d, merged_point3d))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    // Global mutex to ensure environment variable tests run sequentially
    static ENV_TEST_MUTEX: Mutex<()> = Mutex::new(());

    #[test]
    fn test_parse_range_filter() {
        let result = parse_range_filter("latitude:30.0:60.0").unwrap();
        assert_eq!(result.dimension, "latitude");
        assert_eq!(result.min_value, 30.0);
        assert_eq!(result.max_value, 60.0);

        // Test invalid formats
        assert!(parse_range_filter("latitude:30.0").is_err());
        assert!(parse_range_filter("latitude:30.0:60.0:extra").is_err());
        assert!(parse_range_filter("latitude:invalid:60.0").is_err());
        assert!(parse_range_filter("latitude:60.0:30.0").is_err()); // min > max
    }

    #[test]
    fn test_parse_list_filter() {
        let result = parse_list_filter("pressure:850.0,500.0,200.0").unwrap();
        assert_eq!(result.dimension, "pressure");
        assert_eq!(result.values, vec![850.0, 500.0, 200.0]);

        // Test single value
        let result = parse_list_filter("time:0.0").unwrap();
        assert_eq!(result.values, vec![0.0]);

        // Test invalid formats
        assert!(parse_list_filter("pressure:850.0,invalid,200.0").is_err());
        assert!(parse_list_filter("pressure:").is_err());
        assert!(parse_list_filter("pressure").is_err());
    }

    #[test]
    fn test_filter_conversion() {
        let range_arg = RangeFilterArg {
            dimension: "lat".to_string(),
            min_value: 10.0,
            max_value: 50.0,
        };

        let filter_config: FilterConfig = range_arg.into();
        if let FilterConfig::Range { params } = filter_config {
            assert_eq!(params.dimension_name, "lat");
            assert_eq!(params.min_value, 10.0);
            assert_eq!(params.max_value, 50.0);
        } else {
            panic!("Expected Range filter config");
        }
    }

    #[test]
    fn test_parse_point2d_filter() {
        let result = parse_point2d_filter("latitude,longitude:30.5,-120.2:0.1").unwrap();
        assert_eq!(result.lat_dimension, "latitude");
        assert_eq!(result.lon_dimension, "longitude");
        assert_eq!(result.lat, 30.5);
        assert_eq!(result.lon, -120.2);
        assert_eq!(result.tolerance, 0.1);

        // Test invalid formats
        assert!(parse_point2d_filter("latitude,longitude:30.5:-120.2").is_err()); // missing tolerance
        assert!(parse_point2d_filter("latitude:30.5,-120.2:0.1").is_err()); // missing longitude dimension
        assert!(parse_point2d_filter("latitude,longitude:invalid,-120.2:0.1").is_err()); // invalid lat
        assert!(parse_point2d_filter("latitude,longitude:30.5,-120.2:0.0").is_err()); // zero tolerance
        assert!(parse_point2d_filter("latitude,longitude:30.5,-120.2:-0.1").is_err()); // negative tolerance
    }

    #[test]
    fn test_parse_point3d_filter() {
        let result = parse_point3d_filter("time,latitude,longitude:0.0,30.5,-120.2:0.1").unwrap();
        assert_eq!(result.time_dimension, "time");
        assert_eq!(result.lat_dimension, "latitude");
        assert_eq!(result.lon_dimension, "longitude");
        assert_eq!(result.time, 0.0);
        assert_eq!(result.lat, 30.5);
        assert_eq!(result.lon, -120.2);
        assert_eq!(result.tolerance, 0.1);

        // Test invalid formats
        assert!(parse_point3d_filter("time,latitude:0.0,30.5,-120.2:0.1").is_err()); // missing lon dimension
        assert!(parse_point3d_filter("time,latitude,longitude:0.0,30.5:0.1").is_err()); // missing lon coordinate
        assert!(parse_point3d_filter("time,latitude,longitude:invalid,30.5,-120.2:0.1").is_err()); // invalid time
        assert!(parse_point3d_filter("time,latitude,longitude:0.0,30.5,-120.2:0.0").is_err()); // zero tolerance
    }

    #[test]
    fn test_point_filter_conversion() {
        // Test 2D point filter conversion
        let point2d_arg = Point2DFilterArg {
            lat_dimension: "lat".to_string(),
            lon_dimension: "lon".to_string(),
            lat: 45.0,
            lon: -120.0,
            tolerance: 1.0,
        };

        let filter_config: FilterConfig = point2d_arg.into();
        if let FilterConfig::Point2D { params } = filter_config {
            assert_eq!(params.lat_dimension_name, "lat");
            assert_eq!(params.lon_dimension_name, "lon");
            assert_eq!(params.points, vec![(45.0, -120.0)]);
            assert_eq!(params.tolerance, 1.0);
        } else {
            panic!("Expected Point2D filter config");
        }

        // Test 3D point filter conversion
        let point3d_arg = Point3DFilterArg {
            time_dimension: "time".to_string(),
            lat_dimension: "lat".to_string(),
            lon_dimension: "lon".to_string(),
            time: 100.0,
            lat: 45.0,
            lon: -120.0,
            tolerance: 1.0,
        };

        let filter_config: FilterConfig = point3d_arg.into();
        if let FilterConfig::Point3D { params } = filter_config {
            assert_eq!(params.time_dimension_name, "time");
            assert_eq!(params.lat_dimension_name, "lat");
            assert_eq!(params.lon_dimension_name, "lon");
            assert_eq!(params.steps, vec![100.0]);
            assert_eq!(params.points, vec![(45.0, -120.0)]);
            assert_eq!(params.tolerance, 1.0);
        } else {
            panic!("Expected Point3D filter config");
        }
    }

    #[test]
    fn test_environment_variable_filter_parsing() {
        // Acquire mutex to ensure exclusive access to environment variables
        let _guard = ENV_TEST_MUTEX.lock().unwrap();

        use std::env;

        // Save existing environment state
        let original_range = env::var("NC2PARQUET_RANGE_FILTERS").ok();
        let original_list = env::var("NC2PARQUET_LIST_FILTERS").ok();
        let original_point2d = env::var("NC2PARQUET_POINT2D_FILTERS").ok();
        let original_point3d = env::var("NC2PARQUET_POINT3D_FILTERS").ok();

        // Test with environment variables set
        unsafe {
            env::set_var("NC2PARQUET_RANGE_FILTERS", "lat:30:60,lon:-180:180");
            env::set_var(
                "NC2PARQUET_LIST_FILTERS",
                "pressure:1000,850,500;level:1,2,3",
            );
            env::set_var("NC2PARQUET_POINT2D_FILTERS", "lat,lon:30.0,-120.0:0.1");
            env::set_var(
                "NC2PARQUET_POINT3D_FILTERS",
                "time,lat,lon:0.0,30.0,-120.0:0.1",
            );
        }

        let result = parse_filters_from_env().unwrap();
        assert_eq!(result.0.len(), 2); // 2 range filters
        assert_eq!(result.1.len(), 2); // 2 list filters  
        assert_eq!(result.2.len(), 1); // 1 point2d filter
        assert_eq!(result.3.len(), 1); // 1 point3d filter

        // Verify filter content
        assert_eq!(result.0[0].dimension, "lat");
        assert_eq!(result.0[0].min_value, 30.0);
        assert_eq!(result.0[0].max_value, 60.0);

        assert_eq!(result.1[0].dimension, "pressure");
        assert_eq!(result.1[0].values, vec![1000.0, 850.0, 500.0]);

        assert_eq!(result.2[0].lat_dimension, "lat");
        assert_eq!(result.2[0].lon_dimension, "lon");
        assert_eq!(result.2[0].lat, 30.0);
        assert_eq!(result.2[0].lon, -120.0);
        assert_eq!(result.2[0].tolerance, 0.1);

        // Cleanup and restore original state
        unsafe {
            // Remove test variables
            env::remove_var("NC2PARQUET_RANGE_FILTERS");
            env::remove_var("NC2PARQUET_LIST_FILTERS");
            env::remove_var("NC2PARQUET_POINT2D_FILTERS");
            env::remove_var("NC2PARQUET_POINT3D_FILTERS");

            // Restore original variables if they existed
            if let Some(ref val) = original_range {
                env::set_var("NC2PARQUET_RANGE_FILTERS", val);
            }
            if let Some(ref val) = original_list {
                env::set_var("NC2PARQUET_LIST_FILTERS", val);
            }
            if let Some(ref val) = original_point2d {
                env::set_var("NC2PARQUET_POINT2D_FILTERS", val);
            }
            if let Some(ref val) = original_point3d {
                env::set_var("NC2PARQUET_POINT3D_FILTERS", val);
            }
        }

        // Test with empty environment (temporarily clear everything)
        unsafe {
            env::remove_var("NC2PARQUET_RANGE_FILTERS");
            env::remove_var("NC2PARQUET_LIST_FILTERS");
            env::remove_var("NC2PARQUET_POINT2D_FILTERS");
            env::remove_var("NC2PARQUET_POINT3D_FILTERS");
        }

        let result = parse_filters_from_env().unwrap();
        assert!(result.0.is_empty()); // range filters
        assert!(result.1.is_empty()); // list filters
        assert!(result.2.is_empty()); // point2d filters
        assert!(result.3.is_empty()); // point3d filters

        // Final restore of original state
        unsafe {
            if let Some(ref val) = original_range {
                env::set_var("NC2PARQUET_RANGE_FILTERS", val);
            }
            if let Some(ref val) = original_list {
                env::set_var("NC2PARQUET_LIST_FILTERS", val);
            }
            if let Some(ref val) = original_point2d {
                env::set_var("NC2PARQUET_POINT2D_FILTERS", val);
            }
            if let Some(ref val) = original_point3d {
                env::set_var("NC2PARQUET_POINT3D_FILTERS", val);
            }
        }
    }

    #[test]
    fn test_filter_merging_priority() {
        // Acquire mutex to ensure exclusive access to environment variables
        let _guard = ENV_TEST_MUTEX.lock().unwrap();

        use std::env;

        // Save existing environment state
        let original_range = env::var("NC2PARQUET_RANGE_FILTERS").ok();
        let original_list = env::var("NC2PARQUET_LIST_FILTERS").ok();
        let original_point2d = env::var("NC2PARQUET_POINT2D_FILTERS").ok();
        let original_point3d = env::var("NC2PARQUET_POINT3D_FILTERS").ok();

        // Clean up any existing environment variables first
        unsafe {
            env::remove_var("NC2PARQUET_RANGE_FILTERS");
            env::remove_var("NC2PARQUET_LIST_FILTERS");
            env::remove_var("NC2PARQUET_POINT2D_FILTERS");
            env::remove_var("NC2PARQUET_POINT3D_FILTERS");
        }

        // Set environment variables
        unsafe {
            env::set_var("NC2PARQUET_RANGE_FILTERS", "lat:0:90");
            env::set_var("NC2PARQUET_LIST_FILTERS", "pressure:1000,850");
        }

        // Test CLI priority over environment
        let cli_range = vec![RangeFilterArg {
            dimension: "lon".to_string(),
            min_value: -180.0,
            max_value: 180.0,
        }];
        let cli_list = vec![];
        let cli_point2d = vec![];
        let cli_point3d = vec![];

        let (merged_range, merged_list, _, _) = merge_filters(
            cli_range.clone(),
            cli_list.clone(),
            cli_point2d,
            cli_point3d,
        )
        .unwrap();

        // CLI range filter should be used (not environment)
        assert_eq!(merged_range.len(), 1);
        assert_eq!(merged_range[0].dimension, "lon");

        // Environment list filter should be used (CLI is empty)
        assert_eq!(merged_list.len(), 1);
        assert_eq!(merged_list[0].dimension, "pressure");

        // Cleanup and restore original state
        unsafe {
            env::remove_var("NC2PARQUET_RANGE_FILTERS");
            env::remove_var("NC2PARQUET_LIST_FILTERS");
            env::remove_var("NC2PARQUET_POINT2D_FILTERS");
            env::remove_var("NC2PARQUET_POINT3D_FILTERS");

            // Restore original variables if they existed
            if let Some(ref val) = original_range {
                env::set_var("NC2PARQUET_RANGE_FILTERS", val);
            }
            if let Some(ref val) = original_list {
                env::set_var("NC2PARQUET_LIST_FILTERS", val);
            }
            if let Some(ref val) = original_point2d {
                env::set_var("NC2PARQUET_POINT2D_FILTERS", val);
            }
            if let Some(ref val) = original_point3d {
                env::set_var("NC2PARQUET_POINT3D_FILTERS", val);
            }
        }
    }
}
