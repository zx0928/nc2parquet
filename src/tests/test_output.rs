#[cfg(test)]
mod output_tests {
    use crate::output::{write_dataframe_to_parquet, write_dataframe_to_parquet_async};
    use crate::test_helpers::{
        assert_parquet_file_valid, create_simple_test_dataframe, create_temp_output_dir,
    };
    use polars::prelude::*;

    #[test]
    fn test_write_simple_dataframe_produces_valid_parquet() {
        let df = create_simple_test_dataframe();
        let dir = create_temp_output_dir();
        let output_path = dir.path().join("output.parquet");
        let output_path_str = output_path.to_str().expect("path must be valid UTF-8");

        write_dataframe_to_parquet(&df, output_path_str)
            .expect("write_dataframe_to_parquet must succeed");

        assert_parquet_file_valid(&output_path);
    }

    #[test]
    fn test_write_empty_dataframe_succeeds() {
        let df = df! {
            "value" => Vec::<f64>::new(),
        }
        .expect("creating empty DataFrame must succeed");

        let dir = create_temp_output_dir();
        let output_path = dir.path().join("empty.parquet");
        let output_path_str = output_path.to_str().expect("path must be valid UTF-8");

        write_dataframe_to_parquet(&df, output_path_str)
            .expect("write_dataframe_to_parquet must succeed for empty DataFrame");

        assert!(
            output_path.exists(),
            "Parquet file must exist after writing empty DataFrame"
        );
        let file =
            std::fs::File::open(&output_path).expect("must be able to open written parquet file");
        let read_back = ParquetReader::new(file)
            .finish()
            .expect("must be able to read back empty parquet file");
        assert_eq!(read_back.height(), 0, "read-back DataFrame must have 0 rows");
        assert_eq!(
            read_back.width(),
            1,
            "read-back DataFrame must have 1 column"
        );
    }

    #[test]
    fn test_write_creates_nonexistent_parent_directories() {
        let df = create_simple_test_dataframe();
        let dir = create_temp_output_dir();
        let output_path = dir.path().join("level1").join("level2").join("level3").join("out.parquet");
        let output_path_str = output_path.to_str().expect("path must be valid UTF-8");

        write_dataframe_to_parquet(&df, output_path_str)
            .expect("write_dataframe_to_parquet must create parent directories and succeed");

        assert_parquet_file_valid(&output_path);
    }

    #[tokio::test]
    async fn test_async_write_to_local_path_produces_valid_parquet() {
        let df = create_simple_test_dataframe();
        let dir = create_temp_output_dir();
        let output_path = dir.path().join("async_output.parquet");
        let output_path_str = output_path.to_str().expect("path must be valid UTF-8");

        write_dataframe_to_parquet_async(&df, output_path_str)
            .await
            .expect("write_dataframe_to_parquet_async must succeed");

        assert_parquet_file_valid(&output_path);
    }

    #[test]
    fn test_written_parquet_file_starts_with_par1_magic_bytes() {
        let df = create_simple_test_dataframe();
        let dir = create_temp_output_dir();
        let output_path = dir.path().join("magic_check.parquet");
        let output_path_str = output_path.to_str().expect("path must be valid UTF-8");

        write_dataframe_to_parquet(&df, output_path_str)
            .expect("write_dataframe_to_parquet must succeed");

        let raw_bytes =
            std::fs::read(&output_path).expect("must be able to read written parquet file");

        assert!(
            raw_bytes.len() >= 4,
            "Parquet file must be at least 4 bytes long"
        );
        assert_eq!(
            &raw_bytes[..4],
            b"PAR1",
            "Parquet file must begin with the PAR1 magic bytes"
        );
    }
}
