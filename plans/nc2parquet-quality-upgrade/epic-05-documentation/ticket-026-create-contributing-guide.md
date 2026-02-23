# ticket-026 Create CONTRIBUTING Guide

## Context

### Background

The project currently has no `CONTRIBUTING.md` file. The README includes a minimal 5-step contributing section ("Fork, branch, test, PR") that provides no real guidance. New contributors need clear documentation on: how to set up the development environment (including the NetCDF system library dependency), the project's module architecture, coding standards, testing conventions, and the PR review process. This is essential for community adoption.

### Relation to Epic

This is the third ticket in Epic 05. The CONTRIBUTING guide is linked from the README (ticket-024) and provides the onboarding path for new contributors. The ADRs (ticket-027) complement this by explaining why certain architectural decisions were made, and the CONTRIBUTING guide should link to them.

### Current State

- **File**: `/home/rogerio/git/nc2parquet/CONTRIBUTING.md` -- does not exist.
- **Rust edition**: 2024 (set in `Cargo.toml`).
- **Rust version required**: At minimum Rust 1.85+ (edition 2024 requires this; current local version is 1.92.0).
- **System dependency**: `libnetcdf-dev` + `libhdf5-dev` on Linux (installed in CI via `sudo apt-get install -y netcdf-bin libnetcdf-dev libhdf5-dev`). The `netcdf` crate uses `features = ["static"]` which links statically but still requires build-time headers.
- **Module structure** (from learnings):
  - `src/lib.rs` -- public API: `process_netcdf_job`, `process_netcdf_job_async`, `process_netcdf_batch`, `resolve_output_path`; re-exports `BatchConfig`, `BatchResult`, `Nc2ParquetError`
  - `src/extract.rs` -- data extraction (`pub(crate)`)
  - `src/postprocess.rs` -- PostProcessor trait, ProcessingPipeline, 5 processor types
  - `src/input.rs` -- JobConfig, BatchConfig, FilterConfig, OutputConfig
  - `src/output.rs` -- Parquet writing (`pub(crate)`)
  - `src/filters.rs` -- NCFilter trait, 4 filter implementations
  - `src/errors.rs` -- Nc2ParquetError (11 variants, thiserror)
  - `src/storage.rs` -- StorageBackend trait, S3 + local implementations
  - `src/handlers/` -- 8 binary-only handler files (declared in main.rs)
  - `src/cli.rs` -- clap CLI definition
  - `src/tests/` -- 11 test files
  - `benches/` -- 4 Criterion benchmark files
- **CI checks**: `cargo fmt --all -- --check`, `cargo clippy --all-targets --all-features -- -D warnings`, `cargo test --lib --verbose`, `cargo audit`.
- **Test count**: 296 lib tests, 28 doc tests, 1 DHAT test.

## Specification

### Requirements

1. **Create** `/home/rogerio/git/nc2parquet/CONTRIBUTING.md`.
2. **Include the following sections**:
   - **Welcome / Introduction**: Brief welcoming statement, link to Code of Conduct (placeholder if none exists yet).
   - **Getting Started / Prerequisites**: System requirements (Rust 1.85+, system NetCDF libraries per platform).
   - **Development Setup**: Step-by-step clone, build, test instructions for Linux (primary), macOS, and Windows (noting limitations).
   - **Project Architecture**: Overview of module structure with brief descriptions of each module's responsibility. Reference the `src/` layout from the learnings.
   - **Coding Standards**: Formatting (`cargo fmt`), linting (`cargo clippy -- -D warnings`), visibility conventions (`pub(crate)` for internal modules), error handling pattern (thiserror + `Nc2ParquetError`), doc-comment requirements.
   - **Testing**: How to run tests (`cargo test --lib`, `cargo test --doc`), test file organization (`src/tests/` with 11 module-specific files), property-based tests (proptest), benchmarks (`cargo bench`).
   - **Pull Request Process**: Branch naming, commit message conventions (conventional commits), what CI checks must pass, review expectations.
   - **Reporting Issues**: Bug report and feature request templates or guidelines.
   - **Architecture Decision Records**: Brief explanation and link to `docs/adr/` (created in ticket-027).
3. **Include platform-specific build instructions**:
   - Linux (Ubuntu/Debian): `sudo apt-get install -y libnetcdf-dev libhdf5-dev`
   - Linux (Fedora): `sudo dnf install -y netcdf-devel hdf5-devel`
   - macOS: `brew install netcdf`
   - Windows: Note that the `netcdf` crate with `features = ["static"]` may work but is less tested.

### Inputs/Props

- Module structure and conventions from learnings.
- CI workflow from `.github/workflows/ci.yml`.
- `Cargo.toml` for dependency and feature information.

### Outputs/Behavior

A single new `/home/rogerio/git/nc2parquet/CONTRIBUTING.md` file.

### Error Handling

Not applicable (documentation file).

## Acceptance Criteria

- [ ] Given the new CONTRIBUTING.md, when inspecting the file, then it contains a "Prerequisites" or "Getting Started" section listing Rust version requirement (1.85+) and system NetCDF library installation commands for at least Linux (Ubuntu) and macOS
- [ ] Given the new CONTRIBUTING.md, when inspecting the file, then it contains a "Development Setup" section with clone, build, and test commands
- [ ] Given the new CONTRIBUTING.md, when inspecting the file, then it contains a "Project Architecture" section listing at least the key modules: `lib.rs`, `extract.rs`, `postprocess.rs`, `input.rs`, `filters.rs`, `errors.rs`, `storage.rs`, `handlers/`
- [ ] Given the new CONTRIBUTING.md, when inspecting the file, then it contains a "Coding Standards" section mentioning `cargo fmt`, `cargo clippy`, and the thiserror error handling pattern
- [ ] Given the new CONTRIBUTING.md, when inspecting the file, then it contains a "Testing" section explaining how to run lib tests, doc tests, and benchmarks
- [ ] Given the new CONTRIBUTING.md, when inspecting the file, then it contains a "Pull Request Process" section describing branch naming and CI requirements
- [ ] Given the new CONTRIBUTING.md, when inspecting the file, then it contains a link or reference to the Architecture Decision Records in `docs/adr/`

## Implementation Guide

### Suggested Approach

1. Create the file with a welcoming introduction.
2. Write the prerequisites section using the CI workflow's `apt-get install` line as the canonical Linux dependency list.
3. Write development setup as a numbered step-by-step (clone, install deps, build, test).
4. Write the architecture section using the module list from the learnings. Keep descriptions to one sentence per module.
5. Write coding standards based on the CI checks (fmt, clippy) and the patterns documented in learnings (visibility, error handling, doc comments).
6. Write the testing section covering the 3 test categories: lib tests, doc tests, benchmarks.
7. Write the PR process section based on standard Rust open-source conventions.
8. Add a brief "Reporting Issues" section.
9. Add a link to ADRs (even though they don't exist yet -- ticket-027 will create them).

### Key Files to Create

- `/home/rogerio/git/nc2parquet/CONTRIBUTING.md` -- new file

### Patterns to Follow

- Follow the structure of well-known Rust project CONTRIBUTING guides (e.g., ripgrep, tokio).
- Use markdown headers consistently (## for major sections, ### for subsections).
- Keep the guide actionable: every section should tell the reader what to do, not just what exists.
- Use code blocks for all commands.

### Pitfalls to Avoid

- Do not assume contributors are on Linux -- include macOS instructions (the second most common dev platform for Rust).
- Do not forget to mention that the `netcdf` crate uses `features = ["static"]`, which means it links the NetCDF C library statically but still requires headers at compile time.
- Do not include overly specific internal implementation details -- the CONTRIBUTING guide should orient contributors, not replace reading the code.
- Do not reference Rust nightly features -- the project targets stable Rust.
- Do not forget to mention `cargo test --lib` (which is what CI runs) vs `cargo test` (which also runs doc tests and integration tests).

## Testing Requirements

### Unit Tests

Not applicable (documentation).

### Integration Tests

Not applicable (documentation).

### Manual Verification

- Verify the build instructions work by following them on a clean checkout.
- Verify all referenced file paths (e.g., `src/tests/`, `benches/`, `docs/adr/`) are accurate.

## Dependencies

- **Blocked By**: ticket-009 (module structure finalized -- already done)
- **Blocks**: None

## Effort Estimate

**Points**: 2
**Confidence**: High
