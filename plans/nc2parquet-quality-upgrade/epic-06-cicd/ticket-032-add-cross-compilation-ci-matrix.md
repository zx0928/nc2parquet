# ticket-032 Add Cross-Compilation CI Matrix

## Context

### Background

The current CI pipeline (`.github/workflows/ci.yml`) runs exclusively on `ubuntu-latest` for both `stable` and `beta` Rust toolchains. This means the project is only validated on a single platform (Linux x86_64). Adding a CI matrix with macOS runners validates that the codebase compiles and tests pass on macOS, which is one of the four release targets configured in cargo-dist. Cross-compilation to `aarch64-unknown-linux-gnu` is handled by cargo-dist at release time (via its container-based build), so the CI matrix focuses on native compilation on available GitHub Actions runners.

### Relation to Epic

This is the fourth ticket in Epic 06. It must be completed BEFORE ticket-031 (release automation) because ticket-031 adds macOS targets to cargo-dist, and this ticket validates that the code actually compiles and tests pass on macOS. Without this validation, the release pipeline could produce broken macOS binaries.

### Current State

- **CI workflow** (`/home/rogerio/git/nc2parquet/.github/workflows/ci.yml`):
  - `test` job: `ubuntu-latest` with matrix `rust: [stable, beta]`. Runs fmt, clippy, and `cargo test --lib`.
  - `security-audit` job: `ubuntu-latest` with `stable` Rust. Runs `cargo audit`.
  - System dependencies installed via `apt-get`: `netcdf-bin libnetcdf-dev libhdf5-dev`.
- **netcdf crate**: Uses `features = ["static"]` which statically links the NetCDF and HDF5 C libraries. On macOS, the header files are available via `brew install netcdf`.
- **dist-workspace.toml**: Currently targets `aarch64-unknown-linux-gnu` and `x86_64-unknown-linux-gnu` only. Ticket-031 will add macOS targets.
- **CONTRIBUTING.md**: Documents platform-specific build instructions for Ubuntu, Fedora, macOS (Homebrew), Windows, and Docker (lines 69-112).

## Specification

### Requirements

1. Expand the `test` job in `.github/workflows/ci.yml` to use an OS matrix: `ubuntu-latest` and `macos-latest`.
2. Keep the Rust version matrix (`stable`, `beta`) on `ubuntu-latest` only. On `macos-latest`, run `stable` only. This avoids doubling CI costs while still validating macOS compilation.
3. Conditionally install system dependencies based on the runner OS:
   - Ubuntu: `sudo apt-get update && sudo apt-get install -y netcdf-bin libnetcdf-dev libhdf5-dev`
   - macOS: `brew install netcdf` (Homebrew installs both headers and libraries)
4. Run the same checks on both platforms: `cargo fmt --check`, `cargo clippy`, `cargo test --lib`.
5. The `security-audit` job remains unchanged (runs only on `ubuntu-latest`).
6. Do NOT add Windows to the matrix. The netcdf crate's static build on Windows is unreliable (documented in CONTRIBUTING.md lines 95-101).
7. Do NOT add `aarch64-unknown-linux-gnu` cross-compilation to the CI matrix. This target is built via cargo-dist's container-based approach at release time, and cross-compiling it in CI would require QEMU or the `cross` tool, adding complexity without proportional benefit.
8. Update the CI cache keys to include the OS to avoid cache collisions between Ubuntu and macOS builds.

### Inputs/Props

- **GitHub Actions runners**: `ubuntu-latest` (Linux x86_64) and `macos-latest` (macOS arm64, Apple Silicon).
- **Matrix strategy**: Use `include` entries to control which Rust versions run on which OS.

### Outputs/Behavior

- The CI pipeline runs three test configurations: Ubuntu/stable, Ubuntu/beta, macOS/stable.
- All three must pass for a PR to be mergeable.
- macOS builds validate that the netcdf static linkage and all tests work on Apple Silicon.

### Error Handling

- If macOS system dependency installation fails (e.g., Homebrew issue), the macOS matrix entry fails. This is correct and expected.
- If a test fails only on macOS, the CI is red and the failure is visible in the matrix view.

## Acceptance Criteria

- [ ] Given `.github/workflows/ci.yml`, when inspected, then the `test` job uses a matrix strategy that includes `ubuntu-latest` with `stable` and `beta` Rust, and `macos-latest` with `stable` Rust only.
- [ ] Given the `test` job, when inspected, then system dependency installation is conditional: `apt-get` on Ubuntu, `brew install netcdf` on macOS.
- [ ] Given the `test` job, when inspected, then the cache key includes `${{ runner.os }}` to separate Ubuntu and macOS caches.
- [ ] Given the `security-audit` job, when inspected, then it is UNCHANGED (still runs on `ubuntu-latest` only).
- [ ] Given the workflow, when inspected, then no Windows or aarch64-linux cross-compilation entries exist in the matrix.
- [ ] Given the CONTRIBUTING.md CI pipeline table, when inspected, then it mentions the multi-platform matrix (Ubuntu + macOS).

## Implementation Guide

### Suggested Approach

1. **Restructure the `test` job matrix** in `/home/rogerio/git/nc2parquet/.github/workflows/ci.yml`:

Replace the current simple matrix:

```yaml
strategy:
  matrix:
    rust: [stable, beta]
```

With an `include`-based matrix that controls OS and Rust version combinations:

```yaml
strategy:
  fail-fast: false
  matrix:
    include:
      - os: ubuntu-latest
        rust: stable
      - os: ubuntu-latest
        rust: beta
      - os: macos-latest
        rust: stable
runs-on: ${{ matrix.os }}
```

2. **Make system dependency installation conditional**:

Replace the current step:

```yaml
- name: Install system dependencies
  run: |
    sudo apt-get update
    sudo apt-get install -y netcdf-bin libnetcdf-dev libhdf5-dev
```

With conditional steps:

```yaml
- name: Install system dependencies (Linux)
  if: runner.os == 'Linux'
  run: |
    sudo apt-get update
    sudo apt-get install -y netcdf-bin libnetcdf-dev libhdf5-dev

- name: Install system dependencies (macOS)
  if: runner.os == 'macOS'
  run: brew install netcdf
```

3. **Update the cache key** to include the OS:

```yaml
key: ${{ runner.os }}-cargo-${{ hashFiles('**/Cargo.lock') }}
```

(This is already partially correct in the existing config -- verify that `runner.os` is used, not just a hardcoded prefix.)

4. **Update CONTRIBUTING.md**: Modify the CI pipeline table (around line 414-421) to note that tests run on Ubuntu (stable + beta) and macOS (stable).

### Key Files to Modify

- `/home/rogerio/git/nc2parquet/.github/workflows/ci.yml` -- restructure the `test` job matrix
- `/home/rogerio/git/nc2parquet/CONTRIBUTING.md` -- update CI pipeline table description

### Patterns to Follow

- Use `include`-based matrix entries rather than the Cartesian product of `os` x `rust`. This gives precise control over which combinations run.
- Use `fail-fast: false` so that a failure on one platform does not cancel the other platform's run. This provides maximum diagnostic information.
- Follow the existing step naming convention in `ci.yml` (e.g., "Install Rust", "Cache Cargo dependencies").

### Pitfalls to Avoid

- **Do NOT use `macos-13` explicitly**: `macos-latest` currently resolves to an Apple Silicon (arm64) runner, which is the correct target for validation. Hardcoding a specific macOS version makes the config brittle.
- **Do NOT add `rustfmt` and `clippy` component installs for macOS if they are already included in the toolchain**: The `dtolnay/rust-toolchain` action installs components specified in the `components` field. Keep the existing `components: rustfmt, clippy` which works on both platforms.
- **Do NOT forget to update the cache key**: Without OS-specific cache keys, Ubuntu and macOS builds would share a cache, causing compilation errors due to platform-specific compiled artifacts.
- **Do NOT add a separate macOS workflow file**: Keep everything in `ci.yml` using the matrix strategy. A separate workflow would duplicate configuration and make maintenance harder.
- **Beware of `brew install` timing**: Homebrew on GitHub Actions runners may take 30-60 seconds for the `netcdf` formula. This is acceptable and unavoidable.

## Testing Requirements

### Unit Tests

Not applicable -- this ticket modifies CI configuration only.

### Integration Tests

- Validate the YAML syntax of the modified `ci.yml`.
- Verify the matrix expansion produces exactly 3 entries: (ubuntu-latest, stable), (ubuntu-latest, beta), (macos-latest, stable).

### E2E Tests

- After merging, verify that the GitHub Actions CI runs three matrix entries.
- Verify that the macOS entry installs `netcdf` via Homebrew and all tests pass.
- Verify that the Ubuntu entries still work correctly with their `apt-get` installation.

## Dependencies

- **Blocked By**: None (can run in parallel with ticket-029 and ticket-030)
- **Blocks**: ticket-031 (release automation depends on validated macOS builds)

## Effort Estimate

**Points**: 2
**Confidence**: High
