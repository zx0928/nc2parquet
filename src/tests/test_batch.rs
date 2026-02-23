#[cfg(test)]
mod batch_tests {
    use crate::input::{BatchConfig, FilterConfig, RangeParams};
    use crate::test_helpers::get_test_data_path;
    use crate::{process_netcdf_batch, resolve_output_path};
    use std::path::{Path, PathBuf};
    use tempfile::tempdir;

    // -----------------------------------------------------------------------
    // resolve_output_path
    // -----------------------------------------------------------------------

    #[test]
    fn test_resolve_output_path_stem_template() {
        let result = resolve_output_path(
            Path::new("/data/temperature.nc"),
            "/tmp/out",
            "{stem}.parquet",
        );
        assert_eq!(result, PathBuf::from("/tmp/out/temperature.parquet"));
    }

    #[test]
    fn test_resolve_output_path_name_template() {
        let result = resolve_output_path(
            Path::new("/data/temperature.nc"),
            "/tmp/out",
            "{name}.parquet",
        );
        assert_eq!(result, PathBuf::from("/tmp/out/temperature.nc.parquet"));
    }

    #[test]
    fn test_resolve_output_path_custom_stem_suffix() {
        let result = resolve_output_path(
            Path::new("/some/dir/wind_speed.nc"),
            "/results",
            "{stem}_converted.parquet",
        );
        assert_eq!(
            result,
            PathBuf::from("/results/wind_speed_converted.parquet")
        );
    }

    #[test]
    fn test_resolve_output_path_default_template_equivalent() {
        // The default template used inside process_netcdf_batch is "{stem}.parquet"
        let result = resolve_output_path(Path::new("ocean_data.nc"), "/output", "{stem}.parquet");
        assert_eq!(result, PathBuf::from("/output/ocean_data.parquet"));
    }

    // -----------------------------------------------------------------------
    // S3 rejection
    // -----------------------------------------------------------------------

    #[test]
    fn test_s3_pattern_rejected() {
        let config = BatchConfig {
            pattern: "s3://my-bucket/*.nc".to_string(),
            output_dir: "/tmp/out".to_string(),
            variable_name: "data".to_string(),
            filters: vec![],
            postprocessing: None,
            output_template: None,
            output: None,
            fail_fast: false,
        };
        let err = process_netcdf_batch(&config).unwrap_err();
        match err {
            crate::Nc2ParquetError::Config(msg) => {
                assert!(
                    msg.contains("S3") || msg.contains("s3"),
                    "Error message should mention S3, got: {}",
                    msg
                );
            }
            other => panic!("Expected Config error, got: {:?}", other),
        }
    }

    // -----------------------------------------------------------------------
    // Invalid glob pattern
    // -----------------------------------------------------------------------

    #[test]
    fn test_invalid_glob_pattern_rejected() {
        let config = BatchConfig {
            // Unmatched bracket makes this an invalid glob pattern
            pattern: "data/[invalid".to_string(),
            output_dir: "/tmp/out".to_string(),
            variable_name: "data".to_string(),
            filters: vec![],
            postprocessing: None,
            output_template: None,
            output: None,
            fail_fast: false,
        };
        let err = process_netcdf_batch(&config).unwrap_err();
        match err {
            crate::Nc2ParquetError::Config(msg) => {
                assert!(
                    msg.contains("Invalid glob pattern"),
                    "Error message should mention invalid glob pattern, got: {}",
                    msg
                );
            }
            other => panic!("Expected Config error, got: {:?}", other),
        }
    }

    // -----------------------------------------------------------------------
    // No matches
    // -----------------------------------------------------------------------

    #[test]
    fn test_no_files_match_pattern_returns_error() {
        let dir = tempdir().unwrap();
        // Pattern points to a real directory but no .nc files exist there
        let pattern = format!("{}/*.nc", dir.path().display());
        let config = BatchConfig {
            pattern,
            output_dir: dir.path().join("out").to_string_lossy().into_owned(),
            variable_name: "data".to_string(),
            filters: vec![],
            postprocessing: None,
            output_template: None,
            output: None,
            fail_fast: false,
        };
        let err = process_netcdf_batch(&config).unwrap_err();
        match err {
            crate::Nc2ParquetError::Config(msg) => {
                assert!(
                    msg.contains("No files matched"),
                    "Error message should mention no matches, got: {}",
                    msg
                );
            }
            other => panic!("Expected Config error, got: {:?}", other),
        }
    }

    // -----------------------------------------------------------------------
    // Happy path: process multiple files
    // -----------------------------------------------------------------------

    #[test]
    fn test_batch_happy_path() -> Result<(), Box<dyn std::error::Error>> {
        let source_dir = tempdir()?;
        let output_dir = tempdir()?;

        // Copy the small fixture file 3 times to create a mini batch
        let fixture = get_test_data_path("simple_xy.nc");
        for i in 0..3 {
            let dest = source_dir.path().join(format!("file_{}.nc", i));
            std::fs::copy(&fixture, &dest)?;
        }

        let pattern = format!("{}/*.nc", source_dir.path().display());
        let config = BatchConfig {
            pattern,
            output_dir: output_dir.path().to_string_lossy().into_owned(),
            variable_name: "data".to_string(),
            filters: vec![],
            postprocessing: None,
            output_template: None,
            output: None,
            fail_fast: false,
        };

        let result = process_netcdf_batch(&config)?;

        assert_eq!(result.total_files, 3, "Should have found 3 files");
        assert_eq!(result.succeeded.len(), 3, "All 3 files should succeed");
        assert!(result.failed.is_empty(), "No failures expected");

        // Verify all output parquet files were actually created
        for i in 0..3 {
            let expected_output = output_dir.path().join(format!("file_{}.parquet", i));
            assert!(
                expected_output.exists(),
                "Expected parquet file missing: {}",
                expected_output.display()
            );
            let meta = std::fs::metadata(&expected_output)?;
            assert!(meta.len() > 0, "Output parquet should not be empty");
        }

        Ok(())
    }

    // -----------------------------------------------------------------------
    // Fail-fast: stops on first error
    // -----------------------------------------------------------------------

    #[test]
    fn test_fail_fast_stops_on_first_error() -> Result<(), Box<dyn std::error::Error>> {
        let source_dir = tempdir()?;
        let output_dir = tempdir()?;

        // Create one invalid (empty / corrupt) "nc" file and one valid file
        let corrupt = source_dir.path().join("aaa_corrupt.nc");
        std::fs::write(&corrupt, b"this is not a valid netcdf file")?;

        let valid = source_dir.path().join("bbb_valid.nc");
        let fixture = get_test_data_path("simple_xy.nc");
        std::fs::copy(&fixture, &valid)?;

        let pattern = format!("{}/*.nc", source_dir.path().display());
        let config = BatchConfig {
            pattern,
            output_dir: output_dir.path().to_string_lossy().into_owned(),
            variable_name: "data".to_string(),
            filters: vec![],
            postprocessing: None,
            output_template: None,
            output: None,
            fail_fast: true,
        };

        // Should return an error immediately on the corrupt file (alphabetically first)
        let result = process_netcdf_batch(&config);
        assert!(
            result.is_err(),
            "fail_fast=true should return Err on first bad file"
        );

        // The valid file (bbb_valid.nc) must NOT have been processed yet
        let valid_output = output_dir.path().join("bbb_valid.parquet");
        assert!(
            !valid_output.exists(),
            "Valid file should not be processed when fail_fast stops on corrupt file"
        );

        Ok(())
    }

    // -----------------------------------------------------------------------
    // Collect errors: fail_fast=false
    // -----------------------------------------------------------------------

    #[test]
    fn test_collect_errors_when_not_fail_fast() -> Result<(), Box<dyn std::error::Error>> {
        let source_dir = tempdir()?;
        let output_dir = tempdir()?;

        // One corrupt file and two valid files
        let corrupt = source_dir.path().join("bad.nc");
        std::fs::write(&corrupt, b"garbage")?;

        let fixture = get_test_data_path("simple_xy.nc");
        for name in ["good1.nc", "good2.nc"] {
            let dest = source_dir.path().join(name);
            std::fs::copy(&fixture, &dest)?;
        }

        let pattern = format!("{}/*.nc", source_dir.path().display());
        let config = BatchConfig {
            pattern,
            output_dir: output_dir.path().to_string_lossy().into_owned(),
            variable_name: "data".to_string(),
            filters: vec![],
            postprocessing: None,
            output_template: None,
            output: None,
            fail_fast: false,
        };

        let result = process_netcdf_batch(&config)?;

        assert_eq!(result.total_files, 3);
        assert_eq!(
            result.succeeded.len(),
            2,
            "2 valid files should succeed, got: {:?}",
            result.succeeded
        );
        assert_eq!(
            result.failed.len(),
            1,
            "1 corrupt file should fail, got: {:?}",
            result.failed.iter().map(|(p, _)| p).collect::<Vec<_>>()
        );

        // The corrupt file path should appear in the failures
        let (failed_path, _) = &result.failed[0];
        assert!(
            failed_path.contains("bad.nc"),
            "Failed entry should reference bad.nc, got: {}",
            failed_path
        );

        // Verify the two good outputs exist
        for name in ["good1.parquet", "good2.parquet"] {
            let out = output_dir.path().join(name);
            assert!(
                out.exists(),
                "Expected output {} to exist after non-fail-fast batch",
                out.display()
            );
        }

        Ok(())
    }

    // -----------------------------------------------------------------------
    // Custom output template
    // -----------------------------------------------------------------------

    #[test]
    fn test_custom_output_template() -> Result<(), Box<dyn std::error::Error>> {
        let source_dir = tempdir()?;
        let output_dir = tempdir()?;

        let fixture = get_test_data_path("simple_xy.nc");
        let dest = source_dir.path().join("climate.nc");
        std::fs::copy(&fixture, &dest)?;

        let pattern = format!("{}/*.nc", source_dir.path().display());
        let config = BatchConfig {
            pattern,
            output_dir: output_dir.path().to_string_lossy().into_owned(),
            variable_name: "data".to_string(),
            filters: vec![],
            postprocessing: None,
            output_template: Some("{stem}_converted.parquet".to_string()),
            output: None,
            fail_fast: false,
        };

        let result = process_netcdf_batch(&config)?;

        assert_eq!(result.total_files, 1);
        assert_eq!(result.succeeded.len(), 1);
        assert!(result.failed.is_empty());

        let expected = output_dir.path().join("climate_converted.parquet");
        assert!(
            expected.exists(),
            "Expected custom-template output {} to exist",
            expected.display()
        );

        Ok(())
    }

    // -----------------------------------------------------------------------
    // Filters are forwarded to each file in the batch
    // -----------------------------------------------------------------------

    #[test]
    fn test_batch_with_filters() -> Result<(), Box<dyn std::error::Error>> {
        // Use pres_temp_4D.nc which has proper coordinate variables for range filtering.
        let source_dir = tempdir()?;
        let output_dir = tempdir()?;

        let fixture = get_test_data_path("pres_temp_4D.nc");
        let dest = source_dir.path().join("weather.nc");
        std::fs::copy(&fixture, &dest)?;

        let pattern = format!("{}/*.nc", source_dir.path().display());
        let config = BatchConfig {
            pattern,
            output_dir: output_dir.path().to_string_lossy().into_owned(),
            variable_name: "temperature".to_string(),
            filters: vec![FilterConfig::Range {
                params: RangeParams {
                    dimension_name: "latitude".to_string(),
                    min_value: 30.0,
                    max_value: 50.0,
                },
            }],
            postprocessing: None,
            output_template: None,
            output: None,
            fail_fast: false,
        };

        let result = process_netcdf_batch(&config)?;

        assert_eq!(result.total_files, 1);
        assert_eq!(result.succeeded.len(), 1);
        assert!(result.failed.is_empty());

        let expected = output_dir.path().join("weather.parquet");
        assert!(
            expected.exists(),
            "Filtered output should exist: {}",
            expected.display()
        );

        // Verify the output parquet is non-empty (filter matched some rows)
        let meta = std::fs::metadata(&expected)?;
        assert!(meta.len() > 0, "Filtered output should not be empty");

        Ok(())
    }
}
