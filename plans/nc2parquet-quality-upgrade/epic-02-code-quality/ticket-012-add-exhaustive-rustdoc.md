# ticket-012 Add Exhaustive Rustdoc with Examples to All Public Items

## Context

### Background

Most public items have basic rustdoc but many lack usage examples. For a library targeting community adoption, every public function, struct, enum, and trait should have documentation with at least one code example.

### Relation to Epic

Fourth ticket in Epic 02. Depends on visibility audit (ticket-011) so we only document items that are truly public.

### Current State

- `lib.rs`: `process_netcdf_job` and `process_netcdf_job_async` have good rustdoc with arguments/returns/errors sections but no `# Examples` blocks
- `filters.rs`: Module-level doc and `FilterResult` have docs. `NCFilter` trait has minimal docs. Individual filter types have no usage examples.
- `extract.rs`: `DimensionIndexManager` and `extract_data_to_dataframe` have good docs but no examples
- `postprocess.rs`: Module-level doc has one example. Individual processors lack examples.
- `input.rs`: `JobConfig` and `FilterConfig` have good docs. No examples for `from_file`/`from_json`.
- `output.rs`: Functions have good docs with arguments/returns/errors. No examples.
- `storage.rs`: Module-level doc has examples. `StorageBackend` trait has good method docs. `StorageFactory` has examples.
- `info.rs`: Minimal docs.
- `errors.rs`: Does not exist yet (created in ticket-010).

## Specification

### Requirements

1. Add `# Examples` section to all public functions that can be demonstrated without a live NetCDF file:
   - `JobConfig::from_json` -- show JSON parsing
   - `FilterConfig::to_filter` -- show filter creation
   - `FilterConfig::kind` -- show kind string
   - `ProcessingPipeline::new`, `with_name`, `from_config` -- show pipeline creation
   - `ColumnRenamer::new` -- show renamer creation
   - `UnitConverter::new` -- show converter creation
   - `FormulaApplier::new` -- show formula creation
   - `create_processor`, `create_pipeline` -- show config-driven creation

2. Add `# Examples` with `no_run` attribute for functions requiring NetCDF files:
   - `process_netcdf_job` -- show basic usage
   - `process_netcdf_job_async` -- show async usage
   - `extract_data_to_dataframe` -- show extraction

3. Ensure all public structs have field-level documentation

4. Run `cargo doc --no-deps` and verify no warnings

5. Run `cargo test --doc` to verify all doc examples compile

### Inputs/Props

No runtime changes.

### Outputs/Behavior

- `cargo doc --no-deps` produces warning-free documentation
- `cargo test --doc` passes for all compilable examples

### Error Handling

Not applicable.

## Acceptance Criteria

- [ ] Given every public function in the library, when `cargo doc` is run, then each has an `# Examples` section
- [ ] Given `cargo doc --no-deps`, when run, then zero warnings about missing docs
- [ ] Given `cargo test --doc`, when run, then all doc tests pass
- [ ] Given `JobConfig::from_json` docs, when read, then they show a complete JSON parsing example
- [ ] Given `ProcessingPipeline` docs, when read, then they show pipeline creation and execution

## Implementation Guide

### Suggested Approach

Work through each public module adding examples. Use `/// # Examples` format:

````rust
/// Creates a new job configuration from a JSON string.
///
/// # Examples
///
/// ```rust
/// use nc2parquet::input::JobConfig;
///
/// let json = r#"
/// {
///     "nc_key": "input.nc",
///     "variable_name": "temperature",
///     "parquet_key": "output.parquet",
///     "filters": []
/// }
/// "#;
///
/// let config = JobConfig::from_json(json).unwrap();
/// assert_eq!(config.variable_name, "temperature");
/// ```
pub fn from_json(json_str: &str) -> Result<Self, Nc2ParquetError> {
````

For functions that need a NetCDF file, use `no_run`:

````rust
/// # Examples
///
/// ```rust,no_run
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// use nc2parquet::process_netcdf_job;
/// use nc2parquet::input::JobConfig;
///
/// let config = JobConfig::from_json(r#"..."#)?;
/// process_netcdf_job(&config)?;
/// # Ok(())
/// # }
/// ```
````

### Key Files to Modify

- `/home/rogerio/git/nc2parquet/src/lib.rs` -- add examples to pipeline functions
- `/home/rogerio/git/nc2parquet/src/input.rs` -- add examples to JobConfig, FilterConfig
- `/home/rogerio/git/nc2parquet/src/filters.rs` -- add examples to NCFilter, filter types
- `/home/rogerio/git/nc2parquet/src/postprocess.rs` -- add examples to processors
- `/home/rogerio/git/nc2parquet/src/extract.rs` -- add no_run examples
- `/home/rogerio/git/nc2parquet/src/output.rs` -- add no_run examples
- `/home/rogerio/git/nc2parquet/src/info.rs` -- add no_run examples
- `/home/rogerio/git/nc2parquet/src/errors.rs` -- document all error variants

### Patterns to Follow

- Use `/// # Examples` consistently
- Use `rust,no_run` for examples requiring external resources
- Use `# fn main() -> Result<...>` wrapper for fallible examples
- Document all struct fields with `///` comments

### Pitfalls to Avoid

- Doc examples are compiled and run by `cargo test --doc` -- ensure they compile
- The `no_run` attribute prevents execution but still compiles -- useful for NetCDF-dependent code
- Avoid examples that depend on specific file paths existing

## Testing Requirements

### Unit Tests

No new unit tests.

### Integration Tests

`cargo test --doc` must pass.

### E2E Tests

None.

## Dependencies

- **Blocked By**: ticket-011 (visibility audit)
- **Blocks**: ticket-013

## Effort Estimate

**Points**: 3
**Confidence**: High
