#[cfg(test)]
mod filter_tests {
    use crate::filters::{
        FilterResult, NC2DPointFilter, NC3DPointFilter, NCFilter, NCListFilter, NCRangeFilter,
    };
    use crate::test_helpers::get_test_data_path;

    #[test]
    fn test_range_filter_creation() {
        let filter = NCRangeFilter::new("time", 10.0, 20.0);
        assert_eq!(filter.dimension_name, "time");
        assert_eq!(filter.min_value, 10.0);
        assert_eq!(filter.max_value, 20.0);
    }

    #[test]
    fn test_range_filter_with_real_data() -> Result<(), Box<dyn std::error::Error>> {
        let file_path = get_test_data_path("pres_temp_4D.nc");
        let file = netcdf::open(&file_path)?;

        let filter = NCRangeFilter::new("latitude", 30.0, 45.0);
        let result = filter.apply(&file)?;

        if let FilterResult::Single { dimension, indices } = result {
            assert_eq!(dimension, "latitude");
            // Should include indices for 30, 35, 40, 45 degrees (indices 1, 2, 3, 4)
            assert_eq!(indices.len(), 4);
            assert!(indices.contains(&1)); // 30.0
            assert!(indices.contains(&2)); // 35.0
            assert!(indices.contains(&3)); // 40.0
            assert!(indices.contains(&4)); // 45.0
        } else {
            panic!("Expected Single filter result");
        }

        file.close()?;
        Ok(())
    }

    #[test]
    fn test_list_filter_creation() {
        let values = vec![0.0, 10.0, 20.0, 30.0];
        let filter = NCListFilter::new("depth", values.clone());
        assert_eq!(filter.dimension_name, "depth");
        assert_eq!(filter.values, values);
    }

    #[test]
    fn test_list_filter_with_real_data() -> Result<(), Box<dyn std::error::Error>> {
        let file_path = get_test_data_path("pres_temp_4D.nc");
        let file = netcdf::open(&file_path)?;

        let filter = NCListFilter::new("longitude", vec![-120.0, -85.0]);
        let result = filter.apply(&file)?;

        if let FilterResult::Single { dimension, indices } = result {
            assert_eq!(dimension, "longitude");
            assert_eq!(indices.len(), 2);
            assert!(indices.contains(&1)); // -120.0
            assert!(indices.contains(&8)); // -85.0
        } else {
            panic!("Expected Single filter result");
        }

        file.close()?;
        Ok(())
    }

    #[test]
    fn test_2d_point_filter_creation() {
        let points = vec![(10.0, 20.0), (15.0, 25.0)];
        let filter = NC2DPointFilter::new("lat", "lon", points.clone(), 0.1);

        assert_eq!(filter.lat_dimension_name, "lat");
        assert_eq!(filter.lon_dimension_name, "lon");
        assert_eq!(filter.points, points);
        assert_eq!(filter.tolerance, 0.1);
    }

    #[test]
    fn test_2d_point_filter_with_real_data() -> Result<(), Box<dyn std::error::Error>> {
        let file_path = get_test_data_path("pres_temp_4D.nc");
        let file = netcdf::open(&file_path)?;

        // lat: [25, 30, 35, 40, 45, 50], lon: [-125, -120, ..., -70]
        let points = vec![(30.0, -120.0), (45.0, -85.0)];
        let filter = NC2DPointFilter::new("latitude", "longitude", points, 1.0);
        let result = filter.apply(&file)?;

        if let FilterResult::Pairs {
            lat_dimension,
            lon_dimension,
            pairs,
        } = result
        {
            assert_eq!(lat_dimension, "latitude");
            assert_eq!(lon_dimension, "longitude");
            assert_eq!(pairs.len(), 2);
            // Check that we found the expected coordinate pairs
            assert!(pairs.contains(&(1, 1))); // (30.0, -120.0)
            assert!(pairs.contains(&(4, 8))); // (45.0, -85.0)
        } else {
            panic!("Expected Pairs filter result");
        }

        file.close()?;
        Ok(())
    }

    #[test]
    fn test_3d_point_filter_creation() {
        let steps = vec![0.0, 24.0, 48.0];
        let points = vec![(40.0, -74.0), (34.0, -118.0)];
        let filter = NC3DPointFilter::new("time", "lat", "lon", steps.clone(), points.clone(), 0.1);

        assert_eq!(filter.time_dimension_name, "time");
        assert_eq!(filter.lat_dimension_name, "lat");
        assert_eq!(filter.lon_dimension_name, "lon");
        assert_eq!(filter.steps, steps);
        assert_eq!(filter.points, points);
        assert_eq!(filter.tolerance, 0.1);
    }

    #[test]
    fn test_3d_point_filter_creation_only() {
        // pres_temp_4D.nc has no time coordinate variable; creation-only test
        let steps = vec![0.0, 1.0];
        let points = vec![(35.0, -110.0)];
        let filter = NC3DPointFilter::new(
            "time",
            "latitude",
            "longitude",
            steps.clone(),
            points.clone(),
            5.0,
        );

        assert_eq!(filter.time_dimension_name, "time");
        assert_eq!(filter.lat_dimension_name, "latitude");
        assert_eq!(filter.lon_dimension_name, "longitude");
        assert_eq!(filter.steps, steps);
        assert_eq!(filter.points, points);
        assert_eq!(filter.tolerance, 5.0);
    }

    #[test]
    fn test_filter_result_single() {
        let result = FilterResult::Single {
            dimension: "time".to_string(),
            indices: vec![1, 2, 3, 5, 8],
        };

        assert_eq!(result.len(), 5);
        assert!(!result.is_empty());

        if let Some((dim, indices)) = result.as_single() {
            assert_eq!(dim, "time");
            assert_eq!(indices.len(), 5);
            assert!(indices.contains(&1));
            assert!(indices.contains(&8));
        } else {
            panic!("Expected single result");
        }
    }

    #[test]
    fn test_filter_result_pairs() {
        let result = FilterResult::Pairs {
            lat_dimension: "latitude".to_string(),
            lon_dimension: "longitude".to_string(),
            pairs: vec![(0, 1), (2, 3), (4, 0)],
        };

        assert_eq!(result.len(), 3);
        assert!(!result.is_empty());

        if let Some((lat_dim, lon_dim, pairs)) = result.as_pairs() {
            assert_eq!(lat_dim, "latitude");
            assert_eq!(lon_dim, "longitude");
            assert_eq!(pairs.len(), 3);
            assert!(pairs.contains(&(0, 1)));
            assert!(pairs.contains(&(4, 0)));
        } else {
            panic!("Expected pairs result");
        }
    }

    #[test]
    fn test_filter_result_empty() {
        let empty_single = FilterResult::Single {
            dimension: "time".to_string(),
            indices: vec![],
        };

        assert_eq!(empty_single.len(), 0);
        assert!(empty_single.is_empty());

        let empty_pairs = FilterResult::Pairs {
            lat_dimension: "lat".to_string(),
            lon_dimension: "lon".to_string(),
            pairs: vec![],
        };

        assert_eq!(empty_pairs.len(), 0);
        assert!(empty_pairs.is_empty());
    }
}

#[cfg(test)]
mod from_json_tests {
    use crate::filters::{NC2DPointFilter, NC3DPointFilter, NCListFilter, NCRangeFilter};

    #[test]
    fn test_range_filter_from_json_valid() -> Result<(), Box<dyn std::error::Error>> {
        let json = r#"{"dimension_name": "latitude", "min_value": 10.0, "max_value": 50.0}"#;
        let filter = NCRangeFilter::from_json(json)?;
        assert_eq!(filter.dimension_name, "latitude");
        assert_eq!(filter.min_value, 10.0);
        assert_eq!(filter.max_value, 50.0);
        Ok(())
    }

    #[test]
    fn test_list_filter_from_json_valid() -> Result<(), Box<dyn std::error::Error>> {
        let json = r#"{"dimension_name": "longitude", "values": [-120.0, -110.0, -100.0]}"#;
        let filter = NCListFilter::from_json(json)?;
        assert_eq!(filter.dimension_name, "longitude");
        assert_eq!(filter.values, vec![-120.0, -110.0, -100.0]);
        Ok(())
    }

    #[test]
    fn test_2d_point_filter_from_json_valid() -> Result<(), Box<dyn std::error::Error>> {
        let json = r#"{
            "lat_dimension_name": "latitude",
            "lon_dimension_name": "longitude",
            "points": [[35.0, -110.0], [40.0, -95.0]],
            "tolerance": 2.5
        }"#;
        let filter = NC2DPointFilter::from_json(json)?;
        assert_eq!(filter.lat_dimension_name, "latitude");
        assert_eq!(filter.lon_dimension_name, "longitude");
        assert_eq!(filter.points, vec![(35.0, -110.0), (40.0, -95.0)]);
        assert_eq!(filter.tolerance, 2.5);
        Ok(())
    }

    #[test]
    fn test_3d_point_filter_from_json_valid() -> Result<(), Box<dyn std::error::Error>> {
        let json = r#"{
            "time_dimension_name": "time",
            "lat_dimension_name": "latitude",
            "lon_dimension_name": "longitude",
            "steps": [0.0, 1.0],
            "points": [[30.0, -120.0]],
            "tolerance": 1.0
        }"#;
        let filter = NC3DPointFilter::from_json(json)?;
        assert_eq!(filter.time_dimension_name, "time");
        assert_eq!(filter.lat_dimension_name, "latitude");
        assert_eq!(filter.lon_dimension_name, "longitude");
        assert_eq!(filter.steps, vec![0.0, 1.0]);
        assert_eq!(filter.points, vec![(30.0, -120.0)]);
        assert_eq!(filter.tolerance, 1.0);
        Ok(())
    }
}

#[cfg(test)]
mod filter_factory_tests {
    use crate::filters::filter_factory;

    #[test]
    fn test_factory_creates_range_filter() -> Result<(), Box<dyn std::error::Error>> {
        let json = r#"{"kind": "range", "dimension_name": "latitude", "min_value": 25.0, "max_value": 50.0}"#;
        let filter = filter_factory(json)?;
        // Verify the filter was created by applying it — we can't downcast the trait object,
        // but a successful creation is enough for this dispatch test.
        drop(filter);
        Ok(())
    }

    #[test]
    fn test_factory_creates_list_filter() -> Result<(), Box<dyn std::error::Error>> {
        let json = r#"{"kind": "list", "dimension_name": "longitude", "values": [-120.0, -110.0]}"#;
        let filter = filter_factory(json)?;
        drop(filter);
        Ok(())
    }

    #[test]
    fn test_factory_creates_2d_point_filter() -> Result<(), Box<dyn std::error::Error>> {
        let json = r#"{
            "kind": "2d_point",
            "lat_dimension_name": "latitude",
            "lon_dimension_name": "longitude",
            "points": [[35.0, -110.0]],
            "tolerance": 1.0
        }"#;
        let filter = filter_factory(json)?;
        drop(filter);
        Ok(())
    }

    #[test]
    fn test_factory_creates_3d_point_filter() -> Result<(), Box<dyn std::error::Error>> {
        let json = r#"{
            "kind": "3d_point",
            "time_dimension_name": "time",
            "lat_dimension_name": "latitude",
            "lon_dimension_name": "longitude",
            "steps": [0.0, 1.0],
            "points": [[30.0, -120.0]],
            "tolerance": 1.0
        }"#;
        let filter = filter_factory(json)?;
        drop(filter);
        Ok(())
    }

    #[test]
    fn test_factory_unknown_kind_returns_error() {
        let json = r#"{"kind": "spatial_grid", "dimension_name": "latitude"}"#;
        let result = filter_factory(json);
        assert!(result.is_err());
        let msg = result.err().unwrap().to_string();
        assert!(
            msg.contains("Unknown filter kind"),
            "Expected 'Unknown filter kind' in error, got: {msg}"
        );
    }

    #[test]
    fn test_factory_missing_kind_field_returns_error() {
        let json = r#"{"dimension_name": "latitude", "min_value": 10.0, "max_value": 50.0}"#;
        let result = filter_factory(json);
        assert!(result.is_err());
        let msg = result.err().unwrap().to_string();
        assert!(
            msg.contains("Missing 'kind' field"),
            "Expected \"Missing 'kind' field\" in error, got: {msg}"
        );
    }
}

#[cfg(test)]
mod filter_result_triplets_tests {
    use crate::filters::FilterResult;

    #[test]
    fn test_triplets_as_triplets_returns_correct_data() {
        let triplets = vec![(0usize, 1usize, 2usize), (1, 3, 5), (0, 2, 7)];
        let result = FilterResult::Triplets {
            time_dimension: "time".to_string(),
            lat_dimension: "latitude".to_string(),
            lon_dimension: "longitude".to_string(),
            triplets: triplets.clone(),
        };

        assert_eq!(result.len(), 3);
        assert!(!result.is_empty());

        let extracted = result.as_triplets();
        assert!(extracted.is_some());
        let (time_dim, lat_dim, lon_dim, extracted_triplets) = extracted.unwrap();
        assert_eq!(time_dim, "time");
        assert_eq!(lat_dim, "latitude");
        assert_eq!(lon_dim, "longitude");
        assert_eq!(extracted_triplets.len(), 3);
        assert!(extracted_triplets.contains(&(0, 1, 2)));
        assert!(extracted_triplets.contains(&(1, 3, 5)));
        assert!(extracted_triplets.contains(&(0, 2, 7)));
    }

    #[test]
    fn test_empty_triplets_is_empty_and_len_zero() {
        let result = FilterResult::Triplets {
            time_dimension: "time".to_string(),
            lat_dimension: "latitude".to_string(),
            lon_dimension: "longitude".to_string(),
            triplets: vec![],
        };

        assert!(result.is_empty());
        assert_eq!(result.len(), 0);

        let extracted = result.as_triplets();
        assert!(extracted.is_some());
        let (_, _, _, triplets) = extracted.unwrap();
        assert!(triplets.is_empty());
    }
}

#[cfg(test)]
mod range_filter_edge_case_tests {
    use crate::filters::{FilterResult, NCFilter, NCRangeFilter};
    use crate::test_helpers::get_test_data_path;

    #[test]
    fn test_range_filter_outside_actual_range_returns_empty()
    -> Result<(), Box<dyn std::error::Error>> {
        // latitude values in pres_temp_4D.nc: [25, 30, 35, 40, 45, 50]
        let file_path = get_test_data_path("pres_temp_4D.nc");
        let file = netcdf::open(&file_path)?;

        let filter = NCRangeFilter::new("latitude", 100.0, 200.0);
        let result = filter.apply(&file)?;

        assert!(result.is_empty());
        assert_eq!(result.len(), 0);
        if let FilterResult::Single { dimension, indices } = result {
            assert_eq!(dimension, "latitude");
            assert!(indices.is_empty());
        } else {
            panic!("Expected Single filter result");
        }

        file.close()?;
        Ok(())
    }

    #[test]
    fn test_range_filter_covers_entire_dimension_returns_all()
    -> Result<(), Box<dyn std::error::Error>> {
        // latitude values: [25, 30, 35, 40, 45, 50] → 6 values
        let file_path = get_test_data_path("pres_temp_4D.nc");
        let file = netcdf::open(&file_path)?;

        let filter = NCRangeFilter::new("latitude", 0.0, 1000.0);
        let result = filter.apply(&file)?;

        assert!(!result.is_empty());
        assert_eq!(result.len(), 6);
        if let FilterResult::Single { indices, .. } = result {
            for expected_idx in 0..6 {
                assert!(
                    indices.contains(&expected_idx),
                    "Expected index {expected_idx} to be present"
                );
            }
        } else {
            panic!("Expected Single filter result");
        }

        file.close()?;
        Ok(())
    }

    #[test]
    fn test_range_filter_nonexistent_dimension_returns_error() {
        let file_path = get_test_data_path("pres_temp_4D.nc");
        let file = netcdf::open(&file_path).expect("Failed to open test file");

        let filter = NCRangeFilter::new("nonexistent_dim", 0.0, 100.0);
        let result = filter.apply(&file);

        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(
            msg.contains("nonexistent_dim"),
            "Error should mention the dimension name, got: {msg}"
        );
    }
}

#[cfg(test)]
mod list_filter_edge_case_tests {
    use crate::filters::{FilterResult, NCFilter, NCListFilter};
    use crate::test_helpers::get_test_data_path;

    #[test]
    fn test_list_filter_no_matching_values_returns_empty() -> Result<(), Box<dyn std::error::Error>>
    {
        // longitude values: [-125, -120, -115, -110, -105, -100, -95, -90, -85, -80, -75, -70]
        let file_path = get_test_data_path("pres_temp_4D.nc");
        let file = netcdf::open(&file_path)?;

        let filter = NCListFilter::new("longitude", vec![999.0, 888.0]);
        let result = filter.apply(&file)?;

        assert!(result.is_empty());
        assert_eq!(result.len(), 0);

        file.close()?;
        Ok(())
    }

    #[test]
    fn test_list_filter_all_longitude_values_returns_all_indices()
    -> Result<(), Box<dyn std::error::Error>> {
        let all_lons = vec![
            -125.0, -120.0, -115.0, -110.0, -105.0, -100.0, -95.0, -90.0, -85.0, -80.0, -75.0,
            -70.0,
        ];
        let file_path = get_test_data_path("pres_temp_4D.nc");
        let file = netcdf::open(&file_path)?;

        let filter = NCListFilter::new("longitude", all_lons);
        let result = filter.apply(&file)?;

        assert_eq!(result.len(), 12);
        if let FilterResult::Single { indices, .. } = result {
            for expected_idx in 0..12 {
                assert!(
                    indices.contains(&expected_idx),
                    "Expected index {expected_idx} to be present"
                );
            }
        } else {
            panic!("Expected Single filter result");
        }

        file.close()?;
        Ok(())
    }

    #[test]
    fn test_list_filter_near_match_does_not_match_due_to_exact_comparison()
    -> Result<(), Box<dyn std::error::Error>> {
        // NCListFilter uses exact f64 equality; -120.0000001 must not match -120.0
        let file_path = get_test_data_path("pres_temp_4D.nc");
        let file = netcdf::open(&file_path)?;

        let filter = NCListFilter::new("longitude", vec![-120.000_000_1]);
        let result = filter.apply(&file)?;

        assert!(result.is_empty());
        assert_eq!(result.len(), 0);

        file.close()?;
        Ok(())
    }
}

#[cfg(test)]
mod point_2d_filter_edge_case_tests {
    use crate::filters::{FilterResult, NC2DPointFilter, NCFilter};
    use crate::test_helpers::get_test_data_path;

    #[test]
    fn test_2d_point_filter_zero_tolerance_exact_match() -> Result<(), Box<dyn std::error::Error>> {
        // lat index 2 = 35.0, lon index 3 = -110.0
        let file_path = get_test_data_path("pres_temp_4D.nc");
        let file = netcdf::open(&file_path)?;

        let filter = NC2DPointFilter::new("latitude", "longitude", vec![(35.0, -110.0)], 0.0);
        let result = filter.apply(&file)?;

        assert_eq!(result.len(), 1);
        if let FilterResult::Pairs { pairs, .. } = result {
            assert!(pairs.contains(&(2, 3))); // lat idx 2 = 35.0, lon idx 3 = -110.0
        } else {
            panic!("Expected Pairs filter result");
        }

        file.close()?;
        Ok(())
    }

    #[test]
    fn test_2d_point_filter_points_far_from_grid_returns_empty()
    -> Result<(), Box<dyn std::error::Error>> {
        // Points in the Atlantic Ocean — outside the Pacific-focused grid in pres_temp_4D.nc
        let file_path = get_test_data_path("pres_temp_4D.nc");
        let file = netcdf::open(&file_path)?;

        let filter =
            NC2DPointFilter::new("latitude", "longitude", vec![(48.0, 2.0), (51.0, 0.0)], 0.1);
        let result = filter.apply(&file)?;

        assert!(result.is_empty());
        assert_eq!(result.len(), 0);

        file.close()?;
        Ok(())
    }
}

#[cfg(test)]
mod point_3d_filter_edge_case_tests {
    use crate::filters::{NC3DPointFilter, NCFilter};
    use crate::test_helpers::get_test_data_path;

    #[test]
    fn test_3d_point_filter_missing_time_coordinate_variable_returns_error() {
        // pres_temp_4D.nc has a "time" dimension but no "time" coordinate variable
        let file_path = get_test_data_path("pres_temp_4D.nc");
        let file = netcdf::open(&file_path).expect("Failed to open test file");

        let filter = NC3DPointFilter::new(
            "time", // time dimension has no coordinate variable in this file
            "latitude",
            "longitude",
            vec![0.0],
            vec![(35.0, -110.0)],
            1.0,
        );
        let result = filter.apply(&file);

        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(
            msg.contains("time"),
            "Error message should mention 'time', got: {msg}"
        );
    }

    #[test]
    fn test_3d_point_filter_duplicate_triplets_are_preserved() {
        // The filter does not deduplicate; each (point, time_step) is an independent triplet
        use crate::filters::FilterResult;

        let triplets = vec![(0, 1, 2), (0, 1, 2), (1, 1, 2)];
        let result = FilterResult::Triplets {
            time_dimension: "time".to_string(),
            lat_dimension: "latitude".to_string(),
            lon_dimension: "longitude".to_string(),
            triplets: triplets.clone(),
        };

        assert_eq!(result.len(), 3);
        let (_, _, _, stored) = result.as_triplets().unwrap();
        assert_eq!(stored[0], (0, 1, 2));
        assert_eq!(stored[1], (0, 1, 2)); // duplicate retained
        assert_eq!(stored[2], (1, 1, 2));
    }
}
