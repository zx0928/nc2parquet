# nc2parquet

> High-performance NetCDF to Parquet converter with advanced filtering, cloud storage, and post-processing.

[![CI](https://github.com/rjmalves/nc2parquet/actions/workflows/ci.yml/badge.svg)](https://github.com/rjmalves/nc2parquet/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/nc2parquet.svg)](https://crates.io/crates/nc2parquet)
[![docs.rs](https://img.shields.io/docsrs/nc2parquet)](https://docs.rs/nc2parquet)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

## Features

- **Multi-variable extraction** — extract one or many variables per pass into a single Parquet file
- **Batch processing** — convert entire directory trees with a single glob pattern
- **Four filter types** — range, list, 2D spatial point, and 3D spatiotemporal point with intersection logic
- **Post-processing pipeline** — rename columns, convert units, apply mathematical formulas, and parse datetime values
- **Amazon S3 support** — read from and write to S3 buckets with standard AWS credential chains
- **Parquet output control** — choose compression codec, compression level, row group size, and column statistics
- **Multi-source configuration** — compose settings from CLI flags, environment variables, and JSON/YAML config files

## Installation

### Build from Source

```bash
git clone https://github.com/rjmalves/nc2parquet.git
cd nc2parquet
cargo build --release
# Binary is at target/release/nc2parquet
```

> The `netcdf` crate is compiled with `features = ["static"]`, so HDF5/NetCDF system
> headers are only required at build time. The resulting binary is self-contained.

### Library Dependency

```toml
[dependencies]
nc2parquet = "0.1"
```

## Quick Start

### Basic CLI Conversion

```bash
# Convert a single variable to Parquet
nc2parquet convert examples/data/simple_xy.nc output.parquet -n data

# Extract multiple variables into one file
nc2parquet convert examples/data/pres_temp_4D.nc output.parquet \
  -N temperature,pressure

# Batch convert all .nc files under a directory
nc2parquet convert "data/**/*.nc" output/ --glob "data/**/*.nc" -n temperature
```

### CLI with Filters and Post-Processing

```bash
# Range + list filters, then rename and convert units
nc2parquet convert examples/data/pres_temp_4D.nc output.parquet \
  -n temperature \
  --range "latitude:30:50" \
  --list "level:1000,850,500" \
  --rename "temperature:temp_k" \
  --kelvin-to-celsius temp_k \
  --formula "temp_f:temp_k*1.8+32:temp_k"
```

### Inspect a NetCDF File

```bash
# Human-readable file summary
nc2parquet info examples/data/pres_temp_4D.nc

# JSON output for scripting
nc2parquet info examples/data/pres_temp_4D.nc --format json

# Variable-specific detail
nc2parquet info examples/data/pres_temp_4D.nc -n temperature --detailed
```

### Library Usage

```rust
use nc2parquet::input::JobConfig;
use nc2parquet::process_netcdf_job_async;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = JobConfig::from_json(r#"{
        "nc_key": "examples/data/simple_xy.nc",
        "variable_name": "data",
        "parquet_key": "output.parquet",
        "filters": []
    }"#)?;

    process_netcdf_job_async(&config).await?;
    Ok(())
}
```

## CLI Reference

| Subcommand    | Description                                                                      |
| ------------- | -------------------------------------------------------------------------------- |
| `convert`     | Convert a NetCDF file (or glob of files) to Parquet                              |
| `validate`    | Validate a configuration file without processing any data                        |
| `info`        | Inspect NetCDF file structure, dimensions, variables, and metadata               |
| `template`    | Generate a configuration file template (basic, s3, multi-filter, weather, ocean) |
| `completions` | Emit shell completion scripts (bash, zsh, fish, PowerShell)                      |

Global flags available on every subcommand:

| Flag                   | Env variable        | Description                                              |
| ---------------------- | ------------------- | -------------------------------------------------------- |
| `-v / --verbose`       |                     | Enable debug logging                                     |
| `-q / --quiet`         |                     | Suppress all output except errors                        |
| `--output-format`      |                     | Structured output format: `human`, `json`, `yaml`, `csv` |
| `-c / --config <FILE>` | `NC2PARQUET_CONFIG` | Load a JSON or YAML configuration file                   |

Key `convert` flags:

| Flag                              | Env variable                 | Description                                               |
| --------------------------------- | ---------------------------- | --------------------------------------------------------- |
| `-n / --variable <NAME>`          | `NC2PARQUET_VARIABLE`        | Single variable name to extract                           |
| `-N / --variables <A,B,...>`      |                              | Comma-separated list of variables (multi-column output)   |
| `--glob <PATTERN>`                |                              | Glob pattern for batch processing                         |
| `--range <DIM:MIN:MAX>`           | `NC2PARQUET_RANGE_FILTERS`   | Range filter (repeatable)                                 |
| `--list <DIM:V1,V2,...>`          | `NC2PARQUET_LIST_FILTERS`    | List filter (repeatable)                                  |
| `--point2d <DIMS:LAT,LON:TOL>`    | `NC2PARQUET_POINT2D_FILTERS` | 2D spatial point filter (repeatable)                      |
| `--point3d <DIMS:T,LAT,LON:TOL>`  | `NC2PARQUET_POINT3D_FILTERS` | 3D spatiotemporal point filter (repeatable)               |
| `--compression <CODEC>`           |                              | `snappy` (default), `zstd`, `gzip`, `lz4`, `uncompressed` |
| `--compression-level <N>`         |                              | Zstd: 1–22; Gzip: 0–9                                     |
| `--row-group-size <ROWS>`         |                              | Maximum rows per Parquet row group                        |
| `--no-statistics`                 |                              | Disable column statistics (min, max, null count)          |
| `--rename <OLD:NEW>`              |                              | Rename output column (repeatable)                         |
| `--unit-convert <COL:FROM:TO>`    |                              | Unit conversion (repeatable)                              |
| `--kelvin-to-celsius <COL>`       |                              | Shortcut: convert column from Kelvin to Celsius           |
| `--formula <TARGET:EXPR:SOURCES>` |                              | Apply formula to create a new column (repeatable)         |
| `--dry-run`                       | `NC2PARQUET_DRY_RUN`         | Validate and plan without writing output                  |
| `--force`                         | `NC2PARQUET_FORCE`           | Overwrite existing output files                           |

## Configuration

Settings are resolved in the following priority order (highest wins):

1. CLI arguments
2. Environment variables (`NC2PARQUET_*`)
3. Configuration file (`-c / --config`)

Both JSON and YAML formats are supported. Generate a starter template with:

```bash
nc2parquet template basic -o config.json
nc2parquet template s3 --format yaml -o s3.yaml
```

See [`examples/configs/`](examples/configs/) for complete annotated examples covering
single-file, multi-filter, and S3 scenarios.

## Filter Types

All specified filters are intersected — a data point must satisfy every filter to be included.

| Filter     | CLI flag    | Selects                                            | Format                                |
| ---------- | ----------- | -------------------------------------------------- | ------------------------------------- |
| `range`    | `--range`   | Dimension values within `[min, max]`               | `DIM:MIN:MAX`                         |
| `list`     | `--list`    | Specific discrete dimension values                 | `DIM:V1,V2,...`                       |
| `2d_point` | `--point2d` | Grid cells within `tolerance` of (lat, lon) points | `LAT_DIM,LON_DIM:LAT,LON:TOL`         |
| `3d_point` | `--point3d` | Time steps + spatial cells within `tolerance`      | `T_DIM,LAT_DIM,LON_DIM:T,LAT,LON:TOL` |

See [`examples/configs/`](examples/configs/) for JSON/YAML filter configuration examples.

## Post-Processing

Processors run sequentially after extraction. Results from earlier steps are available to later steps.

| Processor         | CLI flag              | Description                                         |
| ----------------- | --------------------- | --------------------------------------------------- |
| Rename columns    | `--rename`            | Rename one or more output DataFrame columns         |
| Unit conversion   | `--unit-convert`      | Convert a column between physical units             |
| Kelvin to Celsius | `--kelvin-to-celsius` | Shortcut for the common temperature conversion      |
| Apply formula     | `--formula`           | Create a new column using a mathematical expression |
| DateTime convert  | config only           | Convert a numeric time offset to a datetime column  |

See [`examples/postprocessing/`](examples/postprocessing/) for complete pipeline examples.

## Storage

Both local paths and S3 URIs are accepted anywhere a file path is expected:

```bash
# Local to local
nc2parquet convert data/input.nc output/result.parquet -n temperature

# S3 to S3
nc2parquet convert s3://input-bucket/data.nc s3://output-bucket/result.parquet -n temperature

# Mixed
nc2parquet convert s3://input-bucket/data.nc local_result.parquet -n temperature
```

AWS credentials are resolved automatically via the standard AWS credential chain
(environment variables, `~/.aws/credentials`, IAM instance roles, etc.).

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for development setup, branch conventions, and the pull request process.

## Changelog

See [CHANGELOG.md](CHANGELOG.md) for release history and upgrade notes.

## License

[MIT](LICENSE)
