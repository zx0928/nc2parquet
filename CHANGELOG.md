# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Multi-variable extraction supporting `--variables`/`-N` CLI flag for extracting multiple variables in a single pass
- Glob pattern batch processing with `--glob` CLI flag for converting multiple NetCDF files matching a pattern
- Parquet output configuration with `--compression`, `--compression-level`, `--row-group-size`, and `--no_statistics` CLI flags
- Extended formula parser with mathematical functions: `sqrt`, `abs`, `ceil`, `floor`, `round`, `ln`, `log10`, `exp`, `sin`, `cos`, `tan` (unary) and `min`, `max`, `pow`, `log` (binary)
- Meteorological unit conversion families: pressure (Pa, hPa, kPa, atm, inHg, mmHg), speed (m/s, km/h, kt, mph, ft/s), and length (m, km, ft, mi, nm, cm, mm)
- DHAT heap profiling support behind `dhat-heap` feature flag

### Changed

- Extracted CLI command handlers into dedicated `src/handlers/` modules for improved code organization
- Migrated error types to `thiserror` with a unified `Nc2ParquetError` enum (11 variants) replacing ad-hoc error handling
- Tightened visibility modifiers using `pub(crate)` for internal modules
- Added exhaustive rustdoc documentation with doc-examples on all public API items
- Optimized extraction pipeline with `CombinationBuffer` for zero-per-row allocation during dimension iteration
- Implemented lazy expression batching in `ProcessingPipeline` for consecutive independent `UnitConverter` operations
- Reduced peak memory usage through scoped NetCDF file lifetimes and eager resource drops

## [0.1.1] - 2025-10-02

### Fixed

- Fixed datetime postprocessing to correctly generate datetime columns

## [0.1.0] - 2025-09-28

### Added

- NetCDF to Parquet conversion with flexible variable extraction
- Dimension-aware filtering with range, value list, index, and regex filters
- Post-processing framework with column renaming, datetime conversion, unit conversion, aggregation, and formula application
- Native Amazon S3 storage support for input and output files
- Multiple configuration formats: CLI arguments, JSON, YAML, and environment variables
- CLI with progress indicators, structured logging, and shell completions
- Async runtime support with tokio for S3 operations

[Unreleased]: https://github.com/rjmalves/nc2parquet/compare/v0.1.1...HEAD
[0.1.1]: https://github.com/rjmalves/nc2parquet/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/rjmalves/nc2parquet/releases/tag/v0.1.0
