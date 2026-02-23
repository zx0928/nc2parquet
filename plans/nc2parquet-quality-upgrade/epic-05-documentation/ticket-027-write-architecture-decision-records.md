# ticket-027 Write Architecture Decision Records

## Context

### Background

During the quality upgrade (epics 01-04), several significant architectural decisions were made that should be documented for future contributors. These decisions include the error handling strategy (thiserror with a unified `Nc2ParquetError` enum), the module structure (binary/library split with `pub(crate)` internals), the storage abstraction (`StorageBackend` trait with boxed error variant), the extraction pipeline design (CombinationBuffer, eager drop, scoped lifetimes), and the post-processing framework (trait-based pipeline with lazy batching). Without ADRs, future contributors may re-litigate these decisions or make incompatible changes.

### Relation to Epic

This is the fourth ticket in Epic 05. ADRs are linked from the CONTRIBUTING guide (ticket-026) and provide the architectural rationale that helps contributors understand not just what the code does, but why it is structured the way it is. The ADRs document decisions that are stable and should not change without deliberate re-evaluation.

### Current State

- **Directory**: `/home/rogerio/git/nc2parquet/docs/` -- does not exist; needs to be created.
- **Error handling** (from `src/errors.rs`): `Nc2ParquetError` has 11 variants using thiserror, with `StorageError` boxed (`Box<StorageError>`) to reduce enum size. Manual `From<StorageError>` implementation because `#[from]` does not support boxed variants.
- **Module structure** (from `src/lib.rs` and learnings): `extract` and `output` are `pub(crate)`; `handlers/` declared in `main.rs` not `lib.rs`; binary-library split keeps CLI deps (indicatif, anyhow) out of the library.
- **Storage abstraction** (from `src/storage.rs`): `StorageBackend` async trait with `read`, `write`, `exists` methods; `StorageFactory::from_path` dispatches to local or S3 based on URI prefix; `StorageError` has 9 variants including 3 AWS SDK error types.
- **Extraction pipeline** (from learnings): `CombinationBuffer` for zero-allocation dimension iteration, eager drop via inner scopes, scoped NetCDF file lifetimes, Cartesian vs explicit dispatch.
- **Post-processing** (from `src/postprocess.rs`): `PostProcessor` trait, `ProcessingPipeline` executor, 5 `ProcessorConfig` variants with serde tagged enum (`#[serde(tag = "type", rename_all = "snake_case")]`), lazy batching for `UnitConverter`.

## Specification

### Requirements

1. **Create directory** `/home/rogerio/git/nc2parquet/docs/adr/`.
2. **Create ADR index** at `/home/rogerio/git/nc2parquet/docs/adr/README.md` listing all ADRs.
3. **Create 4 ADRs** using the MADR (Markdown Any Decision Records) template:
   - **ADR-0001: Error Handling Strategy** -- Documents the choice of thiserror + unified `Nc2ParquetError` enum, why anyhow was rejected for the library, the boxed `StorageError` pattern, and the manual `From` implementation.
   - **ADR-0002: Module Structure and Visibility** -- Documents the binary/library split, `pub(crate)` for internal modules, handler extraction pattern, test organization in `src/tests/`.
   - **ADR-0003: Storage Abstraction** -- Documents the `StorageBackend` async trait, S3 vs local dispatch via URI prefix, temporary file download pattern for S3 NetCDF processing, and the `StorageFactory` pattern.
   - **ADR-0004: Post-Processing Pipeline Design** -- Documents the `PostProcessor` trait, serde-tagged `ProcessorConfig` enum, sequential pipeline execution, lazy batching for unit converters, and the formula parser's recursive descent approach.
4. **Each ADR must contain** these sections: Title, Status (Accepted), Context, Decision, Consequences (positive and negative), Alternatives Considered.
5. **Status** for all ADRs: "Accepted" (these are implemented decisions).

### Inputs/Props

- Error handling design from `src/errors.rs` and learnings.
- Module structure from `src/lib.rs`, `src/main.rs`, and learnings.
- Storage abstraction from `src/storage.rs`.
- Post-processing design from `src/postprocess.rs` and learnings (especially the lazy batching and formula parser patterns).

### Outputs/Behavior

- `/home/rogerio/git/nc2parquet/docs/adr/README.md` -- ADR index
- `/home/rogerio/git/nc2parquet/docs/adr/0001-error-handling-strategy.md`
- `/home/rogerio/git/nc2parquet/docs/adr/0002-module-structure-and-visibility.md`
- `/home/rogerio/git/nc2parquet/docs/adr/0003-storage-abstraction.md`
- `/home/rogerio/git/nc2parquet/docs/adr/0004-post-processing-pipeline-design.md`

### Error Handling

Not applicable (documentation files).

## Acceptance Criteria

- [ ] Given the `docs/adr/` directory, when listing files, then it contains `README.md`, `0001-error-handling-strategy.md`, `0002-module-structure-and-visibility.md`, `0003-storage-abstraction.md`, `0004-post-processing-pipeline-design.md`
- [ ] Given `docs/adr/README.md`, when inspecting the file, then it contains a table listing all 4 ADRs with their title, status, and date
- [ ] Given each ADR file, when inspecting it, then it contains at minimum these sections: Title, Status, Context, Decision, Consequences
- [ ] Given ADR-0001, when inspecting it, then it mentions thiserror, `Nc2ParquetError`, the boxed `StorageError` pattern, and why anyhow was not used for the library
- [ ] Given ADR-0002, when inspecting it, then it mentions the binary/library split, `pub(crate)` visibility, and the handler extraction from `main.rs`
- [ ] Given ADR-0003, when inspecting it, then it mentions the `StorageBackend` trait, S3 URI dispatch, and the temporary file download pattern
- [ ] Given ADR-0004, when inspecting it, then it mentions the `PostProcessor` trait, serde-tagged enum, pipeline execution order, and the formula parser approach
- [ ] Given each ADR file, when inspecting it, then it contains an "Alternatives Considered" section listing at least one rejected alternative

## Implementation Guide

### Suggested Approach

1. Create the `docs/adr/` directory.
2. Start with the ADR index (`README.md`) containing a table with columns: Number, Title, Status, Date.
3. Write each ADR using the MADR template. For each:
   a. **Context**: Describe the problem that needed solving (e.g., "The library needed a consistent error handling strategy...").
   b. **Decision**: State what was decided and the key details of the implementation.
   c. **Consequences**: List both positive outcomes (e.g., "Single error type simplifies the public API") and negative trade-offs (e.g., "Adding new error variants requires touching the enum").
   d. **Alternatives Considered**: List what else was considered and why it was rejected.
4. For ADR-0001, reference the specific error variants and the boxing pattern for `StorageError`.
5. For ADR-0002, reference the `src/lib.rs` module declarations and the `src/handlers/` convention.
6. For ADR-0003, reference the `StorageBackend` trait definition and the `StorageFactory::from_path` dispatch.
7. For ADR-0004, reference the `ProcessorConfig` serde tag convention, the `PostProcessor` trait, and the `ProcessingPipeline::execute` batching logic.

### Key Files to Create

- `/home/rogerio/git/nc2parquet/docs/adr/README.md`
- `/home/rogerio/git/nc2parquet/docs/adr/0001-error-handling-strategy.md`
- `/home/rogerio/git/nc2parquet/docs/adr/0002-module-structure-and-visibility.md`
- `/home/rogerio/git/nc2parquet/docs/adr/0003-storage-abstraction.md`
- `/home/rogerio/git/nc2parquet/docs/adr/0004-post-processing-pipeline-design.md`

### Patterns to Follow

- MADR template: https://adr.github.io/madr/ -- use the basic template with Status, Context, Decision, Consequences, Alternatives.
- Each ADR should be self-contained and readable without external context.
- Use code snippets from the actual codebase to illustrate decisions (e.g., show the `Nc2ParquetError` enum definition).
- Keep each ADR to 50-100 lines (concise, focused on the decision and its rationale).

### Pitfalls to Avoid

- Do not write ADRs as implementation documentation -- they document decisions and their rationale, not how the code works in detail.
- Do not include decisions that are trivial or obvious (e.g., "We chose Rust because the project is in Rust").
- Do not forget the "Alternatives Considered" section -- this is the most valuable part for future contributors because it explains what was rejected and why.
- Do not reference ticket numbers from this plan in the ADRs -- ADRs should be standalone and outlive the plan.
- Do not include code that may change frequently (e.g., exact line numbers); reference module names and type names instead.

## Testing Requirements

### Unit Tests

Not applicable (documentation).

### Integration Tests

Not applicable (documentation).

### Manual Verification

- Verify all ADR files are valid markdown.
- Verify the ADR index in README.md lists all 4 ADRs with correct filenames.
- Verify code references in ADRs (type names, module names) match the current codebase.

## Dependencies

- **Blocked By**: ticket-010 (error handling decisions made -- already done), ticket-009 (module structure finalized -- already done)
- **Blocks**: None

## Effort Estimate

**Points**: 3
**Confidence**: High
