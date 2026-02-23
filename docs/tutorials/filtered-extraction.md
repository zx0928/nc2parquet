# Filtered Extraction and Post-Processing

This tutorial demonstrates how to limit the data extracted from a NetCDF file
using dimension filters, and how to transform the resulting columns with
renaming, formulas, and unit conversion.

## Prerequisites

- [Basic NetCDF to Parquet Conversion](basic-conversion.md) completed
- An `output/` directory available: `mkdir -p output`

---

### Step 1: Inspect the 4D File to Understand Its Dimensions

`pres_temp_4D.nc` is a four-dimensional dataset with `level`, `latitude`, and
`longitude` dimensions. Run `info` before filtering so you know the dimension
names and value ranges.

```bash
nc2parquet info examples/data/pres_temp_4D.nc --detailed
```

The output lists every dimension and its coordinate values. Note the exact
spelling — filter arguments must match the dimension names exactly.

---

### Step 2: Apply a Range Filter on Latitude

The `--range` flag restricts a dimension to a contiguous interval. The
argument format is `dimension:min:max` where both bounds are inclusive.

```bash
nc2parquet convert examples/data/pres_temp_4D.nc output/filtered_lat.parquet \
  -n temperature \
  --range "latitude:25:45"
```

Only rows whose latitude coordinate falls within [25.0, 45.0] will appear in
the output.

---

### Step 3: Apply a List Filter on Level

The `--list` flag keeps only the discrete dimension values you specify. The
argument format is `dimension:val1,val2,...`.

```bash
nc2parquet convert examples/data/pres_temp_4D.nc output/filtered_level.parquet \
  -n temperature \
  --list "level:850,700,500"
```

Only rows whose `level` coordinate equals one of the listed values will be
included.

---

### Step 4: Combine Multiple Filters

Multiple `--range` and `--list` flags are intersected — a row must satisfy
every filter to be included in the output.

```bash
nc2parquet convert examples/data/pres_temp_4D.nc output/multi_filter.parquet \
  -n temperature \
  --range "latitude:25:45" \
  --list "level:1000,850,500"
```

You can also combine range and list filters across different dimensions in a
single command. Two or more `--range` flags are permitted simultaneously:

```bash
nc2parquet convert examples/data/pres_temp_4D.nc output/range_range.parquet \
  -n temperature \
  --range "latitude:30:50" \
  --range "longitude:-120:-80"
```

---

### Step 5: Rename Columns

The `--rename` flag renames a column in the output Parquet file. The argument
format is `old_name:new_name`. Use one `--rename` flag per column to rename.

```bash
nc2parquet convert examples/data/pres_temp_4D.nc output/renamed.parquet \
  -n temperature \
  --range "latitude:25:45" \
  --rename "temperature:temp_kelvin" \
  --rename "latitude:lat" \
  --rename "longitude:lon"
```

Renaming happens after filtering and before any subsequent post-processing
steps.

---

### Step 6: Apply a Mathematical Formula

The `--formula` flag creates a new column by evaluating an expression over
existing columns. The argument format is
`target_column:formula_expression:source1,source2,...`.

- `target_column` — name of the new (or overwritten) column
- `formula_expression` — arithmetic expression referencing source column names
- `source1,source2,...` — comma-delimited list of all columns referenced in the
  expression

```bash
nc2parquet convert examples/data/pres_temp_4D.nc output/with_formula.parquet \
  -n temperature \
  --rename "temperature:temp_k" \
  --formula "temp_c:temp_k - 273.15:temp_k" \
  --formula "temp_f:temp_c * 1.8 + 32.0:temp_c"
```

Because `--formula` uses `splitn(3, ':')`, the expression itself may safely
contain `:` characters. Formulas are evaluated in the order they are declared,
so later formulas can reference columns created by earlier ones.

---

### Step 7: Convert Temperature Units

The `--kelvin-to-celsius` flag is a shorthand unit conversion that subtracts
273.15 from a named column. It is equivalent to the corresponding `--formula`
but more concise.

```bash
nc2parquet convert examples/data/pres_temp_4D.nc output/celsius.parquet \
  -n temperature \
  --range "latitude:25:45" \
  --kelvin-to-celsius temperature
```

For arbitrary unit conversions, use `--unit-convert column:from_unit:to_unit`:

```bash
nc2parquet convert examples/data/pres_temp_4D.nc output/unit_converted.parquet \
  -n temperature \
  --unit-convert "temperature:kelvin:celsius"
```

---

### Step 8: Equivalent JSON Configuration

All of the post-processing steps above can be expressed in a JSON config file
instead of on the command line. The `examples/postprocessing/` directory
contains working examples:

- `examples/postprocessing/column_renaming.json` — column renaming
- `examples/postprocessing/unit_conversion.json` — unit conversion
- `examples/postprocessing/formula_application.json` — formula application
- `examples/postprocessing/complex_pipeline.json` — multi-step pipeline

For example, `examples/postprocessing/formula_application.json` contains:

```json
{
  "nc_key": "examples/data/simple_xy.nc",
  "variable_name": "data",
  "parquet_key": "examples/output/formula_applied.parquet",
  "filters": [],
  "postprocessing": {
    "name": "Formula Application Example",
    "processors": [
      {
        "type": "rename_columns",
        "mappings": {
          "data": "temperature"
        }
      },
      {
        "type": "apply_formula",
        "target_column": "temp_celsius",
        "formula": "temperature - 273.15",
        "source_columns": ["temperature"]
      },
      {
        "type": "apply_formula",
        "target_column": "temp_fahrenheit",
        "formula": "temp_celsius * 1.8 + 32.0",
        "source_columns": ["temp_celsius"]
      }
    ]
  }
}
```

Run a config-based conversion with:

```bash
nc2parquet convert --config examples/postprocessing/formula_application.json
```

---

## What's Next

The next tutorial shows how to process many NetCDF files in a single command
using glob patterns:

[Batch Processing Multiple Files](batch-processing.md)
