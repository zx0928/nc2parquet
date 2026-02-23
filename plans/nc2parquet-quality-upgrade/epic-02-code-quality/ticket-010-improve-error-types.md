# ticket-010 Improve Library Error Types with thiserror

## Context

### Background

The library currently uses a mix of `Box<dyn std::error::Error>` and `PostProcessError` for error handling. The `thiserror` crate is already a dependency. This ticket creates structured error types for the library modules (filters, extract, input) so callers get specific, matchable error types instead of opaque boxed errors.

### Relation to Epic

Second ticket in Epic 02. Depends on the handler extraction (ticket-009) being complete so error type changes do not conflict with the move.

### Current State

- `storage.rs`: Uses `StorageError` (thiserror) -- already good
- `postprocess.rs`: Uses `PostProcessError` (manual Display + Error impls) -- functional but could use thiserror
- `filters.rs`: All functions return `Box<dyn std::error::Error>` -- no structured errors
- `extract.rs`: All functions return `Box<dyn std::error::Error>` -- no structured errors
- `input.rs`: `from_file` and `from_json` return `Box<dyn std::error::Error>` -- no structured errors
- `lib.rs`: `process_netcdf_job` and `process_netcdf_job_async` return `Box<dyn std::error::Error>` -- no structured errors

## Specification

### Requirements

1. Create `src/errors.rs` with a unified `Nc2ParquetError` enum using `thiserror::Error`:

   ```rust
   #[derive(thiserror::Error, Debug)]
   pub enum Nc2ParquetError {
       #[error("NetCDF error: {0}")]
       NetCdf(#[from] netcdf::Error),
       #[error("Variable '{0}' not found in NetCDF file")]
       VariableNotFound(String),
       #[error("Dimension '{0}' not found")]
       DimensionNotFound(String),
       #[error("Filter error: {0}")]
       Filter(String),
       #[error("Extraction error: {0}")]
       Extraction(String),
       #[error("Post-processing error: {0}")]
       PostProcess(#[from] PostProcessError),
       #[error("Storage error: {0}")]
       Storage(#[from] StorageError),
       #[error("IO error: {0}")]
       Io(#[from] std::io::Error),
       #[error("Polars error: {0}")]
       Polars(#[from] polars::prelude::PolarsError),
       #[error("Configuration error: {0}")]
       Config(String),
       #[error("Serialization error: {0}")]
       Serialization(String),
       #[error("Unsupported dimensionality: {0} dimensions")]
       UnsupportedDimensionality(usize),
   }
   ```

2. Convert `PostProcessError` to use `#[derive(thiserror::Error)]` instead of manual impl

3. Update `filters.rs` functions to return `Result<_, Nc2ParquetError>` instead of `Result<_, Box<dyn Error>>`

4. Update `extract.rs` functions to return `Result<_, Nc2ParquetError>`

5. Update `input.rs` functions to return `Result<_, Nc2ParquetError>`

6. Update `lib.rs` pipeline functions to return `Result<(), Nc2ParquetError>`

7. Add `pub mod errors;` to `lib.rs` and re-export `Nc2ParquetError`

8. Update `NCFilter` trait to use `Nc2ParquetError`:
   ```rust
   pub trait NCFilter {
       fn apply(&self, file: &netcdf::File) -> Result<FilterResult, Nc2ParquetError>;
   }
   ```

### Inputs/Props

No runtime changes.

### Outputs/Behavior

- All library functions return `Nc2ParquetError` instead of `Box<dyn Error>`
- Callers (main.rs handlers) can match on specific error variants
- Existing error messages are preserved

### Error Handling

This IS the error handling improvement. The key change is from opaque `Box<dyn Error>` to matchable enum variants.

## Acceptance Criteria

- [ ] Given `src/errors.rs` exists, when it is read, then it contains `Nc2ParquetError` with all variants listed above
- [ ] Given `PostProcessError` definition, when inspected, then it uses `#[derive(thiserror::Error)]`
- [ ] Given `NCFilter::apply` signature, when inspected, then return type is `Result<FilterResult, Nc2ParquetError>`
- [ ] Given `process_netcdf_job` signature, when inspected, then return type is `Result<(), Nc2ParquetError>`
- [ ] Given the handlers in main.rs, when they call library functions, then they can convert `Nc2ParquetError` to `anyhow::Error` with `?`
- [ ] Given all tests, when `cargo test --lib` runs, then all pass (tests may need minor adjustments to error type expectations)
- [ ] Given `cargo clippy -- -D warnings`, when run, then zero warnings

## Implementation Guide

### Suggested Approach

1. Create `src/errors.rs` with the error enum
2. Add to `src/lib.rs`: `pub mod errors; pub use errors::Nc2ParquetError;`
3. Update `PostProcessError` to use `thiserror::Error` derive
4. Update `NCFilter` trait and all 4 implementations
5. Update `extract.rs` functions
6. Update `input.rs` functions
7. Update `lib.rs` pipeline functions
8. Update tests that check for `Box<dyn Error>` to check for `Nc2ParquetError`
9. The handlers in `main.rs` convert with `.map_err(|e| anyhow::anyhow!("{}", e))` or just use `?` since `anyhow::Error: From<Nc2ParquetError>` works via the Error trait

### Key Files to Modify

- `/home/rogerio/git/nc2parquet/src/errors.rs` -- CREATE
- `/home/rogerio/git/nc2parquet/src/lib.rs` -- add errors module, update function signatures
- `/home/rogerio/git/nc2parquet/src/filters.rs` -- update trait and implementations
- `/home/rogerio/git/nc2parquet/src/extract.rs` -- update function signatures
- `/home/rogerio/git/nc2parquet/src/input.rs` -- update function signatures
- `/home/rogerio/git/nc2parquet/src/postprocess.rs` -- convert PostProcessError to thiserror

### Patterns to Follow

- Use `#[from]` for automatic conversions from external error types
- Use descriptive `#[error("...")]` messages that include context
- Keep `PostProcessError` as a separate type that `Nc2ParquetError` wraps (not merge into one)
- The `?` operator should work seamlessly with `From` impls

### Pitfalls to Avoid

- The `NCFilter` trait is used in `Box<dyn NCFilter>` -- changing the return type is a breaking change for any external implementors. Since this is v0.1.x and the trait is not widely used externally, this is acceptable.
- `netcdf::Error` and `polars::prelude::PolarsError` must have `From` impls -- use `#[from]`
- Some `Box<dyn Error>` returns use `format!("...").into()` -- these need to be converted to specific error variants
- Tests that check `result.is_err()` will still work, but tests that downcast errors may need updating

## Testing Requirements

### Unit Tests

Update existing tests that assert error types. No new tests needed.

### Integration Tests

`cargo test --lib` must pass.

### E2E Tests

None.

## Dependencies

- **Blocked By**: ticket-009 (handlers extracted)
- **Blocks**: ticket-011, ticket-012, ticket-013

## Effort Estimate

**Points**: 3
**Confidence**: Medium (scope of changes across multiple files)
