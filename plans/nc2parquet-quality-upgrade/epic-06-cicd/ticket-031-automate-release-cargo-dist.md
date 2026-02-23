# ticket-031 Automate Release Process with cargo-dist

> **[OUTLINE]** This ticket requires refinement before execution.
> It will be refined with learnings from earlier epics.

## Objective

Enhance the existing cargo-dist configuration to provide a fully automated release process: on git tag push, build release binaries for all targets, create a GitHub Release with auto-generated release notes, and optionally publish to crates.io. The project already has a `dist-workspace.toml` with basic cargo-dist configuration targeting Linux, but it needs expansion for more targets and automation.

## Anticipated Scope

- **Files likely to be modified**:
  - `/home/rogerio/git/nc2parquet/dist-workspace.toml` -- expand target list, configure release notes, crates.io publishing
  - `/home/rogerio/git/nc2parquet/.github/workflows/release.yml` -- CREATE or update: release workflow triggered by tags
  - `/home/rogerio/git/nc2parquet/Cargo.toml` -- ensure metadata fields (description, license, repository, homepage) are complete for crates.io
- **Key decisions needed**:
  - Whether to publish to crates.io automatically or require manual confirmation
  - Release note generation: auto from CHANGELOG.md, auto from git commits, or manual?
  - Tag format: `v0.2.0` or `0.2.0`?
  - Whether to include shell completion files and man pages in release artifacts
- **Open questions**:
  - Does the existing dist-workspace.toml need a full `cargo dist init` regeneration or just additions?
  - Should release artifacts include a standalone binary and a tarball/zip with docs?
  - Is there a signing requirement for release artifacts?

## Dependencies

- **Blocked By**: ticket-025 (CHANGELOG exists for release notes), ticket-032 (cross-compilation targets defined)
- **Blocks**: None

## Effort Estimate

**Points**: 3
**Confidence**: Low (will be re-estimated during refinement)
