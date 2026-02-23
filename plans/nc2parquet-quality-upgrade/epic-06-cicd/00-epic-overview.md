# Epic 06: CI/CD and Release Quality

## Goals

1. Add code coverage reporting with cargo-tarpaulin and Codecov integration
2. Add benchmark regression detection in CI using criterion + GitHub Actions
3. Automate release process with cargo-dist and GitHub Releases
4. Add cross-compilation CI matrix for common targets (linux-x86_64, linux-aarch64, macos)
5. Add a dedicated integration test CI job with test NetCDF fixtures

## Scope

This epic covers CI/CD pipeline improvements and release automation. It does NOT cover:

- Writing new tests (that is Epic 01)
- Writing documentation content (that is Epic 05)
- cargo-dist configuration changes beyond what is needed for automation

## Dependencies

- **Requires**: Epic 01 (tests to measure coverage), Epic 03 (benchmarks to track), Epic 05 (docs to publish)
- **Feeds into**: Nothing (final epic)

## Tickets

| ID         | Title                                            | Points | Confidence |
| ---------- | ------------------------------------------------ | ------ | ---------- |
| ticket-029 | Add Code Coverage Reporting with Codecov         | 2      | Low        |
| ticket-030 | Add Benchmark Regression Detection in CI         | 3      | Low        |
| ticket-031 | Automate Release Process with cargo-dist         | 3      | Low        |
| ticket-032 | Add Cross-Compilation CI Matrix                  | 2      | Low        |
| ticket-033 | Add Integration Test CI Job with NetCDF Fixtures | 3      | Low        |

## Success Criteria

- Coverage report uploads to Codecov on every PR
- Benchmark regressions >10% fail the CI check
- `cargo dist` produces release artifacts for all targets on tag push
- CI matrix includes linux-x86_64, linux-aarch64, and macos-x86_64
- Integration tests run with real NetCDF fixtures in a separate CI job
