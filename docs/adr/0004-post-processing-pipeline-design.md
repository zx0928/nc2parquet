# ADR 0004: Post-Processing Pipeline Design

## Status

Accepted

## Date

2026-02-23

## Context

After a NetCDF variable is extracted into a Polars `DataFrame`, users commonly
need to transform it before writing Parquet: rename columns to match downstream
schema conventions, convert numeric time offsets to `Datetime` values, change
physical units, aggregate over groups, or derive new columns via mathematical
expressions. These transformations must be composable, configurable from JSON
or YAML, and efficient enough not to dominate the overall conversion time.

Several design forces are in tension:

- **Composability**: transforms must chain; the output schema of one step is
  the input schema of the next.
- **Efficiency**: each Polars `.collect()` materializes the entire DataFrame.
  Redundant materializations should be eliminated when possible.
- **Extensibility**: new transform types must be addable without modifying
  the pipeline executor.
- **Serializability**: the pipeline configuration must round-trip through
  JSON/YAML so that job configs can be stored and reproduced.

## Decision

The post-processing subsystem lives in `src/postprocess.rs`.

**Trait**: `PostProcessor` defines the extension point. Every processor
implements:

- `process(&self, df: DataFrame) -> PostProcessResult<DataFrame>` — executes
  the transform
- `name(&self) -> &str` and `description(&self) -> &str` — for logging
- `target_columns(&self) -> Vec<String>` — columns read or written; used by
  the pipeline to detect independent processors
- `to_lazy_expr(&self, schema: &Schema) -> Option<Vec<Expr>>` — returns Polars
  lazy expressions when the transform can be expressed as a column expression;
  `None` for schema-level operations that cannot
- `validate_schema(&self, schema: &Schema) -> PostProcessResult<()>` — optional
  pre-flight check
- `output_schema(&self, input_schema: &Schema) -> PostProcessResult<Schema>`
  — describes the schema change

**Executor**: `ProcessingPipeline` holds a `Vec<Box<dyn PostProcessor>>` and
runs them in order. Before each step it checks whether consecutive processors
have disjoint `target_columns` and all return `Some` from `to_lazy_expr`. When
that condition holds, their expressions are accumulated into a single
`.with_columns(batch_exprs).collect()` call, eliminating redundant
materializations.

**Configuration**: `ProcessorConfig` is a serde-tagged enum with five variants:

```
#[serde(tag = "type", rename_all = "snake_case")]
enum ProcessorConfig {
    RenameColumns { mappings: HashMap<String, String> },
    DatetimeConvert { column, base, unit },
    UnitConvert { column, from_unit, to_unit },
    Aggregate { group_by, aggregations },
    ApplyFormula { target_column, formula, source_columns },
}
```

The `type` field in JSON/YAML selects the variant by snake_case name (e.g.
`"type": "unit_convert"`). `ProcessingPipeline::from_config` instantiates
processors from this config.

**Formula parser**: `FormulaApplier` implements a recursive descent parser
that translates formula strings into Polars lazy expressions at execution time.
The grammar is `parse_expression` → `parse_term` → `parse_factor` →
`parse_function_call`, following standard arithmetic precedence (`*`/`/`
bind tighter than `+`/`-`). Eleven unary functions (`abs`, `sqrt`, `exp`,
`ln`, `log10`, `sin`, `cos`, `tan`, `ceil`, `floor`, `round`) and four binary
functions (`pow`, `min`, `max`, `log`) are supported. Function names are
case-insensitive.

**Unit converter**: `UnitConverter` normalizes conversions through base units
per family (pressure → Pa, speed → m/s, length → m) using the
`unit_to_base_factor` lookup function. Temperature conversions use explicit
offset match arms in `build_conversion_expr` because they require an additive
offset that cannot be expressed as a pure scale factor.

## Consequences

### Positive

- The `PostProcessor` trait allows new transform types to be added without
  modifying `ProcessingPipeline`.
- Batching consecutive independent processors reduces DataFrame materializations
  on pipelines with many column-level transforms (unit conversions, formula
  applications).
- `ProcessorConfig`'s tagged serde enum produces readable, self-describing
  JSON/YAML that matches the variant names.
- The formula parser rejects unknown column names and functions at execution
  time with precise error messages, rather than silently producing wrong
  results.

### Negative

- `Aggregate` breaks batching unconditionally because group-by changes the
  number of rows; any processor after an aggregation starts a new sequential
  step regardless of column disjointness.
- The formula parser is a hand-written recursive descent. It does not support
  unary negation as a prefix operator (e.g. `-a`); users must write `0 - a`
  instead.
- `ProcessingPipeline::execute` takes `&mut self` to allow future mutable
  state per processor, but current processors are stateless; the mutability
  is forward-looking overhead.

## Alternatives Considered

1. **SQL-based transformations via Polars SQL context** — Rejected. A custom
   `ProcessorConfig` DSL gives typed configuration, schema-level validation
   before execution, and error messages that reference the specific transform
   step rather than a SQL parse error.

2. **Parallel processor execution** — Rejected. Processors are inherently
   sequential because the output schema of one step is the input to the next.
   The batching optimization handles the independent sub-case without
   introducing concurrency.

3. **External scripting via embedded Lua or Rhai** — Rejected. A scripting
   engine adds a runtime dependency, increases binary size, and provides
   flexibility that exceeds the current use cases. The formula parser covers
   the mathematical expression needs with no additional dependencies.
