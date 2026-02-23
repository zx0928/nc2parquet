# ticket-030 Add Benchmark Regression Detection in CI

> **[OUTLINE]** This ticket requires refinement before execution.
> It will be refined with learnings from earlier epics.

## Objective

Add a CI job that runs criterion benchmarks and detects performance regressions by comparing against a stored baseline. Regressions exceeding a configurable threshold (e.g., 10%) should fail the CI check or post a warning comment on the PR. This prevents accidental performance regressions during feature development.

## Anticipated Scope

- **Files likely to be modified**:
  - `/home/rogerio/git/nc2parquet/.github/workflows/ci.yml` -- add benchmark CI job (or create separate benchmark workflow)
  - `/home/rogerio/git/nc2parquet/.github/workflows/benchmark.yml` -- CREATE (optional): dedicated benchmark workflow
- **Key decisions needed**:
  - Benchmark storage: GitHub Actions cache, git branch (gh-pages), or external service (Bencher)?
  - Comparison strategy: compare against main branch baseline or previous commit?
  - Threshold for failure vs. warning (10% regression fails, 5% warns?)
  - Whether to use criterion's built-in comparison or a third-party tool like `critcmp` or `github-action-benchmark`
- **Open questions**:
  - Are GitHub Actions runners consistent enough for reproducible benchmark results?
  - Should benchmarks run on every PR or only when source files (not docs) change?
  - How to handle the first run when no baseline exists?

## Dependencies

- **Blocked By**: ticket-014 (criterion benchmark suite exists)
- **Blocks**: None

## Effort Estimate

**Points**: 3
**Confidence**: Low (will be re-estimated during refinement)
