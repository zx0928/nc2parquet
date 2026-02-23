# Accumulated Learnings Through Epic 05

## Module Structure (current state after epic-05)

- `src/lib.rs` — exports `process_netcdf_job`, `process_netcdf_job_async`, `process_netcdf_batch`, `resolve_output_path`; re-exports `BatchConfig`, `BatchResult`, `Nc2ParquetError`; `extract` and `output` modules are `pub(crate)`
- `src/extract.rs` — `DimensionIndexManager`, `CombinationBuffer`, `extract_data_to_dataframe`, `extract_multi_variable_dataframe` (`pub(crate)`); private helpers for multi-variable path
- `src/postprocess.rs` — `PostProcessor` trait, `ProcessingPipeline`, 5 processor types; `FormulaApplier` supports 11 unary + 4 binary functions; `UnitFamily` is private
- `src/input.rs` — `JobConfig` (with `variable_names: Option<Vec<String>>`, `output: Option<OutputConfig>`), `FilterConfig`, `BatchConfig`, `BatchResult`, `CompressionCodec`, `OutputConfig`
- `src/output.rs` — all write functions accept `Option<&OutputConfig>`; `build_parquet_writer` is the single config application point (`pub(crate)`)
- `src/filters.rs` — `NCFilter` trait, 4 filter types; `filter_factory` is `pub(crate)`
- `src/errors.rs` — `Nc2ParquetError` (11 variants, thiserror); `StorageError` boxed inside enum with manual `From` impl
- `src/storage.rs` — `StorageBackend` async trait, `Storage` enum dispatch, `StorageFactory::from_path`
- `src/handlers/` — 8 binary-only files declared in `main.rs`, not `lib.rs`; includes `convert.rs`, `validate.rs`, `info.rs`, `template.rs`, `completions.rs`, `config.rs`, `utils.rs`, `mod.rs`
- `src/tests/` — 11 test files: test_batch, test_cli, test_extract, test_filters, test_info, test_input, test_integration, test_memory_profile (dhat-heap feature only), test_multi_variable, test_output, test_postprocess, test_properties
- `benches/` — 4 Criterion benchmark files: extraction_bench, filter_bench, postprocess_bench, combination_bench

## Test Counts

- Total lib tests: 296
- Doc tests: 28 (decreased from 39 after epic-04; investigate before adding more doc examples)
- DHAT profiling test: 1 (gated on `dhat-heap` feature)

## Key Patterns

- **Flat combination buffer**: `CombinationBuffer` in `src/extract.rs`; packs dimension index combinations as `Vec<usize>` with stride indexing; use `for combo in &buffer` via `IntoIterator`; avoids per-row heap allocation
- **Bounding-box slab read**: `extract_data_batch` uses `var.get_values::<f32, _>(extents)` once per variable; use `netcdf::Extents::try_from((starts.as_slice(), counts.as_slice()))` for multi-dimensional extents
- **Cartesian vs explicit dispatch**: `is_cartesian_product()` returns true when `explicit_combinations.is_none()`; dispatch in `extract_data_with_dimension_manager`
- **Eager drop via inner scope**: `let result = { let temp = ...; result };` frees `coordinate_vars` and `slab` before `build_dataframe` allocates Polars columns
- **Scoped NetCDF variable borrows**: each `&netcdf::Variable` borrow lives in its own `{ }` block; first variable is opened three times (DimensionIndexManager, dimension metadata, values) due to borrow checker constraints — see `src/extract.rs` lines 395-428
- **Unit family lookup table**: `unit_to_base_factor` maps lowercased unit aliases to `(UnitFamily, to_base_factor)`; `calculate_conversion_factor` derives any pair as `from_base / to_base`; temperature excluded (requires offset)
- **Depth-counting function call parser**: `parse_factor` detects `identifier(` and uses depth counting to find matching `)`; handles arbitrarily nested function calls in formula strings
- **Additive output config threading**: `build_parquet_writer` in `src/output.rs` accepts `Option<&OutputConfig>`; `None` produces a default writer; all write functions delegate here
- **Dead_code + test-only items**: `pub(crate)` items used only in `#[cfg(test)]` carry `#[allow(dead_code)] // Used in #[cfg(test)] modules`
- **Binary-library split**: `handlers/` declared in `main.rs` not `lib.rs`; all CLI-specific deps (indicatif, anyhow) stay out of lib; handlers import with `use nc2parquet::...`
- **Error enum with boxed SDK variant**: `StorageError` is `Box<StorageError>` inside `Nc2ParquetError`; requires manual `From<StorageError>` impl because `#[from]` does not support boxed variants
- **Processor lazy batching**: `ProcessingPipeline::execute` batches consecutive `UnitConverter` processors with disjoint `target_columns()` into a single `.with_columns(batch_exprs).collect()` call
- **Conventional Commits enforced**: all epic completion commits use `feat:` prefix with imperative subject; CONTRIBUTING.md documents the convention with six concrete examples and a 72-character limit
- **MADR ADR template**: Status, Date, Context, Decision, Consequences (Positive/Negative), Alternatives Considered; four-digit zero-padded filenames under `docs/adr/`; index table in `docs/adr/README.md`
- **Tutorial progression pattern**: four tutorials in dependency order (basic -> filtered -> batch -> config); each uses only bundled `examples/data/` fixtures; `### Step N:` headers throughout; "What's Next" footer
- **README length discipline**: 200-350 lines for a landing-page README; collapse inline JSON examples to links; use reference tables for flags and subcommands

## Fixture File Facts (important for all epics)

- `simple_xy.nc`: 2D (x=6, y=12), variable "data", NO coordinate variables (index values default to integer position)
- `pres_temp_4D.nc`: 4D, time(2), level(2), latitude(6: 25-50), longitude(12: -125 to -70), variables: temperature, pressure; has "time" dimension but NO time coordinate variable; `NC3DPointFilter` must not be used with this file
- Do not use absolute paths in tutorial commands — all tutorial commands reference `examples/data/` from the project root

## Architecture and API Boundaries

- `src/lib.rs` is the only public surface; re-exports four functions and three types; `extract.rs` and `output.rs` are `pub(crate)` and must not become public API
- `src/handlers/` must not be imported by `lib.rs` or any library module; CLI-specific logic belongs here
- `PostProcessor` trait (`src/postprocess.rs`) and `NCFilter` trait (`src/filters.rs`) are the two primary extension points; new processors/filters require: trait impl, serde config struct, factory wiring, integration test
- `ProcessorConfig` enum uses `#[serde(tag = "type", rename_all = "snake_case")]`; new variants are additive and backward-compatible
- `CompressionCodec` uses `#[serde(rename_all = "lowercase")]`; adding a codec requires a new enum variant, a match arm in `to_polars_compression`, and a CLI string case in `handlers/convert.rs`
- `StorageError` variants for AWS SDK errors must stay inside `StorageError`, not promoted into `Nc2ParquetError` directly, to contain large generic instantiations

## Documentation Structure (created in epic-05)

- `README.md` — 217 lines; badges, features, installation, quick-start (CLI + library Rust example), CLI reference table, configuration, filter types, post-processing, storage, links to CONTRIBUTING/CHANGELOG/LICENSE
- `CHANGELOG.md` — Keep a Changelog 1.1.0 format; `[Unreleased]` section for all quality-upgrade changes; version comparison links at bottom
- `CONTRIBUTING.md` — 475 lines; multi-platform build matrix (Ubuntu, Fedora, macOS, Windows, Docker); architecture tree; coding standards; doc-comment template; test table (12 files); benchmark reference; PR process; issue reporting; ADR link
- `docs/adr/` — four ADRs (error handling, module structure, storage abstraction, post-processing pipeline); each covers Context, Decision, Consequences, Alternatives; index at `docs/adr/README.md`
- `docs/tutorials/` — four tutorials (basic-conversion, filtered-extraction, batch-processing, config-files) plus index at `docs/tutorials/README.md`

## Known Limitations and Technical Debt

- Doc test count: 28 (down from 39 after epic-04); root cause not confirmed; investigate with `cargo test --doc -- --list` before adding more doc examples
- `FormulaApplier` does not support unary minus prefix (e.g., `-temperature`); users must write `0 - temperature` — see `src/postprocess.rs` line 978
- `FormulaApplier` and `DateTimeConverter` do not implement `to_lazy_expr()`; each materializes a DataFrame per call
- `Polars 0.51.0`: `AggregationOp::Mean` panics in release builds via streaming engine; `AggregationOp::Sum` works
- `BatchConfig` does not support multi-variable extraction (uses `variable_name: String`, not `variable_names`)
- Multi-variable extraction opens the first `netcdf::Variable` three times due to borrow checker constraints — see `src/extract.rs` lines 395-428
- Epic-05 documentation changes are uncommitted (working tree only); commit before starting epic-06

## Dependencies Added

- `glob = "0.3"` (added in epic-04) — used only in `process_netcdf_batch` in `src/lib.rs`
- No new dependencies added in epics 01-05 beyond test dev-dependencies (proptest, assert_cmd, predicates) and the glob crate

## Serde Conventions

- New optional `JobConfig` fields use `#[serde(skip_serializing_if = "Option::is_none", default)]`
- `CompressionCodec` uses `#[serde(rename_all = "lowercase")]`
- `ProcessorConfig` uses `#[serde(tag = "type", rename_all = "snake_case")]`

## Recommendations for Epic 06

- Add Codecov and benchmark regression jobs to the CONTRIBUTING.md CI table (`CONTRIBUTING.md` lines 414-421) when those tickets land
- CHANGELOG `[Unreleased]` must be split into a release version section when ticket-031 (release automation) runs; update comparison links at the bottom of `CHANGELOG.md`
- Commit epic-05 documentation work as a single `docs:` commit before epic-06 CI changes modify `.github/workflows/ci.yml`
- `BatchConfig` + `process_netcdf_batch` batch path is not covered by property-based tests; add at least one proptest targeting `resolve_output_path` before adding coverage reporting
- `OutputConfig::validate()` has six error paths all covered in `test_output.rs`; the `cargo test --lib` run is sufficient to catch regressions
