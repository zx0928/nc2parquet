# nc2parquet Production Quality Upgrade Plan

## Overview

This progressive master plan transforms nc2parquet from a functional v0.1.1 prototype into a production-quality Rust library and CLI tool suitable for wide community adoption in weather/climate data pipelines. The work is organized into 6 epics with 33 tickets total, executed in dependency order.

## Tech Stack

- **Language**: Rust (edition 2024)
- **Core Dependencies**: netcdf 0.11, polars 0.51, aws-sdk-s3 1.106, tokio 1.x, clap 4.4
- **Test Dependencies**: proptest (new), criterion (new), assert_cmd (new), predicates (new)
- **CI**: GitHub Actions, cargo-tarpaulin, cargo-audit, cargo-dist

## Progressive Planning

This plan uses **progressive planning**. Epics 1-2 have fully detailed tickets ready for implementation. Epics 3-6 have outline tickets that will be refined with learnings from earlier epics before execution.

## Epic Summary

| Epic    | Name                              | Tickets | Detail Level | Duration  |
| ------- | --------------------------------- | ------- | ------------ | --------- |
| epic-01 | Testing Infrastructure & Coverage | 8       | Detailed     | 2-3 weeks |
| epic-02 | Code Quality & Refactoring        | 5       | Detailed     | 2-3 weeks |
| epic-03 | Performance Optimization          | 5       | Refined      | 2-3 weeks |
| epic-04 | Feature Completeness              | 5       | Refined      | 2-3 weeks |
| epic-05 | Documentation & Community         | 5       | Refined      | 1-2 weeks |
| epic-06 | CI/CD & Release Quality           | 5       | Refined      | 1-2 weeks |

## Progress Tracking

| Ticket     | Title                                                  | Epic    | Status    | Detail Level |
| ---------- | ------------------------------------------------------ | ------- | --------- | ------------ |
| ticket-001 | Add Test Dependencies and Create Test Helpers          | epic-01 | completed | Detailed     |
| ticket-002 | Reorganize Tests into Module-Specific Files            | epic-01 | completed | Detailed     |
| ticket-003 | Add Unit Tests for Filter Edge Cases                   | epic-01 | completed | Detailed     |
| ticket-004 | Add Unit Tests for Extract Edge Cases                  | epic-01 | completed | Detailed     |
| ticket-005 | Add Unit Tests for PostProcess Edge Cases              | epic-01 | completed | Detailed     |
| ticket-006 | Add Unit Tests for Output and Storage                  | epic-01 | completed | Detailed     |
| ticket-007 | Add Property-Based Tests with Proptest                 | epic-01 | completed | Detailed     |
| ticket-008 | Add Integration Tests for Error Paths                  | epic-01 | completed | Detailed     |
| ticket-009 | Extract Command Handlers from main.rs                  | epic-02 | completed | Detailed     |
| ticket-010 | Improve Library Error Types with thiserror             | epic-02 | completed | Detailed     |
| ticket-011 | Audit and Tighten Visibility Modifiers                 | epic-02 | completed | Detailed     |
| ticket-012 | Add Exhaustive Rustdoc with Examples                   | epic-02 | completed | Detailed     |
| ticket-013 | Remove Dead Code and Fix Remaining Clippy Warnings     | epic-02 | completed | Detailed     |
| ticket-014 | Add Criterion Benchmark Suite                          | epic-03 | completed | Refined      |
| ticket-015 | Implement Chunked NetCDF Reading                       | epic-03 | completed | Refined      |
| ticket-016 | Reduce Allocation Overhead in Extraction Pipeline      | epic-03 | completed | Refined      |
| ticket-017 | Parallelize Independent PostProcessor Executions       | epic-03 | completed | Refined      |
| ticket-018 | Profile and Optimize Peak Memory Usage                 | epic-03 | completed | Refined      |
| ticket-019 | Extend UnitConverter with Meteorological Unit Families | epic-04 | completed | Refined      |
| ticket-020 | Add Glob Pattern Support for Batch File Processing     | epic-04 | completed | Refined      |
| ticket-021 | Support Multi-Variable Extraction in Single Pass       | epic-04 | completed | Refined      |
| ticket-022 | Add Parquet Output Configuration Options               | epic-04 | completed | Refined      |
| ticket-023 | Extend Formula Parser with Mathematical Functions      | epic-04 | completed | Refined      |
| ticket-024 | Rewrite README with Professional Structure             | epic-05 | completed | Refined      |
| ticket-025 | Create CHANGELOG Following Keep a Changelog            | epic-05 | completed | Refined      |
| ticket-026 | Create CONTRIBUTING Guide                              | epic-05 | completed | Refined      |
| ticket-027 | Write Architecture Decision Records                    | epic-05 | completed | Refined      |
| ticket-028 | Create Usage Tutorials for Common Workflows            | epic-05 | completed | Refined      |
| ticket-029 | Add Code Coverage Reporting with Codecov               | epic-06 | completed | Refined      |
| ticket-030 | Add Benchmark Regression Detection in CI               | epic-06 | completed | Refined      |
| ticket-031 | Automate Release Process with cargo-dist               | epic-06 | completed | Refined      |
| ticket-032 | Add Cross-Compilation CI Matrix                        | epic-06 | completed | Refined      |
| ticket-033 | Add Integration Test CI Job with NetCDF Fixtures       | epic-06 | completed | Refined      |
