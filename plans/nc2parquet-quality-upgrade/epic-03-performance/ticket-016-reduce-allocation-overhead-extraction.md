# ticket-016 Reduce Allocation Overhead in Extraction Pipeline

## Context

### Background

The Epic 02 learnings explicitly identify `generate_combinations` (line 220 of `/home/rogerio/git/nc2parquet/src/extract.rs`) as the primary allocation hot path. This recursive function builds the Cartesian product of all filtered dimension indices by:

1. Cloning a `Vec<usize>` at every leaf node (`result.push(current.clone())` at line 227)
2. Sorting `HashSet<usize>` into `Vec<usize>` at every recursion level (line 233: `indices.iter().cloned().collect()` + `sort()`)
3. Allocating a new `Vec<usize>` per combination for the output

In `extract_data_with_dimension_manager`, additional allocations occur:

- `combination.clone()` at line 288 (clones each combination just to pass to `extract_variable_value`)
- `HashMap<String, Vec<f64>>` for column data that grows via `push()` without pre-allocation

For typical climate data extractions producing 10K-1M rows, these per-row allocations are a significant fraction of total extraction time.

### Relation to Epic

This is the CPU/allocation optimization counterpart to ticket-015 (I/O optimization). While ticket-015 reduces the number of NetCDF read calls, this ticket reduces the allocation overhead of coordinate combination generation and DataFrame construction. Together, they address the two main bottlenecks in the extraction pipeline. Ticket-018 builds on both.

### Current State

- `generate_combinations` at line 220 of `/home/rogerio/git/nc2parquet/src/extract.rs`:
  - Uses `HashSet<usize>` for dimension indices (unordered), converts to sorted `Vec<usize>` every recursion level
  - `current.clone()` at each leaf creates one heap allocation per output row
  - No pre-allocation of the result vector (grows via repeated `push`)
- `generate_combinations_with_pairs` at line 162:
  - `vec![0; self.dimension_order.len()]` at each leaf (line 174) -- one allocation per pair per other-dimension combination
- `extract_data_with_dimension_manager` at line 259:
  - `combinations` vector returned by `get_all_coordinate_combinations()` holds all `Vec<Vec<usize>>` in memory simultaneously
  - `data_columns` HashMap uses `Vec::push` without `with_capacity`
  - `variable_values` Vec uses `push` without `with_capacity`
  - `let indices: Vec<usize> = combination.clone()` at line 288 is unnecessary -- `extract_variable_value` only needs a `&[usize]`
- `DimensionIndexManager::dimension_indices` is `HashMap<String, HashSet<usize>>` -- the HashSet provides O(1) intersection but costs O(n log n) for each sorted iteration

## Specification

### Requirements

1. **Pre-allocate output vectors**: `get_all_coordinate_combinations()` should compute the total combination count before generating them and call `Vec::with_capacity`
2. **Eliminate redundant clone at line 288**: Change `extract_variable_value` to accept `&[usize]` directly from the combination slice instead of cloning
3. **Pre-allocate data column Vecs**: In `extract_data_with_dimension_manager`, compute `combinations.len()` and use `Vec::with_capacity(combinations.len())` for all column vectors and `variable_values`
4. **Cache sorted indices**: In `generate_combinations`, sort the `HashSet` to `Vec` once per dimension (not once per recursion) by pre-computing sorted index vectors before entering the recursion
5. **Use a flat buffer for combinations**: Instead of `Vec<Vec<usize>>`, use a flat `Vec<usize>` with stride = dimension_count to avoid per-row heap allocation. Provide an accessor method that returns `&[usize]` slices.
6. All existing tests must continue to pass with identical outputs
7. Benchmark from ticket-014 should show measurable improvement in extraction and combination benchmarks

### Inputs/Props

Same as `extract_data_to_dataframe` -- no interface changes.

### Outputs/Behavior

- Same `Result<DataFrame, Nc2ParquetError>` with identical schema and values
- Reduced heap allocation count and total allocated bytes during extraction
- Reduced CPU time for combination generation

### Error Handling

No changes to error handling. Same error types and variants as before.

## Acceptance Criteria

- [ ] Given `generate_combinations` is called for a 4D variable with dimensions of size [2, 2, 6, 12], when the result is inspected, then it contains exactly 288 combinations with the same values as the current implementation
- [ ] Given `extract_variable_value` is called with `&[usize]` (not `Vec<usize>`), when it processes a valid index, then it returns the same f32 value as before
- [ ] Given `extract_data_with_dimension_manager` processes `pres_temp_4D.nc`, when the resulting DataFrame is compared to the current output, then they are identical (same column names, same values, same row order)
- [ ] Given the flat buffer representation is used internally, when `get_all_coordinate_combinations()` is called, then it returns data equivalent to the old `Vec<Vec<usize>>` format (possibly via a new return type or accessor)
- [ ] Given all 181 existing tests pass after the change, then the optimization is behavior-preserving
- [ ] Given `cargo bench --bench combination_bench` is run before and after this change, then the benchmark shows reduced time per iteration (target: at least 20% improvement for the 4D unfiltered case)

## Implementation Guide

### Suggested Approach

1. **Flat combination buffer**: Replace the `Vec<Vec<usize>>` representation with a struct:

   ```rust
   pub(crate) struct CombinationBuffer {
       data: Vec<usize>,
       stride: usize, // number of dimensions
   }

   impl CombinationBuffer {
       fn with_capacity(num_combinations: usize, num_dimensions: usize) -> Self {
           Self {
               data: Vec::with_capacity(num_combinations * num_dimensions),
               stride: num_dimensions,
           }
       }
       fn push_combination(&mut self, combo: &[usize]) {
           self.data.extend_from_slice(combo);
       }
       fn get(&self, index: usize) -> &[usize] {
           &self.data[index * self.stride..(index + 1) * self.stride]
       }
       fn len(&self) -> usize {
           self.data.len() / self.stride
       }
       fn iter(&self) -> impl Iterator<Item = &[usize]> {
           self.data.chunks_exact(self.stride)
       }
   }
   ```

2. **Pre-compute sorted indices**: Before entering `generate_combinations`, build a `Vec<Vec<usize>>` of sorted indices per dimension (computed once):

   ```rust
   let sorted_dims: Vec<Vec<usize>> = self.dimension_order.iter()
       .map(|dim_name| {
           let mut indices: Vec<usize> = self.dimension_indices[dim_name].iter().cloned().collect();
           indices.sort_unstable();
           indices
       })
       .collect();
   ```

   Pass `&sorted_dims` into the recursion instead of looking up the `HashSet` each time.

3. **Pre-compute combination count**: The total is the product of all dimension index set sizes:

   ```rust
   let total_combinations: usize = self.dimension_order.iter()
       .map(|d| self.dimension_indices[d].len())
       .product();
   ```

   Use this for `CombinationBuffer::with_capacity`.

4. **Rewrite generate_combinations** to use a fixed-size `current` buffer (stack-allocated array if dimensions <= 4, heap otherwise) and write directly into `CombinationBuffer` without cloning:

   ```rust
   fn generate_combinations_flat(
       &self,
       sorted_dims: &[Vec<usize>],
       current: &mut [usize],
       dim_index: usize,
       result: &mut CombinationBuffer,
   ) {
       if dim_index >= sorted_dims.len() {
           result.push_combination(current);
           return;
       }
       for &idx in &sorted_dims[dim_index] {
           current[dim_index] = idx;
           self.generate_combinations_flat(sorted_dims, current, dim_index + 1, result);
       }
   }
   ```

5. **Remove unnecessary clone** at line 288: Change `extract_variable_value` signature from `fn extract_variable_value(var: &netcdf::Variable, indices: &[usize])` (it already takes `&[usize]` -- the clone is at the call site). Remove the `let indices: Vec<usize> = combination.clone();` line and pass the combination slice directly.

6. **Pre-allocate column vectors** in `extract_data_with_dimension_manager`:

   ```rust
   let num_rows = combinations.len();
   for dim_name in dimension_order {
       data_columns.insert(dim_name.clone(), Vec::with_capacity(num_rows));
   }
   let mut variable_values = Vec::with_capacity(num_rows);
   ```

7. **Update `get_all_coordinate_combinations`** to return `CombinationBuffer` instead of `Vec<Vec<usize>>`. Update all call sites (extract.rs only -- this is `pub(crate)`).

### Key Files to Modify

- `/home/rogerio/git/nc2parquet/src/extract.rs` -- all changes are in this file: new `CombinationBuffer` struct, refactored `generate_combinations`, pre-allocation in `extract_data_with_dimension_manager`, removal of redundant clones

### Patterns to Follow

- Follow the existing `pub(crate)` visibility pattern for `CombinationBuffer`
- Use `#[allow(dead_code)]` with justification comment if any `CombinationBuffer` methods are only used in tests
- Keep the `#[allow(clippy::too_many_arguments)]` on `generate_combinations_with_pairs` but consider whether the flat buffer approach simplifies its signature
- Follow the established convention of `sort_unstable()` over `sort()` for primitive types

### Pitfalls to Avoid

- Do NOT change the row ordering -- the sorted iteration order must remain identical for DataFrame comparison tests to pass
- Do NOT change the public API or `pub(crate)` API of `extract_data_to_dataframe` -- internal optimization only
- The `generate_combinations_with_pairs` function (line 162) also allocates heavily -- apply the same flat buffer pattern there, but be careful with the interleaving of pair indices and other dimension indices
- `apply_explicit_triplets` (line 125) creates one `vec![0; dim_count]` per triplet -- convert this to use `CombinationBuffer` too
- If ticket-015's batch read path is already implemented, ensure that `CombinationBuffer` is compatible with both the batch and cellwise extraction paths
- The `explicit_combinations` field on `DimensionIndexManager` currently stores `Option<Vec<Vec<usize>>>` -- update this to `Option<CombinationBuffer>` and adjust `apply_explicit_pairs` and `apply_explicit_triplets` accordingly

## Testing Requirements

### Unit Tests

- Test `CombinationBuffer::push_combination` and `CombinationBuffer::get` for correctness
- Test `CombinationBuffer::iter` produces the same sequence as `Vec<Vec<usize>>` iteration
- Test that `generate_combinations_flat` produces identical results to the old `generate_combinations` for the test fixtures
- Test pre-allocation: verify that `CombinationBuffer::with_capacity` does not reallocate during generation for known sizes

### Integration Tests

The existing 181 tests serve as regression tests.

### E2E Tests

Not applicable.

## Dependencies

- **Blocked By**: ticket-014 (benchmarks needed to measure improvement)
- **Blocks**: ticket-018

## Effort Estimate

**Points**: 3
**Confidence**: High
