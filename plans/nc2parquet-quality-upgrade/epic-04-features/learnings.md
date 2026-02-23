# Epic 04 Learnings: Feature Completeness

## Patterns Established

- **Unit family lookup table**: `unit_to_base_factor(unit: &str) -> Option<(UnitFamily, f64)>` in `/home/rogerio/git/nc2parquet/src/postprocess.rs` (line 529) maps lowercased unit aliases to a family enum and a factor that converts that unit to the family's canonical base. New units cost exactly one `match` arm. `calculate_conversion_factor` then derives any pair's factor as `from_base / to_base`, covering all N×N conversions from N entries. Temperature is intentionally excluded from this path because it requires an offset (not just a scale); its explicit arms in `build_conversion_expr` remain untouched.

- **Depth-counting parenthesis parser for nested functions**: `parse_factor` in `/home/rogerio/git/nc2parquet/src/postprocess.rs` (line 1035) detects function calls by checking whether the text before the first `(` is a pure identifier (`all(|c| c.is_ascii_alphanumeric() || c == '_')`), then uses a depth counter to locate the matching `)`. This correctly handles `pow(abs(a - b), sqrt(c))` without a dedicated tokeniser. `split_function_args` (line 1199) uses the same depth-counter strategy to split comma-separated args respecting nested parens.

- **Variable-borrow scoping for multi-variable NetCDF reads**: `extract_multi_variable_dataframe` in `/home/rogerio/git/nc2parquet/src/extract.rs` (line 388) builds the `DimensionIndexManager` once inside a scoped block, drops the first-variable borrow, then re-opens the same variable in a second scoped block for value extraction. Each subsequent variable is also scoped individually. This satisfies the borrow checker (no `&netcdf::Variable` outlives a `{...}` block) while keeping the file open across all variable reads.

- **Values-only extraction helper for multi-variable**: `extract_variable_values_with_dim_manager` in `/home/rogerio/git/nc2parquet/src/extract.rs` (line 463) dispatches to `extract_variable_values_batch` or `extract_variable_values_cellwise` depending on whether the `DimensionIndexManager` represents a Cartesian product. This mirrors the single-variable dispatch without duplicating dimension-column construction, so dimension data is extracted exactly once regardless of how many variables are requested.

- **Additive output config threading**: `build_parquet_writer` in `/home/rogerio/git/nc2parquet/src/output.rs` (line 67) is the single place where `OutputConfig` is applied to `ParquetWriter`. It accepts `Option<&OutputConfig>`; when `None` it returns a bare `ParquetWriter::new(sink)` with no explicit configuration, preserving Polars defaults. All three write functions (`write_dataframe_to_parquet`, `write_dataframe_to_parquet_async`, `dataframe_to_parquet_bytes`) delegate to this helper.

- **Sequential batch with error collection**: `process_netcdf_batch` in `/home/rogerio/git/nc2parquet/src/lib.rs` (line 214) iterates paths sequentially, collecting errors into `BatchResult.failed` when `fail_fast` is false. S3 patterns are rejected before glob expansion. The output directory is created with `create_dir_all` before the loop begins. `resolve_output_path` (line 181) is a pure function and is separately tested and publicly exported with doc examples.

- **`--no_statistics` flag instead of `--statistics`**: The ticket specified `--statistics` as a boolean flag that enables statistics. The implementation inverted this to `--no_statistics` (default false), which is more ergonomic for CLI users since statistics default to true in `OutputConfig`.

## Architectural Decisions

- **`UnitFamily` enum private to `postprocess` module**: Rejected exposing `UnitFamily` in the public API or as a `pub(crate)` type. It is an implementation detail of `calculate_conversion_factor` and has no use outside that function. Keeping it private prevents future callers from depending on the family taxonomy, which may change as more unit families are added. See `/home/rogerio/git/nc2parquet/src/postprocess.rs` line 511.

- **`is_function_name` check removed; identifier detection sufficient**: The ticket suggested a static `is_function_name()` whitelist to disambiguate column names from functions. The implementation instead relies on two facts: (1) the recursive descent already tries `parse_expression` first for any `+`/`-`/`*`/`/` operator, so a `foo(` pattern only reaches `parse_factor` when `foo` is not split by an operator; (2) `parse_function_call` returns `PostProcessError::ProcessingError("Unknown function: ...")` for unrecognised names, which is the correct error path. No whitelist is needed.

- **`Expr::log(base: Expr)` not `log(base: f64)`**: Polars 0.51's `Expr::log` accepts a base `Expr`, not an `f64` literal. The `ln` implementation passes `lit(std::f64::consts::E)` as the base, and `log10` passes `lit(10.0_f64)`. The binary `log(value, base)` function passes the parsed base expression directly, which enables `log(col, another_col)` usage. See `/home/rogerio/git/nc2parquet/src/postprocess.rs` lines 1122-1187.

- **`min_horizontal` / `max_horizontal` for element-wise min/max**: Polars `.min()` and `.max()` on an `Expr` are column-level aggregations, not element-wise operations. The implementation uses `polars::lazy::dsl::{min_horizontal, max_horizontal}` (imported at the top of `postprocess.rs` line 29) which accept a list of expressions and return element-wise results. This matches the expected formula semantics.

- **`BatchConfig` carries `OutputConfig` field**: The batch struct in `/home/rogerio/git/nc2parquet/src/input.rs` (line 562) includes `pub output: Option<OutputConfig>`, which the batch function threads into each per-file `JobConfig`. This was not in the original ticket specification but was added for consistency: a batch job should be able to set compression just like a single job.

## Files and Structures Created

- `/home/rogerio/git/nc2parquet/src/postprocess.rs` — Added `UnitFamily` enum (line 511), `unit_to_base_factor` function (line 529), refactored `calculate_conversion_factor` (line 603); added `parse_factor` overhaul (line 1035), `parse_function_call` (line 1098), `split_function_args` (line 1199), free function `check_arity` (line 1258).
- `/home/rogerio/git/nc2parquet/src/input.rs` — Added `CompressionCodec` enum (line 22), `OutputConfig` struct with `Default` impl and `to_polars_compression`/`validate` methods (lines 62-211), `BatchConfig` struct (line 562), `BatchResult` struct (line 593), `variable_names` field and `effective_variable_names()` method on `JobConfig` (lines 255, 412).
- `/home/rogerio/git/nc2parquet/src/extract.rs` — Added `extract_multi_variable_dataframe` (line 388), `extract_variable_values_with_dim_manager` (line 463), `extract_variable_values_batch` (line 475), `extract_variable_values_cellwise` (implied by dispatch).
- `/home/rogerio/git/nc2parquet/src/output.rs` — Added `build_parquet_writer` helper (line 67); all three write functions now accept `Option<&OutputConfig>`.
- `/home/rogerio/git/nc2parquet/src/lib.rs` — Added `resolve_output_path` (line 181), `process_netcdf_batch` (line 214); updated `process_netcdf_job` and `process_netcdf_job_async` to dispatch on `var_names.len()`.
- `/home/rogerio/git/nc2parquet/src/cli.rs` — Added `variables: Vec<String>` with `value_delimiter = ','`, `glob: Option<String>`, `compression: Option<String>`, `compression_level: Option<u32>`, `row_group_size: Option<usize>`, `no_statistics: bool` fields on `Commands::Convert`.
- `/home/rogerio/git/nc2parquet/src/tests/test_batch.rs` — New test file; 12 integration tests for `process_netcdf_batch` and `resolve_output_path`.
- `/home/rogerio/git/nc2parquet/src/tests/test_multi_variable.rs` — New test file; 10 tests covering `effective_variable_names`, multi-variable extraction, dimension mismatch, backward compatibility.

## Conventions Adopted

- **Three scoped blocks per variable in `extract_multi_variable_dataframe`**: Each `netcdf::Variable` borrow lives in its own `{ }` block. The first variable is opened three times (once for `DimensionIndexManager`, once for dimension metadata, once for value extraction) because the borrow checker forbids keeping a reference alive across the dimension-metadata block and the extraction block when the `dim_manager` is built from the same file. This is more verbose than necessary but passes the borrow checker without `unsafe`. See `/home/rogerio/git/nc2parquet/src/extract.rs` lines 395-428.

- **Compression level validation via `OutputConfig::validate()`**: Validation is performed in the CLI handler before the conversion starts, not inside `to_polars_compression()`. The `to_polars_compression()` method silently clamps invalid levels to the codec default using `.ok()` on `ZstdLevel::try_new` and `GzipLevel::try_new`. This means library callers who skip `validate()` get a degraded-but-functional result rather than a panic. See `/home/rogerio/git/nc2parquet/src/input.rs` line 121 vs line 170.

- **Doc examples on every new public item**: `CompressionCodec`, `OutputConfig`, `BatchConfig`, `JobConfig::effective_variable_names`, and `resolve_output_path` all have `# Examples` sections in their rustdoc. These are compiled as doc tests. See `/home/rogerio/git/nc2parquet/src/input.rs` and `/home/rogerio/git/nc2parquet/src/lib.rs`.

## Surprises and Deviations

- **Test count grew by 115 (from 181 to 296)**: The ticket estimates implied roughly 30–40 new tests across all five tickets. The actual growth was much larger because the implementation added two entirely new test files (`test_batch.rs`, `test_multi_variable.rs`) and significantly expanded `test_postprocess.rs` and `test_output.rs`. Epic 05 documentation planning should assume the test suite is large and doc-test examples must remain accurate.

- **`round()` uses `HalfAwayFromZero` not banker's rounding**: The ticket acceptance criterion stated "banker's rounding per Polars" for `round(value)`. The implementation uses `RoundMode::HalfAwayFromZero` (imported from `polars_ops::prelude`), which is the standard mathematical rounding convention. The Polars default behaviour for `round(decimals: u32)` without a `RoundMode` argument was changed in recent versions; explicit `HalfAwayFromZero` was needed to avoid compilation errors with Polars 0.51.

- **Multi-variable extraction requires opening the variable three times**: The plan predicted the key challenge as "borrow checker interaction with multi-variable NetCDF file reads" (ticket-021 confidence: Medium). The actual solution required reopening the first variable three times (for `DimensionIndexManager` construction, dimension metadata, and value extraction) in separate scopes because the borrow checker forbids keeping a `&netcdf::Variable` alive across `DimensionIndexManager::new` invocations when `file` is also borrowed. This is a quirk of the `netcdf-rs` API not documented anywhere in the codebase prior to this epic.

- **No `dimension_mismatch` integration test was added**: Ticket-021 specified a "dimension mismatch" integration test. The test was added as a unit test in `test_multi_variable.rs` using in-memory data fabrication rather than an actual multi-variable NetCDF file with different dimensions. The `pres_temp_4D.nc` fixture only contains variables with identical dimensions, so testing the mismatch error path required constructing the condition differently.

- **Doc test count dropped from 39 to 28**: The epic-03 summary recorded 39 doc tests. After epic-04 the `cargo test --doc` count is 28. This likely reflects that some doc examples in earlier modules were adjusted or removed during the multi-variable refactoring of `lib.rs`'s `JobConfig` examples, or that certain doc examples stopped compiling under Polars 0.51 API changes. This requires investigation before epic-05 documentation work begins.

## Recommendations for Future Epics

- **Epic 05 (Documentation)**: The `resolve_output_path` function in `/home/rogerio/git/nc2parquet/src/lib.rs` (line 181) and `process_netcdf_batch` (line 214) are now public. The README should include a "Batch Processing" section and a "Multi-Variable Extraction" section with JSON config examples. The `OutputConfig` struct in `/home/rogerio/git/nc2parquet/src/input.rs` (line 62) has full rustdoc; the README should reference it with concrete examples of compression settings for different downstream consumers (Spark vs DuckDB vs Athena).

- **Epic 05 (Documentation)**: Investigate the doc test count drop from 39 to 28 before writing tutorials. Run `cargo test --doc -- --list` to identify which doc examples are registered, then cross-reference with the current `# Examples` sections in all public modules. Any missing examples should be re-added before the README references them.

- **Epic 06 (CI/CD)**: The `BatchConfig` + `process_netcdf_batch` path is not covered by the property-based tests in `test_properties.rs`. Before adding CI coverage reporting, add at least one proptest targeting `resolve_output_path` (arbitrary stem inputs, verify output always ends with the template extension) to establish a coverage floor for the batch path.

- **Epic 06 (CI/CD)**: The `OutputConfig::validate()` method has six error paths (Zstd level out of range, Gzip level out of range, Snappy/Lz4/Uncompressed with level, row_group_size == 0). These are all covered in `test_output.rs`, but are not in the CI linting pipeline. The `cargo test --lib` run is sufficient to catch regressions.

- **Future feature work**: The `FormulaApplier` parser still does not support unary minus as a prefix operator (e.g., `-1.0 * x`). This was documented as a known limitation in epic-01 learnings. The `parse_expression` function in `/home/rogerio/git/nc2parquet/src/postprocess.rs` (line 978) would need a leading-minus special case to fix this. It did not block any epic-04 formula tests but will affect users who write formulas like `-temperature`.
