# ADR 0002: Module Structure and Visibility

## Status

Accepted

## Date

2026-02-23

## Context

The project compiles both as a library crate (consumed by other Rust code and
tested in isolation) and as a CLI binary. The library and the binary have
different dependency needs: the binary uses `indicatif` for progress bars and
`anyhow` for error context chains; the library should carry neither.

Without an explicit boundary, binary-only concerns tend to migrate into the
library over time, pulling in dependencies and widening the public API surface
beyond what is intentional.

Additionally, some modules contain implementation details that must be
accessible across the crate but should not form part of the public API. Rust's
visibility system makes it possible to enforce this at compile time, but only
if the distinction is established early and consistently.

## Decision

The codebase is split into two compilation roots:

- `src/lib.rs` — the library crate; re-exports the public API via `pub use`
- `src/main.rs` — the binary crate; declares `mod handlers` and drives the CLI

**Public modules** (accessible to library consumers):
`cli`, `errors`, `filters`, `info`, `input`, `postprocess`, `storage`

**Crate-internal modules** (declared `pub(crate)` in `src/lib.rs`):
`extract`, `output`

These two modules implement data extraction from NetCDF files and Parquet
serialization respectively. They are not stable public API and may change
without a semver bump.

**Handler modules** under `src/handlers/` are declared in `src/main.rs`, not
in `src/lib.rs`. The directory contains seven files — `completions`, `config`,
`convert`, `info`, `template`, `utils`, and `validate` — which export five
public handler functions. Declaring them only from the binary root prevents
`indicatif` and `anyhow` from being required by library builds.

**Tests** live in `src/tests/` with eleven module-specific files, guarded by
`#[cfg(test)]`. The module is declared in `src/lib.rs` so tests can access
`pub(crate)` items. A companion `src/test_helpers.rs` module (also
`#[cfg(test)]`) provides shared fixtures.

**Benchmarks** live in `benches/` with four Criterion files:
`combination_bench`, `extraction_bench`, `filter_bench`, and
`postprocess_bench`. Criterion requires them in a separate directory.

## Consequences

### Positive

- The library compiles without `indicatif` or `anyhow`; consumers do not
  inherit those transitive dependencies.
- `pub(crate)` on `extract` and `output` prevents accidental stabilization of
  internal APIs; the compiler enforces the boundary.
- Handler logic is co-located with the binary entry point, making it obvious
  that it is not library API.

### Negative

- The `src/handlers/` directory can only be tested via integration paths that
  go through the binary, not through `src/tests/`. Unit-testing handler
  functions requires either refactoring them to accept injectable dependencies
  or testing at the CLI level.
- The split between `lib.rs` and `main.rs` means that developers unfamiliar
  with the structure may be confused about where to look for a given piece of
  logic.

## Alternatives Considered

1. **Everything public** — Rejected. It exposes `extract` and `output`
   as public API, committing to their stability and pulling all transitive
   dependencies into library consumers.

2. **Workspace with separate crates** — Rejected. The project is small enough
   that a single-crate binary/library split provides the same separation with
   less build configuration overhead.

3. **Handlers in `lib.rs`** — Rejected. It would make `indicatif` and `anyhow`
   library dependencies, widening the dependency footprint for every consumer.
