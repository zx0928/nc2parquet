# ticket-005 Add Unit Tests for Postprocess Module Edge Cases

## Context

### Background

The postprocess module (`src/postprocess.rs`, ~956 lines) implements `PostProcessor` trait with 5 processors: `ColumnRenamer`, `DateTimeConverter`, `UnitConverter`, `Aggregator`, and `FormulaApplier`. It also has a recursive descent formula parser. Current tests cover basic happy paths but miss many edge cases, especially in the formula parser and unit converter.

### Relation to Epic

Fifth ticket in Epic 01. Comprehensive postprocessor coverage is critical because post-processing is the most complex module with the most potential for subtle bugs (especially the formula parser).

### Current State

Existing postprocess tests (14 in `tests.rs::postprocess_tests`):

- ColumnRenamer: basic rename, skip nonexistent column
- UnitConverter: K->C, multiplication factor, column not found
- Aggregator: group_by with mean and max
- FormulaApplier: addition, sqrt
- Pipeline: chain rename + convert
- Config: create from config, pipeline from config
- DateTimeConverter: hours, days, seconds, column not found

**Missing coverage**:

- UnitConverter: C->K, C->F, F->C, K->F (through C), same-unit conversion, unknown unit pair
- FormulaApplier: subtraction, division, multiplication, nested parentheses, multiple operators with precedence (a+b\*c), comparison operators (==, !=, <, >, <=, >=), constant-only formula, column-copy formula, invalid formula (e.g., unclosed paren, empty operand), formula with nonexistent source column
- Aggregator: global aggregation (no group_by), all 8 aggregation ops individually, aggregation with nonexistent group_by column, empty DataFrame
- DateTimeConverter: milliseconds, microseconds, nanoseconds, invalid base datetime config
- ProcessingPipeline: empty pipeline (passthrough), pipeline name accessor, Default impl
- `create_processor` with all ProcessorConfig variants
- `create_pipeline` from slice of configs
- PostProcessError: Display impl for all variants
- Schema validation: `validate_schema` and `output_schema` methods

## Specification

### Requirements

Add test cases organized by processor:

**UnitConverter** (6 tests):

1. Celsius to Kelvin conversion
2. Celsius to Fahrenheit conversion
3. Fahrenheit to Celsius conversion
4. Unknown unit pair falls back to factor=1.0 (no-op)
5. Short unit names ("k", "c", "f")
6. Case insensitivity ("KELVIN", "Celsius")

**FormulaApplier** (10 tests):

1. Subtraction formula: `"a - b"`
2. Multiplication formula: `"a * 2.0"`
3. Division formula: `"a / b"`
4. Operator precedence: `"a + b * c"` should compute `a + (b * c)`
5. Parenthesized expression: `"(a + b) * c"`
6. Comparison formula: `"a > 5.0"` produces boolean column
7. Constant formula: `"42.0"` assigns constant to all rows
8. Invalid formula (unclosed paren or empty operand) returns error
9. Formula referencing nonexistent column returns ColumnNotFound-like error
10. Nested function: `"sqrt(value)"` with negative input (should produce NaN, not error)

**Aggregator** (4 tests):

1. All individual aggregation ops (Sum, Min, Max, Count, Std, Var, First, Last) on a known dataset
2. Global aggregation without group_by
3. Nonexistent group_by column returns ColumnNotFound error
4. Nonexistent aggregation column returns ColumnNotFound error

**Pipeline** (3 tests):

1. Empty pipeline returns DataFrame unchanged
2. `ProcessingPipeline::default()` creates empty pipeline
3. Pipeline `name()` accessor returns correct name

**PostProcessError** (1 test):

1. Display format for all 5 error variants

### Inputs/Props

Test DataFrames created with `create_simple_test_dataframe()` and `create_weather_test_dataframe()` from test_helpers.

### Outputs/Behavior

Each test asserts specific column values, error types, or DataFrame shapes.

### Error Handling

- Column not found errors should match `PostProcessError::ColumnNotFound`
- Invalid formula errors should match `PostProcessError::ProcessingError`
- Invalid datetime config should match `PostProcessError::ConfigurationError`

## Acceptance Criteria

- [ ] Given UnitConverter with from="celsius", to="kelvin", when processing [0.0, 100.0], then result is [273.15, 373.15]
- [ ] Given FormulaApplier with formula "a + b \* c" and columns a=1, b=2, c=3, when processed, then result is 7.0 (not 9.0)
- [ ] Given FormulaApplier with formula "(a + b) \* c", when processed, then result is 9.0
- [ ] Given an empty ProcessingPipeline, when execute is called, then original DataFrame is returned unchanged
- [ ] Given FormulaApplier with unclosed paren formula, when processed, then returns PostProcessError
- [ ] Given Aggregator with global aggregation (empty group_by), when processed, then returns 1-row DataFrame
- [ ] Given all new tests, when `cargo test --lib` runs, then all pass

## Implementation Guide

### Suggested Approach

Add tests in `src/tests/test_postprocess.rs`. Example for operator precedence:

```rust
#[test]
fn test_formula_operator_precedence() {
    let df = df! {
        "a" => [1.0],
        "b" => [2.0],
        "c" => [3.0],
    }.unwrap();

    let processor = FormulaApplier::new(
        "result".to_string(),
        "a + b * c".to_string(),
        vec!["a".to_string(), "b".to_string(), "c".to_string()],
    );

    let result = processor.process(df).unwrap();
    let col = result.column("result").unwrap();
    let val = col.f64().unwrap().get(0).unwrap();
    // Should be 1 + (2*3) = 7, not (1+2)*3 = 9
    assert!((val - 7.0).abs() < 1e-10);
}
```

### Key Files to Modify

- `/home/rogerio/git/nc2parquet/src/tests/test_postprocess.rs` -- add new tests

### Patterns to Follow

- Use `df!` macro for creating small inline DataFrames
- Assert floating point values with epsilon tolerance `(val - expected).abs() < 1e-10`
- Match error variants explicitly with `if let Err(PostProcessError::...) = result`

### Pitfalls to Avoid

- The formula parser splits on `+` and `-` at depth 0 from LEFT TO RIGHT, which means `a - b - c` is parsed as `(a - b) - c` -- this is correct but verify
- Comparison formulas return boolean columns, not f64 -- use `bool()` accessor
- `sqrt()` of negative number in Polars produces `NaN`, not an error -- the test should expect NaN
- The `create_processor` for `DatetimeConvert` validates the base datetime at creation time, so an invalid base string causes `PostProcessError::ConfigurationError` during creation, not during processing

## Testing Requirements

### Unit Tests

~24 new tests as described above.

### Integration Tests

None for this ticket.

### E2E Tests

None.

## Dependencies

- **Blocked By**: ticket-001 (test helpers)
- **Blocks**: ticket-007 (property-based tests for formula parser)

## Effort Estimate

**Points**: 3
**Confidence**: High
