# Basic NetCDF to Parquet Conversion

This tutorial walks through inspecting a NetCDF file and converting it to
Parquet format. You will use two subcommands: `info` to understand the file
structure, and `convert` to produce the output.

## Prerequisites

- nc2parquet installed and on your `PATH`
- Repository cloned so that `examples/data/` is available
- An `output/` directory to hold results: `mkdir -p output`

---

### Step 1: Inspect the NetCDF File

Before converting, examine the file to learn its dimensions and variable names.

```bash
nc2parquet info examples/data/simple_xy.nc
```

For a more detailed report including variable attributes and coordinate values,
add `--detailed`:

```bash
nc2parquet info examples/data/simple_xy.nc --detailed
```

To get machine-readable JSON output for scripting:

```bash
nc2parquet info examples/data/simple_xy.nc --format json
```

The output shows the available variable names (for example, `data`), their
dimensions, and their shapes. Note the exact variable name — you will need it
in the next step.

---

### Step 2: Convert a Single Variable

Use `convert` with the `-n` flag (`--variable`) to extract one variable and
write it as a Parquet file.

```bash
nc2parquet convert examples/data/simple_xy.nc output/basic.parquet \
  -n data
```

The command reads every value of the `data` variable together with the values
of its coordinate dimensions (`x` and `y`) and writes them as columns in a
flat Parquet table.

To preview what the conversion would do without writing any files, use
`--dry-run`:

```bash
nc2parquet convert examples/data/simple_xy.nc output/basic.parquet \
  -n data \
  --dry-run
```

---

### Step 3: Verify the Output File

After a successful conversion the Parquet file should exist on disk:

```bash
ls -lh output/basic.parquet
```

You can inspect it with any Parquet-capable tool, for example
[DuckDB](https://duckdb.org/):

```sql
-- Inside a DuckDB session
SELECT * FROM read_parquet('output/basic.parquet') LIMIT 5;
```

The table will contain one row per grid point and one column per dimension
coordinate plus one value column named after the variable.

---

### Step 4: Extract Multiple Variables

When a NetCDF file contains several variables that share the same dimensions,
you can extract them all into a single Parquet file in one pass using the `-N`
flag (`--variables`). The value is a comma-delimited list of variable names.

```bash
nc2parquet convert examples/data/pres_temp_4D.nc output/multi_var.parquet \
  -N temperature,pressure
```

Each variable becomes its own column in the output Parquet file. All listed
variables must have identical dimension names and sizes.

---

### Step 5: Cloud Storage (S3)

nc2parquet can read from and write to S3 directly. Both the input and output
arguments accept `s3://bucket/path` URIs (e.g. `s3://my-bucket/data.nc`).

S3 access is configured through the standard AWS credential chain (environment
variables, `~/.aws/credentials`, or IAM roles). See the project
[README](../../README.md) for the full S3 setup guide.

---

## What's Next

Now that you can perform a basic conversion, the next tutorial shows how to
narrow the extracted data using dimension filters and transform the result with
post-processing steps:

[Filtered Extraction and Post-Processing](filtered-extraction.md)
