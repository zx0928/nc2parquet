# ticket-006 Add Unit Tests for Output and Storage Modules

## Context

### Background

The output module (`src/output.rs`, ~131 lines) handles Parquet file writing (sync and async), and the storage module (`src/storage.rs`, ~150 lines + tests) provides the `StorageBackend` trait with `LocalStorage` and `S3Storage` implementations. The storage module already has 7 inline tests. The output module has zero dedicated tests -- it is only tested indirectly through integration pipeline tests.

### Relation to Epic

Sixth ticket in Epic 01. Fills the gap in output module coverage and adds edge-case tests for storage.

### Current State

**Output module tests**: None. `write_dataframe_to_parquet` and `write_dataframe_to_parquet_async` are only tested as part of full pipeline integration tests.

**Storage module tests** (7 inline in `src/storage.rs`):

- `test_local_storage_write_read` -- write/read/exists cycle
- `test_local_storage_not_found` -- read nonexistent file
- `test_s3_path_parsing` -- valid and invalid S3 paths
- `test_storage_factory_path_detection` -- is_s3_path/is_local_path
- `test_storage_enum_local_operations` -- Storage enum dispatch
- S3 integration tests (2) -- NOAA public dataset tests

**Missing coverage**:

- `write_dataframe_to_parquet` with empty DataFrame
- `write_dataframe_to_parquet` creates parent directories
- `write_dataframe_to_parquet_async` with local path
- `dataframe_to_parquet_bytes` produces valid parquet bytes
- `LocalStorage::write` creates parent directories
- `LocalStorage::write` to read-only location (permission error)
- `S3Storage::parse_s3_path` additional edge cases (trailing slash, special characters)
- `StorageFactory::from_path` with local path returns Local variant

## Specification

### Requirements

**Output module tests** (5 tests):

1. `write_dataframe_to_parquet` with a simple DataFrame -- verify file is created and valid
2. `write_dataframe_to_parquet` with an empty DataFrame (0 rows) -- verify it succeeds and produces a valid file
3. `write_dataframe_to_parquet` creates parent directories that do not exist
4. `write_dataframe_to_parquet_async` with local path -- verify it produces identical output to sync version
5. `dataframe_to_parquet_bytes` returns non-empty bytes that start with the Parquet magic bytes ("PAR1")

**Storage module tests** (4 tests):

1. `LocalStorage::write` with nested nonexistent directories -- verify directories are created
2. `S3Storage::parse_s3_path` with deeply nested key -- verify correct bucket/key split
3. `S3Storage::parse_s3_path` with bucket only (no key after slash) -- verify error
4. `StorageFactory::is_s3_path` and `is_local_path` with various edge case strings

### Inputs/Props

- DataFrames created from test helpers
- Temporary directories from `create_temp_output_dir()`

### Outputs/Behavior

- Parquet files created on disk with correct content
- Error results for invalid paths
- Correct path detection for storage factory

### Error Handling

- Writing to invalid paths should propagate IO errors
- S3 path parsing with invalid formats should return `StorageError::InvalidS3Path`

## Acceptance Criteria

- [ ] Given a simple DataFrame, when `write_dataframe_to_parquet` is called, then the file exists and `assert_parquet_file_valid` passes
- [ ] Given an empty DataFrame (0 rows), when `write_dataframe_to_parquet` is called, then it succeeds without error
- [ ] Given a path with nonexistent parent dirs, when `write_dataframe_to_parquet` is called, then parent dirs are created
- [ ] Given a DataFrame, when `dataframe_to_parquet_bytes` is called, then the bytes start with "PAR1"
- [ ] Given a local path, when `write_dataframe_to_parquet_async` is called, then the file is created and valid
- [ ] Given `S3Storage::parse_s3_path("s3://bucket/deeply/nested/path/file.nc")`, when parsed, then bucket="bucket" and key="deeply/nested/path/file.nc"
- [ ] Given `S3Storage::parse_s3_path("s3://bucket/")`, when parsed, then error is returned (empty key)
- [ ] Given all new tests, when `cargo test --lib` runs, then all pass

## Implementation Guide

### Suggested Approach

For output tests, create a new file `src/tests/test_output.rs`:

```rust
use crate::output::{write_dataframe_to_parquet, write_dataframe_to_parquet_async};
use crate::test_helpers::*;
use polars::prelude::*;

#[test]
fn test_write_simple_dataframe() {
    let df = create_simple_test_dataframe();
    let dir = create_temp_output_dir();
    let path = dir.path().join("test.parquet");

    write_dataframe_to_parquet(&df, path.to_str().unwrap()).unwrap();
    assert_parquet_file_valid(&path);
}

#[test]
fn test_write_empty_dataframe() {
    let df = df! {
        "col" => Vec::<f64>::new(),
    }.unwrap();
    let dir = create_temp_output_dir();
    let path = dir.path().join("empty.parquet");

    write_dataframe_to_parquet(&df, path.to_str().unwrap()).unwrap();
    assert!(path.exists());
}

#[test]
fn test_write_creates_parent_dirs() {
    let df = create_simple_test_dataframe();
    let dir = create_temp_output_dir();
    let path = dir.path().join("subdir1").join("subdir2").join("test.parquet");

    write_dataframe_to_parquet(&df, path.to_str().unwrap()).unwrap();
    assert_parquet_file_valid(&path);
}
```

For the `dataframe_to_parquet_bytes` test, note that it is a private function (`fn dataframe_to_parquet_bytes`). Since it is called through the async public API, you can test it indirectly via `write_dataframe_to_parquet_async`, or add a `#[cfg(test)]` re-export. The simplest approach is to test `write_dataframe_to_parquet_async` with a local path.

Update `src/tests/mod.rs` to include `mod test_output;`.

### Key Files to Modify

- `/home/rogerio/git/nc2parquet/src/tests/mod.rs` -- add `mod test_output;`
- `/home/rogerio/git/nc2parquet/src/tests/test_output.rs` -- CREATE
- `/home/rogerio/git/nc2parquet/src/tests/test_filters.rs` or `src/storage.rs` inline -- add storage edge-case tests

### Patterns to Follow

- Use `create_temp_output_dir()` for all test output
- Use `assert_parquet_file_valid()` to verify output files
- Use `#[tokio::test]` for async write tests

### Pitfalls to Avoid

- `dataframe_to_parquet_bytes` is private -- do not try to call it directly from tests outside the module
- Empty DataFrames in Polars need at least one column definition even with 0 rows
- The async write function creates an `S3Storage` client for S3 paths -- for local paths it still works correctly with `StorageFactory::from_path`

## Testing Requirements

### Unit Tests

~9 new tests as described above.

### Integration Tests

None.

### E2E Tests

None.

## Dependencies

- **Blocked By**: ticket-001 (test helpers)
- **Blocks**: None

## Effort Estimate

**Points**: 2
**Confidence**: High
