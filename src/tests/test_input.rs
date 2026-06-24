#[cfg(test)]
mod input_tests {
    use crate::input::{FilterConfig, JobConfig, OutputTarget};

    #[test]
    fn test_job_config_from_json() {
        let json = r#"
        {
            "nc_key": "examples/data/simple_xy.nc",
            "variable_name": "data",
            "output": {"type": "parquet", "parquet_key": "test.parquet"},
            "filters": [
                {
                    "kind": "range",
                    "params": {
                        "dimension_name": "x",
                        "min_value": 1.0,
                        "max_value": 4.0
                    }
                }
            ]
        }"#;

        let config = JobConfig::from_json(json).unwrap();
        assert_eq!(config.nc_key, "examples/data/simple_xy.nc");
        assert_eq!(config.variable_name, "data");
        assert_eq!(config.output.output_path(), "test.parquet");
        assert_eq!(config.filters.len(), 1);
    }

    #[test]
    fn test_filter_config_range() {
        let json = r#"
        {
            "kind": "range",
            "params": {
                "dimension_name": "time",
                "min_value": 0.0,
                "max_value": 1.0
            }
        }"#;

        let filter_config: FilterConfig = serde_json::from_str(json).unwrap();
        assert_eq!(filter_config.kind(), "range");

        filter_config.to_filter().unwrap();
    }

    #[test]
    fn test_filter_config_2d_point() {
        let json = r#"
        {
            "kind": "2d_point",
            "params": {
                "lat_dimension_name": "latitude",
                "lon_dimension_name": "longitude",
                "points": [
                    [30.0, -120.0],
                    [40.0, -100.0]
                ],
                "tolerance": 5.0
            }
        }"#;

        let filter_config: FilterConfig = serde_json::from_str(json).unwrap();
        assert_eq!(filter_config.kind(), "2d_point");

        filter_config.to_filter().unwrap();
    }

    #[test]
    fn test_filter_config_list() {
        let json = r#"
        {
            "kind": "list",
            "params": {
                "dimension_name": "level",
                "values": [0.0, 1.0]
            }
        }"#;

        let filter_config: FilterConfig = serde_json::from_str(json).unwrap();
        assert_eq!(filter_config.kind(), "list");

        filter_config.to_filter().unwrap();
    }

    #[test]
    fn test_filter_config_3d_point() {
        let json = r#"
        {
            "kind": "3d_point",
            "params": {
                "time_dimension_name": "time",
                "lat_dimension_name": "latitude",
                "lon_dimension_name": "longitude",
                "steps": [0.0, 1.0],
                "points": [[35.0, -110.0], [45.0, -85.0]],
                "tolerance": 5.0
            }
        }"#;

        let filter_config: FilterConfig = serde_json::from_str(json).unwrap();
        assert_eq!(filter_config.kind(), "3d_point");

        filter_config.to_filter().unwrap();
    }

    #[test]
    fn test_multiple_filters_config_with_real_data() {
        let json = r#"
        {
            "nc_key": "examples/data/pres_temp_4D.nc",
            "variable_name": "temperature",
            "parquet_key": "filtered_output.parquet",
            "filters": [
                {
                    "kind": "range",
                    "params": {
                        "dimension_name": "time",
                        "min_value": 0.0,
                        "max_value": 1.0
                    }
                },
                {
                    "kind": "2d_point",
                    "params": {
                        "lat_dimension_name": "latitude",
                        "lon_dimension_name": "longitude",
                        "points": [[30.0, -120.0]],
                        "tolerance": 5.0
                    }
                }
            ]
        }"#;

        let config = JobConfig::from_json(json).unwrap();
        assert_eq!(config.filters.len(), 2);
        assert_eq!(config.filters[0].kind(), "range");
        assert_eq!(config.filters[1].kind(), "2d_point");
    }
}

#[cfg(test)]
mod utility_tests {
    use crate::input::{FilterConfig, JobConfig};

    #[test]
    fn test_json_parsing_errors() {
        let invalid_json = "{ invalid json }";
        let result = JobConfig::from_json(invalid_json);
        assert!(result.is_err());

        let incomplete_json = r#"
        {
            "nc_key": "test.nc"
        }"#;
        let result = JobConfig::from_json(incomplete_json);
        assert!(result.is_err());
    }

    #[test]
    fn test_filter_config_invalid_kind() {
        let invalid_filter = r#"
        {
            "kind": "invalid_filter_type",
            "params": {}
        }"#;

        let result: Result<FilterConfig, _> = serde_json::from_str(invalid_filter);
        assert!(result.is_err());
    }

    #[test]
    fn test_empty_filters_array() {
        let json = r#"
        {
            "nc_key": "test.nc",
            "variable_name": "temp",
            "parquet_key": "test.parquet",
            "filters": []
        }"#;

        let config = JobConfig::from_json(json).unwrap();
        assert_eq!(config.filters.len(), 0);
    }
}
