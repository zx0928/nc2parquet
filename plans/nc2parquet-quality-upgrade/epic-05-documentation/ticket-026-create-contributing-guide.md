# ticket-026 Create CONTRIBUTING Guide

> **[OUTLINE]** This ticket requires refinement before execution.
> It will be refined with learnings from earlier epics.

## Objective

Create a CONTRIBUTING.md that enables new contributors to set up the development environment, understand the project structure, build and test the project, and submit pull requests following project conventions. This is essential for community adoption because potential contributors need clear onboarding documentation.

## Anticipated Scope

- **Files likely to be modified**:
  - `/home/rogerio/git/nc2parquet/CONTRIBUTING.md` -- CREATE: development guide
- **Key decisions needed**:
  - Whether to include NetCDF system library installation instructions for each platform (Linux, macOS, Windows)
  - Level of detail for testing instructions (just `cargo test` or explain test categories?)
  - Whether to define a code of conduct or link to an existing one
  - PR review process: how many approvals, what checks must pass?
- **Open questions**:
  - What are the minimum system requirements (Rust version, NetCDF library version)?
  - Should we document the module architecture to help contributors navigate the code?
  - Are there any platform-specific build issues contributors should know about?

## Dependencies

- **Blocked By**: ticket-009 (module structure finalized so architecture section is accurate)
- **Blocks**: None

## Effort Estimate

**Points**: 2
**Confidence**: Low (will be re-estimated during refinement)
