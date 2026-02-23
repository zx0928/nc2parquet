# ticket-029 Add Code Coverage Reporting with Codecov

> **[OUTLINE]** This ticket requires refinement before execution.
> It will be refined with learnings from earlier epics.

## Objective

Add code coverage measurement using cargo-tarpaulin and integrate with Codecov to provide coverage badges, PR coverage comments, and coverage trend tracking. This enables the team to monitor test coverage progress toward the >80% target and catch coverage regressions in pull requests.

## Anticipated Scope

- **Files likely to be modified**:
  - `/home/rogerio/git/nc2parquet/.github/workflows/ci.yml` -- add coverage job with cargo-tarpaulin and Codecov upload
  - `/home/rogerio/git/nc2parquet/codecov.yml` -- CREATE: Codecov configuration (target coverage, PR comment settings)
  - `/home/rogerio/git/nc2parquet/tarpaulin.toml` -- CREATE (optional): tarpaulin configuration for excluded files
- **Key decisions needed**:
  - Coverage tool: cargo-tarpaulin vs. cargo-llvm-cov (tarpaulin is simpler but llvm-cov is more accurate)
  - Coverage target: 80% as fail threshold or as informational-only?
  - Whether to exclude test files, build scripts, and main.rs handlers from coverage calculation
  - Whether to add a coverage badge to README
- **Open questions**:
  - Does cargo-tarpaulin work reliably with the netcdf crate's C library linkage?
  - Should coverage run on every PR or only on pushes to main?
  - What is the current baseline coverage (before Epic 01 test improvements)?

## Dependencies

- **Blocked By**: ticket-008 (all tests from Epic 01 complete, providing meaningful coverage baseline)
- **Blocks**: None

## Effort Estimate

**Points**: 2
**Confidence**: Low (will be re-estimated during refinement)
