# ticket-023 Extend Formula Parser with Mathematical Functions

## Context

### Background

The `FormulaApplier` postprocessor currently supports arithmetic operators (`+`, `-`, `*`, `/`), comparisons (`<`, `>`, `==`, `!=`, `<=`, `>=`), parenthesized expressions, literal constants, and exactly one function: `sqrt()`. The parser in `parse_function_formula` (line 969-984) is hardcoded to check `formula.starts_with("sqrt(")`. Users need additional mathematical functions to express meteorological calculations -- for example, wind chill uses `pow()`, dew point uses `log()` and `exp()`, and absolute error calculations use `abs()`. Currently, these calculations require external tooling or manual multi-step postprocessing.

### Relation to Epic

This ticket extends the formula parser, complementing ticket-019 (unit converter extensions). Together they make the postprocessing pipeline comprehensive for meteorological workflows. The parser changes are self-contained within `FormulaApplier` and do not affect other processors, filters, or extraction logic.

### Current State

In `/home/rogerio/git/nc2parquet/src/postprocess.rs`:

- `FormulaApplier` (line 471-475): struct with `target_column`, `formula`, `source_columns`
- `apply_formula()` (line 826-851): Entry point. Routes to:
  - `parse_comparison_formula()` for formulas containing comparison operators
  - `parse_arithmetic_formula()` for formulas with `+`, `-`, `*`, `/`
  - `parse_function_formula()` for formulas starting with `"sqrt("`
  - `parse_operand_with_validation()` for bare operands (column names or literals)
- `parse_function_formula()` (line 969-984): ONLY handles `sqrt()`. Hardcoded `starts_with("sqrt(")` check. Returns `Unsupported function` error for anything else.
- `parse_expression()` (line 907-931): Recursive descent for `+`, `-` (lowest precedence)
- `parse_term()` (line 933-956): Recursive descent for `*`, `/` (higher precedence)
- `parse_factor()` (line 959-967): Handles parenthesized expressions and operands
- `parse_operand_with_validation()` (line 986-1011): Resolves to `lit(f64)` or `col(name)`

Key limitation: `parse_function_formula()` is only called from the top level `apply_formula()` when the ENTIRE formula is a single function call. Functions nested within arithmetic expressions (e.g., `sqrt(a) + abs(b)`) are NOT supported because `parse_factor()` does not check for function calls -- it only handles parenthesized expressions and bare operands.

## Specification

### Requirements

1. Extend the formula parser to support the following mathematical functions:
   - **Unary** (one argument): `abs`, `sqrt`, `exp`, `ln`, `log10`, `sin`, `cos`, `tan`, `ceil`, `floor`, `round`
   - **Binary** (two arguments): `pow`, `min`, `max`, `log` (log base)

2. Function calls use the syntax `func(arg)` for unary and `func(arg1, arg2)` for binary. Function names are case-insensitive.

3. Functions can appear ANYWHERE in a formula expression, not just as the top-level expression. For example: `abs(temperature) + pow(pressure, 2.0)`, `sqrt(a*a + b*b)`, `max(temperature, 0.0) * 1.8 + 32.0`.

4. The `log` function takes two arguments: `log(value, base)`. Example: `log(value, 10.0)` is equivalent to `log10(value)`. `ln` is natural log (base e).

5. Domain errors (e.g., `sqrt(-1)`, `log(0)`, `log(-1, 10)`) produce NaN per IEEE 754, matching the existing `sqrt` behavior. No runtime errors for domain violations.

6. Backward compatibility: existing `sqrt(x)` formulas continue to work. The top-level function detection in `apply_formula()` can be simplified to route through the new general function parser.

### Inputs/Props

No changes to `ProcessorConfig::ApplyFormula` -- it already accepts arbitrary `formula` strings. The new functions are recognized within the parser.

### Outputs/Behavior

- Unary functions: `abs(x)` -> `col(x).abs()`, `exp(x)` -> `col(x).exp()`, etc.
- Binary functions: `pow(x, 2.0)` -> `col(x).pow(lit(2.0))`, `min(x, y)` -> `col(x).min(col(y))` -- but note that Polars column-level `min`/`max` are aggregations, so binary min/max should use `when(x < y).then(x).otherwise(y)` pattern or Polars `min_horizontal`/`max_horizontal`
- Functions nested in expressions: `abs(a - b) * 2.0` -> `(col(a) - col(b)).abs() * lit(2.0)`
- All function results are `Float64` type

### Error Handling

- Unknown function name: `PostProcessError::ProcessingError(format!("Unknown function: {}", name))`
- Wrong arity (e.g., `pow(x)` or `abs(x, y)`): `PostProcessError::ProcessingError(format!("Function '{}' expects {} argument(s), got {}", name, expected, actual))`
- Missing closing parenthesis: existing parser error handling via `parse_operand_with_validation` (returns error for unrecognized operand)

## Acceptance Criteria

- [ ] Given formula `"abs(temperature)"` with values [-5.0, 3.0, -1.0], when applied, then result is [5.0, 3.0, 1.0]
- [ ] Given formula `"exp(value)"` with values [0.0, 1.0, 2.0], when applied, then result is [1.0, e, e^2] (within 1e-10 tolerance)
- [ ] Given formula `"ln(value)"` with values [1.0, e, e^2], when applied, then result is [0.0, 1.0, 2.0] (within 1e-10 tolerance)
- [ ] Given formula `"log10(value)"` with values [1.0, 10.0, 100.0], when applied, then result is [0.0, 1.0, 2.0]
- [ ] Given formula `"pow(value, 2.0)"` with values [2.0, 3.0, 4.0], when applied, then result is [4.0, 9.0, 16.0]
- [ ] Given formula `"min(a, b)"` with a=[1.0, 5.0] and b=[3.0, 2.0], when applied, then result is [1.0, 2.0]
- [ ] Given formula `"max(a, b)"` with a=[1.0, 5.0] and b=[3.0, 2.0], when applied, then result is [3.0, 5.0]
- [ ] Given formula `"sin(value)"` with value [0.0, pi/2], when applied, then result is [0.0, 1.0] (within 1e-10)
- [ ] Given formula `"cos(value)"` with value [0.0, pi], when applied, then result is [1.0, -1.0] (within 1e-10)
- [ ] Given formula `"ceil(value)"` with values [1.1, 2.9, -0.5], when applied, then result is [2.0, 3.0, 0.0]
- [ ] Given formula `"floor(value)"` with values [1.9, 2.1, -0.5], when applied, then result is [1.0, 2.0, -1.0]
- [ ] Given formula `"round(value)"` with values [1.4, 1.5, 2.5], when applied, then result is [1.0, 2.0, 2.0] (banker's rounding per Polars)
- [ ] Given nested formula `"abs(a - b) * 2.0"`, when applied, then the function is correctly evaluated within the arithmetic expression
- [ ] Given formula `"sqrt(a*a + b*b)"` (nested arithmetic inside function), when applied with a=3.0, b=4.0, then result is 5.0
- [ ] Given formula `"pow(abs(value), 0.5)"` (nested function inside function), when applied with value=-4.0, then result is 2.0
- [ ] Given formula `"log(value, 10.0)"` with values [1.0, 100.0], when applied, then result is [0.0, 2.0]
- [ ] Given formula `"SQRT(value)"` (uppercase), when applied, then it works case-insensitively
- [ ] Given formula `"unknown_func(value)"`, when applied, then `PostProcessError::ProcessingError` is returned with "Unknown function"
- [ ] Given formula `"sqrt(value)"` (existing), when applied, then it continues to work identically (backward compatibility)

## Implementation Guide

### Suggested Approach

1. **Refactor `parse_factor`**: The key architectural change is making `parse_factor()` recognize function calls. Currently it only checks for parenthesized expressions `(expr)` and bare operands. Modify it to also detect `identifier(` patterns:

   ```rust
   fn parse_factor(&self, df: &DataFrame, expr: &str) -> PostProcessResult<Expr> {
       let expr = expr.trim();
       // Check for function call: identifier followed by (
       if let Some(paren_pos) = expr.find('(') {
           let name = expr[..paren_pos].trim();
           if !name.is_empty() && expr.ends_with(')') && Self::is_function_name(name) {
               let args_str = &expr[paren_pos+1..expr.len()-1];
               return self.parse_function_call(df, name, args_str);
           }
       }
       // Existing: parenthesized expression
       if expr.starts_with('(') && expr.ends_with(')') {
           return self.parse_expression(df, &expr[1..expr.len()-1]);
       }
       self.parse_operand_with_validation(df, expr)
   }
   ```

2. **Add `parse_function_call`**: New method that parses the argument list and dispatches:

   ```rust
   fn parse_function_call(&self, df: &DataFrame, name: &str, args_str: &str) -> PostProcessResult<Expr> {
       let func_name = name.to_lowercase();
       let args = self.split_function_args(args_str);
       match func_name.as_str() {
           "abs" => { ensure_arity(&func_name, &args, 1)?; let a = self.parse_expression(df, &args[0])?; Ok(a.abs()) }
           "sqrt" => { ensure_arity(&func_name, &args, 1)?; let a = self.parse_expression(df, &args[0])?; Ok(a.sqrt()) }
           "exp" => { ... Ok(a.exp()) }
           "ln" => { ... Ok(a.log(std::f64::consts::E)) }  // Polars .log() takes base
           "log10" => { ... Ok(a.log(10.0)) }
           "log" => { ensure_arity(&func_name, &args, 2)?; let a = ...; let b = ...; Ok(a.log(b)) }
           "sin" => { ... Ok(a.sin()) }
           "cos" => { ... Ok(a.cos()) }
           "tan" => { ... Ok(a.tan()) }
           "ceil" => { ... Ok(a.ceil()) }
           "floor" => { ... Ok(a.floor()) }
           "round" => { ... Ok(a.round(0)) }  // round to 0 decimal places
           "pow" => { ensure_arity(&func_name, &args, 2)?; let base = ...; let exp = ...; Ok(base.pow(exp)) }
           "min" => { ensure_arity(&func_name, &args, 2)?; ... use min_horizontal or when/then }
           "max" => { ensure_arity(&func_name, &args, 2)?; ... use max_horizontal or when/then }
           _ => Err(PostProcessError::ProcessingError(format!("Unknown function: {}", name)))
       }
   }
   ```

3. **Add `split_function_args`**: Split the argument string on commas, respecting nesting depth:

   ```rust
   fn split_function_args(args_str: &str) -> Vec<String> {
       let mut args = Vec::new();
       let mut depth = 0;
       let mut current = String::new();
       for c in args_str.chars() {
           match c {
               '(' => { depth += 1; current.push(c); }
               ')' => { depth -= 1; current.push(c); }
               ',' if depth == 0 => { args.push(current.trim().to_string()); current.clear(); }
               _ => current.push(c),
           }
       }
       if !current.trim().is_empty() { args.push(current.trim().to_string()); }
       args
   }
   ```

4. **Remove top-level function routing**: In `apply_formula()`, remove the special `starts_with("sqrt(")` branch. Instead, let ALL formulas flow through the general arithmetic parser (`parse_arithmetic_formula` / `parse_expression`), which will now handle function calls via the refactored `parse_factor`. The comparison detection must remain as-is (it checks for comparison operators before routing to the arithmetic parser).

5. **Handle `is_function_name`**: Add a static helper that checks if a string is a known function name (to disambiguate from column names followed by parentheses, which should not happen in practice):

   ```rust
   fn is_function_name(name: &str) -> bool {
       matches!(name.to_lowercase().as_str(),
           "abs" | "sqrt" | "exp" | "ln" | "log10" | "log" |
           "sin" | "cos" | "tan" | "ceil" | "floor" | "round" |
           "pow" | "min" | "max"
       )
   }
   ```

6. **Polars API notes**:
   - `Expr::abs()`, `Expr::sqrt()`, `Expr::exp()` are available in Polars 0.51
   - `Expr::log(base: f64)` computes log with given base
   - `Expr::sin()`, `Expr::cos()`, `Expr::tan()` are available
   - `Expr::ceil()`, `Expr::floor()`, `Expr::round(decimals: u32)` are available
   - `Expr::pow(exponent: Expr)` is available
   - For binary min/max on two expressions: use `polars::lazy::dsl::min_horizontal([a, b])` or `when(a.lt(b)).then(a).otherwise(b)`. Check which is available in Polars 0.51.

### Key Files to Modify

- `/home/rogerio/git/nc2parquet/src/postprocess.rs` -- Refactor `parse_factor`, add `parse_function_call`, `split_function_args`, `is_function_name`, simplify `apply_formula`
- `/home/rogerio/git/nc2parquet/src/tests/test_postprocess.rs` -- Add `formula_parser_functions` test module

### Patterns to Follow

- Follow the existing recursive descent pattern: `parse_expression` -> `parse_term` -> `parse_factor` -> operand
- Follow the existing tolerance-based assertion pattern: `(value - expected).abs() < 1e-10`
- Follow the existing error pattern: `PostProcessError::ProcessingError(format!(...))` for parser errors

### Pitfalls to Avoid

- When checking for function calls in `parse_factor`, be careful to handle nested parentheses correctly. For `sqrt(a + b)`, the outer `(` is at position 4 and the `)` is at the end, but the function argument `a + b` may also contain parentheses in expressions like `sqrt((a + b) * c)`. Use the `find('(')` approach and match the closing `)` at the end of the expression.
- The `log` function in Polars (`Expr::log`) may take `base` as an `f64` literal, not as an `Expr`. Check the Polars 0.51 API. If the base must be a literal, then `log(value, base_expr)` where `base_expr` is a column would need a workaround: `ln(value) / ln(base_expr)`.
- Binary `min`/`max` is NOT the same as Polars column aggregation `min()`/`max()`. The aggregation reduces a column to a scalar. What we need is element-wise min/max of two expressions. Check `min_horizontal` or implement with `when/then/otherwise`.
- Removing the `starts_with("sqrt(")` branch from `apply_formula()` changes the routing logic. The arithmetic parser must now be the default for all non-comparison formulas. Test that bare operands (`"42.0"`, `"column_name"`) still work through the arithmetic parser path.
- The `parse_expression` function handles `+` and `-` operators by scanning left-to-right at depth 0. This can conflict with negative literals (e.g., `-1.0`). The current parser does not handle unary minus explicitly -- it relies on the operand being parsed as a negative float. Ensure this continues to work.

## Testing Requirements

### Unit Tests

Add a `formula_parser_functions` module in `/home/rogerio/git/nc2parquet/src/tests/test_postprocess.rs`:

1. **Each unary function**: `abs`, `sqrt`, `exp`, `ln`, `log10`, `sin`, `cos`, `tan`, `ceil`, `floor`, `round` with known input/output pairs
2. **Each binary function**: `pow`, `min`, `max`, `log` with known input/output pairs
3. **Nested functions**: `abs(a - b)`, `sqrt(a*a + b*b)`, `pow(abs(value), 0.5)`
4. **Functions in arithmetic**: `abs(a) + pow(b, 2.0)`, `ceil(a) * floor(b)`
5. **Domain errors**: `sqrt(-1)` -> NaN, `ln(0)` -> -inf or NaN, `log(-1, 10)` -> NaN
6. **Case insensitivity**: `ABS(x)`, `Sqrt(x)`, `POW(x, 2)` all work
7. **Unknown function**: `"unknown_func(x)"` -> error with "Unknown function"
8. **Wrong arity**: `"pow(x)"` -> error, `"abs(x, y)"` -> error
9. **Backward compatibility**: Existing `sqrt(value)` formulas, arithmetic formulas, comparison formulas all continue to work

### Integration Tests

No new integration tests needed -- formula parser tests are self-contained with in-memory DataFrames.

## Dependencies

- **Blocked By**: ticket-010 (error types -- completed), ticket-013 (clean codebase -- completed)
- **Blocks**: ticket-028 (usage tutorials will reference new formula functions)

## Effort Estimate

**Points**: 3
**Confidence**: Medium (the recursive descent refactoring for nested functions needs careful handling of edge cases around parenthesis matching; Polars API availability for binary min/max at element level needs verification)
