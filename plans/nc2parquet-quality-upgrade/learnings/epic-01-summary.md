# Epic 01 Learnings: Testing Infrastructure & Coverage

## What Was Implemented

- Added 3 dev-dependencies: proptest 1.4, assert_cmd 2.0, predicates 3.1
- Created `src/test_helpers.rs` with 5 shared helper functions
- Reorganized monolithic `src/tests.rs` (2370 lines) into 8 per-module test files under `src/tests/`
- Added 22 filter edge-case tests (from_json, filter_factory, Triplets, range/list/point edge cases)
- Added 9 extract edge-case tests (DimensionIndexManager state, filter intersection, pair/triplet combos, zero-result extraction, multi-filter)
- Added 24 postprocess edge-case tests (UnitConverter variants, FormulaApplier arithmetic/precedence, Aggregator ops, Pipeline edge cases, error Display)
- Added 9 output/storage tests (parquet write, empty DataFrame, parent dir creation, async write, PAR1 magic, S3 path parsing)
- Added 7 property-based tests with proptest (filter construction, FilterResult invariants, formula properties, filter factory)
- Added 8 integration error-path tests (pipeline failure, invalid config, empty results, async error propagation)
- Test count grew from 97 to 181 (84 new tests)

## Codebase Insights

### Module Structure

- `src/lib.rs` is the main library entry point with `process_netcdf_job` and `process_netcdf_job_async`
- `src/main.rs` (~1100 lines) contains CLI handlers, config loading, template generation — ripe for extraction (Epic 02)
- Test modules are `#[cfg(test)]` gated in lib.rs, inline tests exist in cli.rs (8 tests) and storage.rs (7+4 tests)

### API Patterns

- `DimensionIndexManager` handles filter intersection via `apply_filter_result` with Single/Pairs/Triplets variants
- `FilterResult::Triplets` uses `apply_explicit_triplets` — builds one combo per triplet, no cross-product with remaining dims
- `ProcessingPipeline::from_config` validates DatetimeConvert base datetime at creation time (ConfigurationError)
- `PostProcessError` has 5 variants: ColumnNotFound, ConversionError, ConfigurationError, PolarsError, ProcessingError
- `filter_factory` expects JSON with "kind" field and validates it

### Formula Parser Behavior

- Recursive descent parser: splits on +/- at depth 0 left-to-right, then \*/÷, then handles parens and functions
- Negative constants not supported as standalone formulas (leading `-` parsed as binary subtraction)
- `sqrt` of negative → NaN (Polars behavior), not an error
- Comparison `<` supported via `parse_comparison_formula`

### Unit Converter Behavior

- Case-insensitive unit names (lowercased internally)
- Short aliases supported: "k" → "c" works like "kelvin" → "celsius"
- Unknown unit pairs fall back to factor=1.0 (no-op, no error)
- Supports: kelvin↔celsius, celsius↔fahrenheit, fahrenheit↔celsius, hpa↔pa (via factor)

### Data Files

- `simple_xy.nc`: 2D (x=6, y=12), variable "data", no coordinate variables (values default to indices)
- `pres_temp_4D.nc`: 4D, time(2), level(2), latitude(6: 25-50), longitude(12: -125 to -70), variables: temperature, pressure, latitude, longitude
- pres_temp_4D.nc has "time" dimension but NO time coordinate variable → NC3DPointFilter.apply() returns error

### Polars 0.51 API Notes

- `df!` macro for DataFrame creation
- `ParquetWriter::new(&mut file)` takes mutable reference
- `ParquetReader::new(file)` for reading parquet
- Empty DataFrame (0 rows) writes valid parquet with schema only
- Parquet files start with "PAR1" magic bytes

### Ticket Description Inaccuracies Found

- Ticket-002: cli_tests described as "already in src/cli.rs" but actually in tests.rs (16 tests)
- Ticket-003: postprocess_tests count listed as 14 but actually 15 (test_datetime_converter_column_not_found was miscounted)
- These were ticket documentation errors, not implementation issues

## Recommendations for Later Epics

1. **Epic 02 (handlers extraction)**: main.rs is ~1100 lines with handle_convert_command, handle_validate_command, handle_info_command, handle_template_command, handle_completions_command, load_configuration. All can be extracted to handlers/ module.
2. **Epic 03 (performance)**: Criterion benchmarks should target extract_data_to_dataframe (the hot path), PostProcessor pipeline, and parquet writing.
3. **Epic 04 (features)**: Formula parser needs extension for more math functions. The recursive descent structure makes this straightforward.
4. **Epic 05 (docs)**: Existing rustdoc on lib.rs functions is good. Other modules need rustdoc.
5. **Epic 06 (CI)**: cargo-tarpaulin for coverage. The 181 tests give a solid baseline.
