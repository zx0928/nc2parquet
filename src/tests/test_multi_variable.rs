#[cfg(test)]
mod multi_variable_tests {
    use crate::input::JobConfig;
    use crate::test_helpers::get_test_data_path;
    use polars::prelude::{ParquetReader, SerReader};
    use tempfile::tempdir;

    // -----------------------------------------------------------------------
    // effective_variable_names
    // -----------------------------------------------------------------------

    #[test]
    fn test_effective_variable_names_falls_back_to_variable_name() {
        let config = JobConfig::from_json(
            r#"{
            "nc_key": "input.nc",
            "variable_name": "temperature",
            "parquet_key": "output.parquet",
            "filters": []
        }"#,
        )
        .unwrap();

        // variable_names is absent (None) — must fall back to variable_name
        assert!(config.variable_names.is_none());
        let names = config.effective_variable_names();
        assert_eq!(names, vec!["temperature".to_string()]);
    }

    #[test]
    fn test_effective_variable_names_returns_list_when_set() {
        let config = JobConfig::from_json(
            r#"{
            "nc_key": "input.nc",
            "variable_name": "temperature",
            "variable_names": ["temperature", "pressure"],
            "parquet_key": "output.parquet",
            "filters": []
        }"#,
        )
        .unwrap();

        let names = config.effective_variable_names();
        assert_eq!(
            names,
            vec!["temperature".to_string(), "pressure".to_string()]
        );
    }

    #[test]
    fn test_effective_variable_names_single_element_list() {
        let config = JobConfig::from_json(
            r#"{
            "nc_key": "input.nc",
            "variable_name": "temperature",
            "variable_names": ["temperature"],
            "parquet_key": "output.parquet",
            "filters": []
        }"#,
        )
        .unwrap();

        let names = config.effective_variable_names();
        assert_eq!(names, vec!["temperature".to_string()]);
    }

    // -----------------------------------------------------------------------
    // Backward compatibility: configs without variable_names deserialize OK
    // -----------------------------------------------------------------------

    #[test]
    fn test_backward_compatibility_config_without_variable_names() {
        // Old-style config with no variable_names field — must deserialize cleanly
        // and variable_names must be None.
        let json = r#"{
            "nc_key": "data/temperature.nc",
            "variable_name": "t2m",
            "parquet_key": "output/temperature.parquet",
            "filters": []
        }"#;

        let config = JobConfig::from_json(json).unwrap();
        assert!(
            config.variable_names.is_none(),
            "variable_names must default to None when absent in JSON"
        );
        assert_eq!(config.effective_variable_names(), vec!["t2m".to_string()]);
    }

    #[test]
    fn test_backward_compatibility_config_round_trip_serialization() {
        // Configs without variable_names must NOT emit the field in JSON output
        // (skip_serializing_if = "Option::is_none").
        let json = r#"{
            "nc_key": "input.nc",
            "variable_name": "temperature",
            "parquet_key": "output.parquet",
            "filters": []
        }"#;

        let config = JobConfig::from_json(json).unwrap();
        let serialized = serde_json::to_string(&config).unwrap();

        assert!(
            !serialized.contains("variable_names"),
            "variable_names must not appear in serialized JSON when None; got: {}",
            serialized
        );
    }

    // -----------------------------------------------------------------------
    // Multi-variable happy path: pres_temp_4D.nc
    // -----------------------------------------------------------------------

    #[test]
    fn test_multi_variable_happy_path_pres_temp_4d() -> Result<(), Box<dyn std::error::Error>> {
        // pres_temp_4D.nc has "temperature" and "pressure" with identical dimensions:
        // time(2) * level(2) * latitude(6) * longitude(12) = 288 rows
        let temp_dir = tempdir()?;
        let output_path = temp_dir.path().join("multi_var_output.parquet");

        let config = JobConfig {
            nc_key: get_test_data_path("pres_temp_4D.nc")
                .to_string_lossy()
                .to_string(),
            variable_name: "temperature".to_string(),
            variable_names: Some(vec!["temperature".to_string(), "pressure".to_string()]),
            parquet_key: output_path.to_string_lossy().to_string(),
            filters: vec![],
            postprocessing: None,
            output: None,
        };

        crate::process_netcdf_job(&config)?;

        assert!(output_path.exists(), "Output Parquet file must be created");

        let file = std::fs::File::open(&output_path)?;
        let df = ParquetReader::new(file).finish()?;

        // 288 rows (full Cartesian product)
        assert_eq!(
            df.height(),
            288,
            "Multi-variable extraction must yield 288 rows; got {}",
            df.height()
        );

        // 6 columns: time, level, latitude, longitude, temperature, pressure
        assert_eq!(
            df.width(),
            6,
            "DataFrame must have 6 columns (4 dims + 2 variables); got {}",
            df.width()
        );

        let col_names: Vec<String> = df
            .get_column_names()
            .iter()
            .map(|s| s.to_string())
            .collect();

        for expected in &[
            "time",
            "level",
            "latitude",
            "longitude",
            "temperature",
            "pressure",
        ] {
            assert!(
                col_names.contains(&expected.to_string()),
                "Output must contain column '{}'; got columns: {:?}",
                expected,
                col_names
            );
        }

        Ok(())
    }

    #[test]
    fn test_multi_variable_single_variable_via_list_matches_single_extraction()
    -> Result<(), Box<dyn std::error::Error>> {
        // variable_names = Some(["temperature"]) must produce the same result as
        // variable_name = "temperature" with variable_names = None.
        let temp_dir = tempdir()?;
        let single_path = temp_dir.path().join("single_var.parquet");
        let list_path = temp_dir.path().join("list_single_var.parquet");

        // Single-variable path (classic mode)
        let single_config = JobConfig {
            nc_key: get_test_data_path("pres_temp_4D.nc")
                .to_string_lossy()
                .to_string(),
            variable_name: "temperature".to_string(),
            variable_names: None,
            parquet_key: single_path.to_string_lossy().to_string(),
            filters: vec![],
            postprocessing: None,
            output: None,
        };
        crate::process_netcdf_job(&single_config)?;

        // Same variable, but via variable_names = Some(["temperature"])
        let list_config = JobConfig {
            nc_key: get_test_data_path("pres_temp_4D.nc")
                .to_string_lossy()
                .to_string(),
            variable_name: "temperature".to_string(),
            variable_names: Some(vec!["temperature".to_string()]),
            parquet_key: list_path.to_string_lossy().to_string(),
            filters: vec![],
            postprocessing: None,
            output: None,
        };
        crate::process_netcdf_job(&list_config)?;

        let single_df = ParquetReader::new(std::fs::File::open(&single_path)?).finish()?;
        let list_df = ParquetReader::new(std::fs::File::open(&list_path)?).finish()?;

        assert_eq!(
            single_df.height(),
            list_df.height(),
            "Row counts must match: single={}, list={}",
            single_df.height(),
            list_df.height()
        );
        assert_eq!(
            single_df.width(),
            list_df.width(),
            "Column counts must match: single={}, list={}",
            single_df.width(),
            list_df.width()
        );

        Ok(())
    }

    // -----------------------------------------------------------------------
    // Error: variable not found
    // -----------------------------------------------------------------------

    #[test]
    fn test_multi_variable_variable_not_found_returns_error() {
        let temp_dir = tempdir().unwrap();
        let output_path = temp_dir.path().join("should_not_exist.parquet");

        let config = JobConfig {
            nc_key: get_test_data_path("pres_temp_4D.nc")
                .to_string_lossy()
                .to_string(),
            variable_name: "temperature".to_string(),
            variable_names: Some(vec![
                "temperature".to_string(),
                "nonexistent_variable".to_string(),
            ]),
            parquet_key: output_path.to_string_lossy().to_string(),
            filters: vec![],
            postprocessing: None,
            output: None,
        };

        let result = crate::process_netcdf_job(&config);
        assert!(
            result.is_err(),
            "Process must fail when a listed variable does not exist"
        );
        assert!(
            !output_path.exists(),
            "Output file must not be created when extraction fails"
        );

        // Error must be VariableNotFound
        let err = result.unwrap_err();
        match err {
            crate::Nc2ParquetError::VariableNotFound(name) => {
                assert_eq!(
                    name, "nonexistent_variable",
                    "Error must reference the missing variable name"
                );
            }
            other => panic!("Expected VariableNotFound error, got: {:?}", other),
        }
    }

    #[test]
    fn test_multi_variable_first_variable_not_found_returns_error() {
        // First variable is also checked via VariableNotFound
        let temp_dir = tempdir().unwrap();
        let output_path = temp_dir.path().join("should_not_exist.parquet");

        let config = JobConfig {
            nc_key: get_test_data_path("pres_temp_4D.nc")
                .to_string_lossy()
                .to_string(),
            variable_name: "temperature".to_string(),
            variable_names: Some(vec![
                "no_such_variable".to_string(),
                "temperature".to_string(),
            ]),
            parquet_key: output_path.to_string_lossy().to_string(),
            filters: vec![],
            postprocessing: None,
            output: None,
        };

        let result = crate::process_netcdf_job(&config);
        assert!(
            result.is_err(),
            "Process must fail when the first listed variable does not exist"
        );
    }

    // -----------------------------------------------------------------------
    // Multi-variable async path
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn test_multi_variable_async_path() -> Result<(), Box<dyn std::error::Error>> {
        let temp_dir = tempdir()?;
        let output_path = temp_dir.path().join("multi_var_async.parquet");

        let config = JobConfig {
            nc_key: get_test_data_path("pres_temp_4D.nc")
                .to_string_lossy()
                .to_string(),
            variable_name: "temperature".to_string(),
            variable_names: Some(vec!["temperature".to_string(), "pressure".to_string()]),
            parquet_key: output_path.to_string_lossy().to_string(),
            filters: vec![],
            postprocessing: None,
            output: None,
        };

        crate::process_netcdf_job_async(&config).await?;

        assert!(output_path.exists(), "Async output Parquet must be created");

        let file = std::fs::File::open(&output_path)?;
        let df = ParquetReader::new(file).finish()?;

        assert_eq!(df.height(), 288, "Async multi-var: expected 288 rows");
        assert_eq!(df.width(), 6, "Async multi-var: expected 6 columns");

        Ok(())
    }
}
