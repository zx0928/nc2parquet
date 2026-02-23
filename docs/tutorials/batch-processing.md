# Batch Processing Multiple Files

This tutorial shows how to convert a directory of NetCDF files in a single
`convert` invocation using the `--glob` flag.

## Prerequisites

- [Basic NetCDF to Parquet Conversion](basic-conversion.md) completed
- An `output/` directory available: `mkdir -p output`

---

### Step 1: Use `--glob` for Pattern Matching

When `--glob` is provided, `convert` matches all files that satisfy the glob
pattern and converts each one. The positional `OUTPUT` argument is treated as
the **output directory** in this mode.

```bash
nc2parquet convert output/ \
  -n data \
  --glob "examples/data/*.nc"
```

Every matched `.nc` file is converted and written into `output/` with the same
stem and a `.parquet` extension. For example, `examples/data/simple_xy.nc`
becomes `output/simple_xy.parquet`.

> **Note**: The output directory must be supplied as the positional `OUTPUT`
> argument. The `--glob` flag does not accept a separate output directory flag;
> that convention only exists in the `BatchConfig` JSON format (see Step 5).

---

### Step 2: Apply Filters to Every File in the Batch

All filter flags (`--range`, `--list`, `--point2d`) are applied uniformly to
every file matched by the glob pattern.

```bash
nc2parquet convert output/ \
  -n temperature \
  --glob "examples/data/*.nc" \
  --range "latitude:25:45"
```

Files whose variable or dimension names do not match the filter will be
reported as failures in the summary.

---

### Step 3: Configure Compression

The `--compression` flag selects the Parquet compression codec. Supported
values are `snappy`, `zstd`, `gzip`, `lz4`, and `uncompressed`. The default
is `snappy`.

```bash
nc2parquet convert output/ \
  -n data \
  --glob "examples/data/*.nc" \
  --compression zstd \
  --compression-level 3
```

Compression level ranges:

- `zstd`: 1–22 (lower is faster, higher compresses more)
- `gzip`: 0–9
- `snappy`, `lz4`, `uncompressed`: do not accept a level

---

### Step 4: Understand Error Handling

By default, batch mode continues past individual file errors and reports a
summary at the end:

```
Batch complete: 2/3 files succeeded, 1 failed
  OK  output/file_a.parquet
  OK  output/file_b.parquet
  ERR examples/data/bad.nc — variable 'data' not found
```

To stop immediately on the first error instead of collecting all failures, set
`fail_fast: true` in a `BatchConfig` JSON file and supply it with `--config`
(see Step 5).

---

### Step 5: Equivalent `BatchConfig` JSON

The batch behaviour shown above can also be expressed as a JSON config file.
This is the only way to set `output_template` and `fail_fast` from outside the
library, because those options do not have dedicated CLI flags.

```json
{
  "pattern": "examples/data/*.nc",
  "output_dir": "output/",
  "variable_name": "data",
  "filters": [],
  "output_template": "{stem}_converted.parquet",
  "fail_fast": false,
  "output": {
    "compression": "zstd",
    "compression_level": 3
  }
}
```

The `output_template` field supports two placeholders:

- `{stem}` — the filename without extension (e.g. `simple_xy`)
- `{name}` — the full filename including extension (e.g. `simple_xy.nc`)

Save the JSON above to `examples/configs/batch_config.json` and run:

```bash
nc2parquet convert --config examples/configs/batch_config.json
```

> **Note**: When supplying a `BatchConfig` via `--config`, the `convert`
> command reads the `pattern` and `output_dir` fields from the JSON file.
> The positional `INPUT` and `OUTPUT` arguments are not used in this case.

---

## What's Next

The next tutorial explains how to generate, edit, and validate configuration
files, and how CLI flags, environment variables, and config files interact:

[Working with Configuration Files](config-files.md)
