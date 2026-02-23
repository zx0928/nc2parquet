# ticket-019 Extend UnitConverter with Meteorological Unit Families

> **[OUTLINE]** This ticket requires refinement before execution.
> It will be refined with learnings from earlier epics.

## Objective

Extend the `UnitConverter` postprocessor beyond temperature conversions (currently K/C/F) to support common meteorological unit families: pressure (Pa/hPa/mbar/atm/inHg), wind speed (m/s/km/h/kt/mph), and length/distance (m/km/ft/mi/nm). This makes the library useful for a wider range of weather and climate data conversion workflows without requiring formula-based workarounds.

## Anticipated Scope

- **Files likely to be modified**:
  - `/home/rogerio/git/nc2parquet/src/postprocess.rs` -- extend `UnitConverter::process` with new unit families and conversion logic
  - `/home/rogerio/git/nc2parquet/src/input.rs` -- may need to add validation for new unit strings in `ProcessorConfig`
  - Test files for new unit conversion tests
- **Key decisions needed**:
  - Unit string format: should units be case-sensitive? Accept CF-convention names or common abbreviations or both?
  - Whether to use a lookup table approach or a trait-based unit family abstraction
  - Whether to support user-defined custom unit conversions via config or only built-in ones
  - How to handle unit validation errors (at config time vs. at processing time)
- **Open questions**:
  - Should the converter handle offset+scale conversions (like temperature) and pure scale conversions (like pressure) with the same code path?
  - What CF-convention unit strings are standard for the target unit families?
  - Should incompatible unit conversions (e.g., pressure to speed) produce a compile-time or runtime error?

## Dependencies

- **Blocked By**: ticket-010 (error types finalized for proper validation errors), ticket-013 (clean codebase)
- **Blocks**: ticket-028

## Effort Estimate

**Points**: 3
**Confidence**: Low (will be re-estimated during refinement)
