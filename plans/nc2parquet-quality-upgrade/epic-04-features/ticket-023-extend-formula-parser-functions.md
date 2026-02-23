# ticket-023 Extend Formula Parser with Mathematical Functions

> **[OUTLINE]** This ticket requires refinement before execution.
> It will be refined with learnings from earlier epics.

## Objective

Extend the recursive descent formula parser in `FormulaApplier` to support additional mathematical functions beyond the current `sqrt`. The target function set is: `abs`, `min`, `max`, `pow`, `log`, `ln`, `exp`, `sin`, `cos`, `tan`, `ceil`, `floor`, `round`. This enables users to express complex meteorological calculations (e.g., wind chill, heat index, dew point) as postprocessing formulas without writing custom code.

## Anticipated Scope

- **Files likely to be modified**:
  - `/home/rogerio/git/nc2parquet/src/postprocess.rs` -- extend `parse_function_call`, `evaluate_function` (or equivalent) in the formula parser to recognize and evaluate new functions
  - Test files for formula parser tests
- **Key decisions needed**:
  - Function arity: `min` and `max` are binary, others are unary -- how to handle mixed arity in the parser?
  - Whether `pow(x, y)` should also support the `x^y` operator syntax
  - Whether `log` means log10 or natural log (convention varies); should we offer both `log` and `ln`?
  - Error handling for domain errors (e.g., `sqrt(-1)`, `log(0)`)
- **Open questions**:
  - Should the parser support variadic functions like `min(a, b, c)`?
  - Should function names be case-insensitive (`SIN` vs `sin`)?
  - Does the current parser architecture (recursive descent) handle binary functions cleanly, or does it need restructuring?

## Dependencies

- **Blocked By**: ticket-010 (error types for formula evaluation errors), ticket-013 (clean codebase)
- **Blocks**: ticket-028

## Effort Estimate

**Points**: 3
**Confidence**: Low (will be re-estimated during refinement)
