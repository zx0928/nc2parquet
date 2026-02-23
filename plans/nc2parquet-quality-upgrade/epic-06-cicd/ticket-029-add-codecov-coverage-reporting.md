# ticket-029 Add Code Coverage Reporting with Codecov

## Context

### Background

The nc2parquet project has 296 library tests, 28 doc-tests, and 4 property-based test files built across epics 01-04. Coverage measurement has never been configured -- there is no way to track what percentage of library code is exercised by the test suite, detect coverage regressions on PRs, or display a coverage badge in the README. Adding automated coverage reporting with a dedicated CI job and Codecov integration addresses all three needs.

### Relation to Epic

This is the first ticket in Epic 06 (CI/CD & Release Quality). It adds the coverage measurement foundation that the team can use to validate that the test suite (built in Epic 01) actually covers the code paths it targets. The coverage badge also feeds into the README (Epic 05) and the CI pipeline table in CONTRIBUTING.md.

### Current State

- **CI workflow**: `/home/rogerio/git/nc2parquet/.github/workflows/ci.yml` has two jobs: `test` (stable/beta matrix with fmt, clippy, `cargo test --lib`) and `security-audit` (cargo-audit). Neither job measures coverage.
- **Test suite**: 296 lib tests in `src/tests/` (12 files), 28 doc-tests, 1 DHAT profiling test behind `dhat-heap` feature. All use `examples/data/simple_xy.nc` and `examples/data/pres_temp_4D.nc` as fixtures via `src/test_helpers.rs::get_test_data_path()`.
- **System dependency**: The CI installs `netcdf-bin libnetcdf-dev libhdf5-dev` on Ubuntu for the netcdf crate's static build feature.
- **README badges**: Currently has CI, Crates.io, docs.rs, and License badges (lines 5-8 of `/home/rogerio/git/nc2parquet/README.md`). No coverage badge.
- **No `codecov.yml` or `tarpaulin.toml` exists in the repository.**

## Specification

### Requirements

1. Add a new `coverage` job to `.github/workflows/ci.yml` that runs `cargo-tarpaulin` and uploads results to Codecov.
2. Create a `codecov.yml` configuration file at the repository root with:
   - Project coverage target of 70% (informational, not enforced as a hard gate)
   - PR comment enabled showing coverage diff
   - Patch coverage target of 50% (informational)
3. Use `cargo-tarpaulin` (not `cargo-llvm-cov`) because it works out-of-the-box with the netcdf crate's C library linkage via its ptrace-based instrumentation and does not require rebuilding with LLVM instrumentation flags.
4. Exclude from coverage measurement: `src/main.rs`, `src/handlers/` (binary-only CLI logic), `src/test_helpers.rs`, `src/tests/`, and benchmark files.
5. Add a Codecov coverage badge to `README.md` on the badge line (after the CI badge).
6. Add a "Coverage" row to the CI pipeline table in `CONTRIBUTING.md` (lines 414-421).
7. The coverage job should run only on `ubuntu-latest` with `stable` Rust (not in the matrix -- coverage only needs one run).
8. The coverage job should run on pushes to `main` and on PRs targeting `main` (same triggers as existing CI).

### Inputs/Props

- **Codecov repository token**: Must be set as a GitHub Actions secret `CODECOV_TOKEN`. The workflow should work without the token for public repos (Codecov supports tokenless upload for public repos) but should accept the token when provided for reliability.
- **Tarpaulin output format**: `xml` (Cobertura format, required by Codecov).

### Outputs/Behavior

- On every CI run, the `coverage` job produces a `cobertura.xml` coverage report.
- The report is uploaded to Codecov via the `codecov/codecov-action@v4` action.
- Codecov posts a PR comment showing coverage change when a PR is opened or updated.
- The coverage badge in README reflects the current main branch coverage percentage.

### Error Handling

- If `cargo-tarpaulin` fails (e.g., due to a test failure), the coverage job should fail, making the overall CI red. This is correct because a test failure is a real problem.
- If the Codecov upload fails (e.g., network issue), the coverage job should NOT fail the CI. Set `fail_ci_if_error: false` on the Codecov action.

## Acceptance Criteria

- [ ] Given the CI workflow file at `.github/workflows/ci.yml`, when inspected, then it contains a `coverage` job that installs cargo-tarpaulin, runs it with appropriate exclusions, and uploads results to Codecov.
- [ ] Given a `codecov.yml` file at the repository root, when inspected, then it configures project target 70%, patch target 50%, and enables PR comments.
- [ ] Given the README.md badge line, when inspected, then it includes a Codecov badge immediately after the CI badge using the format `[![codecov](https://codecov.io/gh/rjmalves/nc2parquet/branch/main/graph/badge.svg)](https://codecov.io/gh/rjmalves/nc2parquet)`.
- [ ] Given the CONTRIBUTING.md CI pipeline table, when inspected, then it includes a "Coverage" row with the command `cargo tarpaulin --out xml ...` (abbreviated).
- [ ] Given the coverage job definition, when inspected, then `src/main.rs`, `src/handlers/*`, `src/test_helpers.rs`, `src/tests/*`, and `benches/*` are excluded via tarpaulin flags.
- [ ] Given the coverage job definition, when inspected, then it installs system dependencies (`libnetcdf-dev libhdf5-dev`) before running tarpaulin.
- [ ] Given the Codecov action step, when inspected, then `fail_ci_if_error` is set to `false`.

## Implementation Guide

### Suggested Approach

1. **Add the `coverage` job to `.github/workflows/ci.yml`**:
   - Place it after the existing `test` job (no `needs` dependency -- it runs in parallel).
   - Use `ubuntu-latest` and `stable` Rust only.
   - Install system dependencies (same `apt-get` block as the `test` job).
   - Install cargo-tarpaulin via `cargo install cargo-tarpaulin`.
   - Run tarpaulin with exclusions: `cargo tarpaulin --out xml --output-dir coverage --exclude-files "src/main.rs" "src/handlers/*" "src/test_helpers.rs" "src/tests/*" "benches/*" --skip-clean --lib`.
   - Upload with `codecov/codecov-action@v4` pointing to `coverage/cobertura.xml`.

2. **Create `/home/rogerio/git/nc2parquet/codecov.yml`**:

   ```yaml
   coverage:
     status:
       project:
         default:
           target: 70%
           threshold: 5%
       patch:
         default:
           target: 50%
   comment:
     layout: "diff, flags, files"
     behavior: default
     require_changes: true
   ```

3. **Add the Codecov badge to README.md** (line 6, after the CI badge):

   ```
   [![codecov](https://codecov.io/gh/rjmalves/nc2parquet/branch/main/graph/badge.svg)](https://codecov.io/gh/rjmalves/nc2parquet)
   ```

4. **Update the CI pipeline table in CONTRIBUTING.md** (after line 420, before the security-audit row):
   Add a row: `| Coverage | cargo tarpaulin --out xml --lib (uploads to Codecov) |`

### Key Files to Modify

- `/home/rogerio/git/nc2parquet/.github/workflows/ci.yml` -- add `coverage` job
- `/home/rogerio/git/nc2parquet/codecov.yml` -- CREATE new file
- `/home/rogerio/git/nc2parquet/README.md` -- add badge on line 6
- `/home/rogerio/git/nc2parquet/CONTRIBUTING.md` -- add row to CI pipeline table (around line 420)

### Patterns to Follow

- Follow the same step structure as the existing `test` job: checkout, install Rust, cache cargo, install system deps, then run tool.
- Use `actions/cache@v3` with the same cache key pattern as the existing job (but consider using a separate cache key suffix to avoid cache conflicts with the test job's target directory).
- Use the Codecov GitHub Action v4 (not a raw curl upload) for reliability and PR comment integration.

### Pitfalls to Avoid

- **Do NOT use `cargo-llvm-cov`**: It requires rebuilding all dependencies with LLVM instrumentation flags, which can fail with the netcdf crate's C library FFI. `cargo-tarpaulin` uses ptrace and works on pre-built binaries.
- **Do NOT set `fail_ci_if_error: true`** on the Codecov upload step. Codecov service outages should not block PRs.
- **Do NOT run tarpaulin with `--all-features`**: The `dhat-heap` feature enables a global allocator that conflicts with tarpaulin's instrumentation. Use `--lib` only (no `--features`).
- **Do NOT include handler or test files in coverage**: They inflate the denominator without providing useful signal. Handler code is CLI glue; test code coverage is meaningless.
- **Cache tarpaulin binary**: Installing cargo-tarpaulin from source takes 2-3 minutes. Use `taiki-e/install-action@cargo-tarpaulin` for a pre-built binary install, or cache `~/.cargo/bin/cargo-tarpaulin`.

## Testing Requirements

### Unit Tests

Not applicable -- this ticket modifies CI configuration files only.

### Integration Tests

- Validate the YAML syntax of `.github/workflows/ci.yml` by running `python3 -c "import yaml; yaml.safe_load(open('.github/workflows/ci.yml'))"` or equivalent.
- Validate the `codecov.yml` syntax similarly.

### E2E Tests

- After merging, verify that the coverage job appears in the GitHub Actions tab.
- Verify that Codecov receives the upload (check the Codecov dashboard for the repository).
- Verify that the coverage badge renders correctly in the README on GitHub.

## Dependencies

- **Blocked By**: None (all tests from epics 01-04 are already complete and merged)
- **Blocks**: None

## Effort Estimate

**Points**: 2
**Confidence**: High
