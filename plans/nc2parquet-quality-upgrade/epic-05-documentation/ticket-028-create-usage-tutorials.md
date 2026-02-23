# ticket-028 Create Usage Tutorials for Common Workflows

## Context

### Background

The project has reference documentation (rustdoc, README, example configs) but lacks step-by-step tutorials that guide users through common workflows from start to finish. Tutorials bridge the gap between "I know what the tool does" and "I know how to use it for my specific use case." The `examples/` directory already contains sample NetCDF files (`simple_xy.nc`, `pres_temp_4D.nc`), CLI examples (`cli_examples.sh`), and config files (3 basic + 6 postprocessing), providing a solid foundation for tutorials to reference. The tutorials should be CLI-focused (primary audience is data scientists and climate researchers using the command line) with library usage shown where it adds value.

### Relation to Epic

This is the fifth and final ticket in Epic 05. Tutorials are linked from the README (ticket-024) and represent the most user-facing documentation in the epic. They depend on the README being rewritten first (ticket-024) so that the README can link to tutorials and tutorials can link back to the README for reference material.

### Current State

- **Directory**: `/home/rogerio/git/nc2parquet/docs/` -- does not exist (will be created by ticket-027 for ADRs; tutorials go in `docs/tutorials/`).
- **Example fixtures**: `examples/data/simple_xy.nc` (2D, x=6, y=12, variable "data"), `examples/data/pres_temp_4D.nc` (4D, time(2), level(2), latitude(6), longitude(12), variables: temperature, pressure).
- **Example configs**: `examples/configs/{simple_local.json, multi_filter.json, multi_source_example.json}`, `examples/postprocessing/{column_renaming.json, complex_formula.json, complex_pipeline.json, datetime_conversion.json, formula_application.json, unit_conversion.json}`.
- **CLI examples**: `examples/cli/cli_examples.sh` contains 12 example commands.
- **CLI subcommands**: convert (with `--variable`, `--variables`, `--glob`, `--range`, `--list`, `--point2d`, `--point3d`, `--rename`, `--unit-convert`, `--kelvin-to-celsius`, `--formula`, `--compression`, `--row-group-size`, `--no_statistics`, `--config`, `--dry-run`, `--force`), validate, info, template, completions.
- **S3 support**: Both input and output, async via StorageBackend trait, NOAA public dataset available for examples.

## Specification

### Requirements

1. **Create directory** `/home/rogerio/git/nc2parquet/docs/tutorials/`.
2. **Create tutorial index** at `/home/rogerio/git/nc2parquet/docs/tutorials/README.md` listing all tutorials with brief descriptions.
3. **Create 4 tutorials** (minimum 2 required by epic success criteria):
   - **basic-conversion.md**: Covers the simplest possible conversion workflow. Uses the bundled `examples/data/simple_xy.nc` fixture. Demonstrates: `nc2parquet info` to inspect the file, basic `nc2parquet convert` with `-n`, verifying output. Also shows the equivalent library usage with `process_netcdf_job`.
   - **filtered-extraction.md**: Covers filtering and post-processing. Uses `examples/data/pres_temp_4D.nc`. Demonstrates: range filter (`--range`), list filter (`--list`), combining multiple filters, adding postprocessing (`--rename`, `--kelvin-to-celsius`, `--formula`). Shows the equivalent JSON config file.
   - **batch-processing.md**: Covers batch processing with glob patterns. Demonstrates: `--glob` flag, output directory, output templates, `--compression` option, `--fail-fast` behavior. Shows the equivalent `BatchConfig` JSON.
   - **config-files.md**: Covers configuration file usage. Demonstrates: generating templates with `nc2parquet template`, JSON and YAML config files, config validation with `nc2parquet validate`, configuration precedence (CLI > env > file). References the existing `examples/configs/` and `examples/postprocessing/` files.
4. **Each tutorial must contain**:
   - Prerequisites (what the reader needs before starting)
   - Step-by-step instructions with numbered steps
   - Complete CLI commands that can be copy-pasted
   - Expected output (or description of what to expect)
   - "What's next" section linking to the next tutorial or reference docs
5. **All CLI commands in tutorials must use the bundled example files** (`examples/data/`) so that readers can follow along with a fresh clone of the repository.
6. **Do NOT create an S3 tutorial** -- S3 workflows require AWS credentials and cannot be followed by all users. Instead, mention S3 support briefly in the basic-conversion tutorial and reference the README's S3 section.

### Inputs/Props

- Example fixtures: `examples/data/simple_xy.nc`, `examples/data/pres_temp_4D.nc`.
- Example configs: files in `examples/configs/` and `examples/postprocessing/`.
- CLI definition: `src/cli.rs` for accurate flag names and syntax.

### Outputs/Behavior

- `/home/rogerio/git/nc2parquet/docs/tutorials/README.md` -- tutorial index
- `/home/rogerio/git/nc2parquet/docs/tutorials/basic-conversion.md`
- `/home/rogerio/git/nc2parquet/docs/tutorials/filtered-extraction.md`
- `/home/rogerio/git/nc2parquet/docs/tutorials/batch-processing.md`
- `/home/rogerio/git/nc2parquet/docs/tutorials/config-files.md`

### Error Handling

Not applicable (documentation files).

## Acceptance Criteria

- [ ] Given the `docs/tutorials/` directory, when listing files, then it contains `README.md`, `basic-conversion.md`, `filtered-extraction.md`, `batch-processing.md`, `config-files.md`
- [ ] Given `docs/tutorials/README.md`, when inspecting the file, then it lists all 4 tutorials with brief descriptions and links
- [ ] Given `basic-conversion.md`, when inspecting the file, then it contains a step using `nc2parquet info` to inspect a bundled example file, and a step using `nc2parquet convert` with the `-n` flag
- [ ] Given `filtered-extraction.md`, when inspecting the file, then it contains at least one `--range` filter example and at least one post-processing example (`--rename` or `--formula`)
- [ ] Given `batch-processing.md`, when inspecting the file, then it contains a `--glob` example with an output directory
- [ ] Given `config-files.md`, when inspecting the file, then it contains a `nc2parquet template` example and a `nc2parquet validate` example
- [ ] Given each tutorial, when inspecting it, then all CLI commands reference files under `examples/data/` or `examples/configs/` (bundled fixtures)
- [ ] Given each tutorial, when inspecting it, then it contains numbered step-by-step instructions and a "What's Next" or equivalent closing section

## Implementation Guide

### Suggested Approach

1. Create the `docs/tutorials/` directory (the `docs/` directory may already exist from ticket-027 creating `docs/adr/`; if not, create both).
2. Write the tutorial index first to establish the navigation structure.
3. Write `basic-conversion.md` first -- it is the simplest and establishes the tutorial style.
4. Write `filtered-extraction.md` second -- it builds on the basic tutorial.
5. Write `batch-processing.md` third -- demonstrates the glob feature from epic-04.
6. Write `config-files.md` last -- it ties together CLI and config file approaches.
7. For each tutorial, verify CLI flag names against `src/cli.rs` before writing commands.
8. Use consistent formatting: step headers as `### Step N: Description`, CLI commands in fenced code blocks with `bash` syntax highlighting.

### Key Files to Create

- `/home/rogerio/git/nc2parquet/docs/tutorials/README.md`
- `/home/rogerio/git/nc2parquet/docs/tutorials/basic-conversion.md`
- `/home/rogerio/git/nc2parquet/docs/tutorials/filtered-extraction.md`
- `/home/rogerio/git/nc2parquet/docs/tutorials/batch-processing.md`
- `/home/rogerio/git/nc2parquet/docs/tutorials/config-files.md`

### Reference Files (read-only)

- `/home/rogerio/git/nc2parquet/src/cli.rs` -- for accurate CLI flag names and syntax
- `/home/rogerio/git/nc2parquet/examples/cli/cli_examples.sh` -- for command examples to adapt
- `/home/rogerio/git/nc2parquet/examples/configs/*.json` -- for config file examples
- `/home/rogerio/git/nc2parquet/examples/postprocessing/*.json` -- for postprocessing config examples
- `/home/rogerio/git/nc2parquet/examples/data/` -- for fixture file names

### Patterns to Follow

- Tutorial structure: Prerequisites -> Introduction (1-2 sentences) -> Numbered Steps -> Summary -> What's Next.
- Each step should have a brief explanation of what it does and why, followed by the command, followed by expected output.
- Use the `pres_temp_4D.nc` fixture for filter tutorials because it has multiple dimensions (time, level, latitude, longitude) and multiple variables (temperature, pressure).
- Use the `simple_xy.nc` fixture for the basic tutorial because it is the simplest file structure.

### Pitfalls to Avoid

- Do not use hardcoded absolute paths in tutorial commands -- use relative paths from the project root (e.g., `examples/data/simple_xy.nc`).
- Do not include S3 commands that require AWS credentials -- these cannot be reproduced by most readers.
- Do not fabricate expected output -- verify commands produce the described output or describe output generically (e.g., "You should see a summary of the file structure including dimensions and variables").
- Do not forget the `--formula` flag uses 3 colon-delimited parts: `--formula "target:formula:sources"`, not the simplified form shown in the README.
- Do not assume `pres_temp_4D.nc` has a time coordinate variable -- it does NOT (it has a time dimension but no time coordinate variable), so do not use 3D point filters with it.
- Do not confuse `-n` (single variable, `--variable`) with `-N` (multi-variable, `--variables`).

## Testing Requirements

### Unit Tests

Not applicable (documentation).

### Integration Tests

Not applicable (documentation).

### Manual Verification

- Follow each tutorial end-to-end on a fresh checkout to verify all commands work.
- Verify all file paths in tutorials exist in the repository.
- Verify CLI flag names match the current `src/cli.rs` definition.

## Dependencies

- **Blocked By**: ticket-024 (README rewritten with links to tutorials)
- **Blocks**: None

## Effort Estimate

**Points**: 3
**Confidence**: High
