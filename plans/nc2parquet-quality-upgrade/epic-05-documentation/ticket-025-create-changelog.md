# ticket-025 Create CHANGELOG Following Keep a Changelog

## Context

### Background

A minimal `CHANGELOG.md` already exists at the project root with brief entries for v0.1.0 and v0.1.1, but it does not follow the Keep a Changelog format and omits the substantial work done during the quality upgrade (epics 01-04). The changelog needs to be rewritten to follow the standard Keep a Changelog format (https://keepachangelog.com/), include an "Unreleased" section for the quality upgrade work, and properly categorize changes into Added, Changed, Fixed, etc. The git history has 24 commits from initial commit through the 4 completed epics, with enough detail to reconstruct a meaningful changelog.

### Relation to Epic

This is the second ticket in Epic 05. The CHANGELOG.md is linked from the README (ticket-024) and provides users with a structured history of the project's evolution. It is also referenced by the release automation in Epic 06 (ticket-031).

### Current State

- **File**: `/home/rogerio/git/nc2parquet/CHANGELOG.md` -- 13 lines, minimal entries for v0.1.0 and v0.1.1, does not follow Keep a Changelog format.
- **Git history** (relevant commits from `git log --oneline --all`):
  - `c0c29ec` Initial commit
  - `d95802c` through `0313e28`: Core development (filters, S3, CLI, formatting, tests)
  - `f56658a` fixes datetime postprocessing (v0.1.1)
  - `640fb67` feat: complete epic-01 testing infrastructure & coverage
  - `3f3b7aa` feat: complete epic-02 code quality & refactoring
  - `ee1fb4b` feat: complete epic-03 performance optimization
  - `c55aad5` feat: complete epic-04 feature completeness
- **Current version**: `0.1.1` in `Cargo.toml`.
- **Quality upgrade work** (epics 01-04 summary):
  - Epic 01: Test infrastructure (proptest, assert_cmd, criterion), test reorganization, 296 lib tests
  - Epic 02: Handler extraction from main.rs, thiserror error types, visibility audit, exhaustive rustdoc, dead code removal
  - Epic 03: Criterion benchmarks, chunked reading, CombinationBuffer, postprocessor batching, DHAT profiling
  - Epic 04: Meteorological units, glob batch processing, multi-variable extraction, Parquet output config, extended formula parser (11 unary + 4 binary functions)

## Specification

### Requirements

1. **Rewrite** `/home/rogerio/git/nc2parquet/CHANGELOG.md` following the Keep a Changelog format.
2. **Include** the standard Keep a Changelog header with a link to the format specification.
3. **Include an `[Unreleased]` section** at the top documenting all quality upgrade work from epics 01-04, categorized appropriately.
4. **Preserve** the existing v0.1.1 and v0.1.0 entries, reformatting them to follow the standard categories.
5. **Use standard categories**: Added, Changed, Deprecated, Removed, Fixed, Security -- only include categories that have entries.
6. **Include comparison links** at the bottom of the file (e.g., `[Unreleased]: https://github.com/rjmalves/nc2parquet/compare/v0.1.1...HEAD`).
7. The Unreleased section should cover the following high-level changes from the quality upgrade:
   - **Added**: Criterion benchmark suite (4 bench files), property-based tests (proptest), multi-variable extraction (`--variables`/`-N`), glob batch processing (`--glob`), Parquet output configuration (`--compression`, `--row-group-size`, `--no_statistics`), extended formula parser functions (sqrt, abs, ceil, floor, round, ln, log2, log10, exp, sin, cos, min, max, pow, atan2), meteorological unit families (pressure, speed, length), DHAT heap profiling support
   - **Changed**: Handler extraction from main.rs into `src/handlers/`, error types migrated to thiserror with 11 enum variants, visibility modifiers tightened (pub(crate) for internal modules), exhaustive rustdoc with doc-examples on all public items, test reorganization into 11 module-specific files, extraction pipeline optimized with CombinationBuffer, postprocessor batching for UnitConverter
   - **Fixed**: DateTime postprocessing column generation (v0.1.1 carry-forward)

### Inputs/Props

- Existing CHANGELOG at `/home/rogerio/git/nc2parquet/CHANGELOG.md` (to be rewritten).
- Git history for version links.
- Learnings summary for epic-04 and earlier for change details.
- Repository URL: `https://github.com/rjmalves/nc2parquet`.

### Outputs/Behavior

A single rewritten `/home/rogerio/git/nc2parquet/CHANGELOG.md` file following Keep a Changelog format.

### Error Handling

Not applicable (documentation file).

## Acceptance Criteria

- [ ] Given the rewritten CHANGELOG, when inspecting the file, then it starts with a title `# Changelog` and a line referencing the Keep a Changelog format
- [ ] Given the rewritten CHANGELOG, when inspecting the file, then it contains an `## [Unreleased]` section with quality upgrade changes
- [ ] Given the rewritten CHANGELOG, when inspecting the `[Unreleased]` section, then it contains at minimum `### Added` and `### Changed` subsections with multiple entries each
- [ ] Given the rewritten CHANGELOG, when inspecting the file, then it contains a `## [0.1.1]` section with the datetime postprocessing fix
- [ ] Given the rewritten CHANGELOG, when inspecting the file, then it contains a `## [0.1.0]` section with the initial release features
- [ ] Given the rewritten CHANGELOG, when inspecting the bottom of the file, then it contains version comparison links using the GitHub repository URL
- [ ] Given the rewritten CHANGELOG, when inspecting the file, then each version section uses only standard Keep a Changelog categories (Added, Changed, Deprecated, Removed, Fixed, Security)

## Implementation Guide

### Suggested Approach

1. Read the existing `CHANGELOG.md` and the accumulated learnings from `learnings/epic-04-summary.md`.
2. Create the new file structure with the Keep a Changelog header.
3. Write the `[Unreleased]` section first, categorizing the quality upgrade work. Use the learnings summary to identify what was added vs changed.
4. Reformat the v0.1.1 entry as a `### Fixed` entry under `## [0.1.1]`.
5. Reformat the v0.1.0 entry with proper `### Added` categories.
6. Add comparison links at the bottom.

### Key Files to Modify

- `/home/rogerio/git/nc2parquet/CHANGELOG.md` -- complete rewrite

### Patterns to Follow

- Follow the Keep a Changelog 1.1.0 format exactly: https://keepachangelog.com/en/1.1.0/
- Use ISO 8601 date format for version headers: `## [0.1.1] - YYYY-MM-DD` (use actual dates from git log if available, or omit dates for historical versions if not precisely known).
- Each entry should be a brief, user-facing description (not internal implementation detail). For example, "Added glob pattern support for batch processing multiple NetCDF files" rather than "Implemented `process_netcdf_batch` with `glob::glob` in `src/lib.rs`".
- Group related changes together (e.g., all postprocessor additions under one "Added" subsection entry with sub-bullets).

### Pitfalls to Avoid

- Do not include internal implementation details that users do not care about (e.g., "extracted handlers from main.rs" should be phrased as "Improved code organization and module structure").
- Do not include test-only changes as top-level entries (tests are infrastructure, not user-facing).
- Do not use dates that cannot be verified from git history; use approximate dates or omit them.
- Do not forget the comparison links at the bottom -- these are a key part of the format.

## Testing Requirements

### Unit Tests

Not applicable (documentation).

### Integration Tests

Not applicable (documentation).

### Manual Verification

- Verify the file follows Keep a Changelog format by checking against the specification.
- Verify comparison links use the correct GitHub repository URL.

## Dependencies

- **Blocked By**: ticket-013 (all code changes from Epic 02 complete -- already done)
- **Blocks**: None

## Effort Estimate

**Points**: 2
**Confidence**: High
