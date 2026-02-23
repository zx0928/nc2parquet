# Master Plan: nc2parquet Production Quality Upgrade

## Executive Summary

This plan transforms nc2parquet from a functional v0.1.1 prototype into a production-quality Rust library and CLI tool suitable for wide community adoption in weather/climate data pipelines. The work is organized into 6 epics covering testing, code quality, performance, features, documentation, and CI/CD -- executed in dependency order to ensure each phase builds on solid foundations.

## Goals

1. **Comprehensive test coverage**: Achieve >80% line coverage across all modules with unit, integration, and property-based tests
2. **Production code quality**: Clean module boundaries, strong error types, exhaustive rustdoc, no dead code
3. **Measurable performance**: Criterion benchmarks, chunked processing for large files, memory-efficient pipelines
4. **Feature completeness**: Extended unit conversions, batch processing, enhanced formula parser, output format options
5. **Community readiness**: Professional documentation, CHANGELOG, CONTRIBUTING guide, architecture docs
6. **Release automation**: Coverage tracking, benchmark CI, cross-compilation, release automation

## Non-Goals

- GCS/Azure storage support (keep S3-only for now; can be added later via the StorageBackend trait)
- GUI or web interface
- Streaming/incremental NetCDF processing (beyond chunked reads)
- Python bindings or FFI interfaces

## Architecture Overview

### Current State

```
src/
  main.rs      (~1100 lines - CLI handlers, validation, template generation, config loading)
  lib.rs       (~167 lines - sync/async pipeline orchestration)
  cli.rs       (~850 lines - clap structs, filter parsers, env var handling)
  input.rs     (~204 lines - JobConfig, FilterConfig, deserialization)
  filters.rs   (~365 lines - NCFilter trait, 4 filter implementations)
  extract.rs   (~376 lines - DimensionIndexManager, data extraction)
  postprocess.rs (~956 lines - PostProcessor trait, 5 processor implementations, formula parser)
  output.rs    (~131 lines - Parquet writing sync/async)
  storage.rs   (~150 lines - StorageBackend trait, Local/S3 implementations)
  info.rs      (~100 lines - file inspection)
  tests.rs     (~2370 lines - all tests in single file)
```

**Pipeline**: Input (local/S3) -> NetCDF Parse -> DimensionIndexManager -> Filters -> Extraction -> Polars DataFrame -> PostProcessing -> Parquet Write -> Output (local/S3)

### Target State

```
src/
  main.rs          (~100 lines - entry point only)
  lib.rs           (~170 lines - pipeline orchestration, unchanged)
  cli.rs           (~850 lines - unchanged, already well-structured)
  input.rs         (~204 lines - unchanged)
  filters.rs       (~365 lines - unchanged)
  extract.rs       (~376 lines - unchanged)
  postprocess.rs   (~960 lines - extended unit conversions, more formula functions)
  output.rs        (~200 lines - compression options, row group config)
  storage.rs       (~150 lines - unchanged)
  info.rs          (~100 lines - unchanged)
  handlers/
    mod.rs         (~50 lines)
    convert.rs     (~350 lines - handle_convert_command)
    validate.rs    (~200 lines - handle_validate_command, validate_config)
    info.rs        (~100 lines - handle_info_command)
    template.rs    (~150 lines - handle_template_command, generate_template)
    completions.rs (~50 lines - handle_completions_command)
    config.rs      (~150 lines - load_configuration, load_config_file)
    utils.rs       (~100 lines - shared utilities)
  tests/
    mod.rs
    test_helpers.rs
    test_filters.rs
    test_extract.rs
    test_postprocess.rs
    test_input.rs
    test_info.rs
    test_output.rs
    test_cli.rs
    test_handlers.rs
    test_integration.rs
    test_formula_parser.rs
benches/
    pipeline_bench.rs
    filter_bench.rs
    postprocess_bench.rs
```

### Key Design Decisions

1. **Test-first approach**: Epic 1 (testing) comes before Epic 2 (refactoring) to ensure refactoring does not break existing behavior
2. **Progressive planning**: Epics 1-2 are fully detailed; Epics 3-6 are outlined and will be refined with learnings
3. **Backward compatibility**: No breaking public API changes in this release cycle (stay on 0.1.x or bump to 0.2.0 with clear migration guide)
4. **Module extraction over rewrite**: main.rs handlers are extracted into a `handlers/` module; existing modules remain intact
5. **Criterion for benchmarks**: Industry-standard Rust benchmarking with statistical rigor

## Technical Approach

### Tech Stack

- **Language**: Rust (edition 2024)
- **Core deps**: netcdf 0.11, polars 0.51, aws-sdk-s3 1.106, tokio 1.x, clap 4.4
- **Test deps**: proptest (new), criterion (new), assert_cmd (new), predicates (new)
- **CI**: GitHub Actions, cargo-tarpaulin (coverage), cargo-audit

### Testing Strategy

- **Unit tests**: `#[cfg(test)]` modules per source file using test helpers
- **Integration tests**: `tests/` directory for full pipeline tests
- **Property tests**: proptest for filter logic and formula parser
- **Benchmark tests**: criterion for pipeline, filter, and postprocessor performance
- **CI tests**: All tests run in CI with coverage reporting

### Data Flow (unchanged)

```
Config -> StorageFactory -> NetCDF File -> Variable Lookup
  -> DimensionIndexManager -> Filter Application (intersection)
  -> Coordinate Combination Generation -> Data Extraction
  -> DataFrame Construction -> PostProcessing Pipeline
  -> Parquet Serialization -> Storage Write
```

## Phases and Milestones

| Phase | Epic                              | Duration  | Milestone                                                     |
| ----- | --------------------------------- | --------- | ------------------------------------------------------------- |
| 1     | Testing Infrastructure & Coverage | 2-3 weeks | >80% test coverage, property tests, CI integration tests      |
| 2     | Code Quality & Refactoring        | 2-3 weeks | main.rs decomposed, error types improved, exhaustive rustdoc  |
| 3     | Performance Optimization          | 2-3 weeks | Criterion benchmarks, chunked processing, memory optimization |
| 4     | Feature Completeness              | 2-3 weeks | Extended units, batch processing, enhanced formulas           |
| 5     | Documentation & Community         | 1-2 weeks | README rewrite, CHANGELOG, CONTRIBUTING, tutorials            |
| 6     | CI/CD & Release Quality           | 1-2 weeks | Coverage CI, benchmark CI, release automation                 |

## Risk Analysis

| Risk                                      | Probability | Impact | Mitigation                                                               |
| ----------------------------------------- | ----------- | ------ | ------------------------------------------------------------------------ |
| NetCDF crate API changes on update        | Low         | High   | Pin netcdf version, comprehensive tests catch regressions                |
| Polars API changes (0.51 is recent)       | Medium      | Medium | Pin polars version, isolate polars usage behind output.rs/postprocess.rs |
| Performance regression during refactoring | Medium      | Medium | Establish benchmarks in Epic 1 before refactoring in Epic 2              |
| S3 test flakiness in CI                   | High        | Low    | S3 tests already handle network failures gracefully                      |
| Breaking changes needed for clean API     | Low         | Medium | Plan for 0.2.0 release if needed, with migration guide                   |

## Success Metrics

1. **Test coverage**: >80% line coverage as measured by cargo-tarpaulin
2. **Zero clippy warnings**: `cargo clippy -- -D warnings` passes
3. **Zero unsafe except init_logging**: Minimize unsafe blocks (currently only env::set_var)
4. **All modules documented**: Every public item has rustdoc with examples
5. **Benchmark baselines**: Criterion benchmarks established for all key operations
6. **CI green on stable+beta**: All tests pass on both Rust channels
7. **README completeness**: Installation, usage, API reference, contributing sections
