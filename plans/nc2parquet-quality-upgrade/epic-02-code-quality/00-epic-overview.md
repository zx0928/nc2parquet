# Epic 02: Code Quality and Refactoring

## Goals

1. Decompose `main.rs` (~1100 lines) into focused handler modules
2. Improve error types with structured errors and better messages
3. Remove dead code and tighten visibility modifiers
4. Add exhaustive rustdoc to all public items
5. Clean up minor code quality issues (unnecessary clones, unused imports)

## Scope

- **In scope**: main.rs decomposition, error type improvements, visibility audit, rustdoc additions, clippy lint fixes
- **Out of scope**: Feature additions, performance optimizations, new test infrastructure (Epic 1)

## Current State

- `main.rs` contains 1097 lines including all command handlers, config loading, validation, template generation, and utility functions
- Error handling mixes `anyhow` (application) and `Box<dyn Error>` (library) -- mostly correct but some library functions return `Box<dyn Error>` where `thiserror` types would be better
- Most public items have rustdoc but some lack examples
- `PostProcessError` exists but is incomplete (no `From<Box<dyn Error>>` impl)
- Some filter structs have `pub` fields that could be `pub(crate)`

## Dependencies

- **Blocked by**: Epic 1 (testing) must be complete so tests catch any regressions during refactoring

## Tickets

| ID         | Title                                                      | Effort | Dependencies |
| ---------- | ---------------------------------------------------------- | ------ | ------------ |
| ticket-009 | Extract command handlers from main.rs into handlers module | 5 pts  | ticket-002   |
| ticket-010 | Improve library error types with thiserror                 | 3 pts  | ticket-009   |
| ticket-011 | Audit and tighten visibility modifiers                     | 2 pts  | ticket-010   |
| ticket-012 | Add exhaustive rustdoc with examples to all public items   | 3 pts  | ticket-011   |
| ticket-013 | Remove dead code and fix remaining clippy warnings         | 2 pts  | ticket-012   |

## Success Criteria

- `main.rs` is under 150 lines (entry point + init_logging only)
- All command handlers live in `src/handlers/` with clear module boundaries
- `cargo clippy -- -D warnings` passes with zero warnings
- Every public function, struct, enum, and trait has rustdoc
- No `pub` items that should be `pub(crate)` or private
- All 150+ tests from Epic 1 continue to pass
