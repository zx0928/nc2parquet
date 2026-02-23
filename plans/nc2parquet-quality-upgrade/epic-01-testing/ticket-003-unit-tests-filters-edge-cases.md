# ticket-003 Add Unit Tests for Filters Module Edge Cases

## Context

### Background

The filters module (`src/filters.rs`, ~365 lines) implements the `NCFilter` trait with 4 filter types: `NCRangeFilter`, `NCListFilter`, `NC2DPointFilter`, and `NC3DPointFilter`. It also provides `FilterResult` enum and `filter_factory`. Current tests cover basic creation and application with real NetCDF data but miss edge cases and error paths.

### Relation to Epic

This is the third ticket in Epic 01. It adds comprehensive edge-case coverage for the filters module, which is one of the most critical components in the pipeline.

### Current State

Existing filter tests (11 in `src/tests.rs::filter_tests`):

- `test_range_filter_creation` -- basic construction
- `test_range_filter_with_real_data` -- application on pres_temp_4D.nc
- `test_list_filter_creation` -- basic construction
- `test_list_filter_with_real_data` -- application on pres_temp_4D.nc
- `test_2d_point_filter_creation` -- basic construction
- `test_2d_point_filter_with_real_data` -- application on pres_temp_4D.nc
- `test_3d_point_filter_creation` -- basic construction
- `test_3d_point_filter_creation_only` -- creation without application
- `test_filter_result_single` -- FilterResult::Single methods
- `test_filter_result_pairs` -- FilterResult::Pairs methods
- `test_filter_result_empty` -- empty results

**Missing coverage**:

- `NCRangeFilter::from_json` -- not tested
- `NCListFilter::from_json` -- not tested
- `NC2DPointFilter::from_json` -- not tested
- `NC3DPointFilter::from_json` -- not tested
- `filter_factory` -- not tested at all
- `FilterResult::Triplets` accessor (`as_triplets`) -- not tested
- Filter application with no matching results (empty result set)
- Filter application with ALL indices matching (no filtering effect)
- Filter on dimension that does not exist (error path)
- `NCListFilter` with floating point precision issues
- `NC2DPointFilter` with zero tolerance (should match exact only)
- `NC3DPointFilter` with overlapping points that produce duplicates
- `FilterResult::len()` and `is_empty()` on Triplets

## Specification

### Requirements

Add the following test cases to `src/tests/test_filters.rs` (or as inline `#[cfg(test)]` in `src/filters.rs`):

1. **`from_json` tests** (4 tests): Test `from_json` on each filter type with valid JSON
2. **`filter_factory` tests** (5 tests): Test factory with each valid kind, unknown kind, missing kind field
3. **`FilterResult::Triplets`** (2 tests): Test `as_triplets`, `len`, `is_empty` on triplet results
4. **Range filter edge cases** (3 tests): Filter returning empty results, filter matching all values, filter on nonexistent dimension
5. **List filter edge cases** (3 tests): Filter with values not present in data, filter with all values matching, list filter with floating-point values that are very close but not exactly equal
6. **2D point filter edge cases** (2 tests): Filter with tolerance=0 (exact match only with real data), filter with no matching points
7. **3D point filter edge cases** (2 tests): Filter on file without time coordinate variable, filter with overlapping triplets

### Inputs/Props

- Test data files: `examples/data/pres_temp_4D.nc`, `examples/data/simple_xy.nc`
- JSON strings for `from_json` and `filter_factory` tests

### Outputs/Behavior

Each test asserts specific expected outcomes (result counts, error types, empty/non-empty results).

### Error Handling

Tests for error paths should verify that `Err` is returned with a meaningful error message (check `.to_string()` contains relevant text like "not found").

## Acceptance Criteria

- [ ] Given `NCRangeFilter::from_json` is called with valid JSON, when it runs, then it returns `Ok` with correct field values
- [ ] Given `filter_factory` is called with `"kind": "range"`, when it runs, then it returns a boxed NCRangeFilter
- [ ] Given `filter_factory` is called with `"kind": "unknown"`, when it runs, then it returns `Err` containing "Unknown filter kind"
- [ ] Given `filter_factory` is called with JSON missing the "kind" field, when it runs, then it returns `Err` containing "Missing 'kind' field"
- [ ] Given a range filter with min=100, max=200 applied to latitude (range 25-50), when it runs, then FilterResult has 0 indices
- [ ] Given a range filter with min=0, max=1000 applied to latitude, when it runs, then FilterResult contains ALL latitude indices
- [ ] Given a range filter applied to "nonexistent_dim", when it runs, then it returns Err
- [ ] Given a list filter with values [999.0, 888.0] applied to longitude, when it runs, then FilterResult has 0 indices
- [ ] Given `FilterResult::Triplets` with 3 triplets, when `as_triplets()` is called, then it returns Some with correct dimensions and 3 triplets
- [ ] Given `FilterResult::Triplets` with 0 triplets, when `is_empty()` is called, then it returns true
- [ ] Given all new tests are added, when `cargo test --lib` is run, then all tests pass including the original 97

## Implementation Guide

### Suggested Approach

Add tests in `src/tests/test_filters.rs` within new sub-modules:

```rust
mod from_json_tests {
    use crate::filters::*;

    #[test]
    fn test_range_filter_from_json() {
        let json = r#"{"dimension_name": "lat", "min_value": 10.0, "max_value": 50.0}"#;
        let filter = NCRangeFilter::from_json(json).unwrap();
        assert_eq!(filter.dimension_name, "lat");
        assert_eq!(filter.min_value, 10.0);
        assert_eq!(filter.max_value, 50.0);
    }
    // ... similar for NCListFilter, NC2DPointFilter, NC3DPointFilter
}

mod filter_factory_tests {
    use crate::filters::filter_factory;

    #[test]
    fn test_factory_range() {
        let json = r#"{"kind": "range", "dimension_name": "lat", "min_value": 10.0, "max_value": 50.0}"#;
        let filter = filter_factory(json).unwrap();
        // Can't inspect the concrete type easily, but we can verify it was created
    }

    #[test]
    fn test_factory_unknown_kind() {
        let json = r#"{"kind": "foobar"}"#;
        let err = filter_factory(json).unwrap_err();
        assert!(err.to_string().contains("Unknown filter kind"));
    }

    #[test]
    fn test_factory_missing_kind() {
        let json = r#"{"dimension_name": "lat"}"#;
        let err = filter_factory(json).unwrap_err();
        assert!(err.to_string().contains("Missing 'kind' field"));
    }
}
```

### Key Files to Modify

- `/home/rogerio/git/nc2parquet/src/tests/test_filters.rs` -- add new test modules

### Patterns to Follow

- Group related tests in sub-modules (`mod from_json_tests { ... }`)
- Use `crate::test_helpers::get_test_data_path` for file paths
- Test both success and error paths for every function
- Use `assert!(err.to_string().contains("..."))` for error message verification

### Pitfalls to Avoid

- The `from_json` methods on filter structs expect the inner JSON structure (no "kind" or "params" wrapper) -- this is different from `FilterConfig` deserialization
- `NC2DPointFilter::from_json` expects `points` as `[[f64, f64], ...]` -- use proper JSON arrays
- Do not modify the existing 11 filter tests; only add new ones

## Testing Requirements

### Unit Tests

~21 new tests as described in Requirements section.

### Integration Tests

None for this ticket.

### E2E Tests

None.

## Dependencies

- **Blocked By**: ticket-001 (test helpers)
- **Blocks**: ticket-007 (property-based tests for filters)

## Effort Estimate

**Points**: 3
**Confidence**: High
