# Epic 04: Feature Completeness

## Goals

1. Extend UnitConverter beyond temperature to cover common meteorological units (pressure, speed, length)
2. Add glob/wildcard pattern support for batch processing multiple NetCDF files
3. Support multi-variable extraction in a single pass
4. Add Parquet output configuration options (compression codec, row group size)
5. Extend formula parser with additional mathematical functions (abs, min, max, pow, log, exp, sin, cos)

## Scope

This epic covers functional enhancements to the library and CLI. It does NOT cover:

- Performance optimization of new features (that is Epic 03, which establishes baselines first)
- New storage backends (non-goal)
- Documentation of new features in README (that is Epic 05)

## Dependencies

- **Requires**: Epic 02 (clean error types and module boundaries) and Epic 03 (performance baselines to ensure new features do not regress)
- **Feeds into**: Epic 05 (new features need documentation)

## Tickets

| ID         | Title                                                  | Points | Confidence |
| ---------- | ------------------------------------------------------ | ------ | ---------- |
| ticket-019 | Extend UnitConverter with Meteorological Unit Families | 2      | High       |
| ticket-020 | Add Glob Pattern Support for Batch File Processing     | 4      | High       |
| ticket-021 | Support Multi-Variable Extraction in Single Pass       | 4      | Medium     |
| ticket-022 | Add Parquet Output Configuration Options               | 2      | High       |
| ticket-023 | Extend Formula Parser with Mathematical Functions      | 3      | Medium     |

**Total**: 15 points (reduced from original 19 estimate after codebase inspection)

## Success Criteria

- UnitConverter supports at least 4 unit families (temperature, pressure, speed, length)
- Glob patterns process all matching files with a single command
- Multi-variable extraction produces a single DataFrame with all requested variables
- Parquet output supports at least snappy, zstd, and lz4 compression
- Formula parser supports abs, min, max, pow, log, exp, sin, cos functions
