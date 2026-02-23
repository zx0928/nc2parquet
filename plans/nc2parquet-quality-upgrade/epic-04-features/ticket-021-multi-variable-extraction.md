# ticket-021 Support Multi-Variable Extraction in Single Pass

> **[OUTLINE]** This ticket requires refinement before execution.
> It will be refined with learnings from earlier epics.

## Objective

Allow extracting multiple variables from a single NetCDF file in one pass, producing a single DataFrame (or multiple DataFrames) with all requested variables. Currently, `process_netcdf_job` extracts exactly one variable per invocation, requiring users to run the tool multiple times and merge outputs manually when they need temperature, pressure, and humidity from the same file.

## Anticipated Scope

- **Files likely to be modified**:
  - `/home/rogerio/git/nc2parquet/src/input.rs` -- extend `JobConfig` to accept `variable_names: Vec<String>` alongside or replacing `variable_name: String`
  - `/home/rogerio/git/nc2parquet/src/extract.rs` -- modify extraction to handle multiple variables, join on shared dimensions
  - `/home/rogerio/git/nc2parquet/src/lib.rs` -- update pipeline orchestration for multi-variable flow
  - `/home/rogerio/git/nc2parquet/src/cli.rs` -- add CLI support for specifying multiple variables
- **Key decisions needed**:
  - Output format: one wide DataFrame with all variables as columns, or one DataFrame per variable?
  - How to handle variables with different dimension sets (e.g., temperature is [time, lat, lon] but surface_pressure is [time, lat, lon] -- same; but wind_gust is [time, height, lat, lon] -- different)
  - Backward compatibility: keep `variable_name` for single-variable mode and add `variable_names` for multi, or deprecate single?
  - How filters apply across variables: same filters for all, or per-variable filter config?
- **Open questions**:
  - What percentage of real use cases involve multi-variable extraction from the same file?
  - Can polars efficiently join DataFrames on multi-dimensional coordinate columns?
  - How should postprocessors reference specific variables in a multi-variable DataFrame?

## Dependencies

- **Blocked By**: ticket-010 (error types), ticket-014 (benchmarks to measure multi-variable performance)
- **Blocks**: ticket-028

## Effort Estimate

**Points**: 5
**Confidence**: Low (will be re-estimated during refinement)
