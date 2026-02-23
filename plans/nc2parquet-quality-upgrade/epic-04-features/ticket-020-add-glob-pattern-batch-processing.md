# ticket-020 Add Glob Pattern Support for Batch File Processing

> **[OUTLINE]** This ticket requires refinement before execution.
> It will be refined with learnings from earlier epics.

## Objective

Add support for glob/wildcard patterns (e.g., `data/*.nc`, `**/*.nc`) in the CLI and library API so users can process multiple NetCDF files in a single command. Currently, processing multiple files requires repeated invocations or an external script. Batch processing is one of the most requested features for climate data pipelines where users have hundreds or thousands of daily/hourly NetCDF files.

## Anticipated Scope

- **Files likely to be modified**:
  - `/home/rogerio/git/nc2parquet/src/cli.rs` -- add glob pattern argument, parse glob patterns
  - `/home/rogerio/git/nc2parquet/src/lib.rs` -- add `process_netcdf_batch` function
  - `/home/rogerio/git/nc2parquet/src/input.rs` -- may need `BatchJobConfig` or extend `JobConfig` for glob patterns
  - `/home/rogerio/git/nc2parquet/src/main.rs` or handlers -- add batch processing handler logic
  - `/home/rogerio/git/nc2parquet/Cargo.toml` -- add `glob` crate dependency
- **Key decisions needed**:
  - Output naming strategy: preserve input filename structure, or use a template (e.g., `{stem}.parquet`)?
  - Error handling for batch: fail-fast on first error, or collect errors and report at end?
  - Whether to process files sequentially or in parallel (tokio::spawn or rayon)
  - Whether glob patterns work with S3 paths (S3 has its own prefix listing, not filesystem globs)
- **Open questions**:
  - Should batch processing reuse a single config (same variable, same filters) or allow per-file config overrides?
  - How should progress reporting work for batches (per-file progress bars, or overall batch progress)?
  - What is the expected batch size? (10s, 100s, 1000s of files)

## Dependencies

- **Blocked By**: ticket-009 (handlers extracted so batch handler has clean home), ticket-010 (error types for batch error reporting)
- **Blocks**: ticket-028

## Effort Estimate

**Points**: 5
**Confidence**: Low (will be re-estimated during refinement)
