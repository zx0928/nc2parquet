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

/// Write a Polars DataFrame into a DuckDB database via duckdb CLI.
///
/// Writes the DataFrame to a temporary Parquet file, then invokes the
/// `duckdb` CLI to create the database and import the data.
pub(crate) fn write_dataframe_to_duckdb(
    df: &mut DataFrame,
    db_path: &str,
    table_name: &str,
) -> Result<(), Nc2ParquetError> {
    debug!(
        "Writing DataFrame to DuckDB: db={}, table={}",
        db_path, table_name
    );
    debug!("DataFrame shape: {:?}", df.shape());
    debug!("DataFrame schema:\n{:?}", df.schema());

    // Step 1: write DataFrame to a temporary Parquet file
    let tmp_dir = tempfile::Builder::new()
        .prefix("nc2duckdb_")
        .tempdir()?;
    let parquet_path = tmp_dir.path().join("temp.parquet");
    let parquet_path_str = parquet_path.to_string_lossy().to_string();

    {
        let file = std::fs::File::create(&parquet_path)?;
        let writer = ParquetWriter::new(file);
        writer.finish(df)?;
    }

    // Step 2: create parent directory for the duckdb file if needed
    if let Some(parent) = std::path::Path::new(db_path).parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
        }
    }

    // Step 3: invoke duckdb CLI to import the parquet into a new database
    let sql = format!(
        "CREATE TABLE \"{}\" AS SELECT * FROM read_parquet('{}');",
        table_name, parquet_path_str
    );
    let output = std::process::Command::new("duckdb")
        .arg(db_path)
        .arg(&sql)
        .output()
        .map_err(|e| {
            Nc2ParquetError::Config(format!(
                "Failed to run duckdb CLI (is it installed?): {}",
                e
            ))
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        return Err(Nc2ParquetError::Config(format!(
            "duckdb CLI failed:\nstdout: {}\nstderr: {}",
            stdout, stderr
        )));
    }

    debug!(
        "Successfully wrote to DuckDB database: {}, table: {}",
        db_path, table_name
    );

    // Step 4: temp dir is automatically cleaned up when tmp_dir drops
    Ok(())
}
