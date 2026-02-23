# Performance: Peak Memory Profile

This document describes the memory usage characteristics of nc2parquet, the
methodology used to measure them, and how to reproduce the measurements.

## Profiling Methodology

nc2parquet uses [DHAT](https://valgrind.org/docs/manual/dh-manual.html) (a
heap profiler included with Valgrind) via the
[`dhat` Rust crate](https://crates.io/crates/dhat) to instrument heap
allocations at runtime. DHAT records every allocation and deallocation
performed during the profiler's active lifetime, reporting three key figures:

| Metric     | Description                                                                               |
| ---------- | ----------------------------------------------------------------------------------------- |
| **Total**  | Cumulative bytes allocated across the entire run                                          |
| **t-gmax** | Bytes live simultaneously at the point of peak heap usage                                 |
| **t-end**  | Bytes still live when the profiler is shut down (the "high-water mark" visible to the OS) |

The DHAT profiler is gated behind the `dhat-heap` Cargo feature so it is
never active in normal builds. When enabled it replaces the global allocator
with `dhat::Alloc`.

## Running the DHAT Profiler

```bash
cargo test --features dhat-heap --lib \
    -- memory_profile::dhat_profile \
    --nocapture
```

On completion, DHAT prints a summary to stderr and writes `dhat-heap.json` to
the current directory. Visualise the full call-tree breakdown at:

<https://nnethercote.github.io/dh_view/dh_view.html>

Upload `dhat-heap.json` to that page to see which call sites are responsible
for the most allocation.

## Measured Peak Memory

Test fixture: `examples/data/pres_temp_4D.nc`
Extraction: variable `temperature`, all dimensions, no filters
Output: local Parquet file via `process_netcdf_job`
Build: `--release`

| Metric     | Value                        |
| ---------- | ---------------------------- |
| Total      | 1,320,662 bytes (1.3 MB)     |
| **t-gmax** | **1,242,218 bytes (1.2 MB)** |
| t-end      | 1,198,520 bytes (1.2 MB)     |

The `pres_temp_4D.nc` fixture contains a 4-dimensional variable
(time=2, level=2, latitude=6, longitude=12), yielding 288 output rows with
five columns. Peak memory at t-gmax reflects the period when the output
`DataFrame` is live and being serialised to Parquet.

## Memory Optimisations Applied (Ticket 018)

The following changes were made to minimise peak memory usage:

### 1. Eliminate DataFrame clone in Parquet writing (`src/output.rs`)

`write_dataframe_to_parquet`, `write_dataframe_to_parquet_async`, and the
internal `dataframe_to_parquet_bytes` helper all previously cloned the
`DataFrame` because `ParquetWriter::finish` requires `&mut DataFrame`. The
signatures were changed to accept `&mut DataFrame`, eliminating one full
in-memory copy of every column at write time.

### 2. Drop NetCDF file before writing (`src/lib.rs`)

Both `process_netcdf_job` and `process_netcdf_job_async` previously held the
`netcdf::File` open across postprocessing and Parquet serialisation. A
scoped block now ensures the `netcdf::Variable` borrow (which pins the file)
is dropped immediately after extraction, and `file.close()` is called before
postprocessing begins. This releases the file descriptor and any internal
NetCDF library buffers before the peak-memory Parquet write step.

### 3. Drop intermediate data eagerly in extraction (`src/extract.rs`)

In `extract_data_cellwise` and `extract_data_batch`, the `coordinate_vars`
HashMap and the raw slab `Vec<f32>` are now confined to an inner scope. They
are freed before `build_dataframe` copies data into Polars `Series` objects,
preventing the old and new representations from being live simultaneously.

## Interpreting Results

For the 288-row test fixture, peak memory is dominated by Polars internals
(Arrow column buffers, schema metadata) rather than the NetCDF read buffers.
For production workloads with millions of rows, the savings from eliminating
the DataFrame clone scale linearly with the number of rows and columns.

To measure a specific workload, modify the `dhat_profile`
test in `src/tests/test_memory_profile.rs` to point at the target file and
variable, then re-run the command above.
