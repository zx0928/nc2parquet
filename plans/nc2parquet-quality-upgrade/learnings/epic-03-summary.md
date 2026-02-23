# Accumulated Learnings Through Epic 03

## Module Structure (current state after epic-03)

- `src/lib.rs` — crate root; exports `process_netcdf_job`, `process_netcdf_job_async`; `extract` and `output` modules are `pub(crate)`
- `src/extract.rs` — `DimensionIndexManager`, `CombinationBuffer`, `extract_data_to_dataframe` (pub(crate)), `extract_data_batch`, `extract_data_cellwise`, `build_dataframe`
- `src/postprocess.rs` — `PostProcessor` trait, `ProcessingPipeline`, 5 processor types (ColumnRenamer, DateTimeConverter, UnitConverter, Aggregator, FormulaApplier)
- `src/filters.rs` — 4 filter types (NCRangeFilter, NCListFilter, NC2DPointFilter, NC3DPointFilter), `filter_factory` (pub(crate))
- `src/errors.rs` — unified `Nc2ParquetError` (11 variants, thiserror), `StorageError` boxed inside enum
- `src/output.rs` — `write_dataframe_to_parquet`, `write_dataframe_to_parquet_async`, `dataframe_to_parquet_bytes` (all accept `&mut DataFrame`)
- `src/handlers/` — 8 binary-only files (mod.rs, convert.rs, validate.rs, info.rs, template.rs, completions.rs, config.rs, utils.rs); declared in `main.rs` not `lib.rs`
- `src/tests/` — 9 test files: test_cli, test_extract, test_filters, test_info, test_input, test_integration, test_output, test_postprocess, test_properties (+ test_memory_profile behind dhat-heap feature)
- `benches/` — 4 Criterion benchmark files: extraction_bench, filter_bench, postprocess_bench, combination_bench
- `PERFORMANCE.md` — peak memory measurements and DHAT profiling methodology

## Test Counts

- Total lib tests: 181 (unchanged across epic-03)
- Doc tests: 39 (established in epic-02, unchanged in epic-03)
- DHAT profiling test: 1 (gated on `dhat-heap` feature, not counted in normal runs)

## Key Patterns

- **Flat combination buffer**: `CombinationBuffer` in `src/extract.rs` (lines 6-100) packs all dimension index combinations as `Vec<usize>` with `stride` indexing; use `for combo in &buffer` via `IntoIterator`; avoids per-row heap allocation for Cartesian products
- **Bounding-box slab read**: `extract_data_batch` (line 450) computes `starts[d]` and `counts[d]` per dimension, calls `var.get_values::<f32, _>(extents)` once, then iterates selected indices using pre-computed strides and `local_offsets`; use `netcdf::Extents::try_from((starts.as_slice(), counts.as_slice()))` for multi-dimensional extents
- **Cartesian vs explicit dispatch**: `is_cartesian_product()` returns `true` when `explicit_combinations.is_none()`; dispatch happens in `extract_data_with_dimension_manager` (line 385); batch path for range/no-filter, cellwise path for Pairs/Triplets
- **Eager drop via inner scope**: `let (a, b) = { let temp = ...; ...; (a, b) };` frees `coordinate_vars` and `slab` before `build_dataframe` allocates Polars columns; used in both `extract_data_cellwise` and `extract_data_batch`
- **Scoped NetCDF file lifetime**: `process_netcdf_job` uses a block to scope `file` and `var`; requires explicit `drop(var)` before `file.close()` because the borrow checker does not automatically drop `var` before the end of the block when `df` is also live; pattern documented in `src/lib.rs` lines 36-52
- **Dead_code + test-only items**: `pub(crate)` items used only in `#[cfg(test)]` carry `#[allow(dead_code)] // Used in #[cfg(test)] modules`
- **Binary-library split**: handlers/ declared in `main.rs`, not `lib.rs`; handlers import with `use nc2parquet::...`; all CLI-specific deps (indicatif, anyhow) stay out of lib
- **Error enum with boxed SDK variant**: `StorageError` is `Box<StorageError>` inside `Nc2ParquetError`; requires manual `From<StorageError>` impl because `#[from]` does not support boxed variants
- **Processor lazy batching**: `ProcessingPipeline::execute` batches consecutive processors with disjoint `target_columns()` that return `Some(exprs)` from `to_lazy_expr()`; only `UnitConverter` currently implements `to_lazy_expr`; `ColumnRenamer` cannot be lazily expressed because schema rename is not a column expression in Polars

## Fixture File Facts (important for all epics)

- `simple_xy.nc`: 2D (x=6, y=12), variable "data", NO coordinate variables (index values default to integer position)
- `pres_temp_4D.nc`: 4D, time(2), level(2), latitude(6: 25-50°), longitude(12: -125 to -70°), variables: temperature, pressure; has "time" dimension but NO time coordinate variable — `NC3DPointFilter.apply()` returns error on this file
- `NC3DPointFilter` must not be used in benchmarks or tests that open `pres_temp_4D.nc`

## API Extension Points for Epic-04

- `filter_factory` in `src/filters.rs` (line 550): `pub(crate)`, match on "kind" string; new filter types added here
- `ProcessorConfig` enum in `src/postprocess.rs` (line 128): `#[serde(tag = "type", rename_all = "snake_case")]`; new variants are additive and backward-compatible
- `UnitConverter.build_conversion_expr()` (line 533): shared between `process()` and `to_lazy_expr()`; adding unit pairs here enables both execution paths automatically
- Multi-variable extraction must keep `netcdf::File` open across multiple variable reads; the current scoped-block pattern in `src/lib.rs` closes the file immediately after the first variable — this will need refactoring
- `extract_data_batch` returns an empty DataFrame early when any dimension has zero selected indices (line 459); multi-variable pipelines must handle this case consistently across all variables

## Known Limitations and Technical Debt

- Polars 0.51.0: `AggregationOp::Mean` on large DataFrames panics in release builds (streaming engine has an `unimplemented!()` branch); `AggregationOp::Sum` works via hash-based path; documented in `benches/postprocess_bench.rs` line 26; production pipelines using `aggregate: mean` are silently broken at scale
- `FormulaApplier` and `DateTimeConverter` do not implement `to_lazy_expr()`; both processors materialize a DataFrame per call; batching for these types is a future optimization
- Criterion baseline files are gitignored (`target/criterion/`); CI benchmark regression (ticket-030) needs artifact storage strategy
- DHAT `#[global_allocator]` must live at crate root in `src/lib.rs` behind `#[cfg(all(test, feature = "dhat-heap"))]`; cannot be placed inside test submodules
- The `sorted_dimension_indices()` method calls `.dedup()` after `.sort()` (line 341-343); the dedup is unnecessary since `DimensionIndexManager.dimension_indices` uses `HashSet<usize>` which already has no duplicates, but it is harmless

## Benchmarking Conventions

- Use `b.iter_batched(setup, bench, BatchSize::SmallInput)` for benchmarks with mutable state or I/O; use `b.iter()` for pure function benchmarks with pre-created state
- Keep `TempDir` alive alongside `config` in a tuple `(_dir, config)` passed to the benchmark body
- Fixture paths: `PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples/data").join(name)`
- Postprocess benchmarks use `BenchmarkId::from_parameter(size)` to label 1K/10K/100K row variants
