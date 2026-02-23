# ticket-011 Audit and Tighten Visibility Modifiers

## Context

### Background

Many struct fields and functions are currently `pub` that should be `pub(crate)` or private. Tightening visibility prevents accidental use of internal APIs and makes the public API surface clear for library consumers.

### Relation to Epic

Third ticket in Epic 02. Depends on error types being finalized (ticket-010) so visibility changes do not conflict with error type refactoring.

### Current State

Items that may have overly broad visibility:

- `NCRangeFilter`, `NCListFilter`, `NC2DPointFilter`, `NC3DPointFilter` fields are all `pub` -- should they be `pub(crate)` since construction goes through `new()`?
- `DimensionIndexManager` fields (`dimension_indices`, `dimension_order`, `explicit_combinations`) are private (correct)
- `PostProcessError` variants are `pub` (correct for library API)
- `ProcessingPipeline` field `processors` is private (correct)
- `ColumnRenamer`, `DateTimeConverter`, `UnitConverter`, `Aggregator`, `FormulaApplier` fields are `pub` or private (mixed)
- `filter_factory` is `pub` -- is it part of the public API?
- CLI types (`RangeFilterArg`, `ListFilterArg`, etc.) are `pub` -- should be `pub` since they are in the public `cli` module
- `CliConfig`, `CliOptions`, `ProgressConfig`, `ValidationConfig` in cli.rs -- are these used externally?

## Specification

### Requirements

1. Audit every `pub` item in the library (not main.rs/handlers) and classify as:
   - `pub` -- part of the documented public API
   - `pub(crate)` -- used across modules but not externally
   - private -- used only within the module

2. Change filter struct fields to `pub(crate)` since they are constructed via `new()`:
   - `NCRangeFilter::dimension_name`, `min_value`, `max_value`
   - `NCListFilter::dimension_name`, `values`
   - `NC2DPointFilter` fields
   - `NC3DPointFilter` fields

3. Keep public:
   - All trait definitions and methods
   - `JobConfig`, `FilterConfig`, `ProcessorConfig`, `ProcessingPipelineConfig`
   - `NCFilter` trait
   - `PostProcessor` trait
   - `StorageBackend` trait
   - `process_netcdf_job`, `process_netcdf_job_async`
   - Filter and processor constructors (`new()`)

4. Make `pub(crate)`:
   - `filter_factory` (only used internally for JSON-based filter creation)
   - `CliConfig`, `CliOptions`, `ProgressConfig`, `ValidationConfig` (binary-specific config, not part of library API)
   - Processor struct fields that are only accessed internally

5. Verify no external API surface is reduced incorrectly by checking that all public items are used in the documented examples in README and rustdoc

### Inputs/Props

No runtime changes.

### Outputs/Behavior

- Reduced public API surface
- Library consumers see only intentionally public items

### Error Handling

Not applicable.

## Acceptance Criteria

- [ ] Given filter struct fields, when inspected, then they are `pub(crate)` (not `pub`)
- [ ] Given `filter_factory`, when inspected, then it is `pub(crate)`
- [ ] Given all public items, when `cargo doc` is run, then only intentionally public items appear
- [ ] Given `cargo test --lib`, when run, then all tests pass
- [ ] Given `cargo clippy -- -D warnings`, when run, then zero warnings

## Implementation Guide

### Suggested Approach

1. Run `cargo doc --no-deps` and review the generated documentation to see the current public API
2. For each `pub` item, decide if it should be `pub(crate)` or remain `pub`
3. Change visibility one module at a time, running `cargo build` after each
4. Run `cargo test --lib` after all changes

### Key Files to Modify

- `/home/rogerio/git/nc2parquet/src/filters.rs` -- tighten struct field visibility
- `/home/rogerio/git/nc2parquet/src/postprocess.rs` -- tighten processor field visibility
- `/home/rogerio/git/nc2parquet/src/cli.rs` -- tighten CLI-specific config visibility

### Patterns to Follow

- `pub` for items explicitly designed for library consumers
- `pub(crate)` for items used across modules within the crate
- private for items used only within their module

### Pitfalls to Avoid

- Filter structs implement `Deserialize` and their fields need to be accessible for deserialization -- if `from_json` uses `serde_json::from_str`, the fields need to be at least accessible to serde. `pub(crate)` works because serde uses field-level access within the crate.
- Changing `pub` fields on `serde(Deserialize)` structs to `pub(crate)` should still work because serde derives work at the crate level
- Test code uses `filter.dimension_name` etc. -- since tests are within the crate, `pub(crate)` is fine

## Testing Requirements

### Unit Tests

No new tests. Existing tests verify access patterns.

### Integration Tests

`cargo test --lib` must pass.

### E2E Tests

None.

## Dependencies

- **Blocked By**: ticket-010 (error types finalized)
- **Blocks**: ticket-012

## Effort Estimate

**Points**: 2
**Confidence**: High
