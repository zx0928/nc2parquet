use crate::postprocess::PostProcessError;
use crate::storage::StorageError;

/// Top-level error type for nc2parquet library operations.
#[derive(thiserror::Error, Debug)]
pub enum Nc2ParquetError {
    #[error("NetCDF error: {0}")]
    NetCdf(#[from] netcdf::Error),

    #[error("Variable '{0}' not found in NetCDF file")]
    VariableNotFound(String),

    #[error("Dimension '{0}' not found")]
    DimensionNotFound(String),

    #[error("Filter error: {0}")]
    Filter(String),

    #[error("Extraction error: {0}")]
    Extraction(String),

    #[error("Post-processing error: {0}")]
    PostProcess(#[from] PostProcessError),

    /// Boxed to reduce enum size (StorageError contains large S3 SDK error variants).
    #[error("Storage error: {0}")]
    Storage(Box<StorageError>),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Polars error: {0}")]
    Polars(#[from] polars::prelude::PolarsError),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("Unsupported dimensionality: {0} dimensions")]
    UnsupportedDimensionality(usize),
}

impl From<StorageError> for Nc2ParquetError {
    fn from(e: StorageError) -> Self {
        Nc2ParquetError::Storage(Box::new(e))
    }
}
