# ticket-002 Reorganize tests.rs Into Per-Module Test Files

## Context

### Background

All 97 tests currently live in a single `src/tests.rs` file (~2370 lines). This makes it difficult to find tests for specific modules, creates long compile times for the test file, and makes it hard to run tests for a specific module in isolation. This ticket reorganizes tests into per-module test files while keeping every existing test passing.

### Relation to Epic

This is the second ticket in Epic 01. It restructures the test layout so that subsequent tickets (003-008) can add new tests to well-organized, module-specific files.

### Current State

- `src/tests.rs` contains these test modules:
  - `input_tests` (5 tests) -- JobConfig parsing, FilterConfig parsing
  - `filter_tests` (11 tests) -- filter creation, application with real data, FilterResult methods
  - `extract_tests` (5 tests) -- DimensionIndexManager, extract_data_to_dataframe
  - `utility_tests` (3 tests) -- JSON parsing errors, invalid filter kind, empty filters
  - `integration_tests` (4 tests) -- full pipeline simple_xy, latitude filter, spatial filter, multi-filter
  - `workflow_tests` (6 tests) -- local file with all features, async processing, complex pipeline chaining, error handling, performance benchmarking, async vs sync performance
  - `s3_integration_tests` (2 tests) -- NOAA S3 pipeline, NOAA S3 info command
  - `postprocess_tests` (14 tests) -- column renamer, unit converter, aggregator, formula applier, pipeline, config creation, datetime converter, error handling
  - `cli_tests` (16 tests) -- already in `src/cli.rs` inline `#[cfg(test)]` module
  - `netcdf_exploration_tests` (1 test) -- exploratory API test
  - `info_command_tests` (8 tests) -- get_netcdf_info variants, structure tests, format output tests
- `src/cli.rs` has 8 tests inline
- `src/storage.rs` has 7 tests inline

## Specification

### Requirements

1. Create a `src/tests/` directory with the following structure:
   ```
   src/tests/
     mod.rs              -- module declarations
     test_input.rs       -- input_tests + utility_tests from tests.rs
     test_filters.rs     -- filter_tests from tests.rs
     test_extract.rs     -- extract_tests from tests.rs
     test_postprocess.rs -- postprocess_tests from tests.rs
     test_info.rs        -- info_command_tests from tests.rs
     test_integration.rs -- integration_tests + workflow_tests + s3_integration_tests from tests.rs
   ```
2. Move tests from `src/tests.rs` into the corresponding files
3. Keep `netcdf_exploration_tests` in `test_integration.rs` (it is an exploration test using real data)
4. Update `src/lib.rs` to point `mod tests` at `src/tests/mod.rs`
5. Delete the old `src/tests.rs` file
6. Keep inline tests in `src/cli.rs` and `src/storage.rs` unchanged (they test private functions)
7. All imports must be adjusted for the new module paths
8. Every single existing test must pass without modification to its logic

### Inputs/Props

No runtime inputs. This is a code reorganization.

### Outputs/Behavior

- `cargo test --lib` passes with the same 97 tests
- Test files are now organized by module, each under 500 lines

### Error Handling

Not applicable.

## Acceptance Criteria

- [ ] Given the reorganization is complete, when `cargo test --lib` is run, then exactly 97 tests pass with 0 failures
- [ ] Given `src/tests.rs` existed before, when the reorganization is complete, then `src/tests.rs` no longer exists
- [ ] Given `src/tests/mod.rs` exists, when it is read, then it contains module declarations for all test submodules
- [ ] Given `src/tests/test_filters.rs` exists, when it is read, then it contains all 11 filter tests from the old file
- [ ] Given `src/tests/test_postprocess.rs` exists, when it is read, then it contains all 14 postprocess tests
- [ ] Given `src/tests/test_integration.rs` exists, when it is read, then it contains all integration, workflow, and S3 tests
- [ ] Given the new test directory, when `cargo test test_filters` is run, then only filter tests execute

## Implementation Guide

### Suggested Approach

1. Create `src/tests/mod.rs` with module declarations:

   ```rust
   mod test_input;
   mod test_filters;
   mod test_extract;
   mod test_postprocess;
   mod test_info;
   mod test_integration;
   ```

2. For each new file, copy the relevant test module(s) from `src/tests.rs`:
   - Add necessary imports at the top of each file
   - The shared `get_test_data_path` helper should be imported from `crate::test_helpers` (created in ticket-001)
   - Replace `use super::*;` with explicit imports like `use crate::filters::*;`, `use crate::input::*;`, etc.

3. Update `src/lib.rs` -- the existing line `mod tests;` already points to the tests module; once you replace the `src/tests.rs` file with `src/tests/mod.rs`, Rust will resolve it automatically.

4. Delete `src/tests.rs` after verifying all tests pass from the new locations.

5. Run `cargo test --lib` to verify all 97 tests pass.

### Key Files to Modify

- `/home/rogerio/git/nc2parquet/src/tests.rs` -- DELETE after migration
- `/home/rogerio/git/nc2parquet/src/tests/mod.rs` -- CREATE
- `/home/rogerio/git/nc2parquet/src/tests/test_input.rs` -- CREATE
- `/home/rogerio/git/nc2parquet/src/tests/test_filters.rs` -- CREATE
- `/home/rogerio/git/nc2parquet/src/tests/test_extract.rs` -- CREATE
- `/home/rogerio/git/nc2parquet/src/tests/test_postprocess.rs` -- CREATE
- `/home/rogerio/git/nc2parquet/src/tests/test_info.rs` -- CREATE
- `/home/rogerio/git/nc2parquet/src/tests/test_integration.rs` -- CREATE

### Patterns to Follow

- Use `use crate::test_helpers::*;` for shared helpers
- Use `use crate::filters::*;` style explicit imports instead of `use super::*;`
- Keep the same `#[test]` and `#[tokio::test]` annotations
- Preserve the `#[allow(clippy::...)]` attributes from the original file header

### Pitfalls to Avoid

- Do NOT rename any test functions -- test names must remain identical for comparison
- Do NOT change test logic -- only move code and adjust imports
- The `ENV_TEST_MUTEX` used in `cli_tests` (inside `src/cli.rs`) is separate from the test reorganization; leave it alone
- The `tempdir` import in integration tests comes from the `tempfile` crate, not std
- Some tests use `crate::process_netcdf_job` and `crate::process_netcdf_job_async` -- these must be imported from `crate` not from `super`

## Testing Requirements

### Unit Tests

No new tests in this ticket. All 97 existing tests are moved, not modified.

### Integration Tests

Verify `cargo test --lib` passes with 97 tests.

### E2E Tests

None.

## Dependencies

- **Blocked By**: ticket-001 (test helpers must exist for imports)
- **Blocks**: ticket-008 (integration tests for error paths)

## Effort Estimate

**Points**: 3
**Confidence**: High
