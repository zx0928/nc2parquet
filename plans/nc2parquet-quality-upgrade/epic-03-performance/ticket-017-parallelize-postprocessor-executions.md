# ticket-017 Parallelize Independent PostProcessor Executions

## Context

### Background

The `ProcessingPipeline::execute` method in `/home/rogerio/git/nc2parquet/src/postprocess.rs` (line 471) runs all processors sequentially: each processor receives the output DataFrame of the previous one. This is correct for dependent processors (e.g., a ColumnRenamer that renames column "t2" to "temperature" followed by a UnitConverter that operates on "temperature"). However, when processors operate on independent columns, they could be run in parallel.

After analyzing the actual postprocessor architecture and Polars' internal parallelism, this ticket has been rescoped from implementing full parallelism to a more targeted approach: **use Polars lazy evaluation to batch independent column operations into a single `select`/`with_columns` pass**, avoiding the overhead of multiple `lazy().with_columns([...]).collect()` calls. This is more practical than introducing rayon because:

1. Polars already uses rayon internally for column-level parallelism within `collect()`
2. Adding a second rayon thread pool would cause contention with Polars' pool
3. The typical pipeline has 2-5 processors -- the overhead of identifying independent groups and spawning threads would exceed the benefit
4. The real win is reducing the number of `.collect()` calls (each forces materialization) by combining independent lazy expressions

### Relation to Epic

This ticket addresses postprocessing latency, which is typically a smaller fraction of total pipeline time than extraction (the focus of tickets 015-016). The benchmark suite from ticket-014 will quantify the actual impact. This feeds into ticket-018's memory profiling since reducing intermediate DataFrame materializations also reduces peak memory.

### Current State

- `ProcessingPipeline::execute` (line 471) iterates `self.processors` sequentially, calling `processor.process(df)` and passing the result to the next processor
- Each `PostProcessor::process` implementation takes ownership of the DataFrame and returns a new one
- Most processor implementations use `df.lazy().with_columns([...]).collect()` which creates an intermediate materialized DataFrame
- The `PostProcessor` trait (line 94) has `process(&self, df: DataFrame) -> PostProcessResult<DataFrame>` -- it takes `DataFrame` by value, not `LazyFrame`
- The trait is object-safe (`dyn PostProcessor`)
- Trait requires `Send + Sync` (line 94)
- 5 processor types: ColumnRenamer, DateTimeConverter, UnitConverter, Aggregator, FormulaApplier
- Polars 0.51 uses rayon internally for lazy evaluation

## Specification

### Requirements

1. Add a method `target_columns(&self) -> Vec<String>` to the `PostProcessor` trait with a default implementation returning an empty `Vec` (indicating "unknown" target columns for backward compatibility)
2. Implement `target_columns` on each of the 5 processor types to return the columns they read from and write to
3. Add a method `to_lazy_expr(&self, schema: &Schema) -> Option<Vec<Expr>>` to the `PostProcessor` trait with a default implementation returning `None` (indicating the processor cannot be expressed as a lazy expression)
4. Implement `to_lazy_expr` for the simpler processors (UnitConverter, ColumnRenamer) that can be expressed as Polars expressions. More complex processors (Aggregator, FormulaApplier, DateTimeConverter) may return `None`.
5. In `ProcessingPipeline::execute`, before running the sequential loop, analyze the processor list to identify consecutive groups of processors that:
   a. All return `Some(exprs)` from `to_lazy_expr`
   b. Operate on non-overlapping column sets (determined via `target_columns`)
6. For each such group, batch their expressions into a single `df.lazy().with_columns(all_exprs).collect()` call
7. For processors that cannot be batched (return `None` from `to_lazy_expr` or have column conflicts), fall back to the existing sequential `process(df)` call
8. The optimization must be transparent -- the output DataFrame must be identical regardless of whether batching occurs

### Inputs/Props

Same as current `ProcessingPipeline::execute` -- no API changes.

### Outputs/Behavior

- Same `PostProcessResult<DataFrame>` with identical results
- For pipelines with 2+ independent simple processors, fewer `.collect()` calls
- No behavior change for pipelines where all processors are sequential/dependent

### Error Handling

- If `to_lazy_expr` returns `Some(exprs)` but the batched `with_columns(exprs).collect()` fails, fall back to sequential execution for that group and propagate the error from the sequential path
- New trait methods have default implementations that maintain backward compatibility -- custom `PostProcessor` implementations outside this crate will continue to work without changes

## Acceptance Criteria

- [ ] Given a pipeline with 2 UnitConverters on different columns ("temperature" kelvin->celsius, "pressure" hpa->pa), when `execute` is called, then the pipeline batches both into a single `.collect()` call and produces the same result as sequential execution
- [ ] Given a pipeline with a ColumnRenamer followed by a UnitConverter on the renamed column, when `execute` is called, then the processors are executed sequentially (not batched) because the UnitConverter depends on the ColumnRenamer's output
- [ ] Given a pipeline with an Aggregator (cannot be lazily expressed), when `execute` is called, then the Aggregator runs via the sequential `process(df)` path
- [ ] Given a pipeline with a mix of batchable and non-batchable processors, when `execute` is called, then batchable groups are batched and non-batchable processors run sequentially, in the correct order
- [ ] Given `target_columns` is implemented for all 5 processor types, when called, then each returns accurate column dependency information
- [ ] Given `cargo bench --bench postprocess_bench` is run, then the benchmark for a 3-processor independent pipeline (e.g., 3 UnitConverters on different columns) shows improvement compared to sequential execution
- [ ] Given all 181 existing tests pass after the change, then the optimization is behavior-preserving

## Implementation Guide

### Suggested Approach

1. **Add `target_columns` to `PostProcessor` trait** (with default `vec![]`):

   ```rust
   fn target_columns(&self) -> Vec<String> {
       vec![]
   }
   ```

   Implement for each processor type:
   - `ColumnRenamer`: return keys of `self.mappings` (input columns) + values of `self.mappings` (output columns)
   - `UnitConverter`: return `vec![self.column.clone()]`
   - `DateTimeConverter`: return `vec![self.column.clone()]`
   - `Aggregator`: return `self.group_by` + `self.aggregations.keys()`
   - `FormulaApplier`: return `self.source_columns` + `vec![self.target_column.clone()]`

2. **Add `to_lazy_expr` to `PostProcessor` trait** (with default `None`):

   ```rust
   fn to_lazy_expr(&self, schema: &Schema) -> Option<Vec<Expr>> {
       let _ = schema;
       None
   }
   ```

   Implement for UnitConverter:

   ```rust
   fn to_lazy_expr(&self, schema: &Schema) -> Option<Vec<Expr>> {
       if !schema.contains(&self.column) { return None; }
       // Build the same expression as in process(), but return it instead of collecting
       let expr = /* same logic as process() but without .collect() */;
       Some(vec![expr])
   }
   ```

   Implement for ColumnRenamer: return `None` because Polars rename is not an expression-level operation (it modifies schema, not data). ColumnRenamer stays sequential.

3. **Implement batching logic in `execute`**:

   ```rust
   pub fn execute(&mut self, mut df: DataFrame) -> PostProcessResult<DataFrame> {
       if self.processors.is_empty() {
           return Ok(df);
       }

       let mut i = 0;
       while i < self.processors.len() {
           let schema = df.schema();
           if let Some(exprs) = self.processors[i].to_lazy_expr(&schema) {
               // Try to batch consecutive lazy-expressable, independent processors
               let mut batch_exprs = exprs;
               let mut batch_columns: HashSet<String> = self.processors[i]
                   .target_columns().into_iter().collect();
               let mut batch_end = i + 1;

               while batch_end < self.processors.len() {
                   let next_cols: HashSet<String> = self.processors[batch_end]
                       .target_columns().into_iter().collect();
                   if batch_columns.is_disjoint(&next_cols) {
                       if let Some(next_exprs) = self.processors[batch_end].to_lazy_expr(&schema) {
                           batch_exprs.extend(next_exprs);
                           batch_columns.extend(next_cols);
                           batch_end += 1;
                           continue;
                       }
                   }
                   break;
               }

               if batch_end > i + 1 {
                   // Batched execution
                   df = df.lazy().with_columns(batch_exprs).collect()?;
               } else {
                   // Single lazy expr -- still use process() for consistency
                   df = self.processors[i].process(df)?;
               }
               i = batch_end;
           } else {
               df = self.processors[i].process(df)?;
               i += 1;
           }
       }

       Ok(df)
   }
   ```

4. **Add a benchmark** in `benches/postprocess_bench.rs` for a multi-processor pipeline: 3 UnitConverters on different columns with 100K rows.

### Key Files to Modify

- `/home/rogerio/git/nc2parquet/src/postprocess.rs` -- add `target_columns` and `to_lazy_expr` to trait and implementations, update `execute` method with batching logic

### Patterns to Follow

- Follow the established `PostProcessor` trait pattern: new methods have sensible defaults for backward compatibility
- Use `std::collections::HashSet` for column set intersection/disjoint checks (already imported via `std::collections::HashMap`)
- The `to_lazy_expr` approach mirrors how Polars itself composes expressions lazily before a single materialization
- Maintain all debug logging -- log when batching occurs: `debug!("Batching processors {} through {} into single collect()", i, batch_end - 1)`

### Pitfalls to Avoid

- Do NOT introduce a rayon dependency -- Polars already uses rayon internally, and a second thread pool would cause contention
- Do NOT change the `PostProcessor` trait in a backward-incompatible way -- default implementations are required on all new methods
- ColumnRenamer cannot be expressed as a Polars expression because `rename` is a schema-level operation, not a data-level expression. It must always run via `process()`.
- Aggregator cannot be expressed as `with_columns` because it changes the number of rows. It must always run via `process()`.
- FormulaApplier has complex parsing that is hard to represent as a pure Polars expression chain -- start with `None` for `to_lazy_expr` and consider implementing it in a future ticket
- DateTimeConverter could be expressed lazily but has the complex special-case logic for the datetime cast -- start with `None` and consider implementing later
- The batching check `batch_columns.is_disjoint(&next_cols)` is necessary but not sufficient -- it handles the common case of column-independent processors. Edge cases where a processor reads from a column that another writes to are caught by the column overlap check.

## Testing Requirements

### Unit Tests

- Test `target_columns` for each processor type returns the expected columns
- Test `to_lazy_expr` for UnitConverter produces an expression that, when collected, gives the same result as `process()`
- Test batching logic: create a pipeline with 3 independent UnitConverters and verify batching occurs (log output or internal state)
- Test non-batching: create a pipeline where processors have overlapping columns and verify sequential execution

### Integration Tests

The existing 181 tests serve as regression tests.

### E2E Tests

Not applicable.

## Dependencies

- **Blocked By**: ticket-014 (benchmarks needed to measure whether parallelization helps)
- **Blocks**: ticket-018

## Effort Estimate

**Points**: 3
**Confidence**: Medium
