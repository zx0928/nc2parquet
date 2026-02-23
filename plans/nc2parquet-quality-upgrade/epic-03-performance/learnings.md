# Epic 03 Learnings: Performance Optimization

## What Was Implemented

- Added Criterion 0.5 benchmark suite with 4 files under `benches/`: `extraction_bench.rs`, `filter_bench.rs`, `postprocess_bench.rs`, `combination_bench.rs`
- Added `[[bench]]` entries in `Cargo.toml` for each file with `harness = false`
- Rewrote `extract_data_with_dimension_manager` in `src/extract.rs` to dispatch between a new `extract_data_batch` path (Cartesian product) and the existing `extract_data_cellwise` path (explicit Pairs/Triplets)
- `extract_data_batch` reads the entire bounding box with one `var.get_values::<f32, _>(extents)` call using `netcdf::Extents::try_from((starts, counts))`, then iterates over selected indices in row-major order using manually computed strides
- Introduced `CombinationBuffer` struct in `src/extract.rs`: a flat `Vec<usize>` with stride indexing that replaces `Vec<Vec<usize>>`, eliminating per-row heap allocation
- Added `generate_combinations_flat` as a replacement for the recursive `generate_combinations` — uses a single shared mutable buffer and `push_combination` calls instead of `current.clone()` at every leaf
- Added `is_cartesian_product()` and `sorted_dimension_indices()` methods to `DimensionIndexManager`
- Added `target_columns()` and `to_lazy_expr()` methods to the `PostProcessor` trait with default implementations (backward-compatible)
- Implemented `to_lazy_expr()` on `UnitConverter` only; `ColumnRenamer`, `DateTimeConverter`, `Aggregator`, `FormulaApplier` return `None`
- Updated `ProcessingPipeline::execute` to batch consecutive independent processors that return `Some(exprs)` from `to_lazy_expr` into a single `df.lazy().with_columns(batch_exprs).collect()` call
- Changed `write_dataframe_to_parquet`, `write_dataframe_to_parquet_async`, and `dataframe_to_parquet_bytes` in `src/output.rs` to accept `&mut DataFrame` instead of `&DataFrame`, eliminating one full column clone at write time
- Scoped `netcdf::File` and `netcdf::Variable` lifetimes in both `process_netcdf_job` and `process_netcdf_job_async` (`src/lib.rs`) so the file is closed via `file.close()` before postprocessing and Parquet writing begin
- Added explicit inner scopes in `extract_data_cellwise` and `extract_data_batch` to drop `coordinate_vars` HashMap and the raw slab before constructing Polars `Series`
- Added `dhat-heap` feature flag in `Cargo.toml` with `dhat = { version = "0.3", optional = true }` and a `#[cfg(feature = "dhat-heap")]` `#[global_allocator]` in `src/lib.rs`
- Created `src/tests/test_memory_profile.rs` with a `dhat_profile` test gated on `feature = "dhat-heap"`
- Created `PERFORMANCE.md` at the repository root documenting measured peak memory (1.24 MB t-gmax for `pres_temp_4D.nc`), optimizations applied, and profiling methodology
- All 181 existing tests continue to pass; test count did not change for this epic

## Codebase Insights

### Batch Read Architecture

- The `extract_data_batch` path in `/home/rogerio/git/nc2parquet/src/extract.rs` (line 450) uses bounding-box reads: for each dimension it computes `starts[d]` (first selected index) and `counts[d]` (last - first + 1), then reads the full rectangular slab in one NetCDF call
- Non-contiguous indices inside the bounding box are handled by pre-computing `local_offsets[d]` (offsets relative to `starts[d]` for each selected index) and filtering during iteration rather than issuing multiple slab reads
- Row-major flat index into the slab is `sum(local_offsets[d][pos[d]] * strides[d])` where strides are computed as the product of counts for higher dimensions
- This is a conscious trade-off: a slightly enlarged I/O footprint for sparse selections, in exchange for a guaranteed one-call-per-extraction read pattern regardless of filter configuration
- The iteration loop at line 515 uses a manual carry-propagation odometer pattern (`pos` vector with right-to-left increment) instead of the recursive Cartesian product function — this avoids function-call overhead and keeps the loop branch-predictor-friendly

### CombinationBuffer Layout

- `CombinationBuffer` in `/home/rogerio/git/nc2parquet/src/extract.rs` (lines 6-100) packs all dimension index combinations into a single `Vec<usize>` with a fixed `stride` (number of dimensions)
- Combination `i` occupies `data[i*stride..(i+1)*stride]`; retrieved as `&[usize]` slices with zero allocation
- `IntoIterator` is implemented for `&CombinationBuffer` (line 67) via `data.chunks_exact(stride)`, enabling `for combo in &buffer` syntax in both production code and test code
- The separate `iter()` method (line 61) is kept as a named alternative but is marked `#[allow(dead_code)]` with a justification comment because `IntoIterator` is always used in practice
- `DimensionIndexManager.explicit_combinations` changed from `Option<Vec<Vec<usize>>>` to `Option<CombinationBuffer>` (line 81), unifying the pairs/triplets and Cartesian-product storage types

### NetCDF API: get_values vs get

- The ticket originally specified using `var.get::<f32, _>(range_tuple)` for slab reads returning `ArrayD<f32>`
- The actual implementation uses `var.get_values::<f32, _>(extents)` (line 495), which returns a flat `Vec<f32>` in C (row-major) order directly — this eliminates the need to call `.iter()` on an `ndarray::ArrayD` and avoids the ndarray dependency for this path
- `netcdf::Extents::try_from((starts.as_slice(), counts.as_slice()))` is the correct constructor for multi-dimensional range reads in the netcdf 0.11 crate
- The cell-by-cell path still uses `var.get::<f32, _>((i0, i1, ...))` with tuple index dispatch matching on `indices.len()` (1/2/3/4 dimensions)

### PostProcessor Batching Constraint

- Only `UnitConverter` implements `to_lazy_expr` returning `Some`; the other four processor types return `None`
- `ColumnRenamer` cannot be expressed as a `with_columns` expression because `DataFrame::rename` is a schema-level mutation, not a column expression — it always runs via the sequential `process()` path
- `Aggregator` changes row count (group-by), making it incompatible with `with_columns`
- `FormulaApplier` and `DateTimeConverter` were left as `None` in this epic; both are candidates for lazy expression support in future epics if benchmarks show they are bottlenecks
- The batching check in `ProcessingPipeline::execute` (line 313) uses `HashSet::is_disjoint` on `target_columns()` results — two processors are only batched when their column sets do not overlap at all, which is the conservative-correct approach

### Polars Streaming Engine Pitfall

- `AggregationOp::Mean` on large DataFrames triggers the Polars 0.51.0 streaming engine, which has an `unimplemented!()` branch in release builds
- The postprocess benchmark in `/home/rogerio/git/nc2parquet/benches/postprocess_bench.rs` (line 26) documents this with a comment and uses `AggregationOp::Sum` instead for the aggregate benchmark
- This is a Polars 0.51-specific limitation, not a bug in nc2parquet; upgrading Polars may resolve it
- The same issue applies to production use: any pipeline config using `aggregate: mean` on large inputs would panic in release builds; this is a known limitation that should be documented in ticket-022 (Parquet output configuration) or a future fix

### NetCDF Variable Borrow Pins File

- The `netcdf::Variable` borrows from `netcdf::File` via a lifetime — `file.close()` cannot be called while `var` is in scope
- The scoped block pattern in `process_netcdf_job` (`src/lib.rs` lines 36-52) is: open file, get variable reference, build filters, extract, explicitly `drop(var)`, then `file.close()?`, then return `df` from the block
- The `drop(var)` call is required before `file.close()` to satisfy the borrow checker — without it the compiler rejects the code even inside a scoped block
- The same pattern is applied to `process_netcdf_job_async` (lines 71-99) with the additional complication that `temp_file_path` must be declared outside the scoped block so it remains accessible for cleanup after the file is closed

### DHAT Feature Flag Placement

- The `#[global_allocator]` attribute must appear at crate root level — it cannot be placed inside a test submodule
- The placement in `src/lib.rs` (lines 19-21) under `#[cfg(all(test, feature = "dhat-heap"))]` is the only viable location; placing it in `src/tests/test_memory_profile.rs` would fail because Rust requires global allocator declarations at crate root
- The `dhat` crate must be a regular (non-dev) optional dependency for the `#[global_allocator]` to compile correctly when `--features dhat-heap` is passed to `cargo test`

## Architectural Decisions

- **Bounding-box slab read instead of per-run-group reads**: The ticket specified grouping non-contiguous indices into contiguous runs and issuing one read per run. The implementation instead reads the full bounding box (min to max selected index) in one call and filters during iteration. Rejected the run-grouping approach because: (a) for the typical case where range filters produce contiguous selections the bounding box is identical to the contiguous run; (b) a single `get_values` call is simpler than managing a variable number of slab reads; (c) the wasted I/O for sparse selections is bounded by the bounding box size minus the selected count, which is small for realistic filter patterns. See `/home/rogerio/git/nc2parquet/src/extract.rs` lines 467-484.

- **`get_values` returning flat `Vec<f32>` over `get` returning `ArrayD<f32>`**: The ticket guide suggested using `var.get::<f32, _>(range_tuple)` which returns `ArrayD<f32>`. The implementation uses `var.get_values::<f32, _>(extents)` returning a flat `Vec<f32>`. This avoids pulling in the ndarray iteration API, keeps the slab as a plain slice, and allows direct index arithmetic with pre-computed strides. See `/home/rogerio/git/nc2parquet/src/extract.rs` line 495.

- **`UnitConverter`-only lazy expression support**: The ticket asked for lazy expressions on both `UnitConverter` and `ColumnRenamer`. After analyzing the Polars API, `ColumnRenamer` was excluded because schema-level renames cannot be expressed as column expressions in Polars' lazy API — `df.rename()` is an eager, in-place mutation. Implementing lazy rename would require a completely different implementation (wrapping in `select([col("old").alias("new"), col("other"), ...])`), which would change column ordering and break existing tests. The conservative decision is to leave `ColumnRenamer` as sequential-only. See `/home/rogerio/git/nc2parquet/src/postprocess.rs` line 590 (`target_columns` is implemented) and line 108 (default `to_lazy_expr` returning `None` is used).

- **`drop(var)` before `file.close()` pattern**: The alternative was to use a single lifetime-limited block without explicitly dropping `var`. In practice, Rust's borrow checker requires explicit `drop(var)` because the compiler does not see through the intermediate `df` binding when `var` and `df` are both live at the end of the scoped block. Explicit `drop(var)` makes the intent clear and keeps the close error propagated via `?`. See `/home/rogerio/git/nc2parquet/src/lib.rs` lines 49-51.

## Files and Structures Created

- `/home/rogerio/git/nc2parquet/benches/extraction_bench.rs` — 4 extraction benchmarks via `process_netcdf_job`: 2D no-filter, 4D no-filter, 4D range filter, 4D 2D-point filter; all use `BatchSize::SmallInput` with `iter_batched` to isolate setup from measurement
- `/home/rogerio/git/nc2parquet/benches/filter_bench.rs` — 6 filter benchmarks on `pres_temp_4D.nc`: NCRangeFilter (latitude and longitude), NCListFilter (latitude and longitude), NC2DPointFilter (wide and tight tolerance)
- `/home/rogerio/git/nc2parquet/benches/postprocess_bench.rs` — 7 benchmark groups (UnitConverter, ColumnRenamer, DateTimeConverter, FormulaApplier, Aggregator, multi-step pipeline, batched-lazy pipeline) each at 1K/10K/100K rows using `BenchmarkId::from_parameter`
- `/home/rogerio/git/nc2parquet/benches/combination_bench.rs` — 4 benchmarks exercising the Cartesian product path at varying selectivity levels (288, 144, 16 combos from Cartesian product; 8 combos from 2D point filter)
- `/home/rogerio/git/nc2parquet/src/tests/test_memory_profile.rs` — DHAT profiling test gated on `dhat-heap` feature; runs full pipeline on `pres_temp_4D.nc` and asserts output exists
- `/home/rogerio/git/nc2parquet/PERFORMANCE.md` — documents measured peak memory (t-gmax 1.24 MB for 288-row fixture), the three optimizations applied, and the `cargo test --features dhat-heap` profiling invocation

## Conventions Adopted

- Benchmarks use `b.iter_batched(setup_fn, bench_fn, BatchSize::SmallInput)` for all benchmarks requiring file I/O or mutable state (pipeline, tempdir) — this keeps setup outside the measurement window. Pure function benchmarks (filter.apply with a pre-opened file) use `b.iter()` directly.
- Benchmark setup always uses `tempfile::TempDir::new().unwrap()` for output paths; the `TempDir` is kept alive in a tuple `(_dir, config)` passed to the benchmark body to prevent premature directory deletion.
- Fixture file paths in benchmarks use `PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples/data").join(name)`, the same pattern established in test helpers.
- The `NC3DPointFilter` is intentionally absent from filter and combination benchmarks on `pres_temp_4D.nc` because that fixture has no time coordinate variable; this is documented in the benchmark source files with inline comments to prevent future confusion.
- `build_dataframe` is factored out as a private function (line 559) shared by both `extract_data_batch` and `extract_data_cellwise`, avoiding code duplication in the DataFrame construction step.
- Inner scopes for eager drops follow the form `let (output1, output2) = { let temp = ...; ...; (output1, output2) };` rather than explicit `drop()` calls — this communicates the lifetime intent through structure rather than imperative drops.

## Surprises and Deviations

- **`get_values` instead of `get` for slab reads**: The ticket implementation guide specified `var.get::<f32, _>((t..t+1, level_range, lat_range, lon_range))` with an outer loop over time indices, returning per-slab `ArrayD<f32>`. The actual implementation calls `var.get_values::<f32, _>(extents)` once for the entire bounding box. This deviation is simpler, correct, and avoids multiple NetCDF calls at the cost of one slightly larger allocation for the full slab. See `/home/rogerio/git/nc2parquet/src/extract.rs` line 495.

- **ColumnRenamer excluded from lazy expression batching**: Ticket-017 stated "Implement for ColumnRenamer: return None because Polars rename is not an expression-level operation". The implementation follows this guidance, but the underlying reason differs slightly from what the ticket described: the issue is not just API availability but also column-ordering preservation — a `select`-based rename would change column order. `target_columns()` is still implemented for `ColumnRenamer` (line 590 of `/home/rogerio/git/nc2parquet/src/postprocess.rs`) so it participates in the disjoint-column check, but it never enters the batch path because `to_lazy_expr` returns `None`.

- **Polars `Mean` aggregation unimplemented in release builds**: Not mentioned in any ticket. The benchmark for `Aggregator` had to be changed from `Mean` to `Sum` after discovering a panic in Polars 0.51.0 release builds when using `mean()` via the streaming engine. The production postprocessing path is also affected: any user config specifying `aggregate: mean` will panic on large DataFrames. This is a known upstream Polars limitation and should be flagged in Epic-04 (features) and Epic-05 (documentation).

- **Test count unchanged**: All five tickets in this epic were pure optimization or infrastructure changes with no new user-visible behavior, so the 181-test count from Epic-02 was preserved exactly. No new unit or integration tests were written beyond the `dhat_profile` test (which is gated behind a feature flag and not counted in normal `cargo test` runs).

- **`DimensionIndexManager.explicit_combinations` field type changed**: The field changed from `Option<Vec<Vec<usize>>>` to `Option<CombinationBuffer>`. This is a `pub(crate)` internal change, but it silently broke any future code that directly accessed `explicit_combinations`. Since the field is private to the struct, the impact is contained. Callers use `get_all_coordinate_combinations()` which abstracts over both representations.

## Recommendations for Future Epics

- **Epic 04 (Features)**: Multi-variable extraction (ticket-021) will need to call `extract_data_to_dataframe` once per variable and join the resulting DataFrames on dimension columns. The current `extract_data_batch` path performs one slab read per call; for multi-variable extraction from the same file, the file should be opened once and shared across variable reads. The scoped-block pattern in `src/lib.rs` will need to be refactored to keep the file open while iterating variables.
- **Epic 04 (Features)**: `UnitConverter.build_conversion_expr()` (line 533 of `src/postprocess.rs`) is already shared between `process()` and `to_lazy_expr()`. Any new unit conversions added in ticket-019 must be added to `build_conversion_expr()` and the same expression will automatically be available for lazy batching.
- **Epic 04 (Features)**: `FormulaApplier` could implement `to_lazy_expr()` if the recursive descent parser were refactored to return an `Expr` instead of materializing via `with_columns([...]).collect()`. The parser already constructs Polars `Expr` objects internally (see `parse_expression`, `parse_term`, `parse_factor` in `src/postprocess.rs`); the main change would be surfacing the final `Expr` rather than wrapping it in a `collect()`. This would enable formula-based columns to participate in batching.
- **Epic 04 (Features)**: The batch extraction path has an edge case: if `dim_indices` contains an entry with an empty `Vec` (all indices filtered out), `extract_data_batch` returns an empty DataFrame early without a NetCDF read (line 459). Multi-variable extraction must handle this case consistently — an empty result for one variable should produce empty results for all variables in the combined output.
- **Epic 06 (CI)**: The DHAT peak memory measurement (`t-gmax` 1.24 MB for `pres_temp_4D.nc`) provides a baseline for a memory regression gate. Any future change that increases `t-gmax` by more than 20% for this fixture would indicate a regression. The `cargo test --features dhat-heap` command and the `dhat-heap.json` visualizer URL are documented in `PERFORMANCE.md`.
- **Epic 06 (CI)**: Criterion baseline files are stored in `target/criterion/` which is gitignored; the CI benchmark regression job (ticket-030) will need to store baseline results as CI artifacts between runs (e.g., using GitHub Actions cache) and compare with `--save-baseline` / `--baseline` flags.
- **General**: The Polars `Mean` aggregation streaming panic (Polars 0.51.0) affects production users who configure `aggregate: mean` on large DataFrames. Before releasing Epic-04 features, the Polars version should be evaluated for an upgrade, or a runtime guard should be added to `Aggregator::process()` that falls back to a non-streaming mean calculation.
