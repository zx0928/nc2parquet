# ticket-033 Add Integration Test CI Job with NetCDF Fixtures

## Context

### Background

The current CI runs `cargo test --lib` which executes all 296 library tests, including the integration tests in `src/tests/test_integration.rs`. However, the CI does NOT run doc-tests (`cargo test --doc`) or the full `cargo test` command (which would include doc-tests and any future `tests/` directory integration tests). Additionally, the test execution is bundled into a single monolithic step, making it impossible to see at a glance whether unit tests, integration tests, or doc-tests failed. This ticket splits the CI test execution into distinct, clearly named jobs for better failure diagnosis and adds doc-test execution that is currently missing from CI.

### Relation to Epic

This is the fifth and final ticket in Epic 06. It completes the CI/CD pipeline by ensuring that all test categories run in CI with clear separation and failure attribution. Combined with coverage (ticket-029), benchmark regression (ticket-030), cross-compilation (ticket-032), and release automation (ticket-031), this creates a comprehensive quality gate.

### Current State

- **CI workflow** (`/home/rogerio/git/nc2parquet/.github/workflows/ci.yml`): The `test` job runs:
  1. `cargo fmt --all -- --check`
  2. `cargo clippy --all-targets --all-features -- -D warnings`
  3. `cargo test --lib --verbose`
  - Missing: `cargo test --doc` (doc-tests are NOT run in CI)
- **Test suite**: 296 lib tests across 12 files in `src/tests/`, 28 doc-tests, 1 DHAT profiling test (feature-gated).
- **Integration tests**: `src/tests/test_integration.rs` contains end-to-end tests that use `process_netcdf_job()` with real NetCDF fixtures from `examples/data/`. These already run as part of `cargo test --lib` because they are `#[cfg(test)]` modules inside the library crate.
- **Test fixtures**: `examples/data/simple_xy.nc` (2D, 384 bytes) and `examples/data/pres_temp_4D.nc` (4D, 2784 bytes). Both are checked into the repository. Combined size: ~3KB. No external fixtures needed.
- **CONTRIBUTING.md**: Documents running `cargo test --lib --verbose` and `cargo test --doc` separately (lines 271-286), but the CI pipeline table (lines 415-420) only lists "Unit tests" with `cargo test --lib --verbose`.
- **Doc-test count**: 28 doc-tests (down from 39 after Epic 04 refactoring -- documented in learnings as a known issue).

## Specification

### Requirements

1. Split the `test` job in `.github/workflows/ci.yml` into separate steps with clear names:
   - "Check formatting" (existing, unchanged)
   - "Run Clippy" (existing, unchanged)
   - "Run library tests" (`cargo test --lib --verbose`)
   - "Run doc-tests" (`cargo test --doc --verbose`) -- NEW step
2. The doc-test step must run after the library test step (if lib tests fail, doc-tests likely will too, and the lib test failure is more informative).
3. Both test steps run in the same job (not separate jobs) to avoid duplicating the checkout/install/build steps.
4. The doc-test step should run on all matrix entries (Ubuntu stable, Ubuntu beta, macOS stable -- as configured by ticket-032).
5. Update the CI pipeline table in CONTRIBUTING.md to list the doc-test step.
6. Do NOT create a separate workflow file for integration tests. The integration tests in `src/tests/test_integration.rs` already run as part of `cargo test --lib`. They do not need a separate job.
7. Do NOT create external test fixtures or a `tests/` directory. All fixtures exist in `examples/data/` and are accessed via `src/test_helpers.rs::get_test_data_path()`.

### Inputs/Props

- **Test fixtures**: `examples/data/simple_xy.nc` and `examples/data/pres_temp_4D.nc` are checked into the repository and available after `actions/checkout@v4`.
- **No additional secrets or environment variables are needed.**

### Outputs/Behavior

- The CI pipeline shows four distinct steps in the test job: formatting, clippy, library tests, doc-tests.
- If library tests pass but doc-tests fail, the failure is clearly attributed to doc-tests in the GitHub Actions UI.
- The doc-test step validates that all 28 doc-examples in the public API compile and run correctly.

### Error Handling

- If `cargo test --doc` fails, the job fails and the step name ("Run doc-tests") makes the failure source immediately obvious.
- Doc-test failures on `beta` Rust are expected occasionally (compiler changes may affect doc-test compilation). The `fail-fast: false` from ticket-032's matrix prevents a beta failure from cancelling the stable run.

## Acceptance Criteria

- [ ] Given `.github/workflows/ci.yml`, when inspected, then the `test` job contains a "Run doc-tests" step executing `cargo test --doc --verbose`.
- [ ] Given the step ordering, when inspected, then "Run doc-tests" appears AFTER "Run library tests".
- [ ] Given the CI pipeline, when run successfully, then both `cargo test --lib` and `cargo test --doc` execute and pass.
- [ ] Given the CONTRIBUTING.md CI pipeline table, when inspected, then it includes a "Doc-tests" row with the command `cargo test --doc --verbose`.
- [ ] Given the CI workflow, when inspected, then NO separate integration test workflow or job exists (integration tests run within `cargo test --lib`).
- [ ] Given the test fixtures, when inspected, then no new fixture files were added to the repository (existing `examples/data/` fixtures are sufficient).

## Implementation Guide

### Suggested Approach

1. **Add the doc-test step** to the `test` job in `/home/rogerio/git/nc2parquet/.github/workflows/ci.yml`:

After the existing "Run unit tests" step (renamed to "Run library tests" for clarity):

```yaml
- name: Run library tests
  run: cargo test --lib --verbose

- name: Run doc-tests
  run: cargo test --doc --verbose
```

2. **Rename the existing step** from "Run unit tests" to "Run library tests" for accuracy. The `cargo test --lib` command runs ALL library tests including integration tests in `src/tests/test_integration.rs`, not just unit tests.

3. **Update CONTRIBUTING.md CI pipeline table** (around lines 415-420):

Current table:

```
| Job            | Command                                                    |
| -------------- | ---------------------------------------------------------- |
| Format check   | `cargo fmt --all -- --check`                               |
| Clippy         | `cargo clippy --all-targets --all-features -- -D warnings` |
| Unit tests     | `cargo test --lib --verbose`                               |
| Security audit | `cargo audit`                                              |
```

Updated table (incorporating changes from tickets 029, 030, 032):

```
| Job                  | Platform             | Command                                                    |
| -------------------- | -------------------- | ---------------------------------------------------------- |
| Format check         | Ubuntu + macOS       | `cargo fmt --all -- --check`                               |
| Clippy               | Ubuntu + macOS       | `cargo clippy --all-targets --all-features -- -D warnings` |
| Library tests        | Ubuntu + macOS       | `cargo test --lib --verbose`                               |
| Doc-tests            | Ubuntu + macOS       | `cargo test --doc --verbose`                               |
| Coverage             | Ubuntu               | `cargo tarpaulin --out xml --lib` (uploads to Codecov)     |
| Benchmark regression | Ubuntu               | `cargo bench -- --output-format=bencher` (warns on >15%)   |
| Security audit       | Ubuntu               | `cargo audit`                                              |
```

### Key Files to Modify

- `/home/rogerio/git/nc2parquet/.github/workflows/ci.yml` -- add doc-test step, rename existing step
- `/home/rogerio/git/nc2parquet/CONTRIBUTING.md` -- update CI pipeline table

### Patterns to Follow

- Follow the existing step structure: each step has a clear `name` and a single `run` command.
- The doc-test step uses `--verbose` for consistency with the library test step.
- The step name uses the pattern "Run [category] tests" for consistency.

### Pitfalls to Avoid

- **Do NOT use `cargo test` without flags**: Running bare `cargo test` would run lib tests, doc-tests, AND any binary tests, duplicating the lib test run. Keep them as separate explicit steps.
- **Do NOT use `cargo test --all-targets`**: This would include benchmark compilation (which takes a long time) and is not needed for test execution.
- **Do NOT create a `tests/` directory for integration tests**: The Rust convention of `tests/` is for tests that import the library as an external crate. The existing `src/tests/` approach (internal test modules) is correct for nc2parquet because tests need `pub(crate)` access to internal types.
- **Do NOT add `--features dhat-heap` to the CI test run**: The DHAT profiling test requires the `dhat-heap` feature which enables a global allocator that conflicts with normal test execution. It is intentionally excluded from CI.
- **Beware of doc-test count**: The learnings note 28 doc-tests (down from 39 after Epic 04). If the doc-test step shows fewer than 28, investigate but do not block the ticket on this.

## Testing Requirements

### Unit Tests

Not applicable -- this ticket modifies CI configuration and documentation only.

### Integration Tests

- Validate the YAML syntax of the modified `ci.yml`.
- Run `cargo test --doc --verbose` locally to verify all 28 doc-tests pass before committing.

### E2E Tests

- After merging, verify that the GitHub Actions UI shows four distinct steps in the test job: formatting, clippy, library tests, doc-tests.
- Verify that the doc-test step passes on all matrix entries.

## Dependencies

- **Blocked By**: ticket-032 (the CI matrix must be in place so doc-tests run on all platforms)
- **Blocks**: None

## Effort Estimate

**Points**: 1
**Confidence**: High
