# ticket-032 Add Cross-Compilation CI Matrix

> **[OUTLINE]** This ticket requires refinement before execution.
> It will be refined with learnings from earlier epics.

## Objective

Add a CI matrix that builds and tests nc2parquet on multiple target platforms: linux-x86_64, linux-aarch64, and macos-x86_64 (and optionally macos-aarch64). The current CI only builds on the runner's native platform. Cross-compilation validation ensures release binaries work on all advertised platforms before they are distributed.

## Anticipated Scope

- **Files likely to be modified**:
  - `/home/rogerio/git/nc2parquet/.github/workflows/ci.yml` -- add matrix strategy with multiple OS/architecture combinations
  - `/home/rogerio/git/nc2parquet/dist-workspace.toml` -- ensure target list matches CI matrix
- **Key decisions needed**:
  - Cross-compilation approach: native GitHub Actions runners (macos-latest, ubuntu-latest) vs. cross-compilation with `cross` crate
  - Whether aarch64-linux requires QEMU in CI or just cross-compilation without running tests
  - Whether to test on Windows (netcdf C library availability is a challenge)
  - Whether to use a matrix include/exclude strategy for platform-specific test skips
- **Open questions**:
  - Does the netcdf crate's static build (`static` feature) work on all target platforms?
  - Are there any platform-specific issues with the S3 SDK or tokio runtime?
  - What is the build time impact of adding 3-4 matrix entries?

## Dependencies

- **Blocked By**: ticket-029 (basic CI improvements in place)
- **Blocks**: ticket-031

## Effort Estimate

**Points**: 2
**Confidence**: Low (will be re-estimated during refinement)
