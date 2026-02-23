# ticket-007 Add Property-Based Tests for Filters and Formula Parser

## Context

### Background

Property-based testing uses random input generation to discover edge cases that hand-written tests miss. The filters module and formula parser are ideal candidates: filters have mathematical properties (range containment, set membership) and the formula parser handles arbitrary string inputs that could trigger parsing bugs.

### Relation to Epic

Seventh ticket in Epic 01. Uses the `proptest` dependency added in ticket-001 and builds on the filter/postprocess knowledge from tickets 003 and 005.

### Current State

- `proptest` is available as a dev-dependency (added in ticket-001)
- Filters module has comprehensive unit tests (from ticket-003)
- Formula parser has edge-case tests (from ticket-005)
- No property-based tests exist anywhere in the project

## Specification

### Requirements

Create property-based tests in a new file `src/tests/test_properties.rs`:

**Filter property tests** (4 strategies):

1. **Range filter construction**: For any `(min, max)` where `min < max`, `NCRangeFilter::new` creates a valid filter with correct fields
2. **List filter construction**: For any non-empty `Vec<f64>`, `NCListFilter::new` creates a valid filter
3. **FilterResult::Single invariant**: For any set of indices, `len()` equals `indices.len()` and `is_empty()` matches `indices.is_empty()`
4. **FilterResult::Pairs invariant**: For any set of pairs, `len()` equals `pairs.len()`

**Formula parser property tests** (3 strategies):

1. **Constant formulas**: For any `f64` constant (excluding NaN/Inf), parsing `"{constant}"` as a formula produces a column where all values equal that constant
2. **Column identity**: For any valid column name (alphanumeric + underscore), parsing it as a formula produces the original column values
3. **Arithmetic identity**: For formulas like `"col + 0"`, `"col * 1"`, `"col - 0"`, the result equals the original column values

**Filter factory property tests** (1 strategy):

1. **Unknown kinds rejected**: For any random string that is not "range", "list", "2d_point", or "3d_point", `filter_factory` returns an error

### Inputs/Props

- Proptest strategies generate random f64 values, strings, and vectors
- Small DataFrames (1-10 rows) with known column names for formula tests

### Outputs/Behavior

Property tests verify invariants hold for all generated inputs.

### Error Handling

- Invalid formula strings should not cause panics -- they should return `PostProcessError`
- Random string kinds in filter_factory should return error, not panic

## Acceptance Criteria

- [ ] Given proptest generates random (min, max) pairs where min < max, when NCRangeFilter is created, then fields match inputs
- [ ] Given proptest generates random Vec<f64>, when NCListFilter is created, then values field matches input
- [ ] Given proptest generates random indices, when FilterResult::Single is created, then len() equals indices.len()
- [ ] Given proptest generates random f64 constants, when FormulaApplier processes "{constant}", then all output values equal the constant
- [ ] Given proptest generates "col + 0" formulas, when processed, then result equals original column
- [ ] Given proptest generates random non-filter-kind strings, when filter_factory is called, then error is returned
- [ ] Given all property tests, when `cargo test --lib` runs, then all pass (including proptest cases)

## Implementation Guide

### Suggested Approach

Create `src/tests/test_properties.rs`:

```rust
use proptest::prelude::*;
use crate::filters::*;
use crate::postprocess::*;
use polars::prelude::*;

proptest! {
    #[test]
    fn range_filter_construction(
        min in -1000.0f64..0.0,
        max in 0.01f64..1000.0,
        dim_name in "[a-z]{1,10}",
    ) {
        let filter = NCRangeFilter::new(&dim_name, min, max);
        prop_assert_eq!(&filter.dimension_name, &dim_name);
        prop_assert_eq!(filter.min_value, min);
        prop_assert_eq!(filter.max_value, max);
    }

    #[test]
    fn list_filter_construction(
        values in prop::collection::vec(-1000.0f64..1000.0, 1..20),
        dim_name in "[a-z]{1,10}",
    ) {
        let filter = NCListFilter::new(&dim_name, values.clone());
        prop_assert_eq!(&filter.dimension_name, &dim_name);
        prop_assert_eq!(filter.values.len(), values.len());
    }

    #[test]
    fn filter_result_single_len_invariant(
        indices in prop::collection::vec(0usize..100, 0..50),
    ) {
        let result = FilterResult::Single {
            dimension: "test".to_string(),
            indices: indices.clone(),
        };
        prop_assert_eq!(result.len(), indices.len());
        prop_assert_eq!(result.is_empty(), indices.is_empty());
    }

    #[test]
    fn constant_formula_produces_constant(
        constant in -1e6f64..1e6,
    ) {
        // Skip NaN and Inf
        prop_assume!(constant.is_finite());

        let df = df! {
            "dummy" => [1.0, 2.0, 3.0],
        }.unwrap();

        let formula_str = format!("{}", constant);
        let processor = FormulaApplier::new(
            "result".to_string(),
            formula_str,
            vec!["dummy".to_string()],
        );

        let result = processor.process(df);
        // Should succeed (constants are valid formulas)
        if let Ok(result_df) = result {
            let col = result_df.column("result").unwrap();
            let vals: Vec<f64> = col.f64().unwrap().into_iter().map(|v| v.unwrap()).collect();
            for &v in &vals {
                prop_assert!((v - constant).abs() < 1e-6,
                    "Expected {}, got {}", constant, v);
            }
        }
        // If it fails, that's also acceptable for edge-case constants
    }

    #[test]
    fn unknown_filter_kind_rejected(
        kind in "[a-z]{1,20}".prop_filter("Not a valid kind",
            |s| !["range", "list", "2d_point", "3d_point"].contains(&s.as_str()))
    ) {
        let json = format!(r#"{{"kind": "{}"}}"#, kind);
        let result = filter_factory(&json);
        prop_assert!(result.is_err());
    }
}
```

Update `src/tests/mod.rs` to add `mod test_properties;`.

### Key Files to Modify

- `/home/rogerio/git/nc2parquet/src/tests/mod.rs` -- add `mod test_properties;`
- `/home/rogerio/git/nc2parquet/src/tests/test_properties.rs` -- CREATE

### Patterns to Follow

- Use `proptest!` macro for property tests
- Use `prop_assume!` to filter out invalid inputs (NaN, Inf)
- Use `prop_assert!` and `prop_assert_eq!` instead of `assert!`
- Keep generated inputs small (vectors under 50 elements, strings under 20 chars)

### Pitfalls to Avoid

- Proptest generates thousands of cases by default -- keep test data small to avoid slow tests
- The formula parser may legitimately fail on some generated constant formats (e.g., very large exponents) -- use `if let Ok(...)` to handle gracefully
- Proptest regex strategies like `"[a-z]{1,10}"` must be valid regex syntax
- Do not test filter application with proptest (requires real NetCDF files) -- only test construction and invariants

## Testing Requirements

### Unit Tests

~8 proptest test functions as described (each generates 256 test cases by default).

### Integration Tests

None.

### E2E Tests

None.

## Dependencies

- **Blocked By**: ticket-003 (filter tests), ticket-005 (postprocess tests)
- **Blocks**: None

## Effort Estimate

**Points**: 3
**Confidence**: Medium (proptest strategies may need tuning for edge cases)
