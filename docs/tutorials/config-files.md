# Working with Configuration Files

This tutorial covers how to generate configuration file templates, edit them
for your use case, validate them, and supply them to the `convert` command.
It also explains how CLI flags, environment variables, and config files
interact when more than one source of configuration is present.

## Prerequisites

- [Basic NetCDF to Parquet Conversion](basic-conversion.md) completed
- An `output/` directory available: `mkdir -p output`

---

### Step 1: Generate a Config Template

The `template` subcommand prints a ready-to-edit configuration file to stdout
or to a file. Several template types are available:

| Template type  | Description                                  |
| -------------- | -------------------------------------------- |
| `basic`        | Minimal single-file conversion               |
| `s3`           | S3 input/output with authentication settings |
| `multi-filter` | Range, list, and spatial point filters       |
| `weather`      | Weather-data processing pipeline             |
| `ocean`        | Ocean/marine data pipeline                   |

Generate a basic JSON template and save it to a file:

```bash
nc2parquet template basic -o my_config.json
```

Generate a multi-filter YAML template and print it to stdout:

```bash
nc2parquet template multi-filter --format yaml
```

Generate a weather template in YAML format and save it:

```bash
nc2parquet template weather --format yaml -o weather.yaml
```

---

### Step 2: Edit the Template for Your Use Case

Open the generated file and update the fields to match your data. A basic
config file has this structure:

```json
{
  "nc_key": "examples/data/simple_xy.nc",
  "variable_name": "data",
  "parquet_key": "output/simple_xy.parquet",
  "filters": []
}
```

Key fields:

- `nc_key` — path or S3 URI of the input NetCDF file
- `variable_name` — name of the variable to extract (must match the NetCDF
  file exactly; use `nc2parquet info` to discover variable names)
- `parquet_key` — path or S3 URI for the output Parquet file
- `filters` — list of filter objects (range, list, 2d_point, or 3d_point)
- `postprocessing` — optional pipeline of column transformations
- `output` — optional Parquet writer settings (compression, row group size)

See `examples/configs/simple_local.json` for a working minimal example and
`examples/configs/multi_filter.json` for a file with multiple filter types.

---

### Step 3: Validate the Config

The `validate` subcommand checks a config file for syntax errors and logical
problems without performing any conversion:

```bash
nc2parquet validate examples/configs/simple_local.json
```

For a detailed validation report:

```bash
nc2parquet validate examples/configs/simple_local.json --detailed
```

Validation checks include:

- JSON/YAML syntax and required field presence
- Filter parameter ranges (e.g. min must be less than max for range filters)
- Local file existence for `nc_key` paths
- S3 URI format validity
- Compression level ranges

---

### Step 4: Run a Conversion with `--config`

Pass the config file to `convert` using the global `--config` flag:

```bash
nc2parquet convert --config examples/configs/simple_local.json
```

The `--config` flag is a **global** flag and can be placed before or after the
subcommand name. It is also available as an environment variable (see Step 6).

To run one of the provided postprocessing examples directly:

```bash
nc2parquet convert --config examples/postprocessing/complex_pipeline.json
```

---

### Step 5: YAML Config Alternative

Both JSON and YAML formats are supported. The YAML equivalent of the basic
JSON config above is:

```yaml
nc_key: examples/data/simple_xy.nc
variable_name: data
parquet_key: output/simple_xy.parquet
filters: []
```

A YAML config with a filter and post-processing section:

```yaml
nc_key: examples/data/pres_temp_4D.nc
variable_name: temperature
parquet_key: output/result.parquet
filters:
  - kind: range
    params:
      dimension_name: latitude
      min_value: 25.0
      max_value: 45.0
postprocessing:
  name: Unit Conversion
  processors:
    - type: rename_columns
      mappings:
        temperature: temp_kelvin
    - type: unit_convert
      column: temp_kelvin
      from_unit: kelvin
      to_unit: celsius
```

---

### Step 6: Config Precedence

When multiple sources specify the same setting, the order of precedence from
highest to lowest is:

1. **CLI flags** — always override everything else
2. **Environment variables** — override the config file but not CLI flags
3. **Config file** — lowest priority; sets defaults for any unspecified values

Key environment variables:

| Variable                   | Equivalent flag                                                  |
| -------------------------- | ---------------------------------------------------------------- |
| `NC2PARQUET_CONFIG`        | `--config`                                                       |
| `NC2PARQUET_INPUT`         | positional `INPUT` argument                                      |
| `NC2PARQUET_OUTPUT`        | positional `OUTPUT` argument                                     |
| `NC2PARQUET_VARIABLE`      | `-n` / `--variable`                                              |
| `NC2PARQUET_FORCE`         | `--force`                                                        |
| `NC2PARQUET_DRY_RUN`       | `--dry-run`                                                      |
| `NC2PARQUET_RANGE_FILTERS` | `--range` (comma-separated, e.g. `"lat:30:60,lon:-180:0"`)       |
| `NC2PARQUET_LIST_FILTERS`  | `--list` (semicolon-separated, e.g. `"level:1000,850;dim2:val"`) |

Example: supply the config via environment variable, then override the output
path from the CLI:

```bash
export NC2PARQUET_CONFIG=my_config.json
nc2parquet convert --output-override output/override.parquet
```

---

### Step 7: Reference Examples in the Repository

The `examples/` directory contains ready-to-run configuration files organised
by topic:

```
examples/
  configs/
    simple_local.json        — minimal single-variable conversion
    multi_filter.json        — range + list + 2d_point filters combined
    multi_source_example.json — range and list filter on pres_temp_4D.nc
  postprocessing/
    column_renaming.json     — rename multiple columns
    unit_conversion.json     — Kelvin to Celsius via unit_convert processor
    formula_application.json — derive new columns with arithmetic formulas
    complex_pipeline.json    — full pipeline: rename, datetime, unit, formula
    datetime_conversion.json — convert numeric time index to timestamps
    complex_formula.json     — multi-step formula pipeline
```

Run any of them directly to see nc2parquet in action:

```bash
nc2parquet convert --config examples/postprocessing/column_renaming.json
nc2parquet convert --config examples/postprocessing/unit_conversion.json
nc2parquet convert --config examples/postprocessing/formula_application.json
```

---

## What's Next

You have now covered all core nc2parquet workflows. Return to the tutorial
index for an overview, or consult the reference documentation:

- [Tutorial Index](README.md)
- [README](../../README.md) — project overview, installation, and S3 setup
- `nc2parquet --help` — full CLI reference
- `nc2parquet convert --help` — all convert flags
- `nc2parquet template --help` — all available template types
