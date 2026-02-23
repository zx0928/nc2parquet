# ticket-014 Add Criterion Benchmark Suite

## Context

### Background

The nc2parquet codebase has been stabilized by Epic 01 (181 unit/integration/property tests) and cleaned by Epic 02 (handlers extraction, thiserror migration, visibility audit, rustdoc). Before optimizing performance in tickets 015-018, the project needs a criterion benchmark suite to establish quantitative baselines and detect regressions.

The Epic 02 learnings specifically identify `extract_data_to_dataframe` as the hot path and `generate_combinations` (line 220 of `/home/rogerio/git/nc2parquet/src/extract.rs`) as the primary allocation site -- the recursive function that builds the Cartesian product of filtered dimension indices. Benchmarks must target this function at multiple scales, not just top-level pipeline functions.

### Relation to Epic

This is the foundational ticket for Epic 03 (Performance Optimization). Every subsequent optimization ticket (015-018) depends on these benchmarks to measure improvements and guard against regressions. The benchmark suite also feeds into Epic 06's ticket-030 (benchmark regression CI).

### Current State

- No `benches/` directory exists in the repository
- `criterion` is not listed in `Cargo.toml` dev-dependencies
- No `[[bench]]` sections exist in `Cargo.toml`
- Two NetCDF fixture files are available: `examples/data/simple_xy.nc` (2D, x=6 y=12) and `examples/data/pres_temp_4D.nc` (4D, time=2 level=2 lat=6 lon=12)
- `DimensionIndexManager` and `extract_data_to_dataframe` are `pub(crate)` (not accessible from benches directly)
- `ProcessingPipeline`, `PostProcessor`, and all processor constructors are `pub` in the `postprocess` module
- `process_netcdf_job` is `pub` in `lib.rs`

## Specification

### Requirements

1. Add `criterion` as a dev-dependency with `html_reports` feature
2. Create four benchmark files under `benches/`:
   - `extraction_bench.rs` -- benchmarks for the extraction pipeline via `process_netcdf_job` at different dimension sizes
   - `filter_bench.rs` -- benchmarks for filter application (all 4 filter types) on the fixture NetCDF files
   - `postprocess_bench.rs` -- benchmarks for all 5 processor types (ColumnRenamer, DateTimeConverter, UnitConverter, Aggregator, FormulaApplier) on synthetic DataFrames of varying sizes
   - `combination_bench.rs` -- benchmarks for `generate_combinations` via the public pipeline entrypoint, measuring Cartesian product generation at varying dimension counts and sizes
3. Add corresponding `[[bench]]` entries in `Cargo.toml` with `harness = false`
4. Use **real NetCDF fixture files** (`simple_xy.nc`, `pres_temp_4D.nc`) for extraction and filter benchmarks -- these are small enough for fast iteration while still exercising the real I/O path
5. Use **synthetic Polars DataFrames** for postprocess benchmarks -- create DataFrames of 1K, 10K, and 100K rows to measure scaling behavior without NetCDF I/O noise
6. Only benchmark sync paths (not async) -- the async pipeline delegates to the same sync extraction and writing functions

### Inputs/Props

- Fixture NetCDF files at `examples/data/simple_xy.nc` and `examples/data/pres_temp_4D.nc`
- Synthetic DataFrames constructed inline in benchmark functions
- Benchmark groups organized by module and input size

### Outputs/Behavior

- `cargo bench` runs all 4 benchmark files and produces criterion HTML reports in `target/criterion/`
- Each benchmark group should complete in under 30 seconds total (suitable for developer iteration)
- Baseline results are stored by criterion in `target/criterion/` for regression detection

### Error Handling

- Benchmark setup panics are acceptable (they indicate broken fixtures, not runtime errors)
- Use `.expect("msg")` in benchmark setup code for clarity

## Acceptance Criteria

- [ ] Given the repository has `criterion = { version = "0.5", features = ["html_reports"] }` in `[dev-dependencies]` of Cargo.toml, when `cargo bench` is run, then all benchmarks compile and execute without error
- [ ] Given `benches/extraction_bench.rs` exists with `[[bench]] name = "extraction_bench" harness = false`, when `cargo bench --bench extraction_bench` is run, then it benchmarks `process_netcdf_job` with `simple_xy.nc` (2D) and `pres_temp_4D.nc` (4D) with varying filter configurations (no filter, range filter, point filter)
- [ ] Given `benches/filter_bench.rs` exists, when `cargo bench --bench filter_bench` is run, then it benchmarks `NCRangeFilter::apply`, `NCListFilter::apply`, `NC2DPointFilter::apply`, and `NC3DPointFilter::apply` on `pres_temp_4D.nc`
- [ ] Given `benches/postprocess_bench.rs` exists, when `cargo bench --bench postprocess_bench` is run, then it benchmarks all 5 processor types (ColumnRenamer, DateTimeConverter, UnitConverter, Aggregator, FormulaApplier) at 1K, 10K, and 100K row DataFrame sizes
- [ ] Given `benches/combination_bench.rs` exists, when `cargo bench --bench combination_bench` is run, then it benchmarks the extraction pipeline with varying filter configurations that exercise `generate_combinations` (e.g., no filter on pres_temp_4D.nc produces 2*2*6\*12=288 combinations; range filters reducing dimensions)
- [ ] Given all benchmarks pass, when `cargo test` is run, then all 181 existing tests still pass (no regressions from the new dev-dependency or benchmark infrastructure)

## Implementation Guide

### Suggested Approach

1. Add `criterion` dev-dependency and `[[bench]]` entries to `Cargo.toml`
2. Create `benches/extraction_bench.rs`:
   - Use `criterion_group!` and `criterion_main!` macros
   - Build `JobConfig` instances programmatically (using `JobConfig { nc_key, variable_name, parquet_key, filters, postprocessing }` struct construction) pointing to the fixture files
   - Use `tempfile::TempDir` for parquet output paths to avoid polluting the workspace
   - Benchmark group: "extraction" with functions for 2D-no-filter, 4D-no-filter, 4D-range-filter, 4D-point-filter
3. Create `benches/filter_bench.rs`:
   - Open `pres_temp_4D.nc` once in setup, pass the `netcdf::File` reference to each benchmark iteration
   - Benchmark group: "filters" with one function per filter type
   - `NCRangeFilter::new("latitude", 30.0, 45.0).apply(&file)` etc.
4. Create `benches/postprocess_bench.rs`:
   - Create synthetic DataFrames using `polars::prelude::df!` macro
   - Helper function: `make_df(n: usize) -> DataFrame` that creates `n` rows with columns: "temperature" (f64), "pressure" (f64), "time_offset" (f64), "station" (String)
   - Use `BenchmarkGroup::bench_with_input` with `BenchmarkId::new("name", size)` for parameterized benchmarks
   - Benchmark each processor's `.process(df)` method
5. Create `benches/combination_bench.rs`:
   - Similar to extraction_bench but focused on measuring the cost of combination generation at different filter selectivity levels
   - Benchmark configs: unfiltered 4D (288 combos), heavily filtered (2 combos), moderately filtered (~50 combos)

### Key Files to Modify

- `/home/rogerio/git/nc2parquet/Cargo.toml` -- add criterion dev-dependency and 4 `[[bench]]` entries
- `/home/rogerio/git/nc2parquet/benches/extraction_bench.rs` -- CREATE
- `/home/rogerio/git/nc2parquet/benches/filter_bench.rs` -- CREATE
- `/home/rogerio/git/nc2parquet/benches/postprocess_bench.rs` -- CREATE
- `/home/rogerio/git/nc2parquet/benches/combination_bench.rs` -- CREATE

### Patterns to Follow

- Use `criterion::Criterion` with default configuration (no custom sample sizes needed for initial baselines)
- Import library types with `use nc2parquet::...` (benchmarks are external crate consumers like integration tests)
- For filter benchmarks that need `NCFilter::apply`, the filter types are `pub` in `nc2parquet::filters`
- For extraction benchmarks, use `nc2parquet::process_netcdf_job` (the `pub` entry point) rather than trying to access `pub(crate)` internals
- For postprocess benchmarks, construct processors directly via their `pub` constructors and call `.process(df)`
- Follow the established test helper pattern: `PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples/data/filename.nc")` for fixture file paths

### Pitfalls to Avoid

- Do NOT try to benchmark `DimensionIndexManager::generate_combinations` directly -- it is `pub(crate)` and not accessible from benchmarks. Instead, benchmark `process_netcdf_job` with different filter configurations that exercise different combination sizes.
- Do NOT use `criterion::black_box` on the `netcdf::File` open call -- only on the measured operation's return value
- Do NOT benchmark with output to a fixed path -- use `tempfile::TempDir` to avoid concurrent writes if benches run in parallel
- The `pres_temp_4D.nc` file has no time coordinate variable (only a time dimension) -- `NC3DPointFilter::apply()` will error on it. For 3D point filter benchmarks, use a `NC2DPointFilter` instead, or skip the 3D filter bench on this fixture and note it as a known limitation.
- `ProcessingPipeline::execute` takes `&mut self` -- create a fresh pipeline per iteration or use `iter_batched` with setup closure

## Testing Requirements

### Unit Tests

No new unit tests -- this ticket adds benchmarks only. Existing 181 tests must continue to pass.

### Integration Tests

No new integration tests.

### E2E Tests

Not applicable.

## Dependencies

- **Blocked By**: ticket-013 (clean codebase with zero clippy warnings -- completed)
- **Blocks**: ticket-015, ticket-016, ticket-017, ticket-018, ticket-030

## Effort Estimate

**Points**: 3
**Confidence**: High
