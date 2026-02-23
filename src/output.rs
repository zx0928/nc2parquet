use crate::errors::Nc2ParquetError;
use crate::input::OutputConfig;
use crate::storage::{StorageBackend, StorageFactory};
use log::debug;
use polars::prelude::*;
use std::io::Cursor;

pub(crate) fn write_dataframe_to_parquet(
    df: &mut DataFrame,
    output_path: &str,
    output_config: Option<&OutputConfig>,
) -> Result<(), Nc2ParquetError> {
    debug!("Writing DataFrame to parquet file: {}\n", output_path);
    debug!("DataFrame shape: {:?}", df.shape());
    debug!("DataFrame schema:\n{:?}", df.schema());
    debug!("First few rows:\n{}", df.head(Some(5)));

    if let Some(parent) = std::path::Path::new(output_path).parent() {
        std::fs::create_dir_all(parent)?;
    }

    let file = std::fs::File::create(output_path)?;
    let writer = build_parquet_writer(file, output_config);

    writer.finish(df)?;
    debug!("Successfully wrote parquet file: {}", output_path);

    Ok(())
}

pub(crate) async fn write_dataframe_to_parquet_async(
    df: &mut DataFrame,
    output_path: &str,
    output_config: Option<&OutputConfig>,
) -> Result<(), Nc2ParquetError> {
    debug!("Writing DataFrame to parquet file: {}\n", output_path);
    debug!("DataFrame shape: {:?}", df.shape());
    debug!("DataFrame schema:\n{:?}", df.schema());
    debug!("First few rows:\n{}", df.head(Some(5)));

    let parquet_bytes = dataframe_to_parquet_bytes(df, output_config)?;
    let storage = StorageFactory::from_path(output_path).await?;
    storage.write(output_path, &parquet_bytes).await?;

    debug!("Successfully wrote parquet file: {}", output_path);
    Ok(())
}

fn dataframe_to_parquet_bytes(
    df: &mut DataFrame,
    output_config: Option<&OutputConfig>,
) -> Result<Vec<u8>, Nc2ParquetError> {
    let mut buffer = Vec::new();
    let cursor = Cursor::new(&mut buffer);
    let writer = build_parquet_writer(cursor, output_config);

    writer.finish(df)?;
    Ok(buffer)
}

/// Construct a [`ParquetWriter`] configured according to `output_config`.
///
/// When `output_config` is `None` the returned writer has no explicit
/// configuration applied (Polars defaults: Zstd, statistics enabled, no size
/// limits), preserving the identical behaviour that existed before this config
/// option was introduced.
fn build_parquet_writer<W: std::io::Write>(
    sink: W,
    output_config: Option<&OutputConfig>,
) -> ParquetWriter<W> {
    let mut writer = ParquetWriter::new(sink);

    if let Some(config) = output_config {
        writer = writer.with_compression(config.to_polars_compression());

        if let Some(rg) = config.row_group_size {
            writer = writer.with_row_group_size(Some(rg));
        }

        if let Some(dp) = config.data_page_size {
            writer = writer.with_data_page_size(Some(dp));
        }

        if !config.statistics {
            writer = writer.with_statistics(StatisticsOptions::empty());
        }
    }

    writer
}
