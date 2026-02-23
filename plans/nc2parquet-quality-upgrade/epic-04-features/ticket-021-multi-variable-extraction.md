# ticket-021 Support Multi-Variable Extraction in Single Pass

## Context

### Background

Currently, `process_netcdf_job` extracts exactly one variable per invocation. The `JobConfig.variable_name` field is a single `String`, and the extraction pipeline in `extract_data_to_dataframe` opens the file, reads one variable, closes the file, and writes one Parquet file. Users who need temperature, pressure, and humidity from the same file must run the tool three times and merge outputs manually. Multi-variable extraction avoids repeated file I/O and produces a single output with all requested variables as columns, which is the natural format for downstream analysis in Polars, DuckDB, or pandas.

### Relation to Epic

This is the most architecturally impactful ticket in Epic 04. It requires modifications to the extraction pipeline, the job configuration, and the pipeline orchestration in `lib.rs`. However, it does NOT change filters, postprocessing, or output -- it only adds a loop over variables and a DataFrame join before those stages. The scoped-block pattern for NetCDF file lifetime (documented in learnings) must be carefully preserved: the file must remain open across all variable reads, then be closed before postprocessing.

### Current State

- `/home/rogerio/git/nc2parquet/src/input.rs`: `JobConfig.variable_name: String` (single variable)
- `/home/rogerio/git/nc2parquet/src/lib.rs` `process_netcdf_job()` (line 33-63): Opens file, gets one variable, builds filters, extracts to DataFrame, drops `var`, closes file, postprocesses, writes Parquet
- `/home/rogerio/git/nc2parquet/src/extract.rs` `extract_data_to_dataframe()` (line 365-377): Takes `&netcdf::File, &netcdf::Variable, &str, &Vec<Box<dyn NCFilter>>` and returns one `DataFrame` with dimension columns + one variable column
- The borrow checker requires explicit `drop(var)` before `file.close()` because the `var` reference borrows from `file` (documented in learnings)
- `build_dataframe()` (line 559-576) assembles dimension columns + one variable column
- Dimension columns are shared across variables from the same file (e.g., latitude, longitude, time appear once regardless of how many variables are extracted)

## Specification

### Requirements

1. Add an optional `variable_names: Option<Vec<String>>` field to `JobConfig` alongside the existing `variable_name: String`. When `variable_names` is `Some(names)`, it takes precedence. When `None`, fall back to the single `variable_name`. This preserves full backward compatibility.

2. Add a `--variables` CLI flag (alias `-N`) that accepts comma-separated variable names:

   ```
   nc2parquet convert input.nc output.parquet --variables "temperature,pressure,humidity"
   ```

   When `--variables` is provided, it populates `variable_names`. The existing `-n` / `--variable` flag continues to work for single-variable mode.

3. Multi-variable extraction produces a single `DataFrame` where:
   - Dimension columns (latitude, longitude, time, level, etc.) appear once
   - Each extracted variable appears as a separate column
   - All variables must share the same set of dimensions (same dimension names and sizes). Variables with different dimensions produce an error.

4. The extraction flow for multi-variable mode:
   - Open the NetCDF file once
   - For the first variable: run filters, extract to DataFrame (dimensions + variable column)
   - For each subsequent variable: validate it has the same dimensions, extract its values using the same dimension index manager, add the variable column to the DataFrame
   - Close the file
   - Run postprocessing on the combined DataFrame
   - Write one Parquet file

5. Filters apply uniformly to all variables (they operate on dimension coordinates, which are shared).

6. Postprocessors can reference any variable column by name in the combined DataFrame.

### Inputs/Props

- `JobConfig.variable_names: Option<Vec<String>>` -- new field, `#[serde(skip_serializing_if = "Option::is_none")]`
- `--variables "name1,name2,..."` -- new CLI flag, comma-separated
- When both `--variable` and `--variables` are provided, `--variables` takes precedence and `--variable` is ignored (with a debug log warning)

### Outputs/Behavior

- Single DataFrame with schema: `[dim1, dim2, ..., dimN, var1, var2, ..., varM]`
- Single Parquet file containing all variables
- Log messages: `"Extracting {N} variables: {names}"`

### Error Handling

- Variable not found: `Nc2ParquetError::VariableNotFound(name)` for any variable in the list
- Dimension mismatch: `Nc2ParquetError::Extraction(format!("Variable '{}' has dimensions {:?} but expected {:?} (matching first variable '{}')", ...))`
- Empty variable list: `Nc2ParquetError::Config("No variables specified".into())`
- Neither `variable_name` nor `variable_names` provided: existing error path

## Acceptance Criteria

- [ ] Given a single `variable_name` in config (no `variable_names`), when `process_netcdf_job` is called, then behavior is identical to current (backward compatible)
- [ ] Given `variable_names: Some(vec!["temperature", "pressure"])` on `pres_temp_4D.nc`, when processed, then the output DataFrame has columns: time, level, latitude, longitude, temperature, pressure
- [ ] Given `variable_names: Some(vec!["temperature"])` (single name in list), when processed, then it works identically to `variable_name: "temperature"`
- [ ] Given `variable_names` with a non-existent variable name, when processed, then `Nc2ParquetError::VariableNotFound` is returned
- [ ] Given two variables with different dimension sets, when processed, then `Nc2ParquetError::Extraction` is returned with a dimension mismatch message
- [ ] Given `--variables "temperature,pressure"` on the CLI with `pres_temp_4D.nc`, when executed, then the output Parquet file contains both variable columns
- [ ] Given postprocessing config that references "temperature" column in a multi-variable extraction, when the pipeline runs, then postprocessing operates on the combined DataFrame correctly
- [ ] Given both `--variable temp` and `--variables "temperature,pressure"`, when executed, then `--variables` takes precedence and both temperature and pressure are extracted

## Implementation Guide

### Suggested Approach

1. **Extend JobConfig**: In `/home/rogerio/git/nc2parquet/src/input.rs`, add:

   ```rust
   #[serde(skip_serializing_if = "Option::is_none")]
   pub variable_names: Option<Vec<String>>,
   ```

   Add a helper method:

   ```rust
   impl JobConfig {
       pub fn effective_variable_names(&self) -> Vec<String> {
           if let Some(ref names) = self.variable_names {
               names.clone()
           } else {
               vec![self.variable_name.clone()]
           }
       }
   }
   ```

2. **Add multi-variable extraction function**: In `/home/rogerio/git/nc2parquet/src/extract.rs`, add:

   ```rust
   pub(crate) fn extract_multi_variable_dataframe(
       file: &netcdf::File,
       var_names: &[String],
       filters: &[Box<dyn NCFilter>],
   ) -> Result<DataFrame, Nc2ParquetError>
   ```

   This function:
   - Gets the first variable, builds `DimensionIndexManager` with filters
   - Extracts the first variable to a DataFrame (dimensions + var1)
   - For each subsequent variable: validates dimensions match, extracts values only (not dimensions), adds column to DataFrame via `df.with_column(Series::new(name, values))`
   - The key insight is that `DimensionIndexManager` and the dimension columns are shared -- only the variable values differ

3. **Refactor lib.rs**: In `process_netcdf_job`, detect multi-variable mode:

   ```rust
   let var_names = config.effective_variable_names();
   let mut df = {
       let file = netcdf::open(&config.nc_key)?;
       let mut filters = Vec::new();
       for fc in &config.filters { filters.push(fc.to_filter()?); }
       let df = if var_names.len() == 1 {
           // existing single-variable path
       } else {
           extract_multi_variable_dataframe(&file, &var_names, &filters)?
       };
       file.close()?;
       df
   };
   ```

   Note: no `drop(var)` needed in multi-variable path because variable borrows are scoped within the extraction function.

4. **CLI flag**: In `Commands::Convert`, add:

   ```rust
   #[arg(short = 'N', long = "variables", value_delimiter = ',')]
   variables: Vec<String>,
   ```

   In the handler, if `variables` is non-empty, set `config.variable_names = Some(variables)`.

5. **Dimension validation**: Before extracting the second variable, compare its dimensions (names and sizes) against the first. Use `var.dimensions()` to get dimension metadata.

### Key Files to Modify

- `/home/rogerio/git/nc2parquet/src/input.rs` -- Add `variable_names` field to `JobConfig`, add `effective_variable_names()` method
- `/home/rogerio/git/nc2parquet/src/extract.rs` -- Add `extract_multi_variable_dataframe()` function
- `/home/rogerio/git/nc2parquet/src/lib.rs` -- Update `process_netcdf_job` and `process_netcdf_job_async` to handle multi-variable mode
- `/home/rogerio/git/nc2parquet/src/cli.rs` -- Add `--variables` flag
- `/home/rogerio/git/nc2parquet/src/handlers/convert.rs` -- Thread `--variables` to `JobConfig`

### Patterns to Follow

- Follow the scoped-block pattern for NetCDF file lifetime from `process_netcdf_job` (learnings: "Scoped NetCDF file lifetime")
- Follow the existing `extract_data_to_dataframe` signature pattern for the new multi-variable function
- Follow the `DimensionIndexManager` pattern: build it once from the first variable, reuse it for all subsequent variables
- Follow the serde `skip_serializing_if` pattern used on `postprocessing` field in `JobConfig`

### Pitfalls to Avoid

- Do NOT keep individual `&netcdf::Variable` references alive across the loop -- each variable borrow must be scoped tightly within its extraction step, otherwise the borrow checker will prevent `file.close()`
- The `extract_data_batch` path reads a slab via `var.get_values::<f32, _>(extents)`. For multi-variable, the same extents/strides/local_offsets apply but you call `get_values` on each variable separately. Do NOT re-read the dimension data for every variable -- extract dimensions once and reuse.
- `pres_temp_4D.nc` has variables "temperature" and "pressure" with identical dimensions -- this is the primary test fixture
- `simple_xy.nc` has only one variable "data" -- test that multi-variable with a single name still works
- The `build_dataframe()` function currently takes a single `var_name: &str` and `variable_values: Vec<f32>`. For multi-variable, you need to either call it once for the first variable and then add columns, or create a new builder that accepts multiple variable columns. The former approach is simpler and recommended.
- `variable_name` field must remain in `JobConfig` for backward compatibility. Do NOT remove it or make it optional. Deserialization of existing configs must continue to work.

## Testing Requirements

### Unit Tests

1. **Effective variable names**: Test `effective_variable_names()` returns single name when `variable_names` is `None`, and the list when `Some`
2. **Dimension validation**: Test that mismatched dimensions produce the correct error

### Integration Tests

Add tests in `/home/rogerio/git/nc2parquet/src/tests/test_integration.rs`:

1. **Multi-variable happy path**: Use `pres_temp_4D.nc`, extract both "temperature" and "pressure", verify DataFrame has columns: time, level, latitude, longitude, temperature, pressure
2. **Single variable via list**: Extract `["temperature"]` via `variable_names`, verify identical result to `variable_name: "temperature"`
3. **Variable not found**: Extract `["temperature", "nonexistent"]`, verify `VariableNotFound` error
4. **Backward compatibility**: Existing single-variable configs (no `variable_names` field) still work

## Dependencies

- **Blocked By**: ticket-010 (error types -- completed), ticket-014 (benchmarks -- completed, for performance regression checks)
- **Blocks**: ticket-028 (usage tutorials will cover multi-variable extraction)

## Effort Estimate

**Points**: 4
**Confidence**: Medium (the borrow checker interaction with multi-variable NetCDF file reads may require iteration on the scoping approach)
