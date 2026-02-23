#[cfg(test)]
mod integration_tests {
    use crate::input::{FilterConfig, JobConfig, ListParams, Point2DParams, RangeParams};
    use crate::test_helpers::get_test_data_path;
    use tempfile::tempdir;

    #[test]
    fn test_full_pipeline_simple_xy() -> Result<(), Box<dyn std::error::Error>> {
        let file_path = get_test_data_path("simple_xy.nc");
        let temp_dir = tempdir()?;
        let output_path = temp_dir.path().join("simple_xy_output.parquet");

        let config = JobConfig {
            nc_key: file_path.to_string_lossy().to_string(),
            variable_name: "data".to_string(),
            variable_names: None,
            parquet_key: output_path.to_string_lossy().to_string(),
            filters: vec![],
            postprocessing: None,
            output: None,
        };

        crate::process_netcdf_job(&config)?;

        assert!(output_path.exists());

        let metadata = std::fs::metadata(&output_path)?;
        assert!(metadata.len() > 0);

        Ok(())
    }

    #[test]
    fn test_full_pipeline_with_latitude_filter() -> Result<(), Box<dyn std::error::Error>> {
        let file_path = get_test_data_path("pres_temp_4D.nc");
        let temp_dir = tempdir()?;
        let output_path = temp_dir.path().join("filtered_temp_output.parquet");

        let config = JobConfig {
            nc_key: file_path.to_string_lossy().to_string(),
            variable_name: "temperature".to_string(),
            variable_names: None,
            parquet_key: output_path.to_string_lossy().to_string(),
            filters: vec![FilterConfig::Range {
                params: RangeParams {
                    dimension_name: "latitude".to_string(),
                    min_value: 30.0,
                    max_value: 45.0,
                },
            }],
            postprocessing: None,
            output: None,
        };

        crate::process_netcdf_job(&config)?;

        assert!(output_path.exists());
        let metadata = std::fs::metadata(&output_path)?;
        assert!(metadata.len() > 0);

        Ok(())
    }

    #[test]
    fn test_full_pipeline_with_spatial_filter() -> Result<(), Box<dyn std::error::Error>> {
        let file_path = get_test_data_path("pres_temp_4D.nc");
        let temp_dir = tempdir()?;
        let output_path = temp_dir.path().join("spatial_filtered_output.parquet");

        let config = JobConfig {
            nc_key: file_path.to_string_lossy().to_string(),
            variable_name: "pressure".to_string(),
            variable_names: None,
            parquet_key: output_path.to_string_lossy().to_string(),
            filters: vec![FilterConfig::Point2D {
                params: Point2DParams {
                    lat_dimension_name: "latitude".to_string(),
                    lon_dimension_name: "longitude".to_string(),
                    points: vec![(30.0, -120.0), (40.0, -100.0)],
                    tolerance: 1.0,
                },
            }],
            postprocessing: None,
            output: None,
        };

        crate::process_netcdf_job(&config)?;

        assert!(output_path.exists());
        let metadata = std::fs::metadata(&output_path)?;
        assert!(metadata.len() > 0);

        Ok(())
    }

    #[test]
    fn test_full_pipeline_multi_filter() -> Result<(), Box<dyn std::error::Error>> {
        let file_path = get_test_data_path("pres_temp_4D.nc");
        let temp_dir = tempdir()?;
        let output_path = temp_dir.path().join("multi_filter_output.parquet");

        let config = JobConfig {
            nc_key: file_path.to_string_lossy().to_string(),
            variable_name: "temperature".to_string(),
            variable_names: None,
            parquet_key: output_path.to_string_lossy().to_string(),
            filters: vec![
                FilterConfig::Range {
                    params: RangeParams {
                        dimension_name: "latitude".to_string(),
                        min_value: 35.0,
                        max_value: 45.0,
                    },
                },
                FilterConfig::List {
                    params: ListParams {
                        dimension_name: "longitude".to_string(),
                        values: vec![-120.0, -110.0, -100.0],
                    },
                },
            ],
            postprocessing: None,
            output: None,
        };

        crate::process_netcdf_job(&config)?;

        assert!(output_path.exists());
        let metadata = std::fs::metadata(&output_path)?;
        assert!(metadata.len() > 0);

        Ok(())
    }
}

#[cfg(test)]
mod workflow_tests {
    use crate::input::{FilterConfig, JobConfig};
    use crate::postprocess::*;
    use crate::test_helpers::get_test_data_path;
    use std::collections::HashMap;
    use tempfile::tempdir;

    #[test]
    fn test_complete_configuration_workflow_with_real_data() {
        // Create a comprehensive configuration using real file structure
        let json = r#"
        {
            "nc_key": "examples/data/pres_temp_4D.nc",
            "variable_name": "temperature",
            "parquet_key": "filtered_weather.parquet",
            "filters": [
                {
                    "kind": "range",
                    "params": {
                        "dimension_name": "latitude",
                        "min_value": 30.0,
                        "max_value": 45.0
                    }
                },
                {
                    "kind": "list",
                    "params": {
                        "dimension_name": "longitude",
                        "values": [-120.0, -100.0, -80.0]
                    }
                },
                {
                    "kind": "2d_point",
                    "params": {
                        "lat_dimension_name": "latitude",
                        "lon_dimension_name": "longitude",
                        "points": [[30.0, -120.0], [45.0, -85.0]],
                        "tolerance": 5.0
                    }
                }
            ]
        }"#;

        let config = JobConfig::from_json(json).unwrap();
        assert_eq!(config.nc_key, "examples/data/pres_temp_4D.nc");
        assert_eq!(config.variable_name, "temperature");
        assert_eq!(config.parquet_key, "filtered_weather.parquet");
        assert_eq!(config.filters.len(), 3);

        let mut filters = Vec::new();
        for filter_config in &config.filters {
            filters.push(filter_config.to_filter().unwrap());
        }

        assert_eq!(filters.len(), 3);
        assert_eq!(config.filters[0].kind(), "range");
        assert_eq!(config.filters[1].kind(), "list");
        assert_eq!(config.filters[2].kind(), "2d_point");
    }

    #[test]
    fn test_integration_local_file_with_all_features() -> Result<(), Box<dyn std::error::Error>> {
        let temp_dir = tempdir()?;
        let output_path = temp_dir.path().join("full_features.parquet");

        // Create comprehensive config with filtering and post-processing
        let config = JobConfig {
            nc_key: get_test_data_path("simple_xy.nc")
                .to_string_lossy()
                .to_string(),
            variable_name: "data".to_string(),
            variable_names: None,
            parquet_key: output_path.to_string_lossy().to_string(),
            filters: vec![],
            postprocessing: Some(ProcessingPipelineConfig {
                name: Some("Sprint 6 Integration Pipeline".to_string()),
                processors: vec![
                    ProcessorConfig::RenameColumns {
                        mappings: {
                            let mut map = HashMap::new();
                            map.insert("data".to_string(), "temp_k".to_string());
                            map.insert("x".to_string(), "longitude".to_string());
                            map.insert("y".to_string(), "latitude".to_string());
                            map
                        },
                    },
                    ProcessorConfig::ApplyFormula {
                        target_column: "temp_celsius".to_string(),
                        formula: "temp_k - 273.15".to_string(),
                        source_columns: vec!["temp_k".to_string()],
                    },
                    ProcessorConfig::UnitConvert {
                        column: "temp_k".to_string(),
                        from_unit: "kelvin".to_string(),
                        to_unit: "celsius".to_string(),
                    },
                ],
            }),
            output: None,
        };

        crate::process_netcdf_job(&config)?;

        assert!(output_path.exists());
        let metadata = std::fs::metadata(&output_path)?;
        assert!(metadata.len() > 0);

        Ok(())
    }

    #[tokio::test]
    async fn test_integration_async_processing() -> Result<(), Box<dyn std::error::Error>> {
        let temp_dir = tempdir()?;
        let output_path = temp_dir.path().join("async_test.parquet");

        let config = JobConfig {
            nc_key: get_test_data_path("pres_temp_4D.nc")
                .to_string_lossy()
                .to_string(),
            variable_name: "temperature".to_string(),
            variable_names: None,
            parquet_key: output_path.to_string_lossy().to_string(),
            filters: vec![FilterConfig::Range {
                params: crate::input::RangeParams {
                    dimension_name: "latitude".to_string(),
                    min_value: 25.0,
                    max_value: 35.0,
                },
            }],
            postprocessing: Some(ProcessingPipelineConfig {
                name: Some("Async Processing Test".to_string()),
                processors: vec![
                    ProcessorConfig::RenameColumns {
                        mappings: {
                            let mut map = HashMap::new();
                            map.insert("temperature".to_string(), "temp_k".to_string());
                            map
                        },
                    },
                    ProcessorConfig::UnitConvert {
                        column: "temp_k".to_string(),
                        from_unit: "kelvin".to_string(),
                        to_unit: "celsius".to_string(),
                    },
                ],
            }),
            output: None,
        };

        crate::process_netcdf_job_async(&config).await?;

        assert!(output_path.exists());
        let metadata = std::fs::metadata(&output_path)?;
        assert!(metadata.len() > 0);

        Ok(())
    }

    #[test]
    fn test_integration_complex_pipeline_chaining() -> Result<(), Box<dyn std::error::Error>> {
        let temp_dir = tempdir()?;
        let output_path = temp_dir.path().join("complex_pipeline.parquet");

        let config = JobConfig {
            nc_key: get_test_data_path("simple_xy.nc")
                .to_string_lossy()
                .to_string(),
            variable_name: "data".to_string(),
            variable_names: None,
            parquet_key: output_path.to_string_lossy().to_string(),
            filters: vec![],
            postprocessing: Some(ProcessingPipelineConfig {
                name: Some("Complex Pipeline Chaining Test".to_string()),
                processors: vec![
                    ProcessorConfig::RenameColumns {
                        mappings: {
                            let mut map = HashMap::new();
                            map.insert("data".to_string(), "temp_k".to_string());
                            map.insert("x".to_string(), "lon".to_string());
                            map.insert("y".to_string(), "lat".to_string());
                            map
                        },
                    },
                    ProcessorConfig::ApplyFormula {
                        target_column: "temp_celsius".to_string(),
                        formula: "temp_k - 273.15".to_string(),
                        source_columns: vec!["temp_k".to_string()],
                    },
                    ProcessorConfig::ApplyFormula {
                        target_column: "temp_doubled".to_string(),
                        formula: "temp_k * 2.0".to_string(),
                        source_columns: vec!["temp_k".to_string()],
                    },
                    ProcessorConfig::UnitConvert {
                        column: "temp_k".to_string(),
                        from_unit: "kelvin".to_string(),
                        to_unit: "celsius".to_string(),
                    },
                ],
            }),
            output: None,
        };

        crate::process_netcdf_job(&config)?;

        assert!(output_path.exists());
        let metadata = std::fs::metadata(&output_path)?;
        assert!(metadata.len() > 0);

        Ok(())
    }

    #[test]
    fn test_integration_error_handling() {
        let temp_dir = tempdir().unwrap();
        let output_path = temp_dir.path().join("should_not_exist.parquet");

        // Test with nonexistent input file - should fail gracefully
        let config = JobConfig {
            nc_key: "nonexistent_file.nc".to_string(),
            variable_name: "data".to_string(),
            variable_names: None,
            parquet_key: output_path.to_string_lossy().to_string(),
            filters: vec![],
            postprocessing: None,
            output: None,
        };

        let result = crate::process_netcdf_job(&config);
        assert!(result.is_err(), "Should fail with nonexistent input file");
        assert!(
            !output_path.exists(),
            "Output file should not be created on error"
        );

        let config = JobConfig {
            nc_key: get_test_data_path("simple_xy.nc")
                .to_string_lossy()
                .to_string(),
            variable_name: "nonexistent_variable".to_string(),
            variable_names: None,
            parquet_key: output_path.to_string_lossy().to_string(),
            filters: vec![],
            postprocessing: None,
            output: None,
        };

        let result = crate::process_netcdf_job(&config);
        assert!(result.is_err(), "Should fail with nonexistent variable");

        let config = JobConfig {
            nc_key: get_test_data_path("simple_xy.nc")
                .to_string_lossy()
                .to_string(),
            variable_name: "data".to_string(),
            variable_names: None,
            parquet_key: output_path.to_string_lossy().to_string(),
            filters: vec![FilterConfig::Range {
                params: crate::input::RangeParams {
                    dimension_name: "nonexistent_dimension".to_string(),
                    min_value: 0.0,
                    max_value: 10.0,
                },
            }],
            postprocessing: None,
            output: None,
        };

        let result = crate::process_netcdf_job(&config);
        assert!(result.is_err(), "Should fail with nonexistent dimension");
    }

    #[test]
    fn test_performance_benchmarking() -> Result<(), Box<dyn std::error::Error>> {
        use std::time::Instant;

        let temp_dir = tempdir()?;
        let output_path = temp_dir.path().join("performance_test.parquet");

        let start = Instant::now();
        let config = JobConfig {
            nc_key: get_test_data_path("simple_xy.nc")
                .to_string_lossy()
                .to_string(),
            variable_name: "data".to_string(),
            variable_names: None,
            parquet_key: output_path.to_string_lossy().to_string(),
            filters: vec![],
            postprocessing: None,
            output: None,
        };

        crate::process_netcdf_job(&config)?;
        let duration = start.elapsed();

        let output_path2 = temp_dir.path().join("performance_postprocess.parquet");
        let start = Instant::now();
        let config_with_processing = JobConfig {
            nc_key: get_test_data_path("simple_xy.nc")
                .to_string_lossy()
                .to_string(),
            variable_name: "data".to_string(),
            variable_names: None,
            parquet_key: output_path2.to_string_lossy().to_string(),
            filters: vec![],
            postprocessing: Some(crate::postprocess::ProcessingPipelineConfig {
                name: Some("Performance Test Pipeline".to_string()),
                processors: vec![
                    crate::postprocess::ProcessorConfig::RenameColumns {
                        mappings: {
                            let mut map = std::collections::HashMap::new();
                            map.insert("data".to_string(), "measurement".to_string());
                            map
                        },
                    },
                    crate::postprocess::ProcessorConfig::ApplyFormula {
                        target_column: "measurement_squared".to_string(),
                        formula: "measurement * measurement".to_string(),
                        source_columns: vec!["measurement".to_string()],
                    },
                ],
            }),
            output: None,
        };

        crate::process_netcdf_job(&config_with_processing)?;
        let duration_with_processing = start.elapsed();

        // On CI runners with tiny inputs, both durations are sub-millisecond
        // and noise dominates. Use a generous 50x ceiling to catch only
        // catastrophic regressions without flaky failures.
        assert!(
            duration_with_processing < duration * 50 + std::time::Duration::from_millis(100),
            "Post-processing should not add excessive overhead: base={:?}, with_postprocess={:?}",
            duration,
            duration_with_processing
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_async_vs_sync_performance() -> Result<(), Box<dyn std::error::Error>> {
        let temp_dir = tempdir()?;
        let sync_output = temp_dir.path().join("sync_performance.parquet");
        let async_output = temp_dir.path().join("async_performance.parquet");

        let config = JobConfig {
            nc_key: get_test_data_path("pres_temp_4D.nc")
                .to_string_lossy()
                .to_string(),
            variable_name: "temperature".to_string(),
            variable_names: None,
            parquet_key: sync_output.to_string_lossy().to_string(),
            filters: vec![],
            postprocessing: None,
            output: None,
        };

        crate::process_netcdf_job(&config)?;

        let mut async_config = config.clone();
        async_config.parquet_key = async_output.to_string_lossy().to_string();
        crate::process_netcdf_job_async(&async_config).await?;

        Ok(())
    }
}

#[cfg(test)]
mod s3_integration_tests {
    use crate::input::JobConfig;

    #[tokio::test]
    async fn test_public_s3_noaa_dataset_pipeline() -> Result<(), Box<dyn std::error::Error>> {
        // Uses public NOAA OpenData; skips gracefully if network access unavailable
        let noaa_s3_path = "s3://noaa-cdr-total-solar-irradiance-pds/data/daily/tsi_v02r01_daily_s18820101_e18821231_c20170717.nc";

        let temp_dir = tempfile::tempdir()?;
        let output_path = temp_dir.path().join("noaa_tsi_output.parquet");

        let info_result = crate::info::get_netcdf_info(noaa_s3_path, None, false).await;

        if info_result.is_err() {
            return Ok(());
        }

        let info = info_result?;
        assert!(!info.variables.is_empty());

        let variable_name = if info.variables.iter().any(|v| v.name == "tsi") {
            "tsi"
        } else if !info.variables.is_empty() {
            &info.variables[0].name
        } else {
            return Err("No variables found in NOAA dataset".into());
        };

        let json_config = format!(
            r#"{{
            "nc_key": "{}",
            "variable_name": "{}",
            "parquet_key": "{}",
            "filters": []
        }}"#,
            noaa_s3_path,
            variable_name,
            output_path.display()
        );

        let config = JobConfig::from_json(&json_config)?;

        crate::process_netcdf_job_async(&config).await?;

        assert!(output_path.exists());
        let output_metadata = std::fs::metadata(&output_path)?;
        assert!(output_metadata.len() > 0);

        Ok(())
    }

    #[tokio::test]
    async fn test_noaa_s3_info_command() -> Result<(), Box<dyn std::error::Error>> {
        let noaa_s3_path = "s3://noaa-cdr-total-solar-irradiance-pds/data/daily/tsi_v02r01_daily_s18820101_e18821231_c20170717.nc";

        let info_result = crate::info::get_netcdf_info(noaa_s3_path, None, false).await;

        if info_result.is_err() {
            return Ok(());
        }

        let info = info_result?;

        assert_eq!(info.path, noaa_s3_path);
        assert!(info.total_variables > 0);
        assert!(info.total_dimensions > 0);
        assert!(!info.dimensions.is_empty());
        assert!(!info.variables.is_empty());

        let _detailed_info = crate::info::get_netcdf_info(noaa_s3_path, None, true).await?;

        if let Some(first_var) = info.variables.first() {
            let var_info =
                crate::info::get_netcdf_info(noaa_s3_path, Some(&first_var.name), false).await?;
            assert_eq!(var_info.total_variables, 1);
            assert_eq!(var_info.variables[0].name, first_var.name);
        }

        Ok(())
    }
}

#[cfg(test)]
mod netcdf_exploration_tests {
    use crate::test_helpers::get_test_data_path;

    #[test]
    fn test_explore_netcdf_api() -> Result<(), Box<dyn std::error::Error>> {
        let file_path = get_test_data_path("pres_temp_4D.nc");
        let file = netcdf::open(&file_path)?;

        assert!(file.dimensions().count() > 0);
        assert!(file.variables().count() > 0);

        file.close()?;
        Ok(())
    }
}

#[cfg(test)]
mod error_and_edge_case_tests {
    use crate::input::{FilterConfig, JobConfig, RangeParams};
    use crate::postprocess::{
        PostProcessError, ProcessingPipeline, ProcessingPipelineConfig, ProcessorConfig, TimeUnit,
    };
    use crate::test_helpers::get_test_data_path;
    use polars::prelude::{ParquetReader, SerReader};
    use std::collections::HashMap;
    use tempfile::tempdir;

    #[test]
    fn test_pipeline_failure_mid_execution_missing_column() {
        let temp_dir = tempdir().unwrap();
        let output_path = temp_dir.path().join("should_not_exist.parquet");

        let config = JobConfig {
            nc_key: get_test_data_path("simple_xy.nc")
                .to_string_lossy()
                .to_string(),
            variable_name: "data".to_string(),
            variable_names: None,
            parquet_key: output_path.to_string_lossy().to_string(),
            filters: vec![],
            postprocessing: Some(ProcessingPipelineConfig {
                name: Some("Mid-Execution Failure Test".to_string()),
                processors: vec![
                    ProcessorConfig::RenameColumns {
                        mappings: {
                            let mut map = HashMap::new();
                            map.insert("data".to_string(), "renamed_data".to_string());
                            map
                        },
                    },
                    ProcessorConfig::UnitConvert {
                        column: "data".to_string(),
                        from_unit: "kelvin".to_string(),
                        to_unit: "celsius".to_string(),
                    },
                ],
            }),
            output: None,
        };

        let result = crate::process_netcdf_job(&config);
        assert!(
            result.is_err(),
            "Pipeline should fail when UnitConvert references a renamed column"
        );
        assert!(
            !output_path.exists(),
            "Output file must not be created when pipeline fails mid-execution"
        );
    }

    #[test]
    fn test_invalid_datetime_convert_base_returns_configuration_error() {
        let config = ProcessingPipelineConfig {
            name: Some("Invalid Datetime Config Test".to_string()),
            processors: vec![ProcessorConfig::DatetimeConvert {
                column: "time".to_string(),
                base: "not-a-date".to_string(),
                unit: TimeUnit::Hours,
            }],
        };

        let result = ProcessingPipeline::from_config(&config);
        assert!(
            result.is_err(),
            "from_config should fail with an invalid base datetime string"
        );

        let err = result.err().expect("result was Ok — checked above");
        match err {
            PostProcessError::ConfigurationError(msg) => {
                assert!(
                    msg.contains("not-a-date"),
                    "ConfigurationError message should reference the invalid base value, got: {}",
                    msg
                );
            }
            other => panic!(
                "Expected PostProcessError::ConfigurationError, got: {:?}",
                other
            ),
        }
    }

    // Latitude range 90–100 is outside pres_temp_4D.nc bounds (~25–50); yields empty Parquet
    #[test]
    fn test_filter_produces_empty_result_writes_empty_parquet()
    -> Result<(), Box<dyn std::error::Error>> {
        let temp_dir = tempdir()?;
        let output_path = temp_dir.path().join("empty_result.parquet");

        let config = JobConfig {
            nc_key: get_test_data_path("pres_temp_4D.nc")
                .to_string_lossy()
                .to_string(),
            variable_name: "temperature".to_string(),
            variable_names: None,
            parquet_key: output_path.to_string_lossy().to_string(),
            filters: vec![FilterConfig::Range {
                params: RangeParams {
                    dimension_name: "latitude".to_string(),
                    min_value: 90.0,
                    max_value: 100.0,
                },
            }],
            postprocessing: None,
            output: None,
        };

        crate::process_netcdf_job(&config)?;

        assert!(
            output_path.exists(),
            "Output Parquet file must be created even when filter yields zero rows"
        );

        let file = std::fs::File::open(&output_path)?;
        let df: polars::prelude::DataFrame = ParquetReader::new(file).finish()?;
        assert_eq!(
            df.height(),
            0,
            "DataFrame should have 0 rows when filter matches nothing; got {} rows",
            df.height()
        );

        Ok(())
    }

    #[test]
    fn test_job_config_from_file_nonexistent_path_returns_error() {
        let result = JobConfig::from_file("/nonexistent/path/to/config.json");
        assert!(
            result.is_err(),
            "from_file should fail for a nonexistent file path"
        );
    }

    #[test]
    fn test_job_config_from_json_malformed_returns_error() {
        let malformed = r#"{ this is not valid json }"#;
        let result = JobConfig::from_json(malformed);
        assert!(
            result.is_err(),
            "from_json should fail for syntactically invalid JSON"
        );

        let missing_fields = r#"{ "nc_key": "some_file.nc" }"#;
        let result2 = JobConfig::from_json(missing_fields);
        assert!(
            result2.is_err(),
            "from_json should fail when required fields (variable_name, parquet_key, filters) are absent"
        );
    }

    #[tokio::test]
    async fn test_async_pipeline_postprocessing_error_propagation() {
        let temp_dir = tempdir().unwrap();
        let output_path = temp_dir.path().join("async_error_test.parquet");

        let config = JobConfig {
            nc_key: get_test_data_path("simple_xy.nc")
                .to_string_lossy()
                .to_string(),
            variable_name: "data".to_string(),
            variable_names: None,
            parquet_key: output_path.to_string_lossy().to_string(),
            filters: vec![],
            postprocessing: Some(ProcessingPipelineConfig {
                name: Some("Async Error Propagation Test".to_string()),
                processors: vec![ProcessorConfig::UnitConvert {
                    column: "nonexistent_column".to_string(),
                    from_unit: "kelvin".to_string(),
                    to_unit: "celsius".to_string(),
                }],
            }),
            output: None,
        };

        let result = crate::process_netcdf_job_async(&config).await;
        assert!(
            result.is_err(),
            "Async pipeline should propagate the postprocessing error for a missing column"
        );
        assert!(
            !output_path.exists(),
            "Output file must not be created when async postprocessing fails"
        );
    }

    #[test]
    fn test_multiple_postprocessors_chained_correctly() -> Result<(), Box<dyn std::error::Error>> {
        let temp_dir = tempdir()?;
        let output_path = temp_dir.path().join("chained_postprocess.parquet");

        let config = JobConfig {
            nc_key: get_test_data_path("simple_xy.nc")
                .to_string_lossy()
                .to_string(),
            variable_name: "data".to_string(),
            variable_names: None,
            parquet_key: output_path.to_string_lossy().to_string(),
            filters: vec![],
            postprocessing: Some(ProcessingPipelineConfig {
                name: Some("Chained Postprocessors Test".to_string()),
                processors: vec![
                    ProcessorConfig::RenameColumns {
                        mappings: {
                            let mut map = HashMap::new();
                            map.insert("data".to_string(), "temp_k".to_string());
                            map
                        },
                    },
                    ProcessorConfig::ApplyFormula {
                        target_column: "temp_celsius".to_string(),
                        formula: "temp_k - 273.15".to_string(),
                        source_columns: vec!["temp_k".to_string()],
                    },
                    ProcessorConfig::UnitConvert {
                        column: "temp_k".to_string(),
                        from_unit: "kelvin".to_string(),
                        to_unit: "celsius".to_string(),
                    },
                ],
            }),
            output: None,
        };

        crate::process_netcdf_job(&config)?;

        assert!(output_path.exists(), "Output Parquet file must be created");

        let file = std::fs::File::open(&output_path)?;
        let df: polars::prelude::DataFrame = ParquetReader::new(file).finish()?;

        let column_names: Vec<String> = df
            .get_column_names()
            .iter()
            .map(|s: &&polars::prelude::PlSmallStr| s.to_string())
            .collect();

        assert!(
            column_names.contains(&"temp_k".to_string()),
            "Output must contain 'temp_k' column; got columns: {:?}",
            column_names
        );
        assert!(
            column_names.contains(&"temp_celsius".to_string()),
            "Output must contain 'temp_celsius' column; got columns: {:?}",
            column_names
        );
        assert!(
            !column_names.contains(&"data".to_string()),
            "Output must NOT contain original 'data' column after rename; got columns: {:?}",
            column_names
        );

        Ok(())
    }

    // pres_temp_4D.nc: time(2) * level(2) * latitude(6) * longitude(12) = 288 rows
    #[test]
    fn test_full_extraction_without_filters_verifies_row_count()
    -> Result<(), Box<dyn std::error::Error>> {
        let temp_dir = tempdir()?;
        let output_path = temp_dir.path().join("full_extraction.parquet");

        let config = JobConfig {
            nc_key: get_test_data_path("pres_temp_4D.nc")
                .to_string_lossy()
                .to_string(),
            variable_name: "temperature".to_string(),
            variable_names: None,
            parquet_key: output_path.to_string_lossy().to_string(),
            filters: vec![],
            postprocessing: Some(ProcessingPipelineConfig {
                name: Some("Full Extraction Postprocess Test".to_string()),
                processors: vec![
                    ProcessorConfig::RenameColumns {
                        mappings: {
                            let mut map = HashMap::new();
                            map.insert("temperature".to_string(), "temp_celsius".to_string());
                            map
                        },
                    },
                    ProcessorConfig::ApplyFormula {
                        target_column: "temp_kelvin".to_string(),
                        formula: "temp_celsius + 273.15".to_string(),
                        source_columns: vec!["temp_celsius".to_string()],
                    },
                ],
            }),
            output: None,
        };

        crate::process_netcdf_job(&config)?;

        assert!(output_path.exists(), "Output Parquet file must be created");

        let file = std::fs::File::open(&output_path)?;
        let df: polars::prelude::DataFrame = ParquetReader::new(file).finish()?;

        assert_eq!(
            df.height(),
            288,
            "Full extraction of pres_temp_4D.nc temperature must yield exactly 288 rows; got {}",
            df.height()
        );

        let column_names: Vec<String> = df
            .get_column_names()
            .iter()
            .map(|s: &&polars::prelude::PlSmallStr| s.to_string())
            .collect();

        let expected_columns = [
            "time",
            "level",
            "latitude",
            "longitude",
            "temp_celsius",
            "temp_kelvin",
        ];
        for expected in &expected_columns {
            assert!(
                column_names.contains(&expected.to_string()),
                "Output must contain column '{}'; got columns: {:?}",
                expected,
                column_names
            );
        }

        Ok(())
    }
}
