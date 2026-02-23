# ticket-015 Implement Chunked NetCDF Reading for Large Files

## Context

### Background

The current `extract_data_to_dataframe` function in `/home/rogerio/git/nc2parquet/src/extract.rs` (line 245) reads all filtered coordinate combinations in a single pass, calling `extract_variable_value` once per combination to read individual f32 values via `var.get::<f32, _>(indices)`. For small files like the project's test fixtures (pres_temp_4D.nc: 288 cells), this works well. For large climate datasets (e.g., ERA5 with lat=721, lon=1440, time=8760 = 9+ billion cells for hourly annual data), the current approach has two problems:

1. `get_all_coordinate_combinations()` generates the entire `Vec<Vec<usize>>` in memory before any reading begins (one Vec per output row)
2. Each `extract_variable_value` call performs an independent NetCDF seek+read for a single f32 value, which is extremely slow for large extractions because of per-value I/O overhead

The netcdf 0.11 crate supports sliced reads via `var.get::<f32, _>(range_tuple)` where each element can be a `Range<usize>`, enabling batch reads of contiguous dimension slices. This ticket implements a chunked reading strategy that reads data in dimension-aligned slices rather than cell-by-cell.

### Relation to Epic

This is the primary I/O optimization ticket in Epic 03. It addresses the "large file processing does not OOM" success criterion and provides the chunked data access pattern that ticket-018 (memory optimization) builds upon.

### Current State

- `extract_data_with_dimension_manager` (line 259 of extract.rs) calls `get_all_coordinate_combinations()` which clones all explicit combinations or generates the full Cartesian product via `generate_combinations`
- Each combination is a `Vec<usize>` of dimension indices
- For each combination, `extract_variable_value` reads a **single f32 value** using `var.get::<f32, _>((i0, i1, i2, i3))`
- The netcdf 0.11 `Variable::get` method accepts `Extents` which can be constructed from `Range<usize>` for sliced reads, returning an `ArrayD<T>` (ndarray)
- Coordinate variables are read once upfront via `get_coordinate_variables`
- Output is a single Polars `DataFrame`

## Specification

### Requirements

1. Add a **batch reading path** to `extract_data_with_dimension_manager` that reads variable data in contiguous dimension slices rather than cell-by-cell
2. The batch path should be used when the extraction is an unfiltered or range-filtered Cartesian product (i.e., when `explicit_combinations` is `None` in `DimensionIndexManager`)
3. For explicit combinations (Pairs/Triplets), fall back to the existing cell-by-cell path since the access pattern is irregular
4. The batch read should extract one "slab" per outermost dimension index: for a 4D variable `(time, level, lat, lon)` with contiguous indices for the inner 3 dims, read `var.get::<f32, _>((t_idx..t_idx+1, level_range, lat_range, lon_range))` and then scatter the values into the output columns
5. When dimension indices are non-contiguous after filtering (e.g., range filter selects indices [2,3,5,6] skipping 4), group them into contiguous runs and issue one read per run
6. All existing tests must continue to pass -- the output DataFrame must be byte-identical for the same inputs regardless of which code path is taken
7. Add a helper method `DimensionIndexManager::is_cartesian_product(&self) -> bool` that returns `true` when `explicit_combinations.is_none()`
8. Add a helper method `DimensionIndexManager::sorted_dimension_indices(&self) -> Vec<(String, Vec<usize>)>` that returns the dimension names and their filtered index sets, sorted

### Inputs/Props

- Same inputs as current `extract_data_to_dataframe`: `netcdf::File`, `netcdf::Variable`, variable name, filters
- No new configuration parameters -- the chunked path is an internal optimization

### Outputs/Behavior

- Same output: `Result<DataFrame, Nc2ParquetError>` with identical schema and values
- For Cartesian product extractions, significantly reduced number of NetCDF read calls (from N to N/chunk_size)
- Peak memory is bounded by the size of one dimension slab rather than all combinations

### Error Handling

- `Nc2ParquetError::NetCdf` for NetCDF read errors (already existing error variant)
- `Nc2ParquetError::Extraction` if dimension index ranges cannot be constructed
- Batch path errors should produce the same error types as the cell-by-cell path for identical failure modes

## Acceptance Criteria

- [ ] Given a `DimensionIndexManager` with no explicit combinations (Cartesian product), when `is_cartesian_product()` is called, then it returns `true`
- [ ] Given a `DimensionIndexManager` with explicit pairs or triplets, when `is_cartesian_product()` is called, then it returns `false`
- [ ] Given `pres_temp_4D.nc` with no filters, when `extract_data_to_dataframe` is called, then the resulting DataFrame has 288 rows (2*2*6\*12) and is identical to the current implementation's output
- [ ] Given `pres_temp_4D.nc` with a range filter on latitude [30.0, 45.0], when `extract_data_to_dataframe` is called, then the result uses the batch path and produces the same DataFrame as the current cell-by-cell path
- [ ] Given `simple_xy.nc` (2D, no coordinate variables) with no filters, when `extract_data_to_dataframe` is called, then the batch path produces a 72-row DataFrame identical to the current output
- [ ] Given all 181 existing tests pass after the change, then the optimization is behavior-preserving
- [ ] Given the benchmark suite from ticket-014 exists, when `cargo bench --bench extraction_bench` is run before and after this change, then the 4D unfiltered extraction benchmark shows measurable improvement (less wall-clock time per iteration)

## Implementation Guide

### Suggested Approach

1. Add `is_cartesian_product` and `sorted_dimension_indices` methods to `DimensionIndexManager`
2. Create a new function `extract_data_batch` in `extract.rs` that handles the batch read path:
   ```rust
   fn extract_data_batch(
       file: &netcdf::File,
       var: &netcdf::Variable,
       var_name: &str,
       dim_manager: &DimensionIndexManager,
   ) -> Result<DataFrame, Nc2ParquetError>
   ```
3. In `extract_data_batch`:
   a. Get sorted indices for each dimension from `dim_manager`
   b. For each dimension, compute contiguous runs (e.g., [2,3,5,6] becomes [(2..4), (5..7)])
   c. Read one slab per combination of outermost-dimension contiguous runs
   d. For a 4D variable, the outermost loop is over time indices; for each time index, read the full inner-3D slab with one `var.get::<f32, _>((t..t+1, level_range, lat_range, lon_range))`
   e. Flatten the ndarray slab into coordinate+value rows
4. In `extract_data_with_dimension_manager`, dispatch:
   ```rust
   if dim_manager.is_cartesian_product() {
       extract_data_batch(file, var, var_name, dim_manager)
   } else {
       // existing cell-by-cell path
       extract_data_cellwise(file, var, var_name, dim_manager)
   }
   ```
5. Rename the existing body of `extract_data_with_dimension_manager` to `extract_data_cellwise` for the fallback path
6. Handle the case where a slab read returns a multi-dimensional ndarray: iterate over it in the same order as the Cartesian product would to ensure row ordering is identical

### Key Files to Modify

- `/home/rogerio/git/nc2parquet/src/extract.rs` -- add `is_cartesian_product`, `sorted_dimension_indices`, `extract_data_batch`, rename existing extraction to `extract_data_cellwise`, update dispatch in `extract_data_with_dimension_manager`

### Patterns to Follow

- The netcdf 0.11 `Variable::get` method accepts tuples of ranges for sliced reads: `var.get::<f32, _>((0..2, 0..6, 0..12))` returns an `ArrayD<f32>` of shape `[2, 6, 12]`
- Use `ndarray::ArrayD::iter()` for flat iteration in row-major (C) order, which matches the Cartesian product ordering used by `generate_combinations`
- Follow the existing pattern of `extract_variable_value` for dimensionality dispatch (match on indices.len() for 1/2/3/4 dims), but with range tuples instead of scalar indices
- Follow the established `pub(crate)` visibility pattern for new internal functions in extract.rs

### Pitfalls to Avoid

- The `var.get::<f32, _>()` call with range extents returns `ArrayD<f32>`, not a flat `Vec<f32>` -- you need to iterate it in the correct order
- Non-contiguous index sets after range filtering (e.g., latitude indices [0,1,3,4] when index 2 is filtered out) require multiple slab reads for that dimension. Group contiguous runs to minimize read calls.
- The dimension iteration order in the slab must match `dimension_order` from `DimensionIndexManager` to produce identical row ordering
- `simple_xy.nc` has no coordinate variables -- the existing fallback `unwrap_or(idx as f64)` in the coordinate lookup must also be used in the batch path
- Do NOT change the public API of `extract_data_to_dataframe` -- the batch optimization is purely internal

## Testing Requirements

### Unit Tests

- Test `is_cartesian_product` returns `true` for a fresh `DimensionIndexManager` with only `Single` filter results applied, and `false` after `Pairs` or `Triplets` are applied
- Test `sorted_dimension_indices` returns correctly sorted indices after applying a range filter
- Test that `extract_data_batch` produces identical output to `extract_data_cellwise` for both test fixtures with various filter configurations

### Integration Tests

- The existing 181 tests serve as regression tests. No new integration tests needed beyond confirming all pass.

### E2E Tests

Not applicable.

## Dependencies

- **Blocked By**: ticket-014 (benchmarks establish baseline before optimization)
- **Blocks**: ticket-018

## Effort Estimate

**Points**: 5
**Confidence**: Medium
