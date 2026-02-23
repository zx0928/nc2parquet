use crate::errors::Nc2ParquetError;
use crate::storage::{StorageBackend, StorageFactory};
use log::debug;
use polars::prelude::*;
use std::io::Cursor;

pub(crate) fn write_dataframe_to_parquet(
    df: &DataFrame,
    output_path: &str,
) -> Result<(), Nc2ParquetError> {
    debug!("Writing DataFrame to parquet file: {}\n", output_path);
    debug!("DataFrame shape: {:?}", df.shape());
    debug!("DataFrame schema:\n{:?}", df.schema());
    debug!("First few rows:\n{}", df.head(Some(5)));

    if let Some(parent) = std::path::Path::new(output_path).parent() {
        std::fs::create_dir_all(parent)?;
    }

    let file = std::fs::File::create(output_path)?;
    let writer = ParquetWriter::new(file);
    let mut df_clone = df.clone();

    writer.finish(&mut df_clone)?;
    debug!("Successfully wrote parquet file: {}", output_path);

    Ok(())
}

pub(crate) async fn write_dataframe_to_parquet_async(
    df: &DataFrame,
    output_path: &str,
) -> Result<(), Nc2ParquetError> {
    debug!("Writing DataFrame to parquet file: {}\n", output_path);
    debug!("DataFrame shape: {:?}", df.shape());
    debug!("DataFrame schema:\n{:?}", df.schema());
    debug!("First few rows:\n{}", df.head(Some(5)));

    let parquet_bytes = dataframe_to_parquet_bytes(df)?;
    let storage = StorageFactory::from_path(output_path).await?;
    storage.write(output_path, &parquet_bytes).await?;

    debug!("Successfully wrote parquet file: {}", output_path);
    Ok(())
}

fn dataframe_to_parquet_bytes(df: &DataFrame) -> Result<Vec<u8>, Nc2ParquetError> {
    let mut buffer = Vec::new();
    let cursor = Cursor::new(&mut buffer);
    let writer = ParquetWriter::new(cursor);
    let mut df_clone = df.clone();

    writer.finish(&mut df_clone)?;
    Ok(buffer)
}
