# ticket-009 Extract Command Handlers from main.rs into Handlers Module

## Context

### Background

`src/main.rs` is currently ~1097 lines containing the CLI entry point, all 5 command handlers, configuration loading/validation, template generation, and utility functions. This makes the file difficult to navigate and test. The handlers should be extracted into a `src/handlers/` module tree for better modularity.

### Relation to Epic

First ticket in Epic 02 (Code Quality). This is the largest refactoring task, decomposing main.rs into focused modules. It must be done carefully because tests from Epic 01 will catch any regressions.

### Current State

`src/main.rs` contains:

- `main()` (~35 lines) -- entry point, CLI parse, command dispatch
- `init_logging()` (~15 lines) -- log level setup
- `handle_convert_command()` (~265 lines) -- convert pipeline with progress bars
- `handle_validate_command()` (~55 lines) -- config validation
- `handle_info_command()` (~45 lines) -- file info display
- `handle_template_command()` (~25 lines) -- template generation
- `handle_completions_command()` (~50 lines) -- shell completions
- `load_configuration()` (~80 lines) -- multi-source config loading
- `load_config_file()` (~15 lines) -- JSON/YAML file loading
- `validate_config()` (~165 lines) -- comprehensive config validation
- `check_output_overwrite()` (~10 lines) -- output file existence check
- `needs_async_processing()` (~3 lines) -- S3 path detection
- `print_config_summary()` (~25 lines) -- config display
- `show_output_info()` (~25 lines) -- output info display
- `show_detailed_validation()` (~115 lines) -- detailed validation report
- `generate_template()` (~75 lines) -- template generation
- `get_file_size()` (~12 lines) -- file size for metrics

## Specification

### Requirements

1. Create `src/handlers/mod.rs` with public re-exports
2. Create `src/handlers/convert.rs` -- `handle_convert_command`, merge_filters pipeline building, progress bar logic
3. Create `src/handlers/validate.rs` -- `handle_validate_command`, `validate_config`, `show_detailed_validation`
4. Create `src/handlers/info.rs` -- `handle_info_command`
5. Create `src/handlers/template.rs` -- `handle_template_command`, `generate_template`
6. Create `src/handlers/completions.rs` -- `handle_completions_command`
7. Create `src/handlers/config.rs` -- `load_configuration`, `load_config_file`
8. Create `src/handlers/utils.rs` -- `check_output_overwrite`, `needs_async_processing`, `print_config_summary`, `show_output_info`, `get_file_size`
9. Reduce `src/main.rs` to: `main()`, `init_logging()`, and imports from handlers module
10. Add `pub mod handlers;` to `src/lib.rs` (or keep handlers as binary-only by importing in main.rs)

### Inputs/Props

No runtime changes -- this is purely a code reorganization.

### Outputs/Behavior

- `cargo build` succeeds
- `cargo test --lib` passes all tests
- `cargo clippy -- -D warnings` passes
- Binary behavior is identical

### Error Handling

No changes to error handling logic -- only move code between files.

## Acceptance Criteria

- [ ] Given the refactoring is complete, when `cargo build` runs, then it succeeds with zero errors
- [ ] Given the refactoring is complete, when `cargo test --lib` runs, then all tests pass (same count as before)
- [ ] Given `src/main.rs` is read, when lines are counted, then it has fewer than 150 lines
- [ ] Given `src/handlers/convert.rs` exists, when it is read, then it contains `handle_convert_command`
- [ ] Given `src/handlers/validate.rs` exists, when it is read, then it contains `validate_config` and `show_detailed_validation`
- [ ] Given `src/handlers/config.rs` exists, when it is read, then it contains `load_configuration` and `load_config_file`
- [ ] Given `src/handlers/mod.rs` exists, when it is read, then it re-exports all handler functions needed by main.rs
- [ ] Given the binary is built, when `nc2parquet --help` is run, then output is identical to before refactoring
- [ ] Given `cargo clippy -- -D warnings` is run, when it completes, then zero warnings

## Implementation Guide

### Suggested Approach

1. Create `src/handlers/mod.rs`:

   ```rust
   pub mod convert;
   pub mod validate;
   pub mod info;
   pub mod template;
   pub mod completions;
   pub mod config;
   pub mod utils;

   pub use convert::handle_convert_command;
   pub use validate::handle_validate_command;
   pub use info::handle_info_command;
   pub use template::handle_template_command;
   pub use completions::handle_completions_command;
   pub use config::load_configuration;
   ```

2. Move functions file by file. Start with the simplest (completions) and work up to the most complex (convert). For each:
   - Copy the function to the new file
   - Add required `use` imports at the top
   - Verify `cargo build` after each file

3. Once all handlers are moved, update `src/main.rs` to import from `handlers`:

   ```rust
   mod handlers;
   use handlers::*;
   ```

4. The handlers module should NOT be added to `lib.rs` -- it contains binary-specific logic (progress bars, output formatting). Keep it as a module only accessible from the binary crate.

5. Since handlers is not in lib.rs, the `mod handlers;` declaration goes in `main.rs`. The handlers module uses `nc2parquet::*` imports for library types.

### Key Files to Modify

- `/home/rogerio/git/nc2parquet/src/main.rs` -- reduce to ~100 lines
- `/home/rogerio/git/nc2parquet/src/handlers/mod.rs` -- CREATE
- `/home/rogerio/git/nc2parquet/src/handlers/convert.rs` -- CREATE
- `/home/rogerio/git/nc2parquet/src/handlers/validate.rs` -- CREATE
- `/home/rogerio/git/nc2parquet/src/handlers/info.rs` -- CREATE
- `/home/rogerio/git/nc2parquet/src/handlers/template.rs` -- CREATE
- `/home/rogerio/git/nc2parquet/src/handlers/completions.rs` -- CREATE
- `/home/rogerio/git/nc2parquet/src/handlers/config.rs` -- CREATE
- `/home/rogerio/git/nc2parquet/src/handlers/utils.rs` -- CREATE

### Patterns to Follow

- Each handler file has its own imports -- do not use `use super::*`
- Handler functions take `&Cli` as first parameter (same as current signatures)
- Use `pub(crate)` for functions only needed within the binary
- Keep the same function signatures so no callers need to change

### Pitfalls to Avoid

- `handle_convert_command` uses `merge_filters` from `cli.rs` -- this is already a pub function in the library
- `validate_config` references `nc2parquet::input::FilterConfig` -- use the library's public API
- `show_detailed_validation` uses `FilterConfig` variants directly -- ensure imports match
- `generate_template` constructs `JobConfig` and `FilterConfig` values -- use library types
- The `info` handler imports from `nc2parquet::info::*` -- ensure these are re-exported from lib.rs
- Do NOT change any function signatures or logic -- only move code

## Testing Requirements

### Unit Tests

No new tests. All existing tests must pass.

### Integration Tests

Run `cargo test --lib` to verify no regressions.

### E2E Tests

Run `cargo run -- --help` to verify CLI still works.

## Dependencies

- **Blocked By**: ticket-002 (tests must be reorganized first so they catch regressions)
- **Blocks**: ticket-010, ticket-011, ticket-012, ticket-013

## Effort Estimate

**Points**: 5
**Confidence**: High
