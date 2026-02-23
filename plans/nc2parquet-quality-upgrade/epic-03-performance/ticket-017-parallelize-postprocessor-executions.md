# ticket-017 Parallelize Independent PostProcessor Executions

> **[OUTLINE]** This ticket requires refinement before execution.
> It will be refined with learnings from earlier epics.

## Objective

Enable parallel execution of independent postprocessors within a `ProcessingPipeline`. Currently, processors are applied sequentially even when they operate on different columns (e.g., a ColumnRenamer on column A and a UnitConverter on column B are independent). Identifying and parallelizing independent processors could reduce postprocessing latency for pipelines with many processors.

## Anticipated Scope

- **Files likely to be modified**:
  - `/home/rogerio/git/nc2parquet/src/postprocess.rs` -- add dependency analysis between processors, parallel execution path
  - `/home/rogerio/git/nc2parquet/Cargo.toml` -- may need rayon dependency for data parallelism
- **Key decisions needed**:
  - Whether to use rayon for parallel execution or tokio::spawn for async parallelism
  - How to determine processor independence (by target column? by explicit dependency declaration?)
  - Whether the overhead of parallelization is worth it for typical pipeline sizes (usually 2-5 processors)
  - Whether polars already handles column-level parallelism internally, making this optimization redundant
- **Open questions**:
  - What is the typical postprocessing time as a fraction of total pipeline time? (needs profiling)
  - Does polars use rayon internally, and would adding our own rayon usage cause thread pool contention?
  - Is the ProcessingPipeline trait object-safe enough to allow parallel execution without major refactoring?

## Dependencies

- **Blocked By**: ticket-014 (benchmarks needed to measure whether parallelization helps)
- **Blocks**: ticket-018

## Effort Estimate

**Points**: 3
**Confidence**: Low (will be re-estimated during refinement)
