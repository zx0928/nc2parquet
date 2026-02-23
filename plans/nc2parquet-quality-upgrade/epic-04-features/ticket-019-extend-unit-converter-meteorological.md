# ticket-019 Extend UnitConverter with Meteorological Unit Families

## Context

### Background

The `UnitConverter` postprocessor currently supports only temperature conversions: Kelvin/Celsius/Fahrenheit (with short names K/C/F). All other unit pairs fall through to a default multiplicative factor of 1.0, making them effectively no-ops. Users working with weather and climate data frequently need pressure, wind speed, and length/distance conversions. Today they must either use `FormulaApplier` with manually computed constants or use `UnitConverter::with_conversion_factor` from the API -- neither of which is ergonomic or self-documenting in a JSON config file. Adding built-in meteorological unit families makes the library immediately useful for the most common climate data workflows.

### Relation to Epic

This is the first ticket in Epic 04 (Feature Completeness). It extends an existing postprocessor rather than adding new pipeline concepts, making it a low-risk starting point for the epic. The patterns established here (how to organize conversion lookup, how to validate unit strings, how to test new conversions) will inform ticket-023 (formula parser extensions).

### Current State

In `/home/rogerio/git/nc2parquet/src/postprocess.rs`:

- `UnitConverter` struct (line 451-456) holds `column`, `from_unit`, `to_unit`, `conversion_factor`
- `calculate_conversion_factor()` (line 519-530) matches on `(from_unit.to_lowercase(), to_unit.to_lowercase())` and returns `1.0` for unrecognized pairs
- `build_conversion_expr()` (line 533-548) builds a Polars `Expr` with offset+scale for temperature conversions and `col * lit(factor)` for everything else
- Both `process()` and `to_lazy_expr()` delegate to `build_conversion_expr()`, so adding conversion logic there automatically enables both execution paths and pipeline batching
- `ProcessorConfig::UnitConvert` (line 141-145) accepts `column`, `from_unit`, `to_unit` as strings -- no changes needed to the config schema
- Unit names are case-insensitive (lowercased at match time) and support both short ("k", "c", "f") and long ("kelvin", "celsius", "fahrenheit") forms

In `/home/rogerio/git/nc2parquet/src/tests/test_postprocess.rs`:

- `unit_converter_edge_cases` module (line 442-557) tests K->C, C->K, C->F, F->C, unknown pairs, short names, case insensitivity
- `test_unit_converter_multiplication` (line 63-84) tests `with_conversion_factor` for hPa->Pa

## Specification

### Requirements

1. Add **pressure** unit family: Pa, hPa, mbar, kPa, atm, inHg, mmHg
   - Pa <-> hPa: factor 100 (hPa = Pa / 100)
   - hPa <-> mbar: factor 1.0 (identical units)
   - Pa <-> kPa: factor 1000
   - atm <-> Pa: factor 101325.0
   - inHg <-> Pa: factor 3386.389
   - mmHg <-> Pa: factor 133.322

2. Add **speed** unit family: m/s, km/h, kt, mph, ft/s
   - m/s <-> km/h: factor 3.6
   - m/s <-> kt: factor 1.943844
   - m/s <-> mph: factor 2.236936
   - m/s <-> ft/s: factor 3.28084

3. Add **length** unit family: m, km, ft, mi, nm (nautical miles), cm, mm
   - m <-> km: factor 1000
   - m <-> ft: factor 0.3048 (ft -> m)
   - m <-> mi: factor 1609.344
   - m <-> nm: factor 1852.0
   - m <-> cm: factor 0.01
   - m <-> mm: factor 0.001

4. All conversions are **pure scale** (multiply/divide), unlike temperature which has offsets. The implementation should normalize through a canonical base unit per family (Pa for pressure, m/s for speed, m for length) and compute the combined factor.

5. Unit names are case-insensitive. Accept both common abbreviations and full names. Slash notation for speed units: `"m/s"`, `"km/h"`, `"ft/s"`.

6. Unrecognized unit pairs should continue to fall through to `conversion_factor = 1.0` (existing behavior, no breaking change).

7. Incompatible cross-family conversions (e.g., pressure -> speed) are NOT detected at this stage -- they simply fall through to factor 1.0. A future ticket can add validation.

### Inputs/Props

No changes to `ProcessorConfig::UnitConvert` -- it already accepts arbitrary `from_unit` and `to_unit` strings. The new unit families are recognized purely within `calculate_conversion_factor()` and `build_conversion_expr()`.

### Outputs/Behavior

- For pure scale conversions: `output_value = input_value * factor`
- For temperature conversions with offsets: existing behavior unchanged
- The Polars `Expr` from `build_conversion_expr()` uses `col(column) * lit(factor)` for all new unit families (no offsets needed)

### Error Handling

No new error types. Unrecognized pairs fall through to factor 1.0 and log a debug-level warning. This preserves backward compatibility.

## Acceptance Criteria

- [ ] Given a DataFrame with a column of values in Kelvin, when `UnitConverter::new("col", "kelvin", "celsius")` is applied, then existing temperature conversions continue to work identically (regression check)
- [ ] Given a DataFrame with pressure values in Pa, when `UnitConverter::new("col", "pa", "hpa")` is applied, then all values are divided by 100
- [ ] Given a DataFrame with pressure values in hPa, when `UnitConverter::new("col", "hpa", "atm")` is applied, then all values are divided by 1013.25
- [ ] Given a DataFrame with speed values in m/s, when `UnitConverter::new("col", "m/s", "km/h")` is applied, then all values are multiplied by 3.6
- [ ] Given a DataFrame with speed values in kt, when `UnitConverter::new("col", "kt", "mph")` is applied, then conversion goes kt -> m/s -> mph correctly
- [ ] Given a DataFrame with length values in ft, when `UnitConverter::new("col", "ft", "m")` is applied, then all values are multiplied by 0.3048
- [ ] Given a DataFrame with length values in km, when `UnitConverter::new("col", "km", "nm")` is applied, then conversion goes km -> m -> nm correctly
- [ ] Given unit names in mixed case (e.g., "HPA", "Km/H"), when the converter is created, then it matches case-insensitively
- [ ] Given an unrecognized unit pair, when the converter is created, then the factor is 1.0 and the values are unchanged (existing behavior)
- [ ] Given a `ProcessorConfig::UnitConvert` with `from_unit: "pa"` and `to_unit: "hpa"`, when `ProcessingPipeline::from_config()` is called, then the pipeline works correctly
- [ ] Given two consecutive `UnitConverter` processors with disjoint columns, when the pipeline executes, then `to_lazy_expr()` batching works for the new unit families

## Implementation Guide

### Suggested Approach

1. Create a helper function `fn unit_to_base_factor(unit: &str) -> Option<(UnitFamily, f64)>` that maps a lowercased unit string to its family and the factor to convert FROM that unit TO the canonical base unit of its family. For example:
   - `"hpa"` -> `(Pressure, 100.0)` (100 Pa per hPa, so multiply by 100 to get Pa)
   - `"km/h"` -> `(Speed, 1.0/3.6)` (divide by 3.6 to get m/s, so factor = 1/3.6)
   - `"ft"` -> `(Length, 0.3048)` (multiply by 0.3048 to get m)

2. Define a simple `#[derive(PartialEq)] enum UnitFamily { Temperature, Pressure, Speed, Length }` (private to the module).

3. Refactor `calculate_conversion_factor()`:
   - First check if both units map to the same `UnitFamily` via `unit_to_base_factor()`
   - If same family and NOT temperature: `factor = from_to_base / to_to_base`
   - If temperature: use existing hardcoded match arms (temperature has offsets, cannot use pure factor)
   - If different family or unrecognized: return `1.0`

4. Refactor `build_conversion_expr()`:
   - Keep existing temperature match arms (offset+scale expressions)
   - For same-family non-temperature conversions: return `col(column) * lit(factor)` using the pre-computed `conversion_factor`
   - This is already handled by the `_ => col(&self.column) * lit(self.conversion_factor)` fallthrough, so no change needed in `build_conversion_expr()` -- only `calculate_conversion_factor()` needs updating

5. Add unit aliases for each unit (e.g., `"hectopascal"` -> same as `"hpa"`, `"millibar"` -> same as `"mbar"`, `"knot"` / `"knots"` -> same as `"kt"`, `"nautical_mile"` -> same as `"nm"`).

### Key Files to Modify

- `/home/rogerio/git/nc2parquet/src/postprocess.rs` -- Add `UnitFamily` enum, `unit_to_base_factor()` function, refactor `calculate_conversion_factor()`
- `/home/rogerio/git/nc2parquet/src/tests/test_postprocess.rs` -- Add new test module `unit_converter_meteorological` with tests for all new unit families

### Patterns to Follow

- Follow the existing case-insensitive pattern: `from_unit.to_lowercase().as_str()` in match arms
- Follow the existing test pattern in `unit_converter_edge_cases`: helper functions `make_df()` and `extract_f64()`, tolerance-based assertions with `1e-6` for physical conversions
- Keep the `UnitFamily` enum and `unit_to_base_factor()` private to the module (no `pub` visibility needed)

### Pitfalls to Avoid

- Do NOT change the behavior of temperature conversions -- they require offset+scale expressions in `build_conversion_expr()` and must remain as explicit match arms, not go through the generic factor path
- Do NOT add validation that rejects unknown unit pairs -- the current fallthrough to 1.0 is intentional for backward compatibility
- Be careful with the direction of conversion factors. The factor stored is FROM -> TO, meaning `from_base / to_base` where `from_base` converts `from_unit` to the canonical base and `to_base` converts `to_unit` to the canonical base
- `mbar` and `hPa` are identical units (factor 1.0 between them); include this explicitly
- The `conversion_factor` field is computed at construction time in `calculate_conversion_factor()` and stored. The `build_conversion_expr()` method reads it from `self.conversion_factor` for the fallthrough case. This means `calculate_conversion_factor()` is the ONLY place that needs new logic for pure-scale conversions.

## Testing Requirements

### Unit Tests

Add a `unit_converter_meteorological` module in `/home/rogerio/git/nc2parquet/src/tests/test_postprocess.rs`:

1. **Pressure conversions**: Pa->hPa, hPa->Pa, Pa->atm, atm->Pa, Pa->inHg, hPa->mbar (identity), Pa->kPa, Pa->mmHg
2. **Speed conversions**: m/s->km/h, km/h->m/s, m/s->kt, kt->m/s, m/s->mph, mph->m/s, m/s->ft/s
3. **Length conversions**: m->km, km->m, m->ft, ft->m, m->mi, mi->m, m->nm, nm->m, m->cm, m->mm
4. **Cross-family indirect**: km->nm (should go km->m->nm), kt->mph (should go kt->m/s->mph)
5. **Case insensitivity**: "HPA" -> "PA", "Km/H" -> "M/S"
6. **Unknown pair fallthrough**: "furlongs" -> "fortnights" still returns values unchanged
7. **Regression**: K->C, C->F, F->C still produce correct results (run existing tests; add explicit regression test with known values)
8. **Pipeline batching**: Two consecutive `UnitConverter` processors (pressure + speed on different columns) execute via `to_lazy_expr()` batching

### Integration Tests

No new integration tests needed -- unit converter tests are self-contained with in-memory DataFrames.

## Dependencies

- **Blocked By**: ticket-010 (error types finalized -- completed), ticket-013 (clean codebase -- completed)
- **Blocks**: ticket-028 (usage tutorials will reference new unit families)

## Effort Estimate

**Points**: 2
**Confidence**: High
