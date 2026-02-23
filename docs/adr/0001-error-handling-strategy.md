# ADR 0001: Error Handling Strategy

## Status

Accepted

## Date

2026-02-23

## Context

A Rust library needs a consistent error strategy that serves two audiences
simultaneously: callers who need to match on specific error variants to take
corrective action, and application code that simply needs to propagate errors
with context.

Using `anyhow` throughout would satisfy the application side but erase variant
information, making it impossible for library consumers to distinguish, for
example, a missing variable from an I/O failure. Using multiple unrelated error
types per module would preserve that information but force callers to handle a
fragmented hierarchy.

The library also integrates with several external crates (netcdf, Polars,
the AWS SDK) whose error types vary significantly in size. The AWS SDK error
variants are large enough that including `StorageError` directly in the
top-level enum would inflate the size of every `Result` across the codebase.

## Decision

The library exposes a single `Nc2ParquetError` enum defined in `src/errors.rs`.
All public functions return `Result<T, Nc2ParquetError>`. The enum is derived
with `thiserror::Error` and covers every error origin:

- `NetCdf` — wraps `netcdf::Error` via `#[from]`
- `VariableNotFound(String)` — named variable absent from the file
- `DimensionNotFound(String)` — named dimension absent from the file
- `Filter(String)` — filter configuration or evaluation failure
- `Extraction(String)` — data extraction failure
- `PostProcess` — wraps `PostProcessError` via `#[from]`
- `Storage(Box<StorageError>)` — storage backend failure, boxed (see below)
- `Io` — wraps `std::io::Error` via `#[from]`
- `Polars` — wraps `polars::prelude::PolarsError` via `#[from]`
- `Config(String)` — invalid configuration at runtime
- `Serialization(String)` — serialization failure
- `UnsupportedDimensionality(usize)` — variable has more dimensions than supported

`StorageError` is wrapped in a `Box` because it contains three AWS SDK error
variants whose generic instantiations are large. Boxing reduces the size of
`Nc2ParquetError` and therefore the size of every `Result<T, Nc2ParquetError>`
on the stack. Because `#[from]` does not support boxed wrapping, a manual
`impl From<StorageError> for Nc2ParquetError` is provided.

`anyhow` is used only in `src/main.rs` and the handler modules under
`src/handlers/`, where error context chains are logged rather than returned
across API boundaries.

The post-processing subsystem defines its own `PostProcessError` in
`src/postprocess.rs`. It is a separate type because post-processing can be used
independently of the conversion pipeline; it converts into `Nc2ParquetError`
via `#[from]`.

## Consequences

### Positive

- Callers can pattern-match on specific variants to implement retry logic,
  fallback paths, or user-facing error messages.
- `thiserror`-generated `Display` impls produce consistent, human-readable
  messages without manual formatting.
- Boxing `StorageError` prevents enum size bloat and keeps stack frames lean
  on the hot conversion path.
- The `anyhow`/`thiserror` split means the library carries no `anyhow`
  dependency; only the binary does.

### Negative

- The manual `From<StorageError>` implementation must be kept in sync if
  `StorageError` is ever renamed or refactored.
- A single unified enum means adding a new error origin requires modifying
  `src/errors.rs` and re-exporting, even for errors that are only relevant
  to one subsystem.

## Alternatives Considered

1. **`anyhow` everywhere** — Rejected. Callers cannot pattern-match on
   `anyhow::Error`; the library would be unusable for programmatic error
   handling.

2. **One error type per module** — Rejected. It fragments the public API
   surface, forces callers to import multiple error types, and complicates
   the `?` operator across module boundaries.

3. **Box all variants** — Rejected. Only `StorageError` is large enough to
   justify boxing. Boxing all variants adds a heap allocation on every error
   path and obscures the enum structure.
