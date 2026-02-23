# nc2parquet Tutorials

Hands-on guides for common nc2parquet workflows. Each tutorial builds on the
example data files in `examples/data/` so you can run every command directly
from the repository root.

## Prerequisites

- nc2parquet installed and available on your `PATH`
- Repository cloned: `git clone https://github.com/rjmalves/nc2parquet.git && cd nc2parquet`

## Tutorials

| Tutorial                                                          | Description                                                                                                                                           |
| ----------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------- |
| [Basic NetCDF to Parquet Conversion](basic-conversion.md)         | Inspect a NetCDF file, extract a single variable, and produce a Parquet file. Covers the `info` and `convert` subcommands with the `-n` / `-N` flags. |
| [Filtered Extraction and Post-Processing](filtered-extraction.md) | Narrow down data with `--range` and `--list` filters, then transform the output with column renaming, formulas, and unit conversion.                  |
| [Batch Processing Multiple Files](batch-processing.md)            | Use `--glob` to convert many NetCDF files in a single invocation and understand the batch-mode output directory convention.                           |
| [Working with Configuration Files](config-files.md)               | Generate, edit, and validate JSON/YAML config files; understand config precedence relative to CLI flags and environment variables.                    |

## Where to Go Next

After completing the tutorials, see the project-level documentation:

- [README](../../README.md) — installation, quick start, and S3 support overview
- `examples/configs/` — ready-to-use configuration file examples
- `examples/postprocessing/` — post-processing pipeline examples
- `nc2parquet --help` — full CLI reference
