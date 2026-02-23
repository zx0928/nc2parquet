# ticket-001 Add Test Dev-Dependencies and Create Test Helper Module

## Context

### Background

The nc2parquet project (v0.1.1) currently has 97 passing tests but lacks test infrastructure for systematic expansion. The existing test helpers are minimal -- just a `get_test_data_path()` function defined inside `src/tests.rs`. Property-based testing (proptest) and CLI testing (assert_cmd) frameworks are not yet available. This ticket establishes the foundation that all subsequent testing tickets depend on.

### Relation to Epic

This is the first ticket in Epic 01 (Testing Infrastructure & Coverage). It provides the shared test utilities, fixtures, and dependencies that tickets 002-008 will build upon.

### Current State

- `Cargo.toml` dev-dependencies: `tempfile`, `testcontainers`, `testcontainers-modules`, `tokio-test`, `aws-credential-types`
- Test data files exist at `examples/data/simple_xy.nc` and `examples/data/pres_temp_4D.nc`
- A single `get_test_data_path()` helper exists inside `src/tests.rs` (not reusable from other test files)
- No programmatic NetCDF test data generation
- No property-based testing framework
- No CLI integration testing framework

## Specification

### Requirements

1. Add `proptest` to dev-dependencies for property-based testing
2. Add `assert_cmd` and `predicates` to dev-dependencies for CLI integration testing
3. Create a `src/test_helpers.rs` module (conditionally compiled with `#[cfg(test)]`) containing:
   - `get_test_data_path(filename: &str) -> PathBuf` -- returns path to `examples/data/{filename}`
   - `create_temp_output_dir() -> TempDir` -- creates and returns a temporary directory for test outputs
   - `create_simple_test_dataframe() -> DataFrame` -- returns a standard 4-row DataFrame with temperature, pressure, humidity, time_offset columns (matching the existing `create_test_dataframe()` in tests.rs)
   - `create_weather_test_dataframe() -> DataFrame` -- returns a larger DataFrame with station, lat, lon, temperature, pressure columns for aggregation testing
   - `assert_parquet_file_valid(path: &Path)` -- asserts file exists, has non-zero size, and can be read back as a DataFrame
4. Re-export the helper module from `src/lib.rs` under `#[cfg(test)]`

### Inputs/Props

No function inputs -- this ticket only creates infrastructure files and modifies `Cargo.toml`.

### Outputs/Behavior

After this ticket:

- `cargo test` continues to pass all 97 existing tests
- New dev-dependencies are available: `proptest`, `assert_cmd`, `predicates`
- Test helper functions are importable as `crate::test_helpers::*` from any `#[cfg(test)]` module

### Error Handling

Not applicable -- test helpers should panic on failure (using `unwrap()` is acceptable in test code).

## Acceptance Criteria

- [ ] Given the project compiles, when `cargo test` is run, then all 97 existing tests pass unchanged
- [ ] Given `Cargo.toml` is read, when dev-dependencies are inspected, then `proptest`, `assert_cmd`, and `predicates` are present
- [ ] Given `src/test_helpers.rs` exists, when it is compiled with `#[cfg(test)]`, then all 5 helper functions are available
- [ ] Given `get_test_data_path("simple_xy.nc")` is called, when the returned path is checked, then it points to `{CARGO_MANIFEST_DIR}/examples/data/simple_xy.nc`
- [ ] Given `create_simple_test_dataframe()` is called, when the DataFrame is inspected, then it has 4 rows and columns: temperature (f64), pressure (f64), humidity (f64), time_offset (f64)
- [ ] Given `assert_parquet_file_valid` is called with a valid parquet file, when it runs, then it does not panic
- [ ] Given `assert_parquet_file_valid` is called with a nonexistent path, when it runs, then it panics with a descriptive message

## Implementation Guide

### Suggested Approach

1. Add dev-dependencies to `Cargo.toml`:

   ```toml
   [dev-dependencies]
   proptest = "1.4"
   assert_cmd = "2.0"
   predicates = "3.1"
   ```

2. Create `src/test_helpers.rs`:

   ```rust
   //! Test helper utilities for nc2parquet tests.
   //! This module is only compiled when running tests.

   use polars::prelude::*;
   use std::path::{Path, PathBuf};
   use tempfile::TempDir;

   pub fn get_test_data_path(filename: &str) -> PathBuf {
       let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
       path.push("examples");
       path.push("data");
       path.push(filename);
       path
   }

   pub fn create_temp_output_dir() -> TempDir {
       tempfile::tempdir().expect("Failed to create temp directory for test output")
   }

   pub fn create_simple_test_dataframe() -> DataFrame {
       df! {
           "temperature" => [273.15, 283.15, 293.15, 303.15],
           "pressure" => [1013.25, 1012.0, 1010.5, 1009.0],
           "humidity" => [60.0, 65.0, 70.0, 75.0],
           "time_offset" => [0.0, 1.0, 2.0, 3.0],
       }
       .expect("Failed to create simple test DataFrame")
   }

   pub fn create_weather_test_dataframe() -> DataFrame {
       df! {
           "station" => ["A", "A", "A", "B", "B", "B"],
           "latitude" => [40.7, 40.7, 40.7, 34.0, 34.0, 34.0],
           "longitude" => [-74.0, -74.0, -74.0, -118.2, -118.2, -118.2],
           "temperature" => [280.0, 282.0, 281.0, 295.0, 296.0, 294.0],
           "pressure" => [1013.0, 1012.0, 1013.5, 1010.0, 1009.5, 1011.0],
       }
       .expect("Failed to create weather test DataFrame")
   }

   pub fn assert_parquet_file_valid(path: &Path) {
       assert!(path.exists(), "Parquet file does not exist: {}", path.display());
       let metadata = std::fs::metadata(path)
           .unwrap_or_else(|e| panic!("Failed to read metadata for {}: {}", path.display(), e));
       assert!(metadata.len() > 0, "Parquet file is empty: {}", path.display());

       // Verify it can be read back as a DataFrame
       let file = std::fs::File::open(path)
           .unwrap_or_else(|e| panic!("Failed to open {}: {}", path.display(), e));
       let _df = polars::io::parquet::read::ParquetReader::new(file)
           .finish()
           .unwrap_or_else(|e| panic!("Failed to read parquet {}: {}", path.display(), e));
   }
   ```

3. Add to `src/lib.rs`:

   ```rust
   #[cfg(test)]
   pub mod test_helpers;
   ```

4. Add a simple test in `src/test_helpers.rs` to verify the helpers work:

   ```rust
   #[cfg(test)]
   mod tests {
       use super::*;

       #[test]
       fn test_get_test_data_path() {
           let path = get_test_data_path("simple_xy.nc");
           assert!(path.exists(), "Test data file should exist");
       }

       #[test]
       fn test_create_simple_test_dataframe() {
           let df = create_simple_test_dataframe();
           assert_eq!(df.height(), 4);
           assert_eq!(df.width(), 4);
       }

       #[test]
       fn test_create_weather_test_dataframe() {
           let df = create_weather_test_dataframe();
           assert_eq!(df.height(), 6);
           assert_eq!(df.width(), 5);
       }
   }
   ```

### Key Files to Modify

- `/home/rogerio/git/nc2parquet/Cargo.toml` -- add dev-dependencies
- `/home/rogerio/git/nc2parquet/src/lib.rs` -- add `#[cfg(test)] pub mod test_helpers;`
- `/home/rogerio/git/nc2parquet/src/test_helpers.rs` -- create new file

### Patterns to Follow

- Use `#[cfg(test)]` gating consistently for test-only code
- Use `env!("CARGO_MANIFEST_DIR")` for reliable path resolution (already used in existing tests)
- Use `tempfile::TempDir` for temporary test directories (already a dev-dependency)
- Panic with descriptive messages in test helpers (not `Result` returns)

### Pitfalls to Avoid

- Do NOT move or modify existing tests in `src/tests.rs` -- that is ticket-002's job
- Do NOT add the `parquet` read feature to polars production dependencies -- the `ParquetReader` is already available via the existing `parquet` feature in polars
- Ensure `proptest` version is compatible with Rust edition 2024

## Testing Requirements

### Unit Tests

- `test_get_test_data_path` -- verifies path construction and file existence
- `test_create_simple_test_dataframe` -- verifies DataFrame shape and column names
- `test_create_weather_test_dataframe` -- verifies DataFrame shape and column names
- `test_assert_parquet_file_valid_with_valid_file` -- creates a temp parquet, verifies helper passes
- `test_assert_parquet_file_valid_panics_on_missing` -- verifies helper panics on missing file

### Integration Tests

None for this ticket.

### E2E Tests

None for this ticket.

## Dependencies

- **Blocked By**: None
- **Blocks**: ticket-002, ticket-003, ticket-004, ticket-005, ticket-006, ticket-007

## Effort Estimate

**Points**: 2
**Confidence**: High
