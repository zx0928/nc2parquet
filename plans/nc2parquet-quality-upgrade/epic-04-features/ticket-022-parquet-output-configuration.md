# ticket-022 Add Parquet Output Configuration Options

> **[OUTLINE]** This ticket requires refinement before execution.
> It will be refined with learnings from earlier epics.

## Objective

Add configurable Parquet output options including compression codec (snappy, zstd, lz4, gzip, uncompressed), compression level, row group size, and data page size. Currently, the output uses polars defaults which may not be optimal for all downstream consumers. Users working with Spark, DuckDB, or Athena often need specific Parquet settings for optimal query performance.

## Anticipated Scope

- **Files likely to be modified**:
  - `/home/rogerio/git/nc2parquet/src/output.rs` -- accept output configuration, pass to polars ParquetWriter
  - `/home/rogerio/git/nc2parquet/src/input.rs` -- add `OutputConfig` struct to `JobConfig`
  - `/home/rogerio/git/nc2parquet/src/cli.rs` -- add CLI flags for compression, row-group-size, etc.
  - `/home/rogerio/git/nc2parquet/src/lib.rs` -- thread OutputConfig through pipeline
- **Key decisions needed**:
  - Which polars `ParquetWriter` options to expose (compression, row_group_size, data_page_size, statistics)
  - Whether to use an `OutputConfig` struct or individual parameters
  - Default values that balance file size and read performance
  - Whether Parquet version (v1 vs v2) should be configurable
- **Open questions**:
  - Does polars 0.51's ParquetWriter support all the configuration options we want to expose?
  - Should we support per-column compression (different codecs for different columns)?
  - What are the recommended settings for common downstream consumers (Spark, DuckDB, Athena)?

## Dependencies

- **Blocked By**: ticket-010 (error types for config validation errors), ticket-014 (benchmarks to measure compression trade-offs)
- **Blocks**: ticket-028

## Effort Estimate

**Points**: 3
**Confidence**: Low (will be re-estimated during refinement)
