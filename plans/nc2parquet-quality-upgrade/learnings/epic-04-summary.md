# Accumulated Learnings Through Epic 04

## Module Structure (current state after epic-04)

- `src/lib.rs` — exports `process_netcdf_job`, `process_netcdf_job_async`, `process_netcdf_batch`, `resolve_output_path`; also re-exports `BatchConfig`, `BatchResult` from `input`; `extract` and `output` modules are `pub(crate)`
- `src/extract.rs` — `DimensionIndexManager`, `CombinationBuffer`, `extract_data_to_dataframe`, `extract_multi_variable_dataframe` (pub(crate)); also `extract_variable_values_with_dim_manager`, `extract_variable_values_batch`, `extract_variable_values_cellwise` (private helpers for multi-variable path)
- `src/postprocess.rs` — `PostProcessor` trait, `ProcessingPipeline`, 5 processor types; `UnitFamily` enum and `unit_to_base_factor` (both private); `FormulaApplier` now supports 11 unary + 4 binary functions via `parse_function_call` + `split_function_args`
- `src/input.rs` — `JobConfig` (now with `variable_names: Option<Vec<String>>`, `output: Option<OutputConfig>`), `FilterConfig`, `BatchConfig`, `BatchResult`, `CompressionCodec`, `OutputConfig`
- `src/output.rs` — all three write functions accept `Option<&OutputConfig>`; `build_parquet_writer` is the single configuration application point
- `src/filters.rs` — unchanged since epic-02; `filter_factory` is `pub(crate)`
- `src/errors.rs` — `Nc2ParquetError` (11 variants, thiserror); unchanged since epic-02
- `src/handlers/` — 8 binary-only files; unchanged structurally but `convert.rs` updated for `--glob`, `--variables`, `--compression`, `--no_statistics`, `--row-group-size`
- `src/tests/` — 11 test files: original 9 plus `test_batch` and `test_multi_variable`
- `benches/` — 4 Criterion benchmark files (unchanged since epic-03)

## Test Counts

- Total lib tests: 296 (up from 181 after epic-03)
- Doc tests: 28 (decreased from 39; investigate before epic-05 tutorial work)
- DHAT profiling test: 1 (gated on `dhat-heap` feature)

## Key Patterns

- **Flat combination buffer**: `CombinationBuffer` in `src/extract.rs` (lines 6-100); packs dimension index combinations as `Vec<usize>` with `stride` indexing; use `for combo in &buffer` via `IntoIterator`; avoids per-row heap allocation
- **Bounding-box slab read**: `extract_data_batch` (line ~450) calls `var.get_values::<f32, _>(extents)` once per variable per call; use `netcdf::Extents::try_from((starts.as_slice(), counts.as_slice()))` for multi-dimensional extents
- **Cartesian vs explicit dispatch**: `is_cartesian_product()` returns true when `explicit_combinations.is_none()`; dispatch in `extract_data_with_dimension_manager`
- **Eager drop via inner scope**: `let result = { let temp = ...; result };` frees `coordinate_vars` and `slab` before `build_dataframe` allocates Polars columns
- **Scoped NetCDF file lifetime**: `process_netcdf_job` uses a block to scope `file`; for multi-variable, each `&netcdf::Variable` borrow lives in its own `{ }` block because the borrow checker forbids keeping a variable reference alive across a `DimensionIndexManager::new` call on the same file
- **Unit family lookup table**: `unit_to_base_factor` in `src/postprocess.rs` (line 529) maps lowercased unit aliases to `(UnitFamily, to_base_factor)`; `calculate_conversion_factor` derives any pair as `from_base / to_base`; temperature excluded (requires offset)
- **Depth-counting function call parser**: `parse_factor` in `src/postprocess.rs` (line 1035) detects `identifier(` and uses depth counting to find the matching `)`, enabling arbitrarily nested function calls in formula strings
- **Additive output config threading**: `build_parquet_writer` in `src/output.rs` (line 67) accepts `Option<&OutputConfig>`; `None` produces a bare default writer; all write functions delegate here
- **Dead_code + test-only items**: `pub(crate)` items used only in `#[cfg(test)]` carry `#[allow(dead_code)] // Used in #[cfg(test)] modules`
- **Binary-library split**: `handlers/` declared in `main.rs` not `lib.rs`; handlers import with `use nc2parquet::...`; all CLI-specific deps (indicatif, anyhow) stay out of lib
- **Error enum with boxed SDK variant**: `StorageError` is `Box<StorageError>` inside `Nc2ParquetError`; requires manual `From<StorageError>` impl because `#[from]` does not support boxed variants
- **Processor lazy batching**: `ProcessingPipeline::execute` batches consecutive `UnitConverter` processors with disjoint `target_columns()` into a single `.with_columns(...).collect()` call

## Fixture File Facts (important for all epics)

- `simple_xy.nc`: 2D (x=6, y=12), variable "data", NO coordinate variables (index values default to integer position)
- `pres_temp_4D.nc`: 4D, time(2), level(2), latitude(6: 25-50), longitude(12: -125 to -70), variables: temperature, pressure; has "time" dimension but NO time coordinate variable; both variables share identical dimensions — primary fixture for multi-variable tests
- `NC3DPointFilter` must not be used with `pres_temp_4D.nc` (no time coordinate variable causes an error)

## API Extension Points for Epic-05 / Epic-06

- `ProcessorConfig` enum in `src/postprocess.rs` (line 130): `#[serde(tag = "type", rename_all = "snake_case")]`; new variants are additive and backward-compatible
- `CompressionCodec` in `src/input.rs` (line 22): `#[serde(rename_all = "lowercase")]`; adding a new codec requires a new enum variant, a match arm in `to_polars_compression`, and a CLI string case in `handlers/convert.rs`
- `filter_factory` in `src/filters.rs` (line ~550): `pub(crate)`, match on "kind" string; new filter types added here
- `unit_to_base_factor` in `src/postprocess.rs` (line 529): private match; adding a new unit requires one match arm specifying family and base factor
- `parse_function_call` in `src/postprocess.rs` (line 1098): match on `func_name.as_str()`; adding a new function requires one match arm calling the appropriate Polars `Expr` method

## Known Limitations and Technical Debt

- Polars 0.51.0: `AggregationOp::Mean` panics in release builds via streaming engine; `AggregationOp::Sum` works; documented in `benches/postprocess_bench.rs`
- `FormulaApplier` does not support unary minus prefix (e.g., `-temperature`); the `parse_expression` recursive descent treats leading `-` as binary subtraction, which fails when there is no left operand; see `src/postprocess.rs` line 978
- `FormulaApplier` and `DateTimeConverter` do not implement `to_lazy_expr()`; they materialise a DataFrame per call; batching for these types is a future optimization
- Multi-variable extraction opens the first `netcdf::Variable` three times (for `DimensionIndexManager`, for dimension metadata, for values) due to borrow checker constraints; this could be reduced to two opens if `DimensionIndexManager::new` were refactored to not hold a reference to the variable after construction; see `src/extract.rs` lines 395-428
- Doc test count decreased from 39 to 28 after epic-04; root cause not yet determined; must be investigated before epic-05 tutorial examples are added to docs
- Criterion baseline files are gitignored (`target/criterion/`); CI benchmark regression (ticket-030) needs artifact storage strategy
- `BatchConfig` does not support multi-variable extraction (it uses `variable_name: String` not `variable_names`); adding multi-variable batch support would require threading `effective_variable_names` through `process_netcdf_batch`'s per-file `JobConfig` construction

## Benchmarking Conventions

- Use `b.iter_batched(setup, bench, BatchSize::SmallInput)` for benchmarks with mutable state or I/O; use `b.iter()` for pure function benchmarks with pre-created state
- Keep `TempDir` alive alongside `config` in a tuple `(_dir, config)` passed to the benchmark body
- Fixture paths: `PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples/data").join(name)`
- Postprocess benchmarks use `BenchmarkId::from_parameter(size)` to label 1K/10K/100K row variants

## Dependencies Added in Epic-04

- `glob = "0.3"` in `[dependencies]` section of `Cargo.toml`; used only in `process_netcdf_batch` in `src/lib.rs`

## Serde Conventions for New Fields on JobConfig

- New optional fields use `#[serde(skip_serializing_if = "Option::is_none", default)]`; the `default` attribute ensures that existing serialized `JobConfig` JSON files without the field deserialize correctly (field becomes `None`)
- `CompressionCodec` uses `#[serde(rename_all = "lowercase")]` so JSON values are `"snappy"`, `"zstd"`, etc.
- `BatchResult` does not derive `Serialize`/`Deserialize` because it holds `Nc2ParquetError` which is not serializable
