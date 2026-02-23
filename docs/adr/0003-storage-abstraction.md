# ADR 0003: Storage Abstraction

## Status

Accepted

## Date

2026-02-23

## Context

The tool must read NetCDF files from and write Parquet files to both the local
filesystem and Amazon S3. These two backends share the same logical interface
(read bytes, write bytes, test existence) but differ entirely in their I/O
mechanics and error types.

A key constraint shapes the design: the `netcdf` C library requires a
filesystem path to open a file. It has no API for reading from an in-memory
buffer or a stream. This means S3 objects cannot be passed directly to the
NetCDF parser; they must first be materialized on disk.

A second consideration is dispatch overhead. Runtime polymorphism via
`Box<dyn Trait>` incurs a heap allocation at construction and a vtable
indirection on every call. For a tool that makes a handful of storage calls
per file conversion, this is not a performance concern, but it is unnecessary
complexity given that the set of backends is closed.

## Decision

All storage operations are defined in `src/storage.rs`.

`StorageBackend` is an async trait with three methods:

```rust
async fn read(&self, path: &str) -> StorageResult<Vec<u8>>;
async fn write(&self, path: &str, data: &[u8]) -> StorageResult<()>;
async fn exists(&self, path: &str) -> StorageResult<bool>;
```

Two concrete types implement it: `LocalStorage` (wraps `tokio::fs`) and
`S3Storage` (wraps `aws_sdk_s3::Client`).

A `Storage` enum wraps both implementations and also implements
`StorageBackend` via match dispatch. This gives static dispatch through
the enum without heap allocation.

`StorageFactory::from_path` inspects the path string: if it starts with
`s3://`, an `S3Storage` instance is created by loading credentials from the
environment or an IAM role; otherwise `LocalStorage` is returned.

`StorageError` has nine variants. Three cover AWS SDK operation failures
(`S3GetObject`, `S3PutObject`, `S3HeadObject`), one covers byte-stream
collection failures (`ByteStream`), and the remainder cover filesystem and
path validation errors. The AWS SDK variants are kept in `StorageError` rather
than in `Nc2ParquetError` to contain their large generic instantiations; only
the boxed `Nc2ParquetError::Storage(Box<StorageError>)` escapes that module.

For S3 input, `process_netcdf_job_async` in `src/lib.rs` downloads the object
into a `tempfile::NamedTempFile`, opens the NetCDF file from the temporary
path, and removes the temporary file after the Parquet output has been written.
The `temp_file_path` variable is held across the extraction block so cleanup
always runs.

## Consequences

### Positive

- Callers interact with a single `Storage` enum; no type parameters or trait
  objects are needed at call sites.
- Adding a new backend (e.g. Azure Blob Storage) requires only a new struct,
  a new enum variant, and a dispatch arm in `StorageFactory::from_path`.
- S3 credentials are resolved by the AWS SDK's default provider chain; no
  credential configuration is baked into the library.
- `StorageError` isolates the large AWS SDK types; the rest of the codebase
  sees only `Box<StorageError>` through `Nc2ParquetError`.

### Negative

- S3 reads materialize the entire file in memory before writing to disk, then
  read it back from disk. For very large NetCDF files this is a doubling of
  peak memory pressure relative to a local read.
- The temporary file is written to the OS default temp directory. On systems
  where `/tmp` is memory-backed (tmpfs), this does not reduce memory pressure.
- `S3Storage` is always constructed with `aws_config::defaults`, which performs
  network calls to resolve credentials and region. This adds latency on the
  first S3 operation.

## Alternatives Considered

1. **`Box<dyn StorageBackend>` for runtime dispatch** — Rejected. The enum
   variant set is closed; heap allocation and vtable dispatch add complexity
   with no flexibility benefit.

2. **`object_store` crate** — Rejected. The AWS SDK gives finer-grained
   control over S3-specific behaviour (credential resolution, region
   configuration, error mapping) and is already present as a direct
   dependency.

3. **Streaming S3 reads into the NetCDF parser** — Rejected. The `netcdf` C
   library requires a filesystem path and does not provide a stream-based
   API. Streaming is architecturally incompatible.
