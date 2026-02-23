# Epic 01: Test Infrastructure and Core Test Coverage

## Goals

1. Establish reusable test infrastructure (helpers, fixtures, data generators)
2. Reorganize the existing monolithic `tests.rs` into per-module test files
3. Add comprehensive unit tests for all modules with edge cases and error paths
4. Add property-based tests for filters and formula parser
5. Add integration tests for full pipeline scenarios
6. Prepare for coverage measurement in CI

## Scope

- **In scope**: Test helpers, test reorganization, unit tests, property tests, integration tests, dev-dependency additions
- **Out of scope**: Benchmark tests (Epic 3), S3 integration tests (already exist), documentation tests (Epic 5)

## Current State

- 97 tests pass, all in `src/tests.rs` (a single ~2370-line file)
- Tests cover: input parsing, filter creation/application, extraction, some postprocessors, CLI parsing, info command, integration pipelines, S3 with NOAA
- **Missing coverage**: PostProcessor error paths, formula parser edge cases (nested parens, negative numbers, malformed input), output module, storage error paths, DimensionIndexManager edge cases (5+ dimensions, empty filters), FilterConfig::to_filter edge cases

## Dependencies

- No dependencies on other epics (this is the foundation)

## Tickets

| ID         | Title                                                   | Effort | Dependencies           |
| ---------- | ------------------------------------------------------- | ------ | ---------------------- |
| ticket-001 | Add test dev-dependencies and create test helper module | 2 pts  | None                   |
| ticket-002 | Reorganize tests.rs into per-module test files          | 3 pts  | ticket-001             |
| ticket-003 | Add unit tests for filters module edge cases            | 3 pts  | ticket-001             |
| ticket-004 | Add unit tests for extract module edge cases            | 3 pts  | ticket-001             |
| ticket-005 | Add unit tests for postprocess module edge cases        | 3 pts  | ticket-001             |
| ticket-006 | Add unit tests for output and storage modules           | 2 pts  | ticket-001             |
| ticket-007 | Add property-based tests for filters and formula parser | 3 pts  | ticket-003, ticket-005 |
| ticket-008 | Add integration tests for error paths and edge cases    | 3 pts  | ticket-002             |

## Success Criteria

- All existing 97 tests continue to pass
- Test count increases to 150+ tests
- Every public function in filters, extract, postprocess, output, storage, and info modules has at least one dedicated test
- Property tests cover filter creation/application and formula parsing
- `cargo test` completes in under 60 seconds (excluding S3 tests)
