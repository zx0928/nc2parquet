# ticket-024 Rewrite README with Professional Structure

## Context

### Background

The current `README.md` at the project root is approximately 740 lines long and reads more like a specification document than a user-facing landing page. It contains good content but suffers from structural issues: sections are not prioritized for discoverability, there are no CI badges, the installation instructions are minimal, the "Roadmap" section references internal sprint numbers that are no longer relevant, and several sections (like the detailed JSON output structure) should be condensed or moved to separate documentation. The README needs to be rewritten as a professional open-source landing page that quickly communicates what the tool does, how to install it, how to get started, and where to find deeper documentation.

### Relation to Epic

This is the first ticket in Epic 05 (Documentation and Community Readiness). The rewritten README serves as the project's front door and primary discovery surface. It links out to CHANGELOG.md (ticket-025), CONTRIBUTING.md (ticket-026), ADRs (ticket-027), and tutorials (ticket-028). Getting the README structure right first establishes the navigation skeleton for all other documentation artifacts.

### Current State

- **File**: `/home/rogerio/git/nc2parquet/README.md` -- 740 lines, comprehensive but poorly structured for quick scanning.
- **Version**: `0.1.1` in `Cargo.toml`, project hosted at `https://github.com/rjmalves/nc2parquet`.
- **License**: MIT (confirmed in `LICENSE` file and `Cargo.toml`).
- **CI**: GitHub Actions at `.github/workflows/ci.yml` (test matrix: stable + beta, clippy, fmt, cargo-audit).
- **Features available** (from epic-04): multi-variable extraction (`-N/--variables`), glob batch processing (`--glob`), Parquet output config (`--compression`, `--row-group-size`, `--no_statistics`), extended formula parser (11 unary + 4 binary functions), meteorological unit families.
- **CLI subcommands**: `convert`, `validate`, `info`, `template`, `completions` (all defined in `src/cli.rs`).
- **Configuration sources**: CLI args > environment variables (`NC2PARQUET_*`) > JSON/YAML config files.
- **Filter types**: range, list, 2d_point, 3d_point.
- **PostProcessor types**: `RenameColumns`, `DatetimeConvert`, `UnitConvert`, `Aggregate`, `ApplyFormula`.
- **Storage backends**: Local filesystem and Amazon S3.
- **Example files**: `examples/data/` (2 NetCDF files), `examples/configs/` (3 JSON configs), `examples/postprocessing/` (6 JSON configs), `examples/cli/cli_examples.sh`.
- **Existing stale sections**: "Roadmap" references Sprint 2-7 (obsolete), "Public Dataset Integration" is overly detailed for a README.

## Specification

### Requirements

1. **Rewrite** `/home/rogerio/git/nc2parquet/README.md` with professional open-source structure.
2. **Include badges** at the top: CI status (GitHub Actions), crates.io version, docs.rs, license (MIT).
3. **Include sections** in this order:
   - One-line project description / tagline
   - Badges row
   - Features overview (bullet list, concise)
   - Installation (cargo install from crates.io, from source with `cargo install --path .`)
   - Quick Start (CLI: basic convert, with filters, with postprocessing; Library: minimal Rust example)
   - CLI Reference (table of subcommands with one-line descriptions, link to `--help`)
   - Configuration (brief overview of sources: CLI > env vars > config files, link to examples/)
   - Filter Types (condensed table, not full JSON examples -- link to examples/configs/)
   - Post-Processing (condensed overview of 5 processor types, link to examples/postprocessing/)
   - Storage Support (local + S3, brief)
   - Contributing (link to CONTRIBUTING.md)
   - Changelog (link to CHANGELOG.md)
   - License (MIT, link to LICENSE)
4. **Remove** the obsolete "Roadmap" section entirely.
5. **Condense** the "Public Dataset Integration" to 2-3 lines maximum, as a note within the testing or S3 section.
6. **Keep README length** between 200-350 lines (concise landing page, not exhaustive reference).
7. **Ensure all CLI examples** are accurate against the current CLI defined in `src/cli.rs` (especially the new `--variables`, `--glob`, `--compression`, `--row-group-size`, `--no_statistics` flags).
8. **Badge URLs** should use standard shields.io format pointing to `rjmalves/nc2parquet`.

### Inputs/Props

- Current README at `/home/rogerio/git/nc2parquet/README.md` (to be replaced).
- `Cargo.toml` metadata: name=`nc2parquet`, version=`0.1.1`, repository=`https://github.com/rjmalves/nc2parquet`, license=MIT.
- CLI structure from `src/cli.rs`: 5 subcommands (convert, validate, info, template, completions).
- Example directory structure: `examples/{cli,configs,data,postprocessing}/`.
- CI workflow name from `.github/workflows/ci.yml`: "CI/CD Pipeline".

### Outputs/Behavior

A single rewritten `/home/rogerio/git/nc2parquet/README.md` file that serves as a professional landing page.

### Error Handling

Not applicable (documentation file).

## Acceptance Criteria

- [ ] Given the rewritten README, when inspecting the file, then it contains a badges row with at least 3 badges (CI, crates.io, license) using shields.io or similar URLs
- [ ] Given the rewritten README, when inspecting the file, then it contains an "Installation" section with both `cargo install nc2parquet` and build-from-source instructions
- [ ] Given the rewritten README, when inspecting the file, then it contains a "Quick Start" section with at least one CLI example and one Rust library example
- [ ] Given the rewritten README, when inspecting the file, then it contains a concise CLI Reference section listing all 5 subcommands (convert, validate, info, template, completions)
- [ ] Given the rewritten README, when inspecting the file, then it mentions the new epic-04 features: multi-variable extraction (`--variables`), glob batch processing (`--glob`), and output configuration (`--compression`)
- [ ] Given the rewritten README, when inspecting the file, then the obsolete "Roadmap" section with Sprint references is absent
- [ ] Given the rewritten README, when inspecting the file, then it contains links to CONTRIBUTING.md and CHANGELOG.md (these files will be created in subsequent tickets)
- [ ] Given the rewritten README, when counting lines, then the file is between 200 and 350 lines
- [ ] Given the rewritten README, when inspecting code examples, then all CLI flag names match the current CLI definition in `src/cli.rs` (e.g. `-n` for variable, `-N` for variables, `--glob`, `--compression`, `--range`, `--list`)

## Implementation Guide

### Suggested Approach

1. Read the current README thoroughly to understand all content that must be preserved (just restructured).
2. Create the new structure starting with badges, then features, then installation.
3. For the Quick Start CLI examples, pull from `examples/cli/cli_examples.sh` and verify flag names against `src/cli.rs`.
4. For the library example, simplify the existing `process_netcdf_job_async` example to the minimal working case.
5. Create a condensed CLI Reference table (subcommand | description | example).
6. Collapse the verbose filter JSON examples into a summary table and reference `examples/configs/` for full examples.
7. Collapse post-processing details into a summary list and reference `examples/postprocessing/`.
8. Remove the Roadmap and condense the Public Dataset Integration section.
9. Add navigation links to CONTRIBUTING.md, CHANGELOG.md, and (future) docs/tutorials/.

### Key Files to Modify

- `/home/rogerio/git/nc2parquet/README.md` -- complete rewrite

### Patterns to Follow

- Use standard README patterns from popular Rust CLI tools (e.g., ripgrep, bat, fd). These typically have: badges, one-liner, features, install, usage, contributing, license.
- Badge format: `[![CI](https://github.com/rjmalves/nc2parquet/actions/workflows/ci.yml/badge.svg)](...)`.
- Use `<details>` HTML tags for optional expandable sections if needed to keep the README concise.

### Pitfalls to Avoid

- Do not use `cargo install nc2parquet` as if it is already on crates.io -- frame it as "when published" or check actual crates.io availability first.
- Do not include outdated flag names; the `--variable` flag uses `-n` short form, and `--variables` uses `-N` short form.
- Do not duplicate full JSON config examples that already exist in `examples/` -- link to them instead.
- Do not include the `--formula` flag in the simplified format; the actual format is `--formula "target:formula:sources"` with 3 colon-delimited parts.
- Do not forget that `netcdf` crate uses `features = ["static"]` which means system NetCDF headers are NOT required at runtime, only at build time.

## Testing Requirements

### Unit Tests

Not applicable (documentation).

### Integration Tests

Not applicable (documentation).

### Manual Verification

- Verify all CLI examples in the README can be copy-pasted and executed (at minimum, the basic convert example with the bundled `examples/data/simple_xy.nc` fixture).
- Verify badge URLs resolve correctly (CI, license).
- Verify links to CONTRIBUTING.md and CHANGELOG.md use relative paths.

## Dependencies

- **Blocked By**: ticket-012 (rustdoc complete -- already done), ticket-023 (all epic-04 features implemented -- already done)
- **Blocks**: ticket-028 (tutorials link back to README)

## Effort Estimate

**Points**: 3
**Confidence**: High
