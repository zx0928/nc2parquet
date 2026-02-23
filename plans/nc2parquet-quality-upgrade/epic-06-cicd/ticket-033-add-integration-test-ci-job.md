# ticket-033 Add Integration Test CI Job with NetCDF Fixtures

> **[OUTLINE]** This ticket requires refinement before execution.
> It will be refined with learnings from earlier epics.

## Objective

Add a dedicated CI job that runs integration tests using real NetCDF test fixture files. The current CI runs `cargo test --lib` which only executes unit tests with mock/synthetic data. A separate integration test job with actual NetCDF files validates the full pipeline end-to-end and catches issues that unit tests miss (e.g., NetCDF format version compatibility, real-world data edge cases).

## Anticipated Scope

- **Files likely to be modified**:
  - `/home/rogerio/git/nc2parquet/.github/workflows/ci.yml` -- add integration test job
  - `/home/rogerio/git/nc2parquet/tests/` -- CREATE integration test files (if not already created in Epic 01)
  - `/home/rogerio/git/nc2parquet/tests/fixtures/` -- CREATE or populate with small NetCDF test files
- **Key decisions needed**:
  - Fixture management: check small NetCDF files into the repo, download from a public URL, or generate in CI?
  - Maximum fixture size to keep CI fast (target: <50MB total)
  - Whether to include S3 integration tests in CI (requires AWS credentials or localstack)
  - Separate workflow file or job within existing ci.yml?
- **Open questions**:
  - What is the minimum set of NetCDF fixtures that provides good coverage (1D, 2D, 3D, 4D variables; different data types; with/without fill values)?
  - Can we use publicly available climate datasets (e.g., from NOAA or ECMWF) that are small enough for CI?
  - Should the integration test job run on every PR or only on pushes to main (to keep PR CI fast)?

## Dependencies

- **Blocked By**: ticket-008 (integration tests written in Epic 01), ticket-029 (basic CI improvements)
- **Blocks**: None

## Effort Estimate

**Points**: 3
**Confidence**: Low (will be re-estimated during refinement)
