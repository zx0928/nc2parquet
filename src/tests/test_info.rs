#[cfg(test)]
mod info_command_tests {
    use crate::info::{NetCdfDimensionInfo, NetCdfInfo, NetCdfVariableInfo, get_netcdf_info};
    use crate::test_helpers::get_test_data_path;

    #[tokio::test]
    async fn test_get_netcdf_info_basic() -> Result<(), Box<dyn std::error::Error>> {
        let file_path = get_test_data_path("pres_temp_4D.nc");
        let info = get_netcdf_info(&file_path.to_string_lossy(), None, false).await?;

        assert_eq!(info.path, file_path.to_string_lossy());
        assert_eq!(info.total_dimensions, 4);
        assert_eq!(info.total_variables, 4);

        // Check dimensions
        let dim_names: Vec<&str> = info.dimensions.iter().map(|d| d.name.as_str()).collect();
        assert!(dim_names.contains(&"level"));
        assert!(dim_names.contains(&"latitude"));
        assert!(dim_names.contains(&"longitude"));
        assert!(dim_names.contains(&"time"));

        // Check variables
        let var_names: Vec<&str> = info.variables.iter().map(|v| v.name.as_str()).collect();
        assert!(var_names.contains(&"latitude"));
        assert!(var_names.contains(&"longitude"));
        assert!(var_names.contains(&"pressure"));
        assert!(var_names.contains(&"temperature"));

        Ok(())
    }

    #[tokio::test]
    async fn test_get_netcdf_info_detailed() -> Result<(), Box<dyn std::error::Error>> {
        let file_path = get_test_data_path("pres_temp_4D.nc");
        // Verify detailed mode doesn't error
        get_netcdf_info(&file_path.to_string_lossy(), None, true).await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_get_netcdf_info_specific_variable() -> Result<(), Box<dyn std::error::Error>> {
        let file_path = get_test_data_path("pres_temp_4D.nc");
        let info =
            get_netcdf_info(&file_path.to_string_lossy(), Some("temperature"), false).await?;

        assert_eq!(info.total_variables, 1);
        assert_eq!(info.variables[0].name, "temperature");
        assert_eq!(
            info.variables[0].dimensions,
            vec!["time", "level", "latitude", "longitude"]
        );
        assert!(info.variables[0].attributes.contains_key("units"));

        Ok(())
    }

    #[tokio::test]
    async fn test_get_netcdf_info_simple_xy() -> Result<(), Box<dyn std::error::Error>> {
        let file_path = get_test_data_path("simple_xy.nc");
        let info = get_netcdf_info(&file_path.to_string_lossy(), None, false).await?;

        assert_eq!(info.total_dimensions, 2);
        assert_eq!(info.total_variables, 1);
        assert_eq!(info.variables[0].name, "data");
        assert_eq!(info.variables[0].dimensions, vec!["x", "y"]);

        Ok(())
    }

    #[tokio::test]
    async fn test_get_netcdf_info_error_handling() {
        let result = get_netcdf_info("nonexistent.nc", None, false).await;
        assert!(result.is_err());

        std::fs::write("test_invalid.nc", "not a netcdf file").unwrap();
        let result = get_netcdf_info("test_invalid.nc", None, false).await;
        assert!(result.is_err());

        let _ = std::fs::remove_file("test_invalid.nc");
    }

    #[test]
    fn test_dimension_info_structure() {
        let dim = NetCdfDimensionInfo {
            name: "time".to_string(),
            length: 10,
            is_unlimited: true,
        };

        assert_eq!(dim.name, "time");
        assert_eq!(dim.length, 10);
        assert!(dim.is_unlimited);
    }

    #[test]
    fn test_variable_info_structure() {
        let mut attributes = std::collections::HashMap::new();
        attributes.insert("units".to_string(), "celsius".to_string());

        let var = NetCdfVariableInfo {
            name: "temperature".to_string(),
            data_type: "Float(F32)".to_string(),
            dimensions: vec!["time".to_string(), "lat".to_string()],
            attributes,
            shape: vec![10, 20],
        };

        assert_eq!(var.name, "temperature");
        assert_eq!(var.data_type, "Float(F32)");
        assert_eq!(var.dimensions.len(), 2);
        assert_eq!(var.shape, vec![10, 20]);
        assert!(var.attributes.contains_key("units"));
    }

    #[test]
    fn test_format_output_json() -> Result<(), Box<dyn std::error::Error>> {
        let info = create_test_netcdf_info();

        // Test JSON serialization
        let json = serde_json::to_string_pretty(&info)?;
        assert!(json.contains("test.nc"));
        assert!(json.contains("temperature"));

        Ok(())
    }

    #[test]
    fn test_format_output_yaml() -> Result<(), Box<dyn std::error::Error>> {
        let info = create_test_netcdf_info();

        let yaml = serde_yaml::to_string(&info)?;
        assert!(yaml.contains("test.nc"));
        assert!(yaml.contains("temperature"));

        Ok(())
    }

    fn create_test_netcdf_info() -> NetCdfInfo {
        let mut attributes = std::collections::HashMap::new();
        attributes.insert("units".to_string(), "celsius".to_string());

        let variables = vec![NetCdfVariableInfo {
            name: "temperature".to_string(),
            data_type: "Float(F32)".to_string(),
            dimensions: vec!["time".to_string(), "lat".to_string()],
            attributes,
            shape: vec![10, 20],
        }];

        let dimensions = vec![
            NetCdfDimensionInfo {
                name: "time".to_string(),
                length: 10,
                is_unlimited: true,
            },
            NetCdfDimensionInfo {
                name: "lat".to_string(),
                length: 20,
                is_unlimited: false,
            },
        ];

        NetCdfInfo {
            path: "test.nc".to_string(),
            dimensions,
            variables,
            global_attributes: std::collections::HashMap::new(),
            file_size: Some(1024),
            total_variables: 1,
            total_dimensions: 2,
        }
    }
}
