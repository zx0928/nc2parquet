# ticket-022 Add Parquet Output Configuration Options

## Context

### Background

The Parquet output module currently uses Polars defaults for all writer settings. `write_dataframe_to_parquet` creates a bare `ParquetWriter::new(file)` and calls `.finish(df)` with no configuration. Users writing output for specific downstream consumers (Spark, DuckDB, Athena) often need control over compression codec, compression level, row group size, and statistics. The current code forces users to re-process their Parquet files with external tools to achieve optimal settings for their use case.

### Relation to Epic

This ticket adds configurable output options to the existing Parquet writing functions. It is independent of the other Epic 04 tickets -- it does not affect extraction, unit conversion, or formula parsing. The changes are additive: existing configs without output options continue to work with Polars defaults.

### Current State

- `/home/rogerio/git/nc2parquet/src/output.rs` (53 lines total):
  - `write_dataframe_to_parquet(df, output_path)` (line 7-27): Creates `ParquetWriter::new(file)`, calls `writer.finish(df)` with no configuration
  - `write_dataframe_to_parquet_async(df, output_path)` (line 29-44): Calls `dataframe_to_parquet_bytes(df)` then writes via storage backend
  - `dataframe_to_parquet_bytes(df)` (line 46-53): Creates `ParquetWriter::new(cursor)`, calls `writer.finish(df)` with no configuration
- `/home/rogerio/git/nc2parquet/src/input.rs`: `JobConfig` has no output configuration field
- `/home/rogerio/git/nc2parquet/src/lib.rs`: `process_netcdf_job` calls `write_dataframe_to_parquet(&mut df, &config.parquet_key)` with no output config
- Polars 0.51 `ParquetWriter` supports `.with_compression(ParquetCompression::...)`, `.with_row_group_size(Option<usize>)`, `.with_data_page_size(Option<usize>)`, `.with_statistics(StatisticsOptions)`
- The `polars` dependency includes the `"parquet"` feature which enables all Parquet writer options

## Specification

### Requirements

1. Add an `OutputConfig` struct in `src/input.rs`:

   ```rust
   #[derive(Debug, Clone, Serialize, Deserialize)]
   pub struct OutputConfig {
       #[serde(default = "default_compression")]
       pub compression: CompressionCodec,
       #[serde(skip_serializing_if = "Option::is_none")]
       pub compression_level: Option<u32>,
       #[serde(skip_serializing_if = "Option::is_none")]
       pub row_group_size: Option<usize>,
       #[serde(skip_serializing_if = "Option::is_none")]
       pub data_page_size: Option<usize>,
       #[serde(default = "default_statistics")]
       pub statistics: bool,
   }
   ```

2. Add a `CompressionCodec` enum:

   ```rust
   #[derive(Debug, Clone, Serialize, Deserialize)]
   #[serde(rename_all = "lowercase")]
   pub enum CompressionCodec {
       Uncompressed,
       Snappy,
       Gzip,
       Lz4,
       Zstd,
   }
   ```

3. Add `output: Option<OutputConfig>` field to `JobConfig` with `#[serde(skip_serializing_if = "Option::is_none")]`.

4. Thread the `OutputConfig` through the write functions:
   - `write_dataframe_to_parquet(df, output_path, output_config: Option<&OutputConfig>)`
   - `write_dataframe_to_parquet_async(df, output_path, output_config: Option<&OutputConfig>)`
   - `dataframe_to_parquet_bytes(df, output_config: Option<&OutputConfig>)`

5. Apply configuration to `ParquetWriter`:

   ```rust
   let mut writer = ParquetWriter::new(file);
   if let Some(config) = output_config {
       writer = writer.with_compression(config.to_polars_compression());
       if let Some(rg) = config.row_group_size {
           writer = writer.with_row_group_size(Some(rg));
       }
       if let Some(dp) = config.data_page_size {
           writer = writer.with_data_page_size(Some(dp));
       }
       if config.statistics {
           writer = writer.with_statistics(StatisticsOptions::full());
       }
   }
   writer.finish(df)?;
   ```

6. Add CLI flags to `Commands::Convert`:
   - `--compression <codec>` (default: snappy)
   - `--compression-level <u32>` (only for zstd/gzip, optional)
   - `--row-group-size <usize>` (optional)
   - `--statistics` (flag, enables full statistics)

7. Default values: compression = Snappy, statistics = true, row_group_size and data_page_size = None (Polars defaults).

### Inputs/Props

- `OutputConfig` struct -- all fields optional or defaulted, fully backward compatible
- `CompressionCodec` enum -- maps to Polars `ParquetCompression` variants
- CLI flags for each option

### Outputs/Behavior

- When `output` is `None` in `JobConfig`, behavior is identical to current (Polars defaults)
- When `output` is `Some(config)`, the specified settings are applied to the `ParquetWriter`
- The generated Parquet file metadata reflects the chosen compression codec
- Log message at DEBUG: `"Writing Parquet with compression={:?}, row_group_size={:?}"`

### Error Handling

- Invalid compression level for codec: `Nc2ParquetError::Config(format!("Compression level {} not valid for {}", level, codec))`
  - Zstd accepts levels 1-22
  - Gzip accepts levels 1-9
  - Snappy, Lz4, Uncompressed do not accept levels
- Row group size of 0: `Nc2ParquetError::Config("Row group size must be positive")`

## Acceptance Criteria

- [ ] Given no `output` field in JobConfig, when `process_netcdf_job` is called, then output uses Polars defaults (backward compatible)
- [ ] Given `output: { compression: "zstd" }` in config, when the Parquet file is written, then the file uses Zstd compression
- [ ] Given `output: { compression: "snappy", row_group_size: 5000 }`, when a large DataFrame is written, then the Parquet file has row groups of approximately 5000 rows
- [ ] Given `output: { compression: "gzip", compression_level: 6 }`, when the file is written, then Gzip level 6 is used
- [ ] Given `output: { statistics: false }`, when the file is written, then column statistics are omitted
- [ ] Given `--compression zstd` on the CLI, when convert is executed, then the output file uses Zstd compression
- [ ] Given `compression_level: 30` with `compression: "zstd"`, when config is validated, then an error is returned (Zstd max is 22)
- [ ] Given `row_group_size: 0`, when config is validated, then an error is returned
- [ ] Given both JSON config output options and CLI `--compression` override, when executed, then CLI takes precedence

## Implementation Guide

### Suggested Approach

1. **Add data structures**: In `/home/rogerio/git/nc2parquet/src/input.rs`, add `CompressionCodec` enum and `OutputConfig` struct. Add `pub output: Option<OutputConfig>` to `JobConfig`.

2. **Add conversion method**: On `OutputConfig`, add:

   ```rust
   impl OutputConfig {
       pub fn to_polars_compression(&self) -> ParquetCompression {
           match self.compression {
               CompressionCodec::Uncompressed => ParquetCompression::Uncompressed,
               CompressionCodec::Snappy => ParquetCompression::Snappy,
               CompressionCodec::Gzip => ParquetCompression::Gzip(
                   self.compression_level.map(|l| GzipLevel::try_new(l as u8).unwrap_or_default())
               ),
               CompressionCodec::Lz4 => ParquetCompression::Lz4Raw,
               CompressionCodec::Zstd => ParquetCompression::Zstd(
                   self.compression_level.map(|l| ZstdLevel::try_new(l as i32).unwrap_or_default())
               ),
           }
       }

       pub fn validate(&self) -> Result<(), Nc2ParquetError> {
           if let Some(level) = self.compression_level {
               match self.compression {
                   CompressionCodec::Zstd if level > 22 => return Err(...),
                   CompressionCodec::Gzip if level > 9 => return Err(...),
                   CompressionCodec::Snappy | CompressionCodec::Lz4 | CompressionCodec::Uncompressed => {
                       return Err(Nc2ParquetError::Config(
                           format!("{:?} does not accept compression levels", self.compression)
                       ));
                   }
                   _ => {}
               }
           }
           if let Some(rg) = self.row_group_size {
               if rg == 0 { return Err(...); }
           }
           Ok(())
       }
   }
   ```

3. **Update write functions**: Change signatures in `/home/rogerio/git/nc2parquet/src/output.rs` to accept `output_config: Option<&OutputConfig>`. Apply configuration to `ParquetWriter` before calling `.finish()`.

4. **Update callers**: In `process_netcdf_job` and `process_netcdf_job_async`, pass `config.output.as_ref()` to the write functions.

5. **Add CLI flags**: In `Commands::Convert`, add:

   ```rust
   #[arg(long, value_enum, default_value_t = None)]
   compression: Option<CompressionCodec>,
   #[arg(long)]
   compression_level: Option<u32>,
   #[arg(long)]
   row_group_size: Option<usize>,
   #[arg(long)]
   statistics: bool,
   ```

   In the handler, build `OutputConfig` from CLI args and set on `config.output`.

6. **Validate**: Call `config.output.validate()` in the handler before processing, similar to how `validate_config` is called now.

### Key Files to Modify

- `/home/rogerio/git/nc2parquet/src/input.rs` -- Add `CompressionCodec`, `OutputConfig`, add `output` field to `JobConfig`
- `/home/rogerio/git/nc2parquet/src/output.rs` -- Update all three write functions to accept and apply `OutputConfig`
- `/home/rogerio/git/nc2parquet/src/lib.rs` -- Pass `config.output.as_ref()` to write functions
- `/home/rogerio/git/nc2parquet/src/cli.rs` -- Add CLI flags for compression, row-group-size, statistics
- `/home/rogerio/git/nc2parquet/src/handlers/convert.rs` -- Build `OutputConfig` from CLI args

### Patterns to Follow

- Follow the `#[serde(skip_serializing_if = "Option::is_none")]` pattern from `postprocessing` field on `JobConfig`
- Follow the `#[serde(rename_all = "lowercase")]` pattern from `ProcessorConfig`
- Follow the `validate_config` pattern in handlers: validate output config before processing

### Pitfalls to Avoid

- The `ParquetWriter` builder methods in Polars 0.51 return `Self`, so they can be chained. However, each `.with_*` call consumes `self` and returns a new writer, so you must reassign: `writer = writer.with_compression(...)`.
- `ParquetCompression::Gzip` and `ParquetCompression::Zstd` take `Option<GzipLevel>` / `Option<ZstdLevel>` respectively. When no compression level is specified, pass `None` to use the default level.
- The `Lz4` in Polars maps to `Lz4Raw` (not `Lz4Hc`). Use `ParquetCompression::Lz4Raw`.
- Check that `polars::prelude::ParquetCompression`, `GzipLevel`, `ZstdLevel`, and `StatisticsOptions` are available in polars 0.51 with the `"parquet"` feature. If `StatisticsOptions` is not available, use a boolean with `.with_statistics(bool)` instead.
- Do NOT change the default behavior. When `output` is `None`, the code must produce identical output to the current version. This means the write functions must handle `None` by creating a bare `ParquetWriter::new(file).finish(df)` with no configuration.

## Testing Requirements

### Unit Tests

Add a `parquet_output_config` module in `/home/rogerio/git/nc2parquet/src/tests/test_output.rs`:

1. **Default config**: Verify `OutputConfig::default()` has Snappy compression and statistics enabled
2. **Validation happy path**: Verify valid configs pass validation (Zstd level 3, Gzip level 6, Snappy with no level)
3. **Validation failures**: Verify Zstd level 25 fails, Gzip level 10 fails, Snappy with level fails, row_group_size 0 fails
4. **Polars compression mapping**: Verify `to_polars_compression()` returns correct `ParquetCompression` variants
5. **Serialization round-trip**: Verify `OutputConfig` serializes to JSON and deserializes back correctly

### Integration Tests

1. **Zstd output**: Write a DataFrame to Parquet with Zstd compression, read it back, verify data integrity
2. **Custom row group size**: Write with `row_group_size: Some(10)` on a 100-row DataFrame, verify the output file is valid and can be read
3. **Backward compatibility**: Verify existing `JobConfig` JSON without `output` field deserializes correctly (output = None)

## Dependencies

- **Blocked By**: ticket-010 (error types -- completed), ticket-014 (benchmarks -- completed)
- **Blocks**: ticket-028 (usage tutorials will cover output configuration)

## Effort Estimate

**Points**: 2
**Confidence**: High
