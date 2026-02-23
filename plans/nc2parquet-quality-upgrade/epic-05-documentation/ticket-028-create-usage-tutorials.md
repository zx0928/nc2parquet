# ticket-028 Create Usage Tutorials for Common Workflows

> **[OUTLINE]** This ticket requires refinement before execution.
> It will be refined with learnings from earlier epics.

## Objective

Create step-by-step usage tutorials for the most common nc2parquet workflows: basic single-file conversion, batch processing with glob patterns, filtered extraction with postprocessing, S3 input/output, and configuration file usage. Tutorials bridge the gap between the reference documentation (rustdoc, README) and real-world usage patterns.

## Anticipated Scope

- **Files likely to be modified**:
  - `/home/rogerio/git/nc2parquet/docs/tutorials/basic-conversion.md` -- CREATE: simplest possible conversion
  - `/home/rogerio/git/nc2parquet/docs/tutorials/filtered-extraction.md` -- CREATE: filters + postprocessors
  - `/home/rogerio/git/nc2parquet/docs/tutorials/batch-processing.md` -- CREATE: glob patterns and batch workflows
  - `/home/rogerio/git/nc2parquet/docs/tutorials/s3-workflow.md` -- CREATE: S3 input/output configuration
  - `/home/rogerio/git/nc2parquet/docs/tutorials/config-files.md` -- CREATE: JSON/YAML config file usage
  - `/home/rogerio/git/nc2parquet/docs/tutorials/README.md` -- CREATE: tutorial index
- **Key decisions needed**:
  - Whether to include sample NetCDF files in the repo for tutorials (size concern) or reference public datasets
  - Whether tutorials should be CLI-focused, library-focused, or both
  - Level of explanation: brief commands or detailed walk-throughs with expected output?
  - Whether to include "troubleshooting" sections in each tutorial
- **Open questions**:
  - What public NetCDF datasets are stable enough to reference in tutorials without breaking links?
  - Should tutorials reference the examples/ directory that already exists in the repo?
  - Are there user-reported pain points from the current documentation that tutorials should address?

## Dependencies

- **Blocked By**: ticket-020 (batch processing feature), ticket-022 (output options), ticket-024 (README links to tutorials)
- **Blocks**: None

## Effort Estimate

**Points**: 3
**Confidence**: Low (will be re-estimated during refinement)
