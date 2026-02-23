# ticket-014 Add Criterion Benchmark Suite

> **[OUTLINE]** This ticket requires refinement before execution.
> It will be refined with learnings from earlier epics.

## Objective

Establish a criterion-based benchmark suite that measures the performance of the core conversion pipeline, individual filter operations, data extraction, and postprocessing stages. These baselines are essential for detecting performance regressions during optimization work in later tickets and for CI integration in Epic 06.

## Anticipated Scope

- **Files likely to be modified**:
  - `/home/rogerio/git/nc2parquet/Cargo.toml` -- add criterion dev-dependency and `[[bench]]` entries
  - `/home/rogerio/git/nc2parquet/benches/pipeline_bench.rs` -- CREATE: end-to-end pipeline benchmark
  - `/home/rogerio/git/nc2parquet/benches/filter_bench.rs` -- CREATE: filter application benchmarks for all 4 filter types
  - `/home/rogerio/git/nc2parquet/benches/postprocess_bench.rs` -- CREATE: postprocessor benchmarks (unit conversion, formula evaluation, aggregation)
  - `/home/rogerio/git/nc2parquet/benches/extract_bench.rs` -- CREATE: DimensionIndexManager and extraction benchmarks
- **Key decisions needed**:
  - Whether to use real NetCDF fixture files or generate synthetic in-memory data for benchmarks
  - Benchmark group organization (by module vs. by operation size)
  - Whether to include async pipeline benchmarks or only sync
- **Open questions**:
  - What representative file sizes should benchmarks cover (small: 1MB, medium: 100MB, large: 1GB)?
  - Should we benchmark S3 storage operations or only local storage?
  - What is the target benchmark execution time (fast enough for CI, or full suite for manual runs)?

## Dependencies

- **Blocked By**: ticket-001 (test helpers provide fixture utilities), ticket-013 (clean codebase)
- **Blocks**: ticket-015, ticket-016, ticket-017, ticket-018, ticket-030

## Effort Estimate

**Points**: 3
**Confidence**: Low (will be re-estimated during refinement)
