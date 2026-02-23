#[cfg(test)]
mod extract_tests {
    use crate::extract::{extract_data_to_dataframe, DimensionIndexManager};
    use crate::filters::{FilterResult, NC2DPointFilter, NCFilter, NCRangeFilter};
    use crate::test_helpers::get_test_data_path;

    #[test]
    fn test_dimension_index_manager_with_simple_data() -> Result<(), Box<dyn std::error::Error>> {
        let file_path = get_test_data_path("simple_xy.nc");
        let file = netcdf::open(&file_path)?;
        let var = file.variable("data").unwrap();

        let manager = DimensionIndexManager::new(&var)?;
        let dimensions = manager.get_dimension_order();

        assert_eq!(dimensions.len(), 2);
        assert!(dimensions.contains(&"x".to_string()));
        assert!(dimensions.contains(&"y".to_string()));

        file.close()?;
        Ok(())
    }

    #[test]
    fn test_dimension_index_manager_with_4d_data() -> Result<(), Box<dyn std::error::Error>> {
        let file_path = get_test_data_path("pres_temp_4D.nc");
        let file = netcdf::open(&file_path)?;
        let var = file.variable("temperature").unwrap();

        let manager = DimensionIndexManager::new(&var)?;
        let dimensions = manager.get_dimension_order();

        assert_eq!(dimensions.len(), 4);
        assert!(dimensions.contains(&"time".to_string()));
        assert!(dimensions.contains(&"level".to_string()));
        assert!(dimensions.contains(&"latitude".to_string()));
        assert!(dimensions.contains(&"longitude".to_string()));

        file.close()?;
        Ok(())
    }

    #[test]
    fn test_dimension_index_manager_filter_application() -> Result<(), Box<dyn std::error::Error>> {
        let file_path = get_test_data_path("pres_temp_4D.nc");
        let file = netcdf::open(&file_path)?;
        let var = file.variable("temperature").unwrap();

        let mut manager = DimensionIndexManager::new(&var)?;

        let filter_result = FilterResult::Single {
            dimension: "time".to_string(),
            indices: vec![0, 1],
        };

        manager.apply_filter_result(&filter_result)?;

        let time_indices = manager.get_dimension_indices("time").unwrap();
        assert_eq!(time_indices.len(), 2);
        assert!(time_indices.contains(&0));
        assert!(time_indices.contains(&1));

        file.close()?;
        Ok(())
    }

    #[test]
    fn test_extract_data_to_dataframe_simple() -> Result<(), Box<dyn std::error::Error>> {
        let file_path = get_test_data_path("simple_xy.nc");
        let file = netcdf::open(&file_path)?;
        let var = file.variable("data").unwrap();

        let filters: Vec<Box<dyn NCFilter>> = vec![];
        let df = extract_data_to_dataframe(&file, &var, "data", &filters)?;

        // 6 * 12 = 72 rows
        assert_eq!(df.height(), 72);

        let column_names: Vec<String> = df
            .get_column_names()
            .iter()
            .map(|s| s.to_string())
            .collect();
        assert!(column_names.contains(&"x".to_string()));
        assert!(column_names.contains(&"y".to_string()));
        assert!(column_names.contains(&"data".to_string()));

        file.close()?;
        Ok(())
    }

    #[test]
    fn test_extract_data_to_dataframe_with_filter() -> Result<(), Box<dyn std::error::Error>> {
        let file_path = get_test_data_path("pres_temp_4D.nc");
        let file = netcdf::open(&file_path)?;
        let var = file.variable("temperature").unwrap();

        let filter = NCRangeFilter::new("latitude", 30.0, 40.0);
        let filters: Vec<Box<dyn NCFilter>> = vec![Box::new(filter)];

        let df = extract_data_to_dataframe(&file, &var, "temperature", &filters)?;

        // 2 time steps * 2 levels * 3 lats * 12 lons = 144 rows
        assert_eq!(df.height(), 144);

        let column_names: Vec<String> = df
            .get_column_names()
            .iter()
            .map(|s| s.to_string())
            .collect();
        assert!(column_names.contains(&"time".to_string()));
        assert!(column_names.contains(&"level".to_string()));
        assert!(column_names.contains(&"latitude".to_string()));
        assert!(column_names.contains(&"longitude".to_string()));
        assert!(column_names.contains(&"temperature".to_string()));

        file.close()?;
        Ok(())
    }

    #[test]
    fn test_extract_data_to_dataframe_with_spatial_filter() -> Result<(), Box<dyn std::error::Error>>
    {
        let file_path = get_test_data_path("pres_temp_4D.nc");
        let file = netcdf::open(&file_path)?;
        let var = file.variable("temperature").unwrap();

        let points = vec![(30.0, -120.0)];
        let filter = NC2DPointFilter::new("latitude", "longitude", points, 1.0);
        let filters: Vec<Box<dyn NCFilter>> = vec![Box::new(filter)];

        let df = extract_data_to_dataframe(&file, &var, "temperature", &filters)?;

        // 2 time steps * 2 levels * 1 coordinate pair = 4 rows
        assert_eq!(df.height(), 4);

        file.close()?;
        Ok(())
    }
}

#[cfg(test)]
mod dim_index_manager_initial_state_tests {
    use crate::extract::DimensionIndexManager;
    use crate::test_helpers::get_test_data_path;

    #[test]
    fn test_initial_indices_match_dimension_sizes() -> Result<(), Box<dyn std::error::Error>> {
        // pres_temp_4D.nc: time(2), level(2), latitude(6), longitude(12)
        let file_path = get_test_data_path("pres_temp_4D.nc");
        let file = netcdf::open(&file_path)?;
        let var = file.variable("temperature").unwrap();

        let manager = DimensionIndexManager::new(&var)?;

        let dim_order = manager.get_dimension_order();
        assert_eq!(dim_order.len(), 4);

        let time_indices = manager.get_dimension_indices("time").unwrap();
        assert_eq!(
            time_indices.len(),
            2,
            "time dimension should have 2 indices"
        );
        assert!(time_indices.contains(&0));
        assert!(time_indices.contains(&1));

        let level_indices = manager.get_dimension_indices("level").unwrap();
        assert_eq!(
            level_indices.len(),
            2,
            "level dimension should have 2 indices"
        );
        assert!(level_indices.contains(&0));
        assert!(level_indices.contains(&1));

        let lat_indices = manager.get_dimension_indices("latitude").unwrap();
        assert_eq!(
            lat_indices.len(),
            6,
            "latitude dimension should have 6 indices"
        );
        for i in 0..6 {
            assert!(
                lat_indices.contains(&i),
                "latitude index {i} should be present"
            );
        }

        let lon_indices = manager.get_dimension_indices("longitude").unwrap();
        assert_eq!(
            lon_indices.len(),
            12,
            "longitude dimension should have 12 indices"
        );
        for i in 0..12 {
            assert!(
                lon_indices.contains(&i),
                "longitude index {i} should be present"
            );
        }

        file.close()?;
        Ok(())
    }
}

#[cfg(test)]
mod filter_intersection_tests {
    use crate::extract::DimensionIndexManager;
    use crate::filters::FilterResult;
    use crate::test_helpers::get_test_data_path;

    #[test]
    fn test_two_single_filters_narrow_independent_dimensions(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let file_path = get_test_data_path("pres_temp_4D.nc");
        let file = netcdf::open(&file_path)?;
        let var = file.variable("temperature").unwrap();

        let mut manager = DimensionIndexManager::new(&var)?;

        let time_filter = FilterResult::Single {
            dimension: "time".to_string(),
            indices: vec![0],
        };
        manager.apply_filter_result(&time_filter)?;

        let lat_filter = FilterResult::Single {
            dimension: "latitude".to_string(),
            indices: vec![1, 2, 3],
        };
        manager.apply_filter_result(&lat_filter)?;

        let time_indices = manager.get_dimension_indices("time").unwrap();
        assert_eq!(time_indices.len(), 1);
        assert!(time_indices.contains(&0));

        let lat_indices = manager.get_dimension_indices("latitude").unwrap();
        assert_eq!(lat_indices.len(), 3);
        assert!(lat_indices.contains(&1));
        assert!(lat_indices.contains(&2));
        assert!(lat_indices.contains(&3));

        let level_indices = manager.get_dimension_indices("level").unwrap();
        assert_eq!(level_indices.len(), 2);

        let lon_indices = manager.get_dimension_indices("longitude").unwrap();
        assert_eq!(lon_indices.len(), 12);

        file.close()?;
        Ok(())
    }

    #[test]
    fn test_single_filter_with_empty_indices_empties_dimension(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let file_path = get_test_data_path("pres_temp_4D.nc");
        let file = netcdf::open(&file_path)?;
        let var = file.variable("temperature").unwrap();

        let mut manager = DimensionIndexManager::new(&var)?;

        let empty_filter = FilterResult::Single {
            dimension: "latitude".to_string(),
            indices: vec![],
        };
        manager.apply_filter_result(&empty_filter)?;

        let lat_indices = manager.get_dimension_indices("latitude").unwrap();
        assert!(
            lat_indices.is_empty(),
            "latitude indices should be empty after empty-filter intersection"
        );

        let time_indices = manager.get_dimension_indices("time").unwrap();
        assert_eq!(time_indices.len(), 2);

        file.close()?;
        Ok(())
    }

    #[test]
    fn test_single_filter_unknown_dimension_returns_error() -> Result<(), Box<dyn std::error::Error>>
    {
        let file_path = get_test_data_path("pres_temp_4D.nc");
        let file = netcdf::open(&file_path)?;
        let var = file.variable("temperature").unwrap();

        let mut manager = DimensionIndexManager::new(&var)?;

        let bad_filter = FilterResult::Single {
            dimension: "nonexistent".to_string(),
            indices: vec![0, 1],
        };
        let result = manager.apply_filter_result(&bad_filter);

        assert!(result.is_err(), "Expected Err for unknown dimension");
        let msg = result.unwrap_err().to_string();
        assert!(
            msg.contains("nonexistent"),
            "Error message should mention the unknown dimension name, got: {msg}"
        );

        file.close()?;
        Ok(())
    }
}

#[cfg(test)]
mod explicit_combination_tests {
    use crate::extract::DimensionIndexManager;
    use crate::filters::FilterResult;
    use crate::test_helpers::get_test_data_path;

    #[test]
    fn test_pairs_filter_stores_explicit_coordinate_combinations(
    ) -> Result<(), Box<dyn std::error::Error>> {
        // pairs: [(1,1), (2,3)] × time: {0,1} × level: {0,1} = 8 combinations
        let file_path = get_test_data_path("pres_temp_4D.nc");
        let file = netcdf::open(&file_path)?;
        let var = file.variable("temperature").unwrap();

        let mut manager = DimensionIndexManager::new(&var)?;

        let pairs_result = FilterResult::Pairs {
            lat_dimension: "latitude".to_string(),
            lon_dimension: "longitude".to_string(),
            pairs: vec![(1, 1), (2, 3)],
        };
        manager.apply_filter_result(&pairs_result)?;

        let combinations = manager.get_all_coordinate_combinations();
        assert_eq!(
            combinations.len(),
            8,
            "Expected 2 pairs × 2 time × 2 level = 8 combinations, got {}",
            combinations.len()
        );

        for combo in &combinations {
            assert_eq!(
                combo.len(),
                4,
                "Each combination should have 4 indices (one per dimension)"
            );
        }

        file.close()?;
        Ok(())
    }

    #[test]
    fn test_triplets_filter_stores_one_combination_per_triplet(
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Triplets fix (time, lat, lon); no cross-product with remaining dimensions
        let file_path = get_test_data_path("pres_temp_4D.nc");
        let file = netcdf::open(&file_path)?;
        let var = file.variable("temperature").unwrap();

        let mut manager = DimensionIndexManager::new(&var)?;

        let triplets_result = FilterResult::Triplets {
            time_dimension: "time".to_string(),
            lat_dimension: "latitude".to_string(),
            lon_dimension: "longitude".to_string(),
            triplets: vec![(0, 1, 1), (1, 3, 5)],
        };
        manager.apply_filter_result(&triplets_result)?;

        let combinations = manager.get_all_coordinate_combinations();
        assert_eq!(
            combinations.len(),
            2,
            "Expected one combination per triplet (2 total), got {}",
            combinations.len()
        );

        for combo in &combinations {
            assert_eq!(
                combo.len(),
                4,
                "Each combination should have 4 indices (one per dimension)"
            );
        }

        file.close()?;
        Ok(())
    }
}

#[cfg(test)]
mod extraction_edge_case_tests {
    use crate::extract::extract_data_to_dataframe;
    use crate::filters::{NCFilter, NCListFilter, NCRangeFilter};
    use crate::test_helpers::get_test_data_path;

    #[test]
    fn test_zero_result_extraction_returns_empty_dataframe_with_correct_columns(
    ) -> Result<(), Box<dyn std::error::Error>> {
        // latitude range 100–200 matches no values in [25,30,35,40,45,50]
        let file_path = get_test_data_path("pres_temp_4D.nc");
        let file = netcdf::open(&file_path)?;
        let var = file.variable("temperature").unwrap();

        let filter = NCRangeFilter::new("latitude", 100.0, 200.0);
        let filters: Vec<Box<dyn NCFilter>> = vec![Box::new(filter)];

        let df = extract_data_to_dataframe(&file, &var, "temperature", &filters)?;

        assert_eq!(
            df.height(),
            0,
            "DataFrame should have 0 rows when filter matches nothing"
        );

        let column_names: Vec<String> = df
            .get_column_names()
            .iter()
            .map(|s| s.to_string())
            .collect();
        assert!(column_names.contains(&"time".to_string()));
        assert!(column_names.contains(&"level".to_string()));
        assert!(column_names.contains(&"latitude".to_string()));
        assert!(column_names.contains(&"longitude".to_string()));
        assert!(column_names.contains(&"temperature".to_string()));

        file.close()?;
        Ok(())
    }

    #[test]
    fn test_full_4d_extraction_without_filters_returns_all_rows(
    ) -> Result<(), Box<dyn std::error::Error>> {
        // 2×2×6×12 = 288 combinations
        let file_path = get_test_data_path("pres_temp_4D.nc");
        let file = netcdf::open(&file_path)?;
        let var = file.variable("temperature").unwrap();

        let filters: Vec<Box<dyn NCFilter>> = vec![];
        let df = extract_data_to_dataframe(&file, &var, "temperature", &filters)?;

        assert_eq!(
            df.height(),
            288,
            "Full extraction should yield 2*2*6*12 = 288 rows, got {}",
            df.height()
        );

        let column_names: Vec<String> = df
            .get_column_names()
            .iter()
            .map(|s| s.to_string())
            .collect();
        assert!(column_names.contains(&"time".to_string()));
        assert!(column_names.contains(&"level".to_string()));
        assert!(column_names.contains(&"latitude".to_string()));
        assert!(column_names.contains(&"longitude".to_string()));
        assert!(column_names.contains(&"temperature".to_string()));
        assert_eq!(
            column_names.len(),
            5,
            "DataFrame should have exactly 5 columns"
        );

        file.close()?;
        Ok(())
    }

    #[test]
    fn test_multi_filter_extraction_applies_independent_intersections(
    ) -> Result<(), Box<dyn std::error::Error>> {
        // lat range 30–40 → 3 lats; lon list [-120,-100,-80] → 3 lons; 2×2×3×3 = 36 rows
        let file_path = get_test_data_path("pres_temp_4D.nc");
        let file = netcdf::open(&file_path)?;
        let var = file.variable("temperature").unwrap();

        let lat_filter = NCRangeFilter::new("latitude", 30.0, 40.0);
        let lon_filter = NCListFilter::new("longitude", vec![-120.0, -100.0, -80.0]);

        let filters: Vec<Box<dyn NCFilter>> = vec![Box::new(lat_filter), Box::new(lon_filter)];

        let df = extract_data_to_dataframe(&file, &var, "temperature", &filters)?;

        assert_eq!(
            df.height(),
            36,
            "Expected 2*2*3*3 = 36 rows, got {}",
            df.height()
        );

        file.close()?;
        Ok(())
    }
}
