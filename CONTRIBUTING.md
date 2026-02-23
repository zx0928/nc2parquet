# Contributing to nc2parquet

Thank you for your interest in contributing. nc2parquet is a high-performance
NetCDF to Parquet converter written in Rust. This guide covers everything you
need to go from a fresh checkout to a merged pull request.

## Table of Contents

- [Prerequisites](#prerequisites)
- [Development Setup](#development-setup)
- [Platform-Specific Build Instructions](#platform-specific-build-instructions)
- [Project Architecture](#project-architecture)
- [Coding Standards](#coding-standards)
- [Testing](#testing)
- [Benchmarks](#benchmarks)
- [Pull Request Process](#pull-request-process)
- [Reporting Issues](#reporting-issues)
- [Architecture Decision Records](#architecture-decision-records)

---

## Prerequisites

- **Rust**: stable toolchain, 1.85 or newer (edition 2024 features are used)
- **System headers**: The `netcdf` crate is compiled with `features = ["static"]`,
  which links the NetCDF and HDF5 C libraries statically into the binary. The
  resulting binary has no runtime dependency on those libraries, but their
  **header files** must be present on the build machine. See
  [Platform-Specific Build Instructions](#platform-specific-build-instructions)
  for the exact packages to install on each platform.
- **Git**: any recent version

Install or update the Rust toolchain via [rustup](https://rustup.rs/):

```bash
rustup update stable
rustup component add rustfmt clippy
```

---

## Development Setup

```bash
# 1. Clone the repository
git clone https://github.com/rjmalves/nc2parquet.git
cd nc2parquet

# 2. Install platform system headers (see next section)

# 3. Build the project
cargo build

# 4. Run the library test suite
cargo test --lib

# 5. Run the doc-tests
cargo test --doc

# 6. Verify formatting and lints (mirrors the CI checks)
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
```

If all four commands above succeed you are ready to make changes.

---

## Platform-Specific Build Instructions

### Ubuntu / Debian

```bash
sudo apt-get update
sudo apt-get install -y libnetcdf-dev libhdf5-dev
```

### Fedora / RHEL / CentOS Stream

```bash
sudo dnf install -y netcdf-devel hdf5-devel
```

### macOS (Homebrew)

```bash
brew install netcdf
```

Homebrew installs both the runtime libraries and the headers. No additional
steps are required.

### Windows

The `netcdf` crate's static feature is less tested on Windows. Pre-built
NetCDF and HDF5 development packages are available from the
[HDF Group](https://www.hdfgroup.org/downloads/hdf5/) and the
[Unidata NetCDF page](https://downloads.unidata.ucar.edu/netcdf/). You will
need to set the `HDF5_DIR` and `NETCDF_DIR` environment variables to point at
the extracted package roots before running `cargo build`. Community reports on
the Windows build experience are welcome via GitHub Discussions.

### Docker

A `Dockerfile` is included in the repository root. It uses an official Rust
image and installs the required headers automatically:

```bash
docker build -t nc2parquet .
docker run --rm nc2parquet nc2parquet --help
```

---

## Project Architecture

```
nc2parquet/
├── src/
│   ├── lib.rs            # Public library API
│   ├── main.rs           # Binary entry point; declares src/handlers/
│   ├── cli.rs            # clap CLI definition (subcommands, flags, env vars)
│   ├── errors.rs         # Nc2ParquetError (11 variants, thiserror)
│   ├── extract.rs        # NetCDF-to-DataFrame extraction (pub(crate))
│   ├── filters.rs        # NCFilter trait + 4 filter types
│   ├── input.rs          # JobConfig, BatchConfig, OutputConfig (serde)
│   ├── output.rs         # Parquet writer (pub(crate))
│   ├── postprocess.rs    # PostProcessor trait, ProcessingPipeline, 5 processors
│   ├── storage.rs        # StorageBackend trait, S3 + local implementations
│   ├── info.rs           # NetCDF file inspection helpers
│   ├── test_helpers.rs   # Shared test utilities (cfg(test) only)
│   ├── handlers/         # CLI command implementations (binary-only, not re-exported)
│   │   ├── convert.rs    # `nc2parquet convert` handler
│   │   ├── validate.rs   # `nc2parquet validate` handler
│   │   ├── info.rs       # `nc2parquet info` handler
│   │   ├── template.rs   # `nc2parquet template` handler
│   │   ├── completions.rs # `nc2parquet completions` handler
│   │   ├── config.rs     # Shared config-loading helpers
│   │   └── utils.rs      # Shared output-formatting helpers
│   └── tests/            # Library tests (cfg(test))
│       ├── mod.rs
│       ├── test_batch.rs
│       ├── test_cli.rs
│       ├── test_extract.rs
│       ├── test_filters.rs
│       ├── test_info.rs
│       ├── test_input.rs
│       ├── test_integration.rs
│       ├── test_memory_profile.rs  # dhat-heap feature only
│       ├── test_multi_variable.rs
│       ├── test_output.rs
│       ├── test_postprocess.rs
│       └── test_properties.rs     # proptest property-based tests
├── benches/
│   ├── extraction_bench.rs
│   ├── filter_bench.rs
│   ├── postprocess_bench.rs
│   └── combination_bench.rs
├── examples/
│   ├── data/             # Sample .nc files used by tests and examples
│   ├── configs/          # Annotated JSON/YAML configuration examples
│   ├── cli/              # Shell scripts demonstrating CLI usage
│   └── postprocessing/   # Post-processing pipeline examples
└── docs/
    └── adr/              # Architecture Decision Records
```

### Key design boundaries

- **`src/lib.rs`** is the only public surface. It re-exports `Nc2ParquetError`,
  `BatchConfig`, and `BatchResult`, and exposes four public functions:
  `process_netcdf_job`, `process_netcdf_job_async`, `process_netcdf_batch`,
  and `resolve_output_path`.
- **`src/extract.rs`** and **`src/output.rs`** are `pub(crate)`. They are
  internal implementation details and must not become part of the public API.
- **`src/handlers/`** is only reachable from `main.rs`. It must not be imported
  by `lib.rs` or any library module. CLI-specific logic belongs here; anything
  needed by the library belongs in the modules above.
- The **post-processing pipeline** (`src/postprocess.rs`) is public so that
  library users can build and compose `ProcessingPipeline` instances directly.
- The **filter types** (`src/filters.rs`) are public so that library users can
  construct and inspect filter configurations programmatically.

---

## Coding Standards

### Formatting and Linting

All code must pass the CI checks before a PR can be merged:

```bash
# Format (must produce no diff)
cargo fmt --all

# Lint (must produce zero warnings)
cargo clippy --all-targets --all-features -- -D warnings
```

Run both commands before pushing. The CI pipeline runs them on `stable` and
`beta` Rust.

### Visibility

Use the narrowest visibility that satisfies the design:

- Items used only within a single module: no `pub`
- Items shared across library crate modules but not part of the public API:
  `pub(crate)`
- Items that form part of the stable public API: `pub` with a full doc-comment

Do not promote visibility to work around a test or suppress a compiler
warning. Reach for `#[cfg(test)]` or `#[allow(...)]` with an explanatory
comment instead.

### Error Handling

- **Library errors**: use the `Nc2ParquetError` enum in `src/errors.rs`. Add a
  new variant when the existing ones do not cover the failure mode. Use
  `thiserror` attributes for the `Display` message.
- **Post-processing errors**: use `PostProcessError` in `src/postprocess.rs`.
  It converts into `Nc2ParquetError` automatically.
- **Storage errors**: use `StorageError` in `src/storage.rs`. It converts into
  `Nc2ParquetError` automatically.
- **Binary (CLI) error handling**: use `anyhow` only inside `src/main.rs` and
  `src/handlers/`. Never introduce `anyhow` into library modules.
- Do not use `.unwrap()` or `.expect()` outside of tests.

### Documentation Comments

Every `pub` and `pub(crate)` item must have a doc-comment. Follow this pattern:

````rust
/// Short one-line summary ending with a period.
///
/// Longer paragraph if needed. Explain behaviour that is not obvious from the
/// signature alone, including edge cases and any important invariants.
///
/// # Errors
///
/// List each error variant that can be returned and the condition that triggers it.
///
/// # Examples
///
/// ```rust
/// // At least one working example is required on every public function.
/// ```
pub fn my_function(arg: &str) -> Result<Output, Nc2ParquetError> {
````

Doc-tests (`cargo test --doc`) are part of the CI matrix. Keep examples
compilable and runnable.

### Trait Implementations

The `PostProcessor` trait (`src/postprocess.rs`) and the `NCFilter` trait
(`src/filters.rs`) are the two primary extension points. When adding a new
processor or filter:

1. Implement the trait in the appropriate module.
2. Add a serde-deserializable config struct that constructs the implementation.
3. Wire the new variant into the `from_config` factory method.
4. Add at least one integration test in `src/tests/`.

---

## Testing

### Running Tests

```bash
# Library unit tests (what CI runs)
cargo test --lib --verbose

# Doc-tests
cargo test --doc

# All tests in a specific file
cargo test --lib -- test_postprocess

# A single test by name
cargo test --lib -- test_postprocess::rename_column_produces_correct_output

# Property-based tests (proptest)
cargo test --lib -- test_properties
```

### Test Organization

Tests live in `src/tests/` and are gated behind `#[cfg(test)]` in `src/lib.rs`.
There is one file per module being tested:

| File                     | Coverage                                              |
| ------------------------ | ----------------------------------------------------- |
| `test_extract.rs`        | NetCDF extraction, dimension handling                 |
| `test_filters.rs`        | All four filter types and intersection logic          |
| `test_postprocess.rs`    | All five post-processors and `ProcessingPipeline`     |
| `test_input.rs`          | `JobConfig`, `BatchConfig`, `OutputConfig` parsing    |
| `test_output.rs`         | Parquet writer options and compression codecs         |
| `test_batch.rs`          | `process_netcdf_batch` glob expansion and error modes |
| `test_multi_variable.rs` | Multi-variable extraction                             |
| `test_integration.rs`    | End-to-end conversion using sample `.nc` files        |
| `test_cli.rs`            | CLI argument parsing and subcommand dispatch          |
| `test_info.rs`           | NetCDF file inspection output                         |
| `test_properties.rs`     | Property-based tests using `proptest`                 |
| `test_memory_profile.rs` | DHAT heap profile (requires `--features dhat-heap`)   |

Shared test utilities (fixture paths, helper builders) live in
`src/test_helpers.rs`, which is compiled only under `#[cfg(test)]`.

### Writing New Tests

- Place new tests in the file that corresponds to the module under test.
- Use `tempfile::tempdir()` for temporary output paths so tests clean up
  automatically.
- Use `src/test_helpers.rs` for locating sample `.nc` files; do not hardcode
  absolute paths.
- Integration tests that require real AWS credentials should use
  `testcontainers` with `localstack` (see existing S3 tests for the pattern).
- Property-based tests go in `test_properties.rs`. Wrap each `proptest!` block
  in a named `mod` so the test runner output is readable.

### Memory Profiling

A DHAT heap-profiling test exists behind the `dhat-heap` feature flag. Run it
only when investigating allocation regressions:

```bash
cargo test --features dhat-heap --lib -- memory_profile::dhat_profile --nocapture
```

The test writes `dhat-heap.json` to the current directory. Visualise the
result at <https://nnethercote.github.io/dh_view/dh_view.html>.

---

## Benchmarks

Four Criterion benchmark suites cover the main hot paths:

```bash
# Run all benchmarks
cargo bench

# Run a single suite
cargo bench --bench extraction_bench
cargo bench --bench filter_bench
cargo bench --bench postprocess_bench
cargo bench --bench combination_bench

# Save a baseline to compare against later
cargo bench -- --save-baseline before_my_change

# Compare against a saved baseline
cargo bench -- --baseline before_my_change
```

Criterion writes HTML reports to `target/criterion/`. When a PR touches
`src/extract.rs`, `src/filters.rs`, `src/postprocess.rs`, or
`src/output.rs`, include benchmark results in the PR description so
reviewers can assess performance impact.

---

## Pull Request Process

### Branch Naming

```
feature/short-description
fix/issue-number-short-description
docs/what-changed
refactor/short-description
```

### Commit Messages

This project uses [Conventional Commits](https://www.conventionalcommits.org/):

```
feat: add range filter negation support
fix: handle NetCDF files with zero-length dimensions
docs: document PostProcessor trait extension points
refactor: extract filter intersection logic into FilterSet
test: add property tests for formula parser edge cases
chore: update polars to 0.52
```

Use the imperative mood in the subject line. Keep it under 72 characters. Add
a body paragraph when the motivation is not obvious from the subject alone.

### Before Opening a PR

1. All CI checks pass locally:

   ```bash
   cargo fmt --all -- --check
   cargo clippy --all-targets --all-features -- -D warnings
   cargo test --lib --verbose
   cargo test --doc
   cargo audit
   ```

2. New public items have doc-comments with at least one compilable example.
3. `CHANGELOG.md` has an entry under `[Unreleased]` in the appropriate
   category (`Added`, `Changed`, `Fixed`, `Removed`, `Security`).
4. If the change touches a module that has benchmarks, run them and note
   any significant changes in the PR description.

### CI Pipeline

The CI pipeline runs on every push to `main` and `develop` and on every PR
targeting `main`. It runs on a 3-entry matrix: **Ubuntu (stable + beta)** and
**macOS (stable)**:

| Job                  | Command                                                    |
| -------------------- | ---------------------------------------------------------- |
| Format check         | `cargo fmt --all -- --check`                               |
| Clippy               | `cargo clippy --all-targets --all-features -- -D warnings` |
| Library tests        | `cargo test --lib --verbose`                               |
| Doc-tests            | `cargo test --doc --verbose`                               |
| Coverage             | `cargo tarpaulin --out xml --lib` (uploads to Codecov)     |
| Security audit       | `cargo audit`                                              |
| Benchmark regression | `cargo bench` (separate workflow, path-filtered)           |

A PR must be green on all three matrix entries before it will be reviewed.

Binary releases are built via `cargo-dist` for Linux (x86_64, aarch64) and macOS (x86_64, aarch64) and published to GitHub Releases on each version tag.

---

## Reporting Issues

### Bug Reports

Open an issue on [GitHub Issues](https://github.com/rjmalves/nc2parquet/issues)
and include:

- The nc2parquet version (`nc2parquet --version`) or the Git commit hash.
- The Rust toolchain version (`rustc --version`).
- The operating system and version.
- A minimal reproduction — ideally a single command or a short Rust snippet
  together with the smallest `.nc` file that triggers the problem.
- The full error output, including any backtraces (`RUST_BACKTRACE=1 nc2parquet ...`).

### Feature Requests

Open a GitHub Issue with the title prefix `[Feature]`. Describe:

- The problem you are trying to solve and why the existing behaviour does not
  cover it.
- A concrete proposed interface — a CLI flag, a config key, a new public
  function signature, or a trait method.
- Any trade-offs or alternative designs you considered.

If you plan to implement the feature yourself, mention that in the issue so
we can discuss the design before you write code.

### Security Vulnerabilities

Do not open a public GitHub issue for security vulnerabilities. Send a private
report via the
[GitHub Security Advisory](https://github.com/rjmalves/nc2parquet/security/advisories/new)
interface.

---

## Architecture Decision Records

Significant design decisions are captured as Architecture Decision Records in
`docs/adr/`. Each ADR records the context that motivated a decision, the
decision itself, its consequences, and the alternatives that were rejected.

ADRs use four-digit sequential numbering (`docs/adr/0001-short-title.md`).
An accepted ADR is never edited. When a decision is revisited, a new ADR is
created that supersedes the old one.

When opening a PR that introduces a non-obvious architectural change — a new
trait, a change to the public API contract, a new storage backend, or a
structural reorganisation — consider whether the decision warrants an ADR. If
you are unsure, raise it in the PR description and a maintainer will advise.
