use polars::prelude::*;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

pub fn get_test_data_path(filename: &str) -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("examples");
    path.push("data");
    path.push(filename);
    path
}

pub fn create_temp_output_dir() -> TempDir {
    tempfile::tempdir().expect("Failed to create temp directory for test output")
}

pub fn create_simple_test_dataframe() -> DataFrame {
    df! {
        "temperature" => [273.15, 283.15, 293.15, 303.15],
        "pressure" => [1013.25, 1012.0, 1010.5, 1009.0],
        "humidity" => [60.0, 65.0, 70.0, 75.0],
        "time_offset" => [0.0, 1.0, 2.0, 3.0],
    }
    .expect("Failed to create simple test DataFrame")
}

pub fn create_weather_test_dataframe() -> DataFrame {
    df! {
        "station" => ["A", "A", "A", "B", "B", "B"],
        "latitude" => [40.7, 40.7, 40.7, 34.0, 34.0, 34.0],
        "longitude" => [-74.0, -74.0, -74.0, -118.2, -118.2, -118.2],
        "temperature" => [280.0, 282.0, 281.0, 295.0, 296.0, 294.0],
        "pressure" => [1013.0, 1012.0, 1013.5, 1010.0, 1009.5, 1011.0],
    }
    .expect("Failed to create weather test DataFrame")
}

pub fn assert_parquet_file_valid(path: &Path) {
    assert!(
        path.exists(),
        "Parquet file does not exist: {}",
        path.display()
    );
    let metadata = std::fs::metadata(path)
        .unwrap_or_else(|e| panic!("Failed to read metadata for {}: {}", path.display(), e));
    assert!(
        metadata.len() > 0,
        "Parquet file is empty: {}",
        path.display()
    );

    let file = std::fs::File::open(path)
        .unwrap_or_else(|e| panic!("Failed to open {}: {}", path.display(), e));
    let _df = ParquetReader::new(file)
        .finish()
        .unwrap_or_else(|e| panic!("Failed to read parquet {}: {}", path.display(), e));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_test_data_path() {
        let path = get_test_data_path("simple_xy.nc");
        assert!(path.exists(), "Test data file should exist");
    }

    #[test]
    fn test_create_simple_test_dataframe() {
        let df = create_simple_test_dataframe();
        assert_eq!(df.height(), 4);
        assert_eq!(df.width(), 4);
        assert!(df.column("temperature").is_ok());
        assert!(df.column("pressure").is_ok());
        assert!(df.column("humidity").is_ok());
        assert!(df.column("time_offset").is_ok());
    }

    #[test]
    fn test_create_weather_test_dataframe() {
        let df = create_weather_test_dataframe();
        assert_eq!(df.height(), 6);
        assert_eq!(df.width(), 5);
        assert!(df.column("station").is_ok());
        assert!(df.column("latitude").is_ok());
        assert!(df.column("longitude").is_ok());
    }

    #[test]
    fn test_assert_parquet_file_valid_with_valid_file() {
        let df = create_simple_test_dataframe();
        let dir = create_temp_output_dir();
        let path = dir.path().join("test.parquet");
        let mut file = std::fs::File::create(&path).unwrap();
        ParquetWriter::new(&mut file)
            .finish(&mut df.clone())
            .unwrap();
        assert_parquet_file_valid(&path);
    }

    #[test]
    #[should_panic(expected = "Parquet file does not exist")]
    fn test_assert_parquet_file_valid_panics_on_missing() {
        let path = std::path::PathBuf::from("/nonexistent/path/to/test.parquet");
        assert_parquet_file_valid(&path);
    }
}
