# ticket-016 Reduce Allocation Overhead in Extraction Pipeline

> **[OUTLINE]** This ticket requires refinement before execution.
> It will be refined with learnings from earlier epics.

## Objective

Profile and reduce unnecessary heap allocations in the extraction pipeline, particularly in `DimensionIndexManager::generate_coordinate_combinations` and `extract_data_to_dataframe`. The current implementation creates multiple intermediate Vec allocations during coordinate combination generation that could be reduced or eliminated with iterators and pre-allocated buffers.

## Anticipated Scope

- **Files likely to be modified**:
  - `/home/rogerio/git/nc2parquet/src/extract.rs` -- optimize `generate_coordinate_combinations`, reduce intermediate Vecs
  - `/home/rogerio/git/nc2parquet/src/filters.rs` -- potentially optimize FilterResult to avoid cloning large index vectors
- **Key decisions needed**:
  - Whether to use iterators/lazy evaluation instead of collecting into Vecs
  - Whether FilterResult should use `Cow<[usize]>` or `Arc<[usize]>` instead of `Vec<usize>` for shared index data
  - Trade-off between code readability and allocation reduction
- **Open questions**:
  - How many allocations does the current pipeline make for a typical 3D extraction? (needs profiling from ticket-014 baselines)
  - Is the bottleneck in allocation count, allocation size, or memory fragmentation?
  - Does polars DataFrame construction dominate total allocation cost, making extraction optimizations negligible?

## Dependencies

- **Blocked By**: ticket-014 (benchmarks needed to measure improvement)
- **Blocks**: ticket-018

## Effort Estimate

**Points**: 3
**Confidence**: Low (will be re-estimated during refinement)
