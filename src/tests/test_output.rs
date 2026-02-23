#[cfg(test)]
mod output_tests {
    use crate::input::{CompressionCodec, OutputConfig};
    use crate::output::{write_dataframe_to_parquet, write_dataframe_to_parquet_async};
    use crate::test_helpers::{
        assert_parquet_file_valid, create_simple_test_dataframe, create_temp_output_dir,
    };
    use polars::prelude::*;

    // -----------------------------------------------------------------------
    // Existing tests updated to pass None for output_config
    // -----------------------------------------------------------------------

    #[test]
    fn test_write_simple_dataframe_produces_valid_parquet() {
        let mut df = create_simple_test_dataframe();
        let dir = create_temp_output_dir();
        let output_path = dir.path().join("output.parquet");
        let output_path_str = output_path.to_str().expect("path must be valid UTF-8");

        write_dataframe_to_parquet(&mut df, output_path_str, None)
            .expect("write_dataframe_to_parquet must succeed");

        assert_parquet_file_valid(&output_path);
    }

    #[test]
    fn test_write_empty_dataframe_succeeds() {
        let mut df = df! {
            "value" => Vec::<f64>::new(),
        }
        .expect("creating empty DataFrame must succeed");

        let dir = create_temp_output_dir();
        let output_path = dir.path().join("empty.parquet");
        let output_path_str = output_path.to_str().expect("path must be valid UTF-8");

        write_dataframe_to_parquet(&mut df, output_path_str, None)
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
        assert_eq!(
            read_back.height(),
            0,
            "read-back DataFrame must have 0 rows"
        );
        assert_eq!(
            read_back.width(),
            1,
            "read-back DataFrame must have 1 column"
        );
    }

    #[test]
    fn test_write_creates_nonexistent_parent_directories() {
        let mut df = create_simple_test_dataframe();
        let dir = create_temp_output_dir();
        let output_path = dir
            .path()
            .join("level1")
            .join("level2")
            .join("level3")
            .join("out.parquet");
        let output_path_str = output_path.to_str().expect("path must be valid UTF-8");

        write_dataframe_to_parquet(&mut df, output_path_str, None)
            .expect("write_dataframe_to_parquet must create parent directories and succeed");

        assert_parquet_file_valid(&output_path);
    }

    #[tokio::test]
    async fn test_async_write_to_local_path_produces_valid_parquet() {
        let mut df = create_simple_test_dataframe();
        let dir = create_temp_output_dir();
        let output_path = dir.path().join("async_output.parquet");
        let output_path_str = output_path.to_str().expect("path must be valid UTF-8");

        write_dataframe_to_parquet_async(&mut df, output_path_str, None)
            .await
            .expect("write_dataframe_to_parquet_async must succeed");

        assert_parquet_file_valid(&output_path);
    }

    #[test]
    fn test_written_parquet_file_starts_with_par1_magic_bytes() {
        let mut df = create_simple_test_dataframe();
        let dir = create_temp_output_dir();
        let output_path = dir.path().join("magic_check.parquet");
        let output_path_str = output_path.to_str().expect("path must be valid UTF-8");

        write_dataframe_to_parquet(&mut df, output_path_str, None)
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

    // -----------------------------------------------------------------------
    // OutputConfig::default() tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_output_config_default_has_snappy_and_statistics_true() {
        let config = OutputConfig::default();
        assert!(
            matches!(config.compression, CompressionCodec::Snappy),
            "default compression must be Snappy"
        );
        assert!(config.statistics, "default statistics must be true");
        assert!(
            config.compression_level.is_none(),
            "default compression_level must be None"
        );
        assert!(
            config.row_group_size.is_none(),
            "default row_group_size must be None"
        );
        assert!(
            config.data_page_size.is_none(),
            "default data_page_size must be None"
        );
    }

    // -----------------------------------------------------------------------
    // validate() tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_validate_zstd_level_3_passes() {
        let config = OutputConfig {
            compression: CompressionCodec::Zstd,
            compression_level: Some(3),
            ..Default::default()
        };
        assert!(config.validate().is_ok(), "Zstd level 3 must be valid");
    }

    #[test]
    fn test_validate_zstd_level_25_fails() {
        let config = OutputConfig {
            compression: CompressionCodec::Zstd,
            compression_level: Some(25),
            ..Default::default()
        };
        assert!(
            config.validate().is_err(),
            "Zstd level 25 must be invalid (max is 22)"
        );
    }

    #[test]
    fn test_validate_zstd_level_0_fails() {
        let config = OutputConfig {
            compression: CompressionCodec::Zstd,
            compression_level: Some(0),
            ..Default::default()
        };
        assert!(
            config.validate().is_err(),
            "Zstd level 0 must be invalid (min is 1)"
        );
    }

    #[test]
    fn test_validate_gzip_level_6_passes() {
        let config = OutputConfig {
            compression: CompressionCodec::Gzip,
            compression_level: Some(6),
            ..Default::default()
        };
        assert!(config.validate().is_ok(), "Gzip level 6 must be valid");
    }

    #[test]
    fn test_validate_gzip_level_10_fails() {
        let config = OutputConfig {
            compression: CompressionCodec::Gzip,
            compression_level: Some(10),
            ..Default::default()
        };
        assert!(
            config.validate().is_err(),
            "Gzip level 10 must be invalid (max is 9)"
        );
    }

    #[test]
    fn test_validate_snappy_with_level_fails() {
        let config = OutputConfig {
            compression: CompressionCodec::Snappy,
            compression_level: Some(1),
            ..Default::default()
        };
        assert!(
            config.validate().is_err(),
            "Snappy must not accept a compression level"
        );
    }

    #[test]
    fn test_validate_lz4_with_level_fails() {
        let config = OutputConfig {
            compression: CompressionCodec::Lz4,
            compression_level: Some(1),
            ..Default::default()
        };
        assert!(
            config.validate().is_err(),
            "Lz4 must not accept a compression level"
        );
    }

    #[test]
    fn test_validate_uncompressed_with_level_fails() {
        let config = OutputConfig {
            compression: CompressionCodec::Uncompressed,
            compression_level: Some(1),
            ..Default::default()
        };
        assert!(
            config.validate().is_err(),
            "Uncompressed must not accept a compression level"
        );
    }

    #[test]
    fn test_validate_row_group_size_zero_fails() {
        let config = OutputConfig {
            row_group_size: Some(0),
            ..Default::default()
        };
        assert!(
            config.validate().is_err(),
            "row_group_size of 0 must be invalid"
        );
    }

    #[test]
    fn test_validate_row_group_size_nonzero_passes() {
        let config = OutputConfig {
            row_group_size: Some(1024),
            ..Default::default()
        };
        assert!(
            config.validate().is_ok(),
            "row_group_size of 1024 must be valid"
        );
    }

    #[test]
    fn test_validate_snappy_no_level_passes() {
        let config = OutputConfig {
            compression: CompressionCodec::Snappy,
            compression_level: None,
            ..Default::default()
        };
        assert!(
            config.validate().is_ok(),
            "Snappy without a level must be valid"
        );
    }

    // -----------------------------------------------------------------------
    // to_polars_compression() mapping tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_to_polars_compression_uncompressed() {
        let config = OutputConfig {
            compression: CompressionCodec::Uncompressed,
            ..Default::default()
        };
        assert!(
            matches!(
                config.to_polars_compression(),
                ParquetCompression::Uncompressed
            ),
            "Uncompressed must map to ParquetCompression::Uncompressed"
        );
    }

    #[test]
    fn test_to_polars_compression_snappy() {
        let config = OutputConfig {
            compression: CompressionCodec::Snappy,
            ..Default::default()
        };
        assert!(
            matches!(config.to_polars_compression(), ParquetCompression::Snappy),
            "Snappy must map to ParquetCompression::Snappy"
        );
    }

    #[test]
    fn test_to_polars_compression_gzip_no_level() {
        let config = OutputConfig {
            compression: CompressionCodec::Gzip,
            compression_level: None,
            ..Default::default()
        };
        assert!(
            matches!(
                config.to_polars_compression(),
                ParquetCompression::Gzip(None)
            ),
            "Gzip without level must map to ParquetCompression::Gzip(None)"
        );
    }

    #[test]
    fn test_to_polars_compression_gzip_with_level() {
        let config = OutputConfig {
            compression: CompressionCodec::Gzip,
            compression_level: Some(6),
            ..Default::default()
        };
        assert!(
            matches!(
                config.to_polars_compression(),
                ParquetCompression::Gzip(Some(_))
            ),
            "Gzip with level must map to ParquetCompression::Gzip(Some(...))"
        );
    }

    #[test]
    fn test_to_polars_compression_lz4() {
        let config = OutputConfig {
            compression: CompressionCodec::Lz4,
            ..Default::default()
        };
        assert!(
            matches!(config.to_polars_compression(), ParquetCompression::Lz4Raw),
            "Lz4 must map to ParquetCompression::Lz4Raw"
        );
    }

    #[test]
    fn test_to_polars_compression_zstd_no_level() {
        let config = OutputConfig {
            compression: CompressionCodec::Zstd,
            compression_level: None,
            ..Default::default()
        };
        assert!(
            matches!(
                config.to_polars_compression(),
                ParquetCompression::Zstd(None)
            ),
            "Zstd without level must map to ParquetCompression::Zstd(None)"
        );
    }

    #[test]
    fn test_to_polars_compression_zstd_with_level() {
        let config = OutputConfig {
            compression: CompressionCodec::Zstd,
            compression_level: Some(3),
            ..Default::default()
        };
        assert!(
            matches!(
                config.to_polars_compression(),
                ParquetCompression::Zstd(Some(_))
            ),
            "Zstd with level must map to ParquetCompression::Zstd(Some(...))"
        );
    }

    // -----------------------------------------------------------------------
    // Serialization round-trip tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_output_config_serialization_round_trip() {
        let original = OutputConfig {
            compression: CompressionCodec::Zstd,
            compression_level: Some(5),
            row_group_size: Some(1024),
            data_page_size: Some(65536),
            statistics: false,
        };

        let json = serde_json::to_string(&original).expect("serialization must succeed");
        let restored: OutputConfig =
            serde_json::from_str(&json).expect("deserialization must succeed");

        assert_eq!(
            original, restored,
            "round-trip must produce identical struct"
        );
    }

    #[test]
    fn test_output_config_defaults_on_deserialize() {
        // Minimal JSON — only required fields present; defaults fill the rest
        let json = r#"{"compression": "snappy"}"#;
        let config: OutputConfig =
            serde_json::from_str(json).expect("deserialization must succeed");

        assert!(matches!(config.compression, CompressionCodec::Snappy));
        assert!(config.compression_level.is_none());
        assert!(config.row_group_size.is_none());
        assert!(config.data_page_size.is_none());
        assert!(config.statistics);
    }

    #[test]
    fn test_compression_codec_serialization() {
        let cases = [
            (CompressionCodec::Uncompressed, "\"uncompressed\""),
            (CompressionCodec::Snappy, "\"snappy\""),
            (CompressionCodec::Gzip, "\"gzip\""),
            (CompressionCodec::Lz4, "\"lz4\""),
            (CompressionCodec::Zstd, "\"zstd\""),
        ];

        for (codec, expected_json) in cases {
            let serialized = serde_json::to_string(&codec).expect("must serialize");
            assert_eq!(
                serialized, expected_json,
                "codec {:?} must serialize to {}",
                codec, expected_json
            );
            let deserialized: CompressionCodec =
                serde_json::from_str(expected_json).expect("must deserialize");
            assert_eq!(
                codec, deserialized,
                "codec must round-trip through JSON correctly"
            );
        }
    }

    // -----------------------------------------------------------------------
    // Write with compression and verify data is correct
    // -----------------------------------------------------------------------

    #[test]
    fn test_write_with_zstd_compression_reads_back_correctly() {
        let mut df = create_simple_test_dataframe();
        let dir = create_temp_output_dir();
        let output_path = dir.path().join("zstd_output.parquet");
        let output_path_str = output_path.to_str().expect("path must be valid UTF-8");

        let config = OutputConfig {
            compression: CompressionCodec::Zstd,
            compression_level: Some(3),
            ..Default::default()
        };

        write_dataframe_to_parquet(&mut df, output_path_str, Some(&config))
            .expect("write with Zstd must succeed");

        assert_parquet_file_valid(&output_path);

        let file = std::fs::File::open(&output_path).expect("must open written file");
        let read_back = ParquetReader::new(file)
            .finish()
            .expect("must read back Zstd-compressed parquet");

        assert_eq!(
            read_back.height(),
            df.height(),
            "read-back row count must match original"
        );
        assert_eq!(
            read_back.width(),
            df.width(),
            "read-back column count must match original"
        );
    }

    #[test]
    fn test_write_with_snappy_compression_reads_back_correctly() {
        let mut df = create_simple_test_dataframe();
        let dir = create_temp_output_dir();
        let output_path = dir.path().join("snappy_output.parquet");
        let output_path_str = output_path.to_str().expect("path must be valid UTF-8");

        let config = OutputConfig {
            compression: CompressionCodec::Snappy,
            ..Default::default()
        };

        write_dataframe_to_parquet(&mut df, output_path_str, Some(&config))
            .expect("write with Snappy must succeed");

        assert_parquet_file_valid(&output_path);

        let file = std::fs::File::open(&output_path).expect("must open written file");
        let read_back = ParquetReader::new(file)
            .finish()
            .expect("must read back Snappy-compressed parquet");

        assert_eq!(read_back.height(), df.height());
        assert_eq!(read_back.width(), df.width());
    }

    #[test]
    fn test_write_with_gzip_compression_reads_back_correctly() {
        let mut df = create_simple_test_dataframe();
        let dir = create_temp_output_dir();
        let output_path = dir.path().join("gzip_output.parquet");
        let output_path_str = output_path.to_str().expect("path must be valid UTF-8");

        let config = OutputConfig {
            compression: CompressionCodec::Gzip,
            compression_level: Some(6),
            ..Default::default()
        };

        write_dataframe_to_parquet(&mut df, output_path_str, Some(&config))
            .expect("write with Gzip must succeed");

        assert_parquet_file_valid(&output_path);

        let file = std::fs::File::open(&output_path).expect("must open written file");
        let read_back = ParquetReader::new(file)
            .finish()
            .expect("must read back Gzip-compressed parquet");

        assert_eq!(read_back.height(), df.height());
        assert_eq!(read_back.width(), df.width());
    }

    #[test]
    fn test_write_with_uncompressed_reads_back_correctly() {
        let mut df = create_simple_test_dataframe();
        let dir = create_temp_output_dir();
        let output_path = dir.path().join("uncompressed_output.parquet");
        let output_path_str = output_path.to_str().expect("path must be valid UTF-8");

        let config = OutputConfig {
            compression: CompressionCodec::Uncompressed,
            ..Default::default()
        };

        write_dataframe_to_parquet(&mut df, output_path_str, Some(&config))
            .expect("write with Uncompressed must succeed");

        assert_parquet_file_valid(&output_path);

        let file = std::fs::File::open(&output_path).expect("must open written file");
        let read_back = ParquetReader::new(file)
            .finish()
            .expect("must read back uncompressed parquet");

        assert_eq!(read_back.height(), df.height());
        assert_eq!(read_back.width(), df.width());
    }

    // -----------------------------------------------------------------------
    // Custom row_group_size — write and verify valid output
    // -----------------------------------------------------------------------

    #[test]
    fn test_write_with_custom_row_group_size_produces_valid_parquet() {
        let mut df = create_simple_test_dataframe();
        let dir = create_temp_output_dir();
        let output_path = dir.path().join("row_group_output.parquet");
        let output_path_str = output_path.to_str().expect("path must be valid UTF-8");

        // Use a row group size smaller than the DataFrame height (4 rows)
        // so Polars is forced to actually split row groups
        let config = OutputConfig {
            compression: CompressionCodec::Snappy,
            row_group_size: Some(2),
            ..Default::default()
        };

        write_dataframe_to_parquet(&mut df, output_path_str, Some(&config))
            .expect("write with custom row_group_size must succeed");

        assert_parquet_file_valid(&output_path);

        let file = std::fs::File::open(&output_path).expect("must open written file");
        let read_back = ParquetReader::new(file)
            .finish()
            .expect("must read back parquet with custom row groups");

        assert_eq!(
            read_back.height(),
            df.height(),
            "row count must match despite custom row group size"
        );
    }

    // -----------------------------------------------------------------------
    // statistics: false disables statistics in output
    // -----------------------------------------------------------------------

    #[test]
    fn test_write_with_statistics_disabled_produces_valid_parquet() {
        let mut df = create_simple_test_dataframe();
        let dir = create_temp_output_dir();
        let output_path = dir.path().join("no_stats_output.parquet");
        let output_path_str = output_path.to_str().expect("path must be valid UTF-8");

        let config = OutputConfig {
            compression: CompressionCodec::Snappy,
            statistics: false,
            ..Default::default()
        };

        write_dataframe_to_parquet(&mut df, output_path_str, Some(&config))
            .expect("write with statistics=false must succeed");

        assert_parquet_file_valid(&output_path);

        let file = std::fs::File::open(&output_path).expect("must open written file");
        let read_back = ParquetReader::new(file)
            .finish()
            .expect("must read back parquet with no statistics");
        assert_eq!(read_back.height(), df.height());
    }

    // -----------------------------------------------------------------------
    // Backward compatibility — JobConfig without "output" deserializes to None
    // -----------------------------------------------------------------------

    #[test]
    fn test_job_config_without_output_field_deserializes_to_none() {
        use crate::input::JobConfig;

        let json = r#"{
            "nc_key": "input.nc",
            "variable_name": "temperature",
            "parquet_key": "output.parquet",
            "filters": []
        }"#;

        let config = JobConfig::from_json(json).expect("must deserialize without output field");
        assert!(
            config.output.is_none(),
            "output must default to None when absent from JSON"
        );
    }

    #[test]
    fn test_job_config_with_output_field_deserializes_correctly() {
        use crate::input::JobConfig;

        let json = r#"{
            "nc_key": "input.nc",
            "variable_name": "temperature",
            "parquet_key": "output.parquet",
            "filters": [],
            "output": {
                "compression": "zstd",
                "compression_level": 5,
                "row_group_size": 512,
                "statistics": false
            }
        }"#;

        let config = JobConfig::from_json(json).expect("must deserialize with output field");
        let output = config.output.expect("output must be Some");
        assert!(matches!(output.compression, CompressionCodec::Zstd));
        assert_eq!(output.compression_level, Some(5));
        assert_eq!(output.row_group_size, Some(512));
        assert!(!output.statistics);
    }
}
