# ticket-015 Implement Chunked NetCDF Reading for Large Files

> **[OUTLINE]** This ticket requires refinement before execution.
> It will be refined with learnings from earlier epics.

## Objective

Implement chunked reading of NetCDF variables so that large files (>1GB) can be processed without loading the entire variable into memory at once. The current `extract_data_to_dataframe` function reads the entire variable array in a single call, which can cause OOM for large climate datasets with high-resolution grids.

## Anticipated Scope

- **Files likely to be modified**:
  - `/home/rogerio/git/nc2parquet/src/extract.rs` -- add chunked reading strategy alongside existing full-read path
  - `/home/rogerio/git/nc2parquet/src/lib.rs` -- may need to adjust pipeline orchestration for chunked processing
  - `/home/rogerio/git/nc2parquet/src/output.rs` -- may need to support appending chunks to Parquet output
- **Key decisions needed**:
  - Chunking strategy: by dimension slices, by coordinate combinations, or by byte budget
  - Whether chunked output produces one Parquet file (appended row groups) or multiple files
  - How filters interact with chunks (apply per-chunk or pre-compute indices for all chunks)
  - Whether the netcdf crate's `get_values_at` / slice operations support the needed chunked access patterns
- **Open questions**:
  - What is the actual memory profile of current extraction for a 1GB file?
  - Does polars support efficient row-group-at-a-time Parquet writing?
  - Should chunk size be configurable via CLI or automatically determined from available memory?

## Dependencies

- **Blocked By**: ticket-014 (benchmarks establish baseline before optimization)
- **Blocks**: ticket-018

## Effort Estimate

**Points**: 5
**Confidence**: Low (will be re-estimated during refinement)
