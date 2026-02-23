# ticket-004 Add Unit Tests for Extract Module Edge Cases

## Context

### Background

The extract module (`src/extract.rs`, ~376 lines) is responsible for `DimensionIndexManager` and `extract_data_to_dataframe`. It handles dimension index tracking, filter intersection, coordinate combination generation, and data extraction from NetCDF variables. Current tests cover basic 2D/4D data and single filter application but miss edge cases.

### Relation to Epic

Fourth ticket in Epic 01. Adds comprehensive edge-case coverage for the extraction logic, which is the core data path of the application.

### Current State

Existing extract tests (5 in `tests.rs::extract_tests`):

- `test_dimension_index_manager_with_simple_data` -- 2D variable dimensions
- `test_dimension_index_manager_with_4d_data` -- 4D variable dimensions
- `test_dimension_index_manager_filter_application` -- single filter result application
- `test_extract_data_to_dataframe_simple` -- no filters, all data
- `test_extract_data_to_dataframe_with_filter` -- range filter
- `test_extract_data_to_dataframe_with_spatial_filter` -- 2D point filter (in integration tests)

**Missing coverage**:

- `DimensionIndexManager::apply_filter_result` with `FilterResult::Pairs` (only tested through full pipeline)
- `DimensionIndexManager::apply_filter_result` with `FilterResult::Triplets`
- `DimensionIndexManager::apply_filter_result` with unknown dimension name (error path)
- Multiple filter intersection (two range filters on different dimensions)
- Multiple filter intersection (range + list on same variable)
- `get_all_coordinate_combinations` with explicit combinations set
- `generate_combinations` with all indices filtered to empty for one dimension (zero-row result)
- `extract_variable_value` with 1, 2, 3, 4 dimension indices (tested implicitly but not directly)
- `extract_data_to_dataframe` with empty filter vector on 4D data
- `extract_data_to_dataframe` when filter reduces result to 0 rows

## Specification

### Requirements

Add the following test cases:

1. **DimensionIndexManager construction** (1 test): Verify initial indices contain all values for each dimension
2. **Filter intersection** (3 tests): Apply two single filters to different dimensions and verify intersection narrows both; apply filter that empties a dimension; apply filter to unknown dimension (error)
3. **Pair/Triplet combinations** (2 tests): Apply pair filter result and verify explicit combinations are generated correctly; verify triplet filter application
4. **Zero-result extraction** (1 test): Apply filter that matches nothing, verify empty DataFrame (0 rows, correct columns)
5. **Full 4D extraction without filters** (1 test): Extract all data from pres_temp_4D.nc without filters, verify row count = 2*2*6\*12 = 288
6. **Multi-filter extraction** (1 test): Apply range + list filters and verify exact row count and column content

### Inputs/Props

- Test data files: `examples/data/pres_temp_4D.nc`, `examples/data/simple_xy.nc`

### Outputs/Behavior

Each test verifies dimension counts, index contents, DataFrame shapes, and error messages.

### Error Handling

- `apply_filter_result` with unknown dimension should return `Err` containing "Unknown dimension"
- `apply_explicit_pairs`/`apply_explicit_triplets` with unknown dimension should return `Err`

## Acceptance Criteria

- [ ] Given a DimensionIndexManager for a 4D variable, when created, then it has 4 dimensions with indices 0..N for each
- [ ] Given two range filters on latitude and longitude applied sequentially, when combinations are generated, then only valid intersections appear
- [ ] Given a filter that matches no indices, when `extract_data_to_dataframe` runs, then the resulting DataFrame has 0 rows and correct column names
- [ ] Given no filters on pres_temp_4D.nc temperature variable, when extraction runs, then DataFrame has 288 rows
- [ ] Given a filter applied to a nonexistent dimension, when `apply_filter_result` runs, then it returns Err
- [ ] Given all new tests are added, when `cargo test --lib` runs, then all tests pass

## Implementation Guide

### Suggested Approach

Add tests in `src/tests/test_extract.rs`:

```rust
#[test]
fn test_dimension_index_manager_initial_state() -> Result<(), Box<dyn std::error::Error>> {
    let file_path = get_test_data_path("pres_temp_4D.nc");
    let file = netcdf::open(&file_path)?;
    let var = file.variable("temperature").unwrap();
    let manager = DimensionIndexManager::new(&var)?;

    // time=2, level=2, latitude=6, longitude=12
    assert_eq!(manager.get_dimension_indices("time").unwrap().len(), 2);
    assert_eq!(manager.get_dimension_indices("level").unwrap().len(), 2);
    assert_eq!(manager.get_dimension_indices("latitude").unwrap().len(), 6);
    assert_eq!(manager.get_dimension_indices("longitude").unwrap().len(), 12);

    file.close()?;
    Ok(())
}

#[test]
fn test_apply_filter_unknown_dimension() -> Result<(), Box<dyn std::error::Error>> {
    let file_path = get_test_data_path("pres_temp_4D.nc");
    let file = netcdf::open(&file_path)?;
    let var = file.variable("temperature").unwrap();
    let mut manager = DimensionIndexManager::new(&var)?;

    let result = FilterResult::Single {
        dimension: "nonexistent".to_string(),
        indices: vec![0],
    };

    let err = manager.apply_filter_result(&result);
    assert!(err.is_err());
    assert!(err.unwrap_err().to_string().contains("Unknown dimension"));

    file.close()?;
    Ok(())
}
```

### Key Files to Modify

- `/home/rogerio/git/nc2parquet/src/tests/test_extract.rs` -- add new tests

### Patterns to Follow

- Open NetCDF files with `netcdf::open`, close with `file.close()?`
- Use `get_test_data_path` from test_helpers
- Return `Result<(), Box<dyn std::error::Error>>` for tests that can fail
- Check both row counts and column names on DataFrames

### Pitfalls to Avoid

- The `get_dimension_indices` method returns `Option<&HashSet<usize>>` -- use `.unwrap()` only when the dimension is known to exist
- `get_all_coordinate_combinations` returns `Vec<Vec<usize>>` -- each inner Vec has one index per dimension in order
- simple_xy.nc has 6x12 grid without coordinate variables (x and y dimensions have no corresponding variables), so coordinate values default to index values

## Testing Requirements

### Unit Tests

~9 new tests as described above.

### Integration Tests

None (covered by test_integration.rs).

### E2E Tests

None.

## Dependencies

- **Blocked By**: ticket-001 (test helpers)
- **Blocks**: None directly (but builds foundation for ticket-007)

## Effort Estimate

**Points**: 3
**Confidence**: High
