# ticket-018 Profile and Optimize Peak Memory Usage

> **[OUTLINE]** This ticket requires refinement before execution.
> It will be refined with learnings from earlier epics.

## Objective

Profile the full conversion pipeline with representative workloads to identify peak memory usage points, then apply targeted optimizations to reduce the memory high-water mark. This is the final optimization ticket that applies learnings from tickets 015-017 and addresses any remaining memory hotspots.

## Anticipated Scope

- **Files likely to be modified**:
  - Any files identified by profiling as memory hotspots (likely extract.rs, output.rs, postprocess.rs)
  - `/home/rogerio/git/nc2parquet/src/output.rs` -- potentially optimize Parquet serialization buffer management
- **Key decisions needed**:
  - Profiling tool choice: DHAT (Valgrind), heaptrack, or jemalloc profiling via `MALLOC_CONF`
  - Whether to add a memory budget configuration option or keep optimization transparent
  - Whether to drop intermediate DataFrames eagerly to reduce peak memory
- **Open questions**:
  - What is the current peak memory for a 100MB NetCDF file with 3D extraction and 3 postprocessors?
  - Does the polars DataFrame hold references to the original NetCDF data or copy it?
  - Is there a significant difference between sync and async pipeline memory profiles?

## Dependencies

- **Blocked By**: ticket-015, ticket-016, ticket-017 (optimizations from these tickets affect memory profile)
- **Blocks**: None

## Effort Estimate

**Points**: 3
**Confidence**: Low (will be re-estimated during refinement)
