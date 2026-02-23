# ticket-013 Remove Dead Code and Fix Remaining Clippy Warnings

## Context

### Background

After the refactoring in tickets 009-012, there may be dead code, unused imports, or new clippy warnings introduced. This ticket is a cleanup pass to ensure the codebase is pristine.

### Relation to Epic

Fifth and final ticket in Epic 02. This is the polish pass after all structural changes.

### Current State

Known issues to address:

- `#[allow(clippy::needless_borrows_for_generic_args)]` at top of tests.rs -- may no longer be needed
- `#[allow(clippy::match_ref_pats)]` at top of tests.rs -- may no longer be needed
- `#[allow(clippy::large_enum_variant)]` on Commands enum -- review if still needed
- `#[allow(clippy::too_many_arguments)]` in extract.rs -- consider refactoring
- `unsafe { std::env::set_var(...) }` in main.rs and tests -- the `init_logging` function uses unsafe env::set_var
- Potential unused imports after module restructuring

## Specification

### Requirements

1. Run `cargo clippy -- -D warnings` and fix all warnings
2. Remove all `#[allow(clippy::...)]` attributes that are no longer needed
3. For `#[allow]` attributes that ARE still needed, add a comment explaining why
4. Run `cargo build` with `#[warn(dead_code)]` and remove any dead code
5. Remove unused `use` statements
6. Address the `unsafe` block in `init_logging`:
   - Option A: Use `env_logger::Builder` instead of setting RUST_LOG env var
   - Option B: Keep the unsafe with a `// SAFETY:` comment explaining it is single-threaded at this point
7. Verify the `netcdf_exploration_tests` module -- if it is purely exploratory and not testing behavior, consider removing or converting to a doc comment

### Inputs/Props

No runtime changes.

### Outputs/Behavior

- `cargo clippy -- -D warnings` passes with zero warnings and zero `#[allow]` suppressions (except justified ones)
- No dead code warnings

### Error Handling

Not applicable.

## Acceptance Criteria

- [ ] Given `cargo clippy -- -D warnings`, when run, then zero warnings
- [ ] Given all `#[allow(clippy::...)]` attributes, when inspected, then each has a justification comment or is removed
- [ ] Given `cargo build`, when run with default warnings, then no dead_code warnings
- [ ] Given the `unsafe` block in init_logging, when inspected, then it either uses a safe alternative or has a SAFETY comment
- [ ] Given all tests, when `cargo test --lib` runs, then all pass

## Implementation Guide

### Suggested Approach

1. Run `cargo clippy -- -D warnings 2>&1` and address each warning
2. Run `cargo build 2>&1 | grep "warning"` to find any build warnings
3. For each `#[allow]` attribute, try removing it and see if clippy complains
4. For `init_logging`, prefer the safe alternative:
   ```rust
   fn init_logging(cli: &Cli) {
       let log_level = if cli.quiet { "error" } else if cli.verbose { "debug" } else { "info" };
       env_logger::Builder::new()
           .filter_module("nc2parquet", log_level.parse().unwrap())
           .init();
   }
   ```
5. Search for unused imports with `cargo build 2>&1 | grep "unused import"`

### Key Files to Modify

- `/home/rogerio/git/nc2parquet/src/main.rs` -- fix init_logging unsafe
- Any files flagged by clippy
- Test files with `#[allow]` attributes

### Patterns to Follow

- `// SAFETY: <reason>` comment before any remaining `unsafe` blocks
- No `#[allow]` without a `// Reason: <explanation>` comment
- Prefer safe alternatives to unsafe code

### Pitfalls to Avoid

- The `env::set_var` in tests is harder to avoid (some tests need it for env var testing) -- those are fine with unsafe since they use a Mutex for synchronization
- Do not remove the `#[allow(clippy::large_enum_variant)]` on Commands if the variants genuinely have large size differences -- measure first with `std::mem::size_of`
- Removing the exploration test module is optional -- if it provides value for understanding the NetCDF API, keep it with a comment

## Testing Requirements

### Unit Tests

No new tests.

### Integration Tests

`cargo test --lib` and `cargo test --doc` must pass.

### E2E Tests

None.

## Dependencies

- **Blocked By**: ticket-012 (rustdoc complete)
- **Blocks**: None

## Effort Estimate

**Points**: 2
**Confidence**: High
