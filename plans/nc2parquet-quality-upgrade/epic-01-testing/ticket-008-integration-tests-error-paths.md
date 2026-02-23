# ticket-008 Add Integration Tests for Error Paths and Edge Cases

## Context

### Background

The existing integration tests cover successful pipeline executions but do not systematically test error propagation, configuration validation failures, and edge-case scenarios. This ticket adds integration tests that verify the system fails gracefully and produces helpful error messages.

### Relation to Epic

Eighth and final ticket in Epic 01. Builds on the reorganized test structure from ticket-002 to add comprehensive integration-level error path coverage.

### Current State

Existing integration tests:

- `test_full_pipeline_simple_xy` -- success path, no filters
- `test_full_pipeline_with_latitude_filter` -- success path, range filter
- `test_full_pipeline_with_spatial_filter` -- success path, 2D point filter
- `test_full_pipeline_multi_filter` -- success path, range + list filters
- `test_integration_error_handling` -- nonexistent file, nonexistent variable, nonexistent dimension (basic)
- `test_integration_local_file_with_all_features` -- success path with postprocessing
- `test_integration_complex_pipeline_chaining` -- success path with multi-step pipeline

**Missing coverage**:

- PostProcessing pipeline failure mid-execution (second processor fails)
- Config with invalid postprocessing configuration (bad datetime base)
- Pipeline with conflicting rename (rename to column that already exists)
- Config file loading from nonexistent path
- Config file loading from malformed JSON/YAML
- Variable exists but has unsupported dimensionality (would need > 4 dims, may not be testable with example data)
- Full pipeline with filter that produces empty result set
- Async pipeline error propagation

## Specification

### Requirements

Add the following integration test cases to `src/tests/test_integration.rs`:

1. **Pipeline failure mid-execution** (1 test): Create a pipeline where processor 1 renames "temperature" to "temp", then processor 2 tries to unit-convert "temperature" (which no longer exists). Verify `PostProcessError::ColumnNotFound`.

2. **Invalid postprocessing config** (1 test): Create a config with `DatetimeConvert` using invalid base datetime string. Verify pipeline creation fails with `ConfigurationError`.

3. **Filter produces empty results** (1 test): Apply range filter on latitude with min=90, max=100 (beyond data range). Verify pipeline succeeds but output has 0 rows.

4. **Config file errors** (2 tests):
   - Call `JobConfig::from_file` with nonexistent path -- verify error
   - Call `JobConfig::from_json` with malformed JSON that has extra fields -- verify it still parses (serde default behavior)

5. **Full async pipeline with postprocessing error** (1 test): Run `process_netcdf_job_async` with a config where postprocessing references nonexistent column. Verify error propagation.

6. **Multiple postprocessors chained correctly** (1 test): Rename -> Formula (using new name) -> UnitConvert. Verify the entire chain works and produces correct values.

7. **Large-ish extraction** (1 test): Extract ALL data from pres_temp_4D.nc (no filters) with postprocessing. Verify row count (2*2*6\*12=288) and column names.

### Inputs/Props

- Test NetCDF files: `examples/data/pres_temp_4D.nc`, `examples/data/simple_xy.nc`
- Programmatic `JobConfig` construction

### Outputs/Behavior

Each test verifies either successful output with correct data or specific error types.

### Error Handling

Error tests should verify the error type (not just that an error occurred) and where possible check the error message content.

## Acceptance Criteria

- [ ] Given a pipeline where processor 2 references a column renamed by processor 1's old name, when the pipeline executes, then it returns PostProcessError::ColumnNotFound
- [ ] Given a DatetimeConvert config with base="not-a-date", when ProcessingPipeline::from_config is called, then it returns ConfigurationError
- [ ] Given a range filter with min=90 max=100 on latitude (data range 25-50), when the full pipeline runs, then the output file has 0 data rows
- [ ] Given JobConfig::from_file with "nonexistent.json", when called, then it returns Err
- [ ] Given process_netcdf_job_async with invalid postprocessing, when called, then it returns Err
- [ ] Given all new tests, when `cargo test --lib` runs, then all pass

## Implementation Guide

### Suggested Approach

Add tests to `src/tests/test_integration.rs`:

```rust
#[test]
fn test_pipeline_failure_mid_execution() {
    let dir = create_temp_output_dir();
    let output = dir.path().join("fail.parquet");

    let config = JobConfig {
        nc_key: get_test_data_path("simple_xy.nc").to_string_lossy().to_string(),
        variable_name: "data".to_string(),
        parquet_key: output.to_string_lossy().to_string(),
        filters: vec![],
        postprocessing: Some(ProcessingPipelineConfig {
            name: Some("Failing Pipeline".to_string()),
            processors: vec![
                ProcessorConfig::RenameColumns {
                    mappings: {
                        let mut map = HashMap::new();
                        map.insert("data".to_string(), "renamed_data".to_string());
                        map
                    },
                },
                // This will fail because "data" was already renamed
                ProcessorConfig::UnitConvert {
                    column: "data".to_string(),
                    from_unit: "kelvin".to_string(),
                    to_unit: "celsius".to_string(),
                },
            ],
        }),
    };

    let result = crate::process_netcdf_job(&config);
    assert!(result.is_err());
    assert!(!output.exists(), "Output should not be created on error");
}

#[test]
fn test_empty_filter_result_pipeline() -> Result<(), Box<dyn std::error::Error>> {
    let dir = create_temp_output_dir();
    let output = dir.path().join("empty_result.parquet");

    let config = JobConfig {
        nc_key: get_test_data_path("pres_temp_4D.nc").to_string_lossy().to_string(),
        variable_name: "temperature".to_string(),
        parquet_key: output.to_string_lossy().to_string(),
        filters: vec![FilterConfig::Range {
            params: RangeParams {
                dimension_name: "latitude".to_string(),
                min_value: 90.0,  // Beyond data range (25-50)
                max_value: 100.0,
            },
        }],
        postprocessing: None,
    };

    crate::process_netcdf_job(&config)?;
    // File should exist but with 0 data rows
    assert!(output.exists());
    Ok(())
}
```

### Key Files to Modify

- `/home/rogerio/git/nc2parquet/src/tests/test_integration.rs` -- add new integration tests

### Patterns to Follow

- Use `create_temp_output_dir()` for all outputs
- Construct `JobConfig` programmatically (not from JSON) for precise control
- Check both error occurrence AND error message content where possible
- Use `crate::process_netcdf_job` for sync tests and `crate::process_netcdf_job_async` for async tests

### Pitfalls to Avoid

- `process_netcdf_job` writes the parquet file BEFORE postprocessing failure can prevent it -- actually, the pipeline creates the DataFrame first, then postprocesses, then writes. So a postprocessing failure will prevent file creation. Verify this behavior.
- Empty DataFrame extraction (0 rows) may or may not produce a valid Parquet file depending on Polars behavior -- test empirically
- `JobConfig::from_file` returns `Box<dyn Error>`, not a specific error type -- use `is_err()` check

## Testing Requirements

### Unit Tests

None (this ticket is integration-level).

### Integration Tests

~8 new integration tests as described.

### E2E Tests

None.

## Dependencies

- **Blocked By**: ticket-002 (test reorganization must be complete)
- **Blocks**: None

## Effort Estimate

**Points**: 3
**Confidence**: High
