# ticket-020 Add Glob Pattern Support for Batch File Processing

## Context

### Background

Currently, `process_netcdf_job` and `process_netcdf_job_async` accept a single `JobConfig` with one input file and one output file. Users processing hundreds of daily/hourly NetCDF files must invoke the tool repeatedly via shell scripts or external orchestration. Adding glob/wildcard pattern support (e.g., `data/*.nc`, `**/*.nc`) as a library-level batch function and a CLI flag allows processing multiple files with a single command, which is the most requested feature for climate data pipelines.

### Relation to Epic

This ticket is the largest in Epic 04, adding a new public API function and CLI flag. It operates independently of the other feature tickets (unit converter, multi-variable, parquet config, formula parser) since it wraps the existing single-file processing loop. The batch function reuses `process_netcdf_job` / `process_netcdf_job_async` per file, so it benefits from all earlier performance work.

### Current State

- `/home/rogerio/git/nc2parquet/src/lib.rs` exports `process_netcdf_job` and `process_netcdf_job_async`, both accepting `&JobConfig` with a single `nc_key` and `parquet_key`
- `/home/rogerio/git/nc2parquet/src/input.rs` defines `JobConfig` with `nc_key: String`, `variable_name: String`, `filters: Vec<FilterConfig>`, `parquet_key: String`, `postprocessing: Option<ProcessingPipelineConfig>`
- `/home/rogerio/git/nc2parquet/src/handlers/convert.rs` builds a single `JobConfig` from CLI args and calls `process_netcdf_job` or `process_netcdf_job_async`
- `/home/rogerio/git/nc2parquet/src/cli.rs` defines `Commands::Convert` with positional `input` and `output` arguments
- `/home/rogerio/git/nc2parquet/Cargo.toml` does not include a `glob` crate dependency
- S3 paths are handled via `StorageFactory::from_path()` which detects `s3://` prefixes -- glob patterns are filesystem-only

## Specification

### Requirements

1. Add the `glob` crate (version `0.3`) as a dependency in `Cargo.toml`

2. Add a new public function in `src/lib.rs`:

   ```rust
   pub fn process_netcdf_batch(config: &BatchConfig) -> Result<BatchResult, Nc2ParquetError>
   ```

3. Add a `BatchConfig` struct in `src/input.rs`:

   ```rust
   pub struct BatchConfig {
       pub pattern: String,          // glob pattern, e.g., "data/**/*.nc"
       pub output_dir: String,       // directory where .parquet files are written
       pub variable_name: String,
       pub filters: Vec<FilterConfig>,
       pub postprocessing: Option<ProcessingPipelineConfig>,
       pub output_template: Option<String>,  // e.g., "{stem}.parquet" (default)
       pub fail_fast: bool,          // true = stop on first error; false = collect errors
   }
   ```

4. Add a `BatchResult` struct in `src/input.rs`:

   ```rust
   pub struct BatchResult {
       pub succeeded: Vec<String>,   // paths of successfully processed files
       pub failed: Vec<(String, Nc2ParquetError)>,  // (path, error) pairs
       pub total_files: usize,
   }
   ```

5. The batch function:
   - Resolves the glob pattern to a list of matching file paths using `glob::glob()`
   - For each matched file, constructs a `JobConfig` with the matched path as `nc_key` and a generated `parquet_key` based on the output template
   - Processes files sequentially (parallel processing is a future optimization)
   - If `fail_fast` is true, returns immediately on the first error
   - If `fail_fast` is false, collects all errors and reports them in `BatchResult.failed`

6. Add a `--glob` CLI flag to `Commands::Convert`:

   ```
   nc2parquet convert --glob "data/**/*.nc" -n temperature -o output_dir/
   ```

   When `--glob` is provided, `input` and `output` positional args become optional. The `--glob` flag takes a pattern string and `output` is treated as a directory.

7. Output naming: default template is `{stem}.parquet` where `{stem}` is the input filename without extension. For `data/2024/jan_temp.nc`, the output would be `output_dir/jan_temp.parquet`.

8. Glob patterns do NOT work with S3 paths. If the pattern starts with `s3://`, return `Nc2ParquetError::Config("Glob patterns are not supported for S3 paths. Use S3 prefix listing instead.".into())`.

### Inputs/Props

- `BatchConfig.pattern`: A glob pattern string. Supports `*`, `**`, `?`, and `[...]` character classes per the `glob` crate.
- `BatchConfig.output_dir`: Must be an existing directory or the function creates it.
- `BatchConfig.output_template`: Optional. Default `"{stem}.parquet"`. Supports `{stem}` (filename without extension) and `{name}` (filename with extension) placeholders.

### Outputs/Behavior

- `BatchResult` with counts of succeeded/failed files
- Each matched file produces one `.parquet` file in `output_dir`
- Log messages at INFO level for each file processed: `"Processing file {i}/{total}: {path}"`
- Log messages at WARN level for each failed file: `"Failed to process {path}: {error}"`
- CLI handler prints a summary: `"Batch complete: {succeeded}/{total} files processed, {failed} errors"`

### Error Handling

- Invalid glob pattern: `Nc2ParquetError::Config(format!("Invalid glob pattern '{}': {}", pattern, err))`
- No files matched: `Nc2ParquetError::Config(format!("No files matched pattern '{}'", pattern))`
- S3 glob attempt: `Nc2ParquetError::Config("Glob patterns are not supported for S3 paths...")`
- Per-file errors: captured in `BatchResult.failed` when `fail_fast` is false, or propagated immediately when true
- Output directory creation failure: propagated as `Nc2ParquetError::Io`

## Acceptance Criteria

- [ ] Given a directory with 3 NetCDF files matching `*.nc`, when `process_netcdf_batch` is called with pattern `"dir/*.nc"`, then 3 Parquet files are produced in the output directory
- [ ] Given a glob pattern that matches no files, when `process_netcdf_batch` is called, then `Nc2ParquetError::Config` is returned with a message containing "No files matched"
- [ ] Given an invalid glob pattern like `"[invalid"`, when `process_netcdf_batch` is called, then `Nc2ParquetError::Config` is returned
- [ ] Given `fail_fast: false` and one file that fails processing, when the batch completes, then `BatchResult.succeeded` has N-1 entries and `BatchResult.failed` has 1 entry
- [ ] Given `fail_fast: true` and one file that fails, when the batch runs, then it stops immediately and returns the error
- [ ] Given an S3 glob pattern `"s3://bucket/*.nc"`, when `process_netcdf_batch` is called, then `Nc2ParquetError::Config` is returned explaining S3 is not supported
- [ ] Given `output_template: Some("{stem}_converted.parquet")`, when a file `temp.nc` is processed, then the output is named `temp_converted.parquet`
- [ ] Given the CLI command `nc2parquet convert --glob "examples/data/*.nc" -n data -o /tmp/batch_out/`, when executed, then matching files are processed and output files appear in `/tmp/batch_out/`
- [ ] Given the default output template and an input file `/path/to/weather_2024.nc`, when the output is generated, then it is named `weather_2024.parquet` in the output directory

## Implementation Guide

### Suggested Approach

1. **Add dependency**: Add `glob = "0.3"` to `[dependencies]` in `Cargo.toml`

2. **Add data structures**: In `/home/rogerio/git/nc2parquet/src/input.rs`, add `BatchConfig` and `BatchResult` structs with `#[derive(Deserialize, Serialize, Clone)]` on `BatchConfig` and `#[derive(Debug)]` on `BatchResult`. Do NOT derive Serialize/Deserialize on `BatchResult` since it holds `Nc2ParquetError`.

3. **Add batch function**: In `/home/rogerio/git/nc2parquet/src/lib.rs`, add:

   ```rust
   pub fn process_netcdf_batch(config: &BatchConfig) -> Result<BatchResult, Nc2ParquetError> {
       if config.pattern.starts_with("s3://") {
           return Err(Nc2ParquetError::Config("Glob patterns are not supported for S3 paths.".into()));
       }
       let paths: Vec<PathBuf> = glob::glob(&config.pattern)
           .map_err(|e| Nc2ParquetError::Config(format!("Invalid glob pattern: {}", e)))?
           .filter_map(|entry| entry.ok())
           .collect();
       if paths.is_empty() {
           return Err(Nc2ParquetError::Config(format!("No files matched pattern '{}'", config.pattern)));
       }
       // create output_dir, iterate paths, build JobConfig per file, call process_netcdf_job
   }
   ```

4. **Output name resolution**: Create a helper function `fn resolve_output_path(input_path: &Path, output_dir: &str, template: &str) -> PathBuf` that replaces `{stem}` and `{name}` in the template and joins with `output_dir`.

5. **CLI integration**: In `/home/rogerio/git/nc2parquet/src/cli.rs`, add to `Commands::Convert`:

   ```rust
   #[arg(long, value_name = "PATTERN")]
   glob: Option<String>,
   ```

   In `/home/rogerio/git/nc2parquet/src/handlers/convert.rs`, detect when `glob` is `Some(pattern)` and call `process_netcdf_batch` instead of `process_netcdf_job`. When `--glob` is used, treat the `output` positional arg as `output_dir`.

6. **Progress reporting**: In the CLI handler, use `indicatif::ProgressBar::new(total as u64)` with a determinate progress bar for batch mode.

### Key Files to Modify

- `/home/rogerio/git/nc2parquet/Cargo.toml` -- Add `glob = "0.3"` dependency
- `/home/rogerio/git/nc2parquet/src/input.rs` -- Add `BatchConfig`, `BatchResult` structs
- `/home/rogerio/git/nc2parquet/src/lib.rs` -- Add `process_netcdf_batch` function, add `use` for `BatchConfig`/`BatchResult`
- `/home/rogerio/git/nc2parquet/src/cli.rs` -- Add `--glob` flag to `Commands::Convert`
- `/home/rogerio/git/nc2parquet/src/handlers/convert.rs` -- Add batch processing branch

### Patterns to Follow

- Follow the existing scoped-block pattern from `process_netcdf_job` (open file, extract, drop, postprocess, write) for each file in the batch
- Follow the binary-library split: batch processing logic lives in `lib.rs`, CLI-specific progress reporting lives in `handlers/convert.rs`
- Follow the existing error pattern: `Nc2ParquetError::Config(String)` for configuration-level errors

### Pitfalls to Avoid

- Do NOT try to parallelize batch processing in this ticket. Sequential processing is correct and sufficient. Parallel batch processing would require careful handling of peak memory across concurrent NetCDF file reads.
- The `glob` crate's `glob()` function returns an iterator of `Result<PathBuf, GlobError>`. Filter out errors (log them at WARN level) rather than failing the entire batch on a single path error.
- When `--glob` is used, the `output` positional arg should be treated as a directory, not a file path. Validate that it does not end in `.parquet` to catch user mistakes.
- `BatchResult` cannot derive `Serialize` because `Nc2ParquetError` does not implement `Serialize`. This is fine -- the CLI handler formats it for display.
- Do NOT forget to `use crate::input::{BatchConfig, BatchResult}` in `lib.rs` and re-export them.

## Testing Requirements

### Unit Tests

Add a `batch_processing` module in `/home/rogerio/git/nc2parquet/src/tests/test_integration.rs` (or a new `test_batch.rs`):

1. **Glob resolution**: Test that `resolve_output_path` correctly replaces `{stem}` and `{name}` placeholders
2. **S3 rejection**: Test that `process_netcdf_batch` returns `Config` error for S3 patterns
3. **Invalid pattern**: Test that `process_netcdf_batch` returns `Config` error for malformed glob patterns
4. **No matches**: Test that empty glob results return `Config` error

### Integration Tests

1. **Happy path batch**: Create a temp directory, copy `simple_xy.nc` 3 times with different names, run `process_netcdf_batch` with `"tempdir/*.nc"`, verify 3 `.parquet` files are created
2. **Fail-fast mode**: Include one invalid `.nc` file in the batch, verify processing stops at the first error
3. **Collect-errors mode**: Same setup with `fail_fast: false`, verify `BatchResult.failed` contains the bad file and `BatchResult.succeeded` contains the good ones
4. **Custom template**: Verify `{stem}_output.parquet` naming works

### E2E Tests

1. **CLI batch**: Use `assert_cmd` to run `nc2parquet convert --glob "examples/data/*.nc" -n data -o /tmp/test_batch/` and verify output files exist

## Dependencies

- **Blocked By**: ticket-009 (handlers extracted -- completed), ticket-010 (error types -- completed)
- **Blocks**: ticket-028 (usage tutorials will cover batch processing)

## Effort Estimate

**Points**: 4
**Confidence**: High
