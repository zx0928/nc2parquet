# ticket-018 Profile and Optimize Peak Memory Usage

## Context

### Background

Tickets 015-017 implement targeted optimizations to the extraction and postprocessing pipelines. This final ticket in Epic 03 profiles the full conversion pipeline end-to-end to identify remaining memory hotspots, applies targeted optimizations, and documents the peak memory characteristics for representative workloads.

The key memory consumers in the current pipeline are:

1. **Coordinate combinations**: `get_all_coordinate_combinations()` in `/home/rogerio/git/nc2parquet/src/extract.rs` holds all combinations in memory simultaneously. Ticket-016's `CombinationBuffer` reduces per-combination overhead, but the total buffer size still scales linearly with output rows.
2. **Intermediate DataFrame**: `extract_data_with_dimension_manager` builds column vectors and then constructs a DataFrame. The column vectors and the DataFrame coexist briefly during `DataFrame::new(columns)`.
3. **Parquet writing**: `/home/rogerio/git/nc2parquet/src/output.rs` clones the DataFrame at line 22 (`let mut df_clone = df.clone()`) before passing to `ParquetWriter::finish`. The async path also serializes to a `Vec<u8>` buffer before uploading.
4. **Postprocessing**: Each `processor.process(df)` takes ownership and returns a new DataFrame. Polars lazy evaluation may or may not reuse memory.

### Relation to Epic

This is the capstone ticket for Epic 03. It applies learnings from tickets 014-017 to address any remaining memory inefficiencies. The profiling results also inform Epic 06's CI memory budget if one is added.

### Current State

After tickets 015-017:

- Extraction for Cartesian products uses batch slab reads (ticket-015)
- `CombinationBuffer` uses a flat `Vec<usize>` instead of `Vec<Vec<usize>>` (ticket-016)
- Independent postprocessors may be batched into single `.collect()` calls (ticket-017)
- `write_dataframe_to_parquet` at line 7 of `/home/rogerio/git/nc2parquet/src/output.rs` clones the DataFrame (`df.clone()`) because `ParquetWriter::finish` requires `&mut DataFrame` but receives `&DataFrame`
- `dataframe_to_parquet_bytes` at line 47 also clones the DataFrame and buffers the entire parquet content in a `Vec<u8>` before writing
- `process_netcdf_job` at line 67 of `/home/rogerio/git/nc2parquet/src/lib.rs` holds the `netcdf::File` open until the very end (line 88: `file.close()?`) even after extraction is complete -- this keeps the memory-mapped file handle alive during parquet writing
- No DHAT or heaptrack profiling infrastructure exists in the project

## Specification

### Requirements

1. **Eliminate DataFrame clone in parquet writing**: Change `write_dataframe_to_parquet` to accept `&mut DataFrame` instead of `&DataFrame`, removing the clone. Update all call sites in `lib.rs` to pass `&mut df`.
2. **Eliminate DataFrame clone in async parquet writing**: Change `dataframe_to_parquet_bytes` and `write_dataframe_to_parquet_async` to accept `&mut DataFrame`.
3. **Drop NetCDF file before writing**: In `process_netcdf_job`, close/drop the `netcdf::File` immediately after extraction is complete (before postprocessing and parquet writing) to release the underlying file descriptor and any associated buffers.
4. **Drop intermediate data eagerly**: In `extract_data_with_dimension_manager` (or its refactored equivalent after ticket-015/016), drop the `CombinationBuffer`/combination data and `coordinate_vars` HashMap as soon as the column vectors are fully populated, before constructing the DataFrame.
5. **Add DHAT integration test**: Create a `#[cfg(test)]` function that runs the full pipeline on `pres_temp_4D.nc` under DHAT (via `dhat` crate) and asserts on total bytes allocated. This is not a CI-blocking test but a developer profiling helper.
6. **Document peak memory profile**: Add a `PERFORMANCE.md` file in the repository root documenting:
   - Peak memory for `pres_temp_4D.nc` extraction (baseline and optimized)
   - Peak memory scaling characteristics (how memory grows with input size)
   - Profiling methodology (how to reproduce measurements)

### Inputs/Props

- Same as `process_netcdf_job` -- no public API changes
- DHAT profiling is invoked via `cargo test --features dhat-heap` (behind a feature flag)

### Outputs/Behavior

- Reduced peak memory usage for the full pipeline
- `write_dataframe_to_parquet` no longer allocates a DataFrame clone
- NetCDF file resources are released earlier in the pipeline
- DHAT test provides measurable allocation data for regression tracking

### Error Handling

- No changes to error types or error handling paths
- The `file.close()` call currently returns `Result` -- handle the error before moving to postprocessing, or use `drop(file)` which discards the close error (acceptable since the file was opened read-only)

## Acceptance Criteria

- [ ] Given `write_dataframe_to_parquet` accepts `&mut DataFrame`, when called, then no DataFrame clone occurs during parquet writing (verified by DHAT profiling or code inspection)
- [ ] Given `write_dataframe_to_parquet_async` accepts `&mut DataFrame`, when called, then no DataFrame clone occurs during async parquet serialization (only the bytes buffer is allocated)
- [ ] Given `process_netcdf_job` is called with `pres_temp_4D.nc`, when the pipeline reaches the parquet writing stage, then the `netcdf::File` has already been closed/dropped
- [ ] Given `extract_data_with_dimension_manager` has completed column population, when the DataFrame is constructed, then the combination buffer and coordinate variable HashMap have been dropped (verified by scoping or explicit `drop()`)
- [ ] Given the DHAT test is run via `cargo test dhat_profile --features dhat-heap`, when it completes, then it reports total bytes allocated for the pres_temp_4D.nc pipeline
- [ ] Given `PERFORMANCE.md` exists in the repository root, when read, then it documents peak memory for the test fixture, scaling characteristics, and profiling methodology
- [ ] Given all 181 existing tests pass after the change, then the optimization is behavior-preserving
- [ ] Given `cargo bench` is run, then all benchmarks still compile and pass (the `&mut DataFrame` change does not break benchmark code)

## Implementation Guide

### Suggested Approach

1. **Change parquet write signatures**:
   In `/home/rogerio/git/nc2parquet/src/output.rs`:

   ```rust
   pub(crate) fn write_dataframe_to_parquet(
       df: &mut DataFrame,  // was &DataFrame
       output_path: &str,
   ) -> Result<(), Nc2ParquetError> {
       // ... same code but remove: let mut df_clone = df.clone();
       // Pass df directly to writer.finish(df)
   }
   ```

   Similarly for `write_dataframe_to_parquet_async` and `dataframe_to_parquet_bytes`.

2. **Update call sites in lib.rs**:
   In `/home/rogerio/git/nc2parquet/src/lib.rs`:

   ```rust
   // process_netcdf_job (line 67)
   pub fn process_netcdf_job(config: &JobConfig) -> Result<(), Nc2ParquetError> {
       let file = netcdf::open(&config.nc_key)?;
       let var = file.variable(&config.variable_name)...;
       let mut filters = Vec::new();
       // ... build filters ...
       let mut df = extract_data_to_dataframe(&file, &var, &config.variable_name, &filters)?;

       // Drop file early -- extraction is done, release resources
       drop(file);

       if let Some(ref postprocess_config) = config.postprocessing {
           let mut pipeline = ProcessingPipeline::from_config(postprocess_config)?;
           df = pipeline.execute(df)?;
       }

       write_dataframe_to_parquet(&mut df, &config.parquet_key)?;
       Ok(())
   }
   ```

   Note: `file.close()?` at line 88 is replaced by `drop(file)` because the `var` borrow prevents calling `close()` before extraction. The variable `var` borrows from `file`, so `file` cannot be moved/closed while `var` is alive. Solution: scope the `file` and `var` usage:

   ```rust
   let mut df = {
       let file = netcdf::open(&config.nc_key)?;
       let var = file.variable(&config.variable_name)
           .ok_or_else(|| ...)?;
       let mut filters = Vec::new();
       for filter_config in &config.filters {
           let filter = filter_config.to_filter()?;
           filters.push(filter);
       }
       let df = extract_data_to_dataframe(&file, &var, &config.variable_name, &filters)?;
       file.close()?;
       df
   };
   // file and var are dropped here
   ```

   Apply the same pattern to `process_netcdf_job_async`.

3. **Eager drops in extraction**:
   In `extract_data_with_dimension_manager` (or the refactored cellwise/batch paths), use explicit scoping:

   ```rust
   let (data_columns, variable_values) = {
       let coordinate_vars = get_coordinate_variables(file, dimension_order)?;
       let combinations = dim_manager.get_all_coordinate_combinations();
       let num_rows = combinations.len();
       let mut data_columns: HashMap<String, Vec<f64>> = HashMap::new();
       let mut variable_values = Vec::with_capacity(num_rows);
       // ... populate ...
       // coordinate_vars and combinations are dropped at end of this block
       (data_columns, variable_values)
   };
   // Build DataFrame from data_columns and variable_values
   ```

4. **Add DHAT feature flag and profiling test**:
   In `Cargo.toml`:

   ```toml
   [features]
   dhat-heap = ["dhat"]

   [dev-dependencies]
   dhat = { version = "0.3", optional = true }
   ```

   Wait -- `dhat` is a regular dependency when the feature is active, not a dev-dependency, because it needs to be linked into the test binary. Actually, since it is only used in `#[cfg(test)]` code, it should be a dev-dependency with a feature gate. The standard pattern is:

   ```toml
   [features]
   dhat-heap = ["dep:dhat"]

   [dependencies]
   dhat = { version = "0.3", optional = true }
   ```

   Then create a profiling test in `/home/rogerio/git/nc2parquet/src/tests/test_memory_profile.rs`:

   ```rust
   #[cfg(feature = "dhat-heap")]
   #[global_allocator]
   static ALLOC: dhat::Alloc = dhat::Alloc;

   #[cfg(feature = "dhat-heap")]
   #[test]
   fn dhat_profile_pres_temp_4d() {
       let _profiler = dhat::Profiler::builder().testing().build();
       // Run full pipeline on pres_temp_4D.nc
       let config = JobConfig::from_json(...).unwrap();
       process_netcdf_job(&config).unwrap();
       let stats = dhat::HeapStats::get();
       // Print stats for manual inspection
       eprintln!("Total bytes allocated: {}", stats.total_bytes);
       eprintln!("Peak bytes: {}", stats.max_bytes);
       // Optionally assert a ceiling (loose, to avoid flaky tests)
   }
   ```

5. **Create `PERFORMANCE.md`** in the repository root with profiling results and methodology.

### Key Files to Modify

- `/home/rogerio/git/nc2parquet/src/output.rs` -- change `&DataFrame` to `&mut DataFrame`, remove clones
- `/home/rogerio/git/nc2parquet/src/lib.rs` -- scope file/var lifetime, drop file early, pass `&mut df` to write functions
- `/home/rogerio/git/nc2parquet/src/extract.rs` -- add explicit scoping for eager drops of combinations and coordinate vars
- `/home/rogerio/git/nc2parquet/Cargo.toml` -- add `dhat` optional dependency and `dhat-heap` feature
- `/home/rogerio/git/nc2parquet/src/tests/mod.rs` -- add `test_memory_profile` module declaration (behind `#[cfg(feature = "dhat-heap")]`)
- `/home/rogerio/git/nc2parquet/src/tests/test_memory_profile.rs` -- CREATE: DHAT profiling test
- `/home/rogerio/git/nc2parquet/PERFORMANCE.md` -- CREATE: performance documentation

### Patterns to Follow

- Follow the established test module pattern: declare in `src/tests/mod.rs`, implement in `src/tests/test_memory_profile.rs`
- Use explicit block scoping (`{ let x = ...; ... }`) rather than `drop(x)` calls for clarity -- scoping communicates intent better
- Follow the `rust,no_run` rustdoc pattern for PERFORMANCE.md code examples (they reference profiling tools that may not be available)
- The `dhat-heap` feature flag pattern is standard in the Rust ecosystem for optional profiling instrumentation

### Pitfalls to Avoid

- `netcdf::File` borrows: `var` borrows from `file`, so you cannot call `file.close()` while `var` is alive. Use a scoped block to limit `var`'s lifetime as shown in the implementation guide.
- The `#[global_allocator]` attribute for DHAT must be at crate root or in a `#[cfg(test)]` block. Since test code uses `src/tests/`, and the `#[global_allocator]` is in a test file, it only applies when that specific test file is compiled. However, `#[global_allocator]` must be at the crate level -- it might need to go in `lib.rs` behind `#[cfg(all(test, feature = "dhat-heap"))]`. Research the exact placement.
- `ParquetWriter::finish` takes `&mut DataFrame` -- this is why the current code clones. After changing the write function signature to `&mut DataFrame`, the caller must have a mutable reference. Since `process_netcdf_job` already owns `df` mutably, this is straightforward.
- The async path in `dataframe_to_parquet_bytes` also clones -- make sure to fix both sync and async paths
- Do NOT add `dhat` as a non-optional dependency -- it must be behind the feature flag to avoid impacting normal build times and binary size
- The `PERFORMANCE.md` file should document the actual measured values, not theoretical estimates. Run the profiling before writing the document.

## Testing Requirements

### Unit Tests

- Test that `write_dataframe_to_parquet` with `&mut DataFrame` produces a valid parquet file (update existing output tests in `src/tests/test_output.rs` to pass `&mut df`)
- Test that the scoped-file pattern in `process_netcdf_job` works correctly (existing integration tests cover this)

### Integration Tests

- The existing 181 tests serve as regression tests
- The DHAT profiling test is a new integration test that runs only with `--features dhat-heap`

### E2E Tests

Not applicable.

## Dependencies

- **Blocked By**: ticket-015 (chunked reading), ticket-016 (allocation reduction), ticket-017 (postprocessor batching)
- **Blocks**: None

## Effort Estimate

**Points**: 3
**Confidence**: Medium
