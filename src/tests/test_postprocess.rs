#[cfg(test)]
mod postprocess_tests {
    use crate::postprocess::*;
    use polars::prelude::*;
    use std::collections::HashMap;

    fn create_test_dataframe() -> DataFrame {
        df! {
            "temperature" => [273.15, 283.15, 293.15, 303.15],
            "pressure" => [1013.25, 1012.0, 1010.5, 1009.0],
            "humidity" => [60.0, 65.0, 70.0, 75.0],
            "time_offset" => [0.0, 1.0, 2.0, 3.0],
        }
        .unwrap()
    }

    #[test]
    fn test_column_renamer() {
        let df = create_test_dataframe();
        let mut mappings = HashMap::new();
        mappings.insert("temperature".to_string(), "temp_k".to_string());
        mappings.insert("pressure".to_string(), "pres_hpa".to_string());

        let processor = ColumnRenamer::new(mappings);
        let result = processor.process(df).unwrap();

        let columns: Vec<&str> = result
            .get_column_names()
            .iter()
            .map(|s| s.as_str())
            .collect();
        assert!(columns.contains(&"temp_k"));
        assert!(columns.contains(&"pres_hpa"));
        assert!(columns.contains(&"humidity"));
        assert!(!columns.contains(&"temperature"));
        assert!(!columns.contains(&"pressure"));
    }

    #[test]
    fn test_unit_converter_kelvin_to_celsius() {
        let df = create_test_dataframe();
        let processor = UnitConverter::new(
            "temperature".to_string(),
            "kelvin".to_string(),
            "celsius".to_string(),
        );

        let result = processor.process(df).unwrap();
        let temp_col = result.column("temperature").unwrap();
        let values: Vec<f64> = temp_col
            .f64()
            .unwrap()
            .into_iter()
            .map(|v| v.unwrap())
            .collect();

        assert!((values[0] - 0.0).abs() < 1e-10);
        assert!((values[1] - 10.0).abs() < 1e-10);
        assert!((values[2] - 20.0).abs() < 1e-10);
        assert!((values[3] - 30.0).abs() < 1e-10);
    }

    #[test]
    fn test_unit_converter_multiplication() {
        let df = create_test_dataframe();
        let processor = UnitConverter::with_conversion_factor(
            "pressure".to_string(),
            "hpa".to_string(),
            "pa".to_string(),
            100.0,
        );

        let result = processor.process(df).unwrap();
        let pres_col = result.column("pressure").unwrap();
        let values: Vec<f64> = pres_col
            .f64()
            .unwrap()
            .into_iter()
            .map(|v| v.unwrap())
            .collect();

        assert!((values[0] - 101325.0).abs() < 1e-6);
        assert!((values[1] - 101200.0).abs() < 1e-6);
    }

    #[test]
    fn test_aggregator() {
        let df = df! {
            "station" => ["A", "A", "B", "B", "A", "B"],
            "temperature" => [20.0, 22.0, 18.0, 19.0, 21.0, 17.0],
            "pressure" => [1013.0, 1012.0, 1015.0, 1014.0, 1013.5, 1016.0],
        }
        .unwrap();

        let group_by = vec!["station".to_string()];
        let mut aggregations = HashMap::new();
        aggregations.insert("temperature".to_string(), AggregationOp::Mean);
        aggregations.insert("pressure".to_string(), AggregationOp::Max);

        let processor = Aggregator::new(group_by, aggregations);
        let result = processor.process(df).unwrap();

        assert_eq!(result.height(), 2);

        let columns: Vec<&str> = result
            .get_column_names()
            .iter()
            .map(|s| s.as_str())
            .collect();
        assert!(columns.contains(&"station"));
        assert!(columns.contains(&"temperature_mean"));
        assert!(columns.contains(&"pressure_max"));
    }

    #[test]
    fn test_formula_applier_arithmetic() {
        let df = create_test_dataframe();
        let processor = FormulaApplier::new(
            "apparent_temp".to_string(),
            "temperature + humidity".to_string(),
            vec!["temperature".to_string(), "humidity".to_string()],
        );

        let result = processor.process(df).unwrap();

        let columns: Vec<&str> = result
            .get_column_names()
            .iter()
            .map(|s| s.as_str())
            .collect();
        assert!(columns.contains(&"apparent_temp"));

        let new_col = result.column("apparent_temp").unwrap();
        let values: Vec<f64> = new_col
            .f64()
            .unwrap()
            .into_iter()
            .map(|v| v.unwrap())
            .collect();

        // 273.15 + 60.0 = 333.15
        assert!((values[0] - 333.15).abs() < 1e-10);
    }

    #[test]
    fn test_formula_applier_sqrt() {
        let df = df! {
            "value" => [4.0, 9.0, 16.0, 25.0],
        }
        .unwrap();

        let processor = FormulaApplier::new(
            "sqrt_value".to_string(),
            "sqrt(value)".to_string(),
            vec!["value".to_string()],
        );

        let result = processor.process(df).unwrap();
        let new_col = result.column("sqrt_value").unwrap();
        let values: Vec<f64> = new_col
            .f64()
            .unwrap()
            .into_iter()
            .map(|v| v.unwrap())
            .collect();

        assert!((values[0] - 2.0).abs() < 1e-10);
        assert!((values[1] - 3.0).abs() < 1e-10);
        assert!((values[2] - 4.0).abs() < 1e-10);
        assert!((values[3] - 5.0).abs() < 1e-10);
    }

    #[test]
    fn test_processing_pipeline() {
        let df = create_test_dataframe();
        let mut pipeline = ProcessingPipeline::new();

        let mut mappings = HashMap::new();
        mappings.insert("temperature".to_string(), "temp".to_string());
        pipeline.add_processor(Box::new(ColumnRenamer::new(mappings)));

        let converter = UnitConverter::new(
            "temp".to_string(),
            "kelvin".to_string(),
            "celsius".to_string(),
        );
        pipeline.add_processor(Box::new(converter));

        let result = pipeline.execute(df).unwrap();

        let columns: Vec<&str> = result
            .get_column_names()
            .iter()
            .map(|s| s.as_str())
            .collect();
        assert!(columns.contains(&"temp"));
        assert!(!columns.contains(&"temperature"));

        let temp_col = result.column("temp").unwrap();
        let values: Vec<f64> = temp_col
            .f64()
            .unwrap()
            .into_iter()
            .map(|v| v.unwrap())
            .collect();
        assert!((values[0] - 0.0).abs() < 1e-10);
    }

    #[test]
    fn test_create_processor_from_config() {
        let mut mappings = HashMap::new();
        mappings.insert("old_name".to_string(), "new_name".to_string());
        let config = ProcessorConfig::RenameColumns { mappings };

        let processor = create_processor(&config).unwrap();
        assert_eq!(processor.name(), "ColumnRenamer");
        assert_eq!(
            processor.description(),
            "Renames columns based on provided mappings"
        );
    }

    #[test]
    fn test_unit_converter_with_config() {
        let config = ProcessorConfig::UnitConvert {
            column: "temperature".to_string(),
            from_unit: "kelvin".to_string(),
            to_unit: "celsius".to_string(),
        };

        let processor = create_processor(&config).unwrap();
        assert_eq!(processor.name(), "UnitConverter");

        let df = create_test_dataframe();
        let result = processor.process(df).unwrap();

        let temp_col = result.column("temperature").unwrap();
        let values: Vec<f64> = temp_col
            .f64()
            .unwrap()
            .into_iter()
            .map(|v| v.unwrap())
            .collect();
        assert!((values[0] - 0.0).abs() < 1e-10);
    }

    #[test]
    fn test_error_handling() {
        let df = create_test_dataframe();

        let processor = UnitConverter::new(
            "nonexistent".to_string(),
            "kelvin".to_string(),
            "celsius".to_string(),
        );

        let result = processor.process(df);
        assert!(result.is_err());

        if let Err(PostProcessError::ColumnNotFound(col)) = result {
            assert_eq!(col, "nonexistent");
        } else {
            panic!("Expected ColumnNotFound error");
        }
    }

    #[test]
    fn test_pipeline_from_config() {
        let config = ProcessingPipelineConfig {
            name: Some("Test Pipeline".to_string()),
            processors: vec![
                ProcessorConfig::RenameColumns {
                    mappings: {
                        let mut map = HashMap::new();
                        map.insert("temperature".to_string(), "temp".to_string());
                        map
                    },
                },
                ProcessorConfig::UnitConvert {
                    column: "temp".to_string(),
                    from_unit: "kelvin".to_string(),
                    to_unit: "celsius".to_string(),
                },
            ],
        };

        let mut pipeline = ProcessingPipeline::from_config(&config).unwrap();
        assert_eq!(pipeline.name(), "Test Pipeline");

        let df = create_test_dataframe();
        let result = pipeline.execute(df).unwrap();

        let columns: Vec<&str> = result
            .get_column_names()
            .iter()
            .map(|s| s.as_str())
            .collect();
        assert!(columns.contains(&"temp"));
        assert!(!columns.contains(&"temperature"));

        let temp_col = result.column("temp").unwrap();
        let values: Vec<f64> = temp_col
            .f64()
            .unwrap()
            .into_iter()
            .map(|v| v.unwrap())
            .collect();
        assert!((values[0] - 0.0).abs() < 1e-10);
    }

    #[test]
    fn test_datetime_converter_basic() {
        let df = df! {
            "time" => [0.0, 1.0, 24.0],
        }
        .unwrap();

        let base_datetime = chrono::DateTime::parse_from_rfc3339("2000-01-01T00:00:00Z")
            .unwrap()
            .with_timezone(&chrono::Utc);

        let processor = DateTimeConverter::new(
            "time".to_string(),
            base_datetime,
            crate::postprocess::TimeUnit::Hours,
        );

        let result = processor.process(df).unwrap();
        let time_col = result.column("time").unwrap();

        assert!(matches!(
            time_col.dtype(),
            DataType::Datetime(polars::prelude::TimeUnit::Milliseconds, None)
        ));

        let base_ms = 946684800000i64; // 2000-01-01T00:00:00Z in ms
        let datetime_physical = time_col.datetime().unwrap().physical();
        let first_val = datetime_physical.get(0).unwrap();
        let second_val = datetime_physical.get(1).unwrap();
        let third_val = datetime_physical.get(2).unwrap();

        assert_eq!(first_val, base_ms);
        assert_eq!(second_val, base_ms + 3600000);
        assert_eq!(third_val, base_ms + 86400000);
    }

    #[test]
    fn test_datetime_converter_days() {
        let df = df! {
            "time" => [0.0, 1.0, 7.0],
        }
        .unwrap();

        let base_datetime = chrono::DateTime::parse_from_rfc3339("1990-01-01T00:00:00Z")
            .unwrap()
            .with_timezone(&chrono::Utc);

        let processor = DateTimeConverter::new(
            "time".to_string(),
            base_datetime,
            crate::postprocess::TimeUnit::Days,
        );

        let result = processor.process(df).unwrap();
        let time_col = result.column("time").unwrap();

        assert!(matches!(
            time_col.dtype(),
            DataType::Datetime(polars::prelude::TimeUnit::Milliseconds, None)
        ));

        let base_ms = chrono::DateTime::parse_from_rfc3339("1990-01-01T00:00:00Z")
            .unwrap()
            .timestamp_millis();

        let datetime_physical = time_col.datetime().unwrap().physical();

        assert_eq!(datetime_physical.get(0).unwrap(), base_ms);
        assert_eq!(datetime_physical.get(1).unwrap(), base_ms + 86400000);
        assert_eq!(datetime_physical.get(2).unwrap(), base_ms + 604800000);
    }

    #[test]
    fn test_datetime_converter_seconds() {
        let df = df! {
            "time" => [0.0, 60.0, 3600.0],
        }
        .unwrap();

        let base_datetime = chrono::DateTime::parse_from_rfc3339("2000-01-01T00:00:00Z")
            .unwrap()
            .with_timezone(&chrono::Utc);

        let processor = DateTimeConverter::new(
            "time".to_string(),
            base_datetime,
            crate::postprocess::TimeUnit::Seconds,
        );

        let result = processor.process(df).unwrap();
        let time_col = result.column("time").unwrap();

        assert!(matches!(
            time_col.dtype(),
            DataType::Datetime(polars::prelude::TimeUnit::Milliseconds, None)
        ));

        let base_ms = 946684800000i64;
        let datetime_physical = time_col.datetime().unwrap().physical();

        assert_eq!(datetime_physical.get(0).unwrap(), base_ms);
        assert_eq!(datetime_physical.get(1).unwrap(), base_ms + 60000);
        assert_eq!(datetime_physical.get(2).unwrap(), base_ms + 3600000);
    }

    #[test]
    fn test_datetime_converter_column_not_found() {
        let df = create_test_dataframe();

        let base_datetime = chrono::DateTime::parse_from_rfc3339("2000-01-01T00:00:00Z")
            .unwrap()
            .with_timezone(&chrono::Utc);

        let processor = DateTimeConverter::new(
            "nonexistent".to_string(),
            base_datetime,
            crate::postprocess::TimeUnit::Hours,
        );

        let result = processor.process(df);
        assert!(result.is_err());

        if let Err(PostProcessError::ColumnNotFound(col)) = result {
            assert_eq!(col, "nonexistent");
        } else {
            panic!("Expected ColumnNotFound error");
        }
    }
}

#[cfg(test)]
mod unit_converter_edge_cases {
    use crate::postprocess::{PostProcessor, UnitConverter};
    use polars::prelude::*;

    fn make_df(values: &[f64]) -> DataFrame {
        df! { "temp" => values }.unwrap()
    }

    fn extract_f64(df: &DataFrame, col_name: &str) -> Vec<f64> {
        df.column(col_name)
            .unwrap()
            .f64()
            .unwrap()
            .into_iter()
            .map(|v| v.unwrap())
            .collect()
    }

    #[test]
    fn test_celsius_to_kelvin() {
        let df = make_df(&[0.0, 100.0]);
        let processor = UnitConverter::new(
            "temp".to_string(),
            "celsius".to_string(),
            "kelvin".to_string(),
        );
        let result = processor.process(df).unwrap();
        let values = extract_f64(&result, "temp");
        assert!((values[0] - 273.15).abs() < 1e-10, "0°C → 273.15 K");
        assert!((values[1] - 373.15).abs() < 1e-10, "100°C → 373.15 K");
    }

    #[test]
    fn test_celsius_to_fahrenheit() {
        let df = make_df(&[0.0, 100.0]);
        let processor = UnitConverter::new(
            "temp".to_string(),
            "celsius".to_string(),
            "fahrenheit".to_string(),
        );
        let result = processor.process(df).unwrap();
        let values = extract_f64(&result, "temp");
        assert!((values[0] - 32.0).abs() < 1e-10, "0°C → 32°F");
        assert!((values[1] - 212.0).abs() < 1e-10, "100°C → 212°F");
    }

    #[test]
    fn test_fahrenheit_to_celsius() {
        let df = make_df(&[32.0, 212.0]);
        let processor = UnitConverter::new(
            "temp".to_string(),
            "fahrenheit".to_string(),
            "celsius".to_string(),
        );
        let result = processor.process(df).unwrap();
        let values = extract_f64(&result, "temp");
        assert!((values[0] - 0.0).abs() < 1e-10, "32°F → 0°C");
        assert!((values[1] - 100.0).abs() < 1e-10, "212°F → 100°C");
    }

    #[test]
    fn test_unknown_unit_pair_is_noop() {
        // Unknown unit pair falls back to conversion_factor = 1.0, so values are unchanged.
        let df = make_df(&[42.0, 7.5]);
        let processor = UnitConverter::new(
            "temp".to_string(),
            "furlongs".to_string(),
            "fortnights".to_string(),
        );
        let result = processor.process(df).unwrap();
        let values = extract_f64(&result, "temp");
        assert!(
            (values[0] - 42.0).abs() < 1e-10,
            "Unknown units: value should be unchanged"
        );
        assert!(
            (values[1] - 7.5).abs() < 1e-10,
            "Unknown units: value should be unchanged"
        );
    }

    #[test]
    fn test_short_unit_names_k_to_c() {
        // Short names "k" → "c" should behave like "kelvin" → "celsius"
        let df = make_df(&[273.15, 373.15]);
        let processor = UnitConverter::new("temp".to_string(), "k".to_string(), "c".to_string());
        let result = processor.process(df).unwrap();
        let values = extract_f64(&result, "temp");
        assert!(
            (values[0] - 0.0).abs() < 1e-10,
            "273.15 K → 0°C via short names"
        );
        assert!(
            (values[1] - 100.0).abs() < 1e-10,
            "373.15 K → 100°C via short names"
        );
    }

    #[test]
    fn test_case_insensitive_unit_names() {
        // "KELVIN" → "Celsius" should work the same as "kelvin" → "celsius"
        let df = make_df(&[273.15, 283.15]);
        let processor = UnitConverter::new(
            "temp".to_string(),
            "KELVIN".to_string(),
            "Celsius".to_string(),
        );
        let result = processor.process(df).unwrap();
        let values = extract_f64(&result, "temp");
        assert!((values[0] - 0.0).abs() < 1e-10, "273.15 KELVIN → 0 Celsius");
        assert!(
            (values[1] - 10.0).abs() < 1e-10,
            "283.15 KELVIN → 10 Celsius"
        );
    }
}

#[cfg(test)]
mod formula_applier_edge_cases {
    use crate::postprocess::{FormulaApplier, PostProcessor};
    use polars::prelude::*;

    fn extract_f64(df: &DataFrame, col_name: &str) -> Vec<f64> {
        df.column(col_name)
            .unwrap()
            .f64()
            .unwrap()
            .into_iter()
            .map(|v| v.unwrap())
            .collect()
    }

    #[test]
    fn test_subtraction() {
        let df = df! {
            "a" => [10.0_f64, 20.0],
            "b" => [3.0_f64, 5.0],
        }
        .unwrap();
        let processor = FormulaApplier::new(
            "result".to_string(),
            "a - b".to_string(),
            vec!["a".to_string(), "b".to_string()],
        );
        let result = processor.process(df).unwrap();
        let values = extract_f64(&result, "result");
        assert!((values[0] - 7.0).abs() < 1e-10, "10 - 3 = 7");
        assert!((values[1] - 15.0).abs() < 1e-10, "20 - 5 = 15");
    }

    #[test]
    fn test_multiplication_with_constant() {
        let df = df! {
            "a" => [3.0_f64, 5.0],
        }
        .unwrap();
        let processor = FormulaApplier::new(
            "result".to_string(),
            "a * 2.0".to_string(),
            vec!["a".to_string()],
        );
        let result = processor.process(df).unwrap();
        let values = extract_f64(&result, "result");
        assert!((values[0] - 6.0).abs() < 1e-10, "3 * 2 = 6");
        assert!((values[1] - 10.0).abs() < 1e-10, "5 * 2 = 10");
    }

    #[test]
    fn test_division() {
        let df = df! {
            "a" => [10.0_f64, 9.0],
            "b" => [2.0_f64, 3.0],
        }
        .unwrap();
        let processor = FormulaApplier::new(
            "result".to_string(),
            "a / b".to_string(),
            vec!["a".to_string(), "b".to_string()],
        );
        let result = processor.process(df).unwrap();
        let values = extract_f64(&result, "result");
        assert!((values[0] - 5.0).abs() < 1e-10, "10 / 2 = 5");
        assert!((values[1] - 3.0).abs() < 1e-10, "9 / 3 = 3");
    }

    #[test]
    fn test_operator_precedence_add_mul() {
        // "a + b * c" should be a + (b*c), not (a+b)*c
        // a=1, b=2, c=3 → expected = 1 + (2*3) = 7
        let df = df! {
            "a" => [1.0_f64],
            "b" => [2.0_f64],
            "c" => [3.0_f64],
        }
        .unwrap();
        let processor = FormulaApplier::new(
            "result".to_string(),
            "a + b * c".to_string(),
            vec!["a".to_string(), "b".to_string(), "c".to_string()],
        );
        let result = processor.process(df).unwrap();
        let values = extract_f64(&result, "result");
        assert!(
            (values[0] - 7.0).abs() < 1e-10,
            "a + b * c should be a + (b*c) = 7, got {}",
            values[0]
        );
    }

    #[test]
    fn test_parenthesized_expression() {
        // "(a + b) * c" should override precedence → (1+2)*3 = 9
        let df = df! {
            "a" => [1.0_f64],
            "b" => [2.0_f64],
            "c" => [3.0_f64],
        }
        .unwrap();
        let processor = FormulaApplier::new(
            "result".to_string(),
            "(a + b) * c".to_string(),
            vec!["a".to_string(), "b".to_string(), "c".to_string()],
        );
        let result = processor.process(df).unwrap();
        let values = extract_f64(&result, "result");
        assert!(
            (values[0] - 9.0).abs() < 1e-10,
            "(a + b) * c should equal 9, got {}",
            values[0]
        );
    }

    #[test]
    fn test_constant_formula() {
        // A bare constant "42.0" should produce a column with the literal value repeated
        let df = df! {
            "a" => [1.0_f64, 2.0, 3.0],
        }
        .unwrap();
        let processor = FormulaApplier::new("constant_col".to_string(), "42.0".to_string(), vec![]);
        let result = processor.process(df).unwrap();
        let values = extract_f64(&result, "constant_col");
        assert_eq!(values.len(), 3);
        for v in &values {
            assert!(
                (v - 42.0).abs() < 1e-10,
                "Constant formula should yield 42.0, got {}",
                v
            );
        }
    }

    #[test]
    fn test_invalid_formula_unclosed_paren_returns_error() {
        let df = df! {
            "a" => [1.0_f64, 2.0],
        }
        .unwrap();
        // "(a + 1" has an unclosed paren — parsed as an operand that is neither a
        // valid number nor an existing column name
        let processor = FormulaApplier::new(
            "result".to_string(),
            "(a + 1".to_string(),
            vec!["a".to_string()],
        );
        let result = processor.process(df);
        assert!(
            result.is_err(),
            "Invalid formula with unclosed paren should return an error"
        );
    }

    #[test]
    fn test_nonexistent_source_column_returns_error() {
        let df = df! {
            "a" => [1.0_f64, 2.0],
        }
        .unwrap();
        let processor = FormulaApplier::new(
            "result".to_string(),
            "a + missing_col".to_string(),
            vec!["a".to_string(), "missing_col".to_string()],
        );
        let result = processor.process(df);
        assert!(
            result.is_err(),
            "Formula referencing nonexistent column should return an error"
        );
    }

    #[test]
    fn test_sqrt_negative_produces_nan() {
        // sqrt(-n) produces NaN per IEEE 754
        let df = df! {
            "value" => [-4.0_f64, 9.0],
        }
        .unwrap();
        let processor = FormulaApplier::new(
            "root".to_string(),
            "sqrt(value)".to_string(),
            vec!["value".to_string()],
        );
        let result = processor.process(df).unwrap();
        let col = result.column("root").unwrap();
        let values: Vec<Option<f64>> = col.f64().unwrap().into_iter().collect();
        let first = values[0];
        assert!(
            first.is_none() || first.map(|v| v.is_nan()).unwrap_or(false),
            "sqrt(-4) should be NaN or null, got {:?}",
            first
        );
        // sqrt(9) = 3
        assert!((values[1].unwrap() - 3.0).abs() < 1e-10, "sqrt(9) = 3");
    }

    #[test]
    fn test_comparison_less_than() {
        // "a < b" should produce a boolean column
        let df = df! {
            "a" => [1.0_f64, 5.0, 3.0],
            "b" => [2.0_f64, 3.0, 3.0],
        }
        .unwrap();
        let processor = FormulaApplier::new(
            "cmp".to_string(),
            "a < b".to_string(),
            vec!["a".to_string(), "b".to_string()],
        );
        let result = processor.process(df).unwrap();
        let col = result.column("cmp").unwrap();
        let values: Vec<bool> = col
            .bool()
            .unwrap()
            .into_iter()
            .map(|v| v.unwrap())
            .collect();
        assert!(values[0], "1 < 2 is true");
        assert!(!values[1], "5 < 3 is false");
        assert!(!values[2], "3 < 3 is false");
    }
}

#[cfg(test)]
mod aggregator_edge_cases {
    use crate::postprocess::{AggregationOp, Aggregator, PostProcessError, PostProcessor};
    use polars::prelude::*;
    use std::collections::HashMap;

    fn make_df() -> DataFrame {
        df! {
            "group"  => ["x", "x", "y", "y"],
            "values" => [1.0_f64, 3.0, 5.0, 7.0],
        }
        .unwrap()
    }

    #[test]
    fn test_all_agg_ops_on_known_data() {
        let df = df! {
            "group"  => ["x", "x", "x"],
            "values" => [2.0_f64, 4.0, 6.0],
        }
        .unwrap();

        let ops = [
            ("sum", AggregationOp::Sum, 12.0_f64),
            ("min", AggregationOp::Min, 2.0_f64),
            ("max", AggregationOp::Max, 6.0_f64),
        ];

        for (label, op, expected) in ops {
            let group_by = vec!["group".to_string()];
            let mut aggregations = HashMap::new();
            aggregations.insert("values".to_string(), op);

            let processor = Aggregator::new(group_by, aggregations);
            let result = processor.process(df.clone()).unwrap();

            assert_eq!(result.height(), 1, "Expected 1 row for op={}", label);

            let col_name = format!("values_{}", label);
            let col = result
                .column(&col_name)
                .expect(&format!("column {} missing", col_name));
            let val = col.f64().unwrap().get(0).unwrap();
            assert!(
                (val - expected).abs() < 1e-6,
                "op={} expected {} got {}",
                label,
                expected,
                val
            );
        }
    }

    #[test]
    fn test_global_aggregation_empty_group_by() {
        let df = df! {
            "values" => [1.0_f64, 2.0, 3.0, 4.0],
        }
        .unwrap();

        let mut aggregations = HashMap::new();
        aggregations.insert("values".to_string(), AggregationOp::Sum);

        let processor = Aggregator::new(vec![], aggregations);
        let result = processor.process(df).unwrap();

        assert_eq!(result.height(), 1, "Global aggregation should return 1 row");
        let val = result
            .column("values_sum")
            .unwrap()
            .f64()
            .unwrap()
            .get(0)
            .unwrap();
        assert!(
            (val - 10.0).abs() < 1e-10,
            "Global sum of [1,2,3,4] = 10, got {}",
            val
        );
    }

    #[test]
    fn test_nonexistent_group_by_column_returns_error() {
        let df = make_df();
        let mut aggregations = HashMap::new();
        aggregations.insert("values".to_string(), AggregationOp::Mean);

        let processor = Aggregator::new(vec!["nonexistent_group".to_string()], aggregations);
        let result = processor.process(df);

        assert!(
            result.is_err(),
            "Nonexistent group_by column should return an error"
        );
        if let Err(PostProcessError::ColumnNotFound(col)) = result {
            assert_eq!(col, "nonexistent_group");
        } else {
            panic!("Expected ColumnNotFound error for group_by column");
        }
    }

    #[test]
    fn test_nonexistent_aggregation_column_returns_error() {
        let df = make_df();
        let mut aggregations = HashMap::new();
        aggregations.insert("nonexistent_values".to_string(), AggregationOp::Sum);

        let processor = Aggregator::new(vec!["group".to_string()], aggregations);
        let result = processor.process(df);

        assert!(
            result.is_err(),
            "Nonexistent aggregation column should return an error"
        );
        if let Err(PostProcessError::ColumnNotFound(col)) = result {
            assert_eq!(col, "nonexistent_values");
        } else {
            panic!("Expected ColumnNotFound error for aggregation column");
        }
    }
}

#[cfg(test)]
mod pipeline_edge_cases {
    use crate::postprocess::ProcessingPipeline;
    use polars::prelude::*;

    #[test]
    fn test_empty_pipeline_returns_dataframe_unchanged() {
        let df = df! {
            "a" => [1.0_f64, 2.0],
            "b" => [3.0_f64, 4.0],
        }
        .unwrap();

        let mut pipeline = ProcessingPipeline::new();
        let result = pipeline.execute(df.clone()).unwrap();

        assert_eq!(result.shape(), df.shape());

        let orig_cols: Vec<&str> = df.get_column_names().iter().map(|s| s.as_str()).collect();
        let res_cols: Vec<&str> = result
            .get_column_names()
            .iter()
            .map(|s| s.as_str())
            .collect();
        assert_eq!(orig_cols, res_cols, "Column names should be unchanged");
    }

    #[test]
    fn test_pipeline_default_creates_empty_pipeline() {
        let mut pipeline = ProcessingPipeline::default();

        let df = df! { "x" => [1.0_f64] }.unwrap();
        let result = pipeline.execute(df).unwrap();
        assert_eq!(result.height(), 1);
        assert!(result.column("x").is_ok());
    }

    #[test]
    fn test_pipeline_name_accessor() {
        let unnamed = ProcessingPipeline::new();
        assert_eq!(unnamed.name(), "Unnamed Pipeline");

        let named = ProcessingPipeline::with_name("My Custom Pipeline".to_string());
        assert_eq!(named.name(), "My Custom Pipeline");

        let config = crate::postprocess::ProcessingPipelineConfig {
            name: Some("Config Pipeline".to_string()),
            processors: vec![],
        };
        let from_config = ProcessingPipeline::from_config(&config).unwrap();
        assert_eq!(from_config.name(), "Config Pipeline");

        let config_no_name = crate::postprocess::ProcessingPipelineConfig {
            name: None,
            processors: vec![],
        };
        let from_config_unnamed = ProcessingPipeline::from_config(&config_no_name).unwrap();
        assert_eq!(from_config_unnamed.name(), "Configured Pipeline");
    }
}

#[cfg(test)]
mod unit_converter_meteorological {
    use crate::postprocess::{PostProcessor, UnitConverter};
    use polars::prelude::*;

    /// Build a single-column DataFrame containing the supplied values.
    fn make_df(values: &[f64]) -> DataFrame {
        df! { "val" => values }.unwrap()
    }

    /// Extract all f64 values from the named column.
    fn extract(df: &DataFrame, col_name: &str) -> Vec<f64> {
        df.column(col_name)
            .unwrap()
            .f64()
            .unwrap()
            .into_iter()
            .map(|v| v.unwrap())
            .collect()
    }

    /// Tolerance-based assertion for physical conversions (1e-4 relative to magnitude).
    fn assert_approx(actual: f64, expected: f64, label: &str) {
        assert!(
            (actual - expected).abs() < 1e-4_f64.max(expected.abs() * 1e-4),
            "{}: expected {}, got {}",
            label,
            expected,
            actual
        );
    }

    // ── Pressure ──────────────────────────────────────────────────────────────

    #[test]
    fn test_pa_to_hpa() {
        let df = make_df(&[101_325.0, 100_000.0]);
        let conv = UnitConverter::new("val".into(), "pa".into(), "hpa".into());
        let result = conv.process(df).unwrap();
        let v = extract(&result, "val");
        assert_approx(v[0], 1013.25, "101325 Pa → 1013.25 hPa");
        assert_approx(v[1], 1000.0, "100000 Pa → 1000.0 hPa");
    }

    #[test]
    fn test_hpa_to_pa() {
        let df = make_df(&[1013.25, 1000.0]);
        let conv = UnitConverter::new("val".into(), "hpa".into(), "pa".into());
        let result = conv.process(df).unwrap();
        let v = extract(&result, "val");
        assert_approx(v[0], 101_325.0, "1013.25 hPa → 101325 Pa");
        assert_approx(v[1], 100_000.0, "1000.0 hPa → 100000 Pa");
    }

    #[test]
    fn test_pa_to_atm() {
        let df = make_df(&[101_325.0]);
        let conv = UnitConverter::new("val".into(), "pa".into(), "atm".into());
        let result = conv.process(df).unwrap();
        let v = extract(&result, "val");
        assert_approx(v[0], 1.0, "101325 Pa → 1.0 atm");
    }

    #[test]
    fn test_atm_to_pa() {
        let df = make_df(&[1.0]);
        let conv = UnitConverter::new("val".into(), "atm".into(), "pa".into());
        let result = conv.process(df).unwrap();
        let v = extract(&result, "val");
        assert_approx(v[0], 101_325.0, "1 atm → 101325 Pa");
    }

    #[test]
    fn test_pa_to_inhg() {
        // 101325 Pa ≈ 29.921 inHg
        let df = make_df(&[101_325.0]);
        let conv = UnitConverter::new("val".into(), "pa".into(), "inhg".into());
        let result = conv.process(df).unwrap();
        let v = extract(&result, "val");
        assert_approx(v[0], 29.921, "101325 Pa → 29.921 inHg");
    }

    #[test]
    fn test_hpa_to_mbar() {
        // hPa and mbar are identical units (both = 100 Pa)
        let df = make_df(&[1013.25, 850.0]);
        let conv = UnitConverter::new("val".into(), "hpa".into(), "mbar".into());
        let result = conv.process(df).unwrap();
        let v = extract(&result, "val");
        assert_approx(v[0], 1013.25, "1013.25 hPa → 1013.25 mbar");
        assert_approx(v[1], 850.0, "850 hPa → 850 mbar");
    }

    #[test]
    fn test_pa_to_kpa() {
        let df = make_df(&[101_325.0, 50_000.0]);
        let conv = UnitConverter::new("val".into(), "pa".into(), "kpa".into());
        let result = conv.process(df).unwrap();
        let v = extract(&result, "val");
        assert_approx(v[0], 101.325, "101325 Pa → 101.325 kPa");
        assert_approx(v[1], 50.0, "50000 Pa → 50 kPa");
    }

    #[test]
    fn test_pa_to_mmhg() {
        // 101325 Pa ≈ 760.0 mmHg
        let df = make_df(&[101_325.0]);
        let conv = UnitConverter::new("val".into(), "pa".into(), "mmhg".into());
        let result = conv.process(df).unwrap();
        let v = extract(&result, "val");
        assert_approx(v[0], 760.0, "101325 Pa → 760 mmHg");
    }

    // ── Speed ──────────────────────────────────────────────────────────────────

    #[test]
    fn test_ms_to_kmh() {
        let df = make_df(&[1.0, 10.0]);
        let conv = UnitConverter::new("val".into(), "m/s".into(), "km/h".into());
        let result = conv.process(df).unwrap();
        let v = extract(&result, "val");
        assert_approx(v[0], 3.6, "1 m/s → 3.6 km/h");
        assert_approx(v[1], 36.0, "10 m/s → 36 km/h");
    }

    #[test]
    fn test_kmh_to_ms() {
        let df = make_df(&[3.6, 36.0]);
        let conv = UnitConverter::new("val".into(), "km/h".into(), "m/s".into());
        let result = conv.process(df).unwrap();
        let v = extract(&result, "val");
        assert_approx(v[0], 1.0, "3.6 km/h → 1 m/s");
        assert_approx(v[1], 10.0, "36 km/h → 10 m/s");
    }

    #[test]
    fn test_ms_to_kt() {
        // 1 kt ≈ 0.514444 m/s → 1 m/s ≈ 1.943844 kt
        let df = make_df(&[1.0]);
        let conv = UnitConverter::new("val".into(), "m/s".into(), "kt".into());
        let result = conv.process(df).unwrap();
        let v = extract(&result, "val");
        assert_approx(v[0], 1.943_844, "1 m/s → 1.943844 kt");
    }

    #[test]
    fn test_kt_to_ms() {
        let df = make_df(&[1.943_844]);
        let conv = UnitConverter::new("val".into(), "kt".into(), "m/s".into());
        let result = conv.process(df).unwrap();
        let v = extract(&result, "val");
        assert_approx(v[0], 1.0, "1.943844 kt → 1 m/s");
    }

    #[test]
    fn test_ms_to_mph() {
        // 1 m/s ≈ 2.236936 mph
        let df = make_df(&[1.0]);
        let conv = UnitConverter::new("val".into(), "m/s".into(), "mph".into());
        let result = conv.process(df).unwrap();
        let v = extract(&result, "val");
        assert_approx(v[0], 2.236_936, "1 m/s → 2.236936 mph");
    }

    #[test]
    fn test_mph_to_ms() {
        let df = make_df(&[2.236_936]);
        let conv = UnitConverter::new("val".into(), "mph".into(), "m/s".into());
        let result = conv.process(df).unwrap();
        let v = extract(&result, "val");
        assert_approx(v[0], 1.0, "2.236936 mph → 1 m/s");
    }

    #[test]
    fn test_ms_to_fts() {
        // 1 m/s ≈ 3.28084 ft/s
        let df = make_df(&[1.0]);
        let conv = UnitConverter::new("val".into(), "m/s".into(), "ft/s".into());
        let result = conv.process(df).unwrap();
        let v = extract(&result, "val");
        assert_approx(v[0], 3.280_84, "1 m/s → 3.28084 ft/s");
    }

    // ── Length ─────────────────────────────────────────────────────────────────

    #[test]
    fn test_m_to_km() {
        let df = make_df(&[1000.0, 500.0]);
        let conv = UnitConverter::new("val".into(), "m".into(), "km".into());
        let result = conv.process(df).unwrap();
        let v = extract(&result, "val");
        assert_approx(v[0], 1.0, "1000 m → 1 km");
        assert_approx(v[1], 0.5, "500 m → 0.5 km");
    }

    #[test]
    fn test_km_to_m() {
        let df = make_df(&[1.0, 0.5]);
        let conv = UnitConverter::new("val".into(), "km".into(), "m".into());
        let result = conv.process(df).unwrap();
        let v = extract(&result, "val");
        assert_approx(v[0], 1000.0, "1 km → 1000 m");
        assert_approx(v[1], 500.0, "0.5 km → 500 m");
    }

    #[test]
    fn test_m_to_ft() {
        // 1 m = 1/0.3048 ft ≈ 3.28084 ft
        let df = make_df(&[1.0]);
        let conv = UnitConverter::new("val".into(), "m".into(), "ft".into());
        let result = conv.process(df).unwrap();
        let v = extract(&result, "val");
        assert_approx(v[0], 3.280_84, "1 m → 3.28084 ft");
    }

    #[test]
    fn test_ft_to_m() {
        let df = make_df(&[3.280_84]);
        let conv = UnitConverter::new("val".into(), "ft".into(), "m".into());
        let result = conv.process(df).unwrap();
        let v = extract(&result, "val");
        assert_approx(v[0], 1.0, "3.28084 ft → 1 m");
    }

    #[test]
    fn test_m_to_mi() {
        // 1609.344 m = 1 mi
        let df = make_df(&[1609.344]);
        let conv = UnitConverter::new("val".into(), "m".into(), "mi".into());
        let result = conv.process(df).unwrap();
        let v = extract(&result, "val");
        assert_approx(v[0], 1.0, "1609.344 m → 1 mi");
    }

    #[test]
    fn test_mi_to_m() {
        let df = make_df(&[1.0]);
        let conv = UnitConverter::new("val".into(), "mi".into(), "m".into());
        let result = conv.process(df).unwrap();
        let v = extract(&result, "val");
        assert_approx(v[0], 1609.344, "1 mi → 1609.344 m");
    }

    #[test]
    fn test_m_to_nm() {
        // 1852 m = 1 nautical mile
        let df = make_df(&[1852.0]);
        let conv = UnitConverter::new("val".into(), "m".into(), "nm".into());
        let result = conv.process(df).unwrap();
        let v = extract(&result, "val");
        assert_approx(v[0], 1.0, "1852 m → 1 nm");
    }

    #[test]
    fn test_nm_to_m() {
        let df = make_df(&[1.0]);
        let conv = UnitConverter::new("val".into(), "nm".into(), "m".into());
        let result = conv.process(df).unwrap();
        let v = extract(&result, "val");
        assert_approx(v[0], 1852.0, "1 nm → 1852 m");
    }

    #[test]
    fn test_m_to_cm() {
        let df = make_df(&[1.0, 0.5]);
        let conv = UnitConverter::new("val".into(), "m".into(), "cm".into());
        let result = conv.process(df).unwrap();
        let v = extract(&result, "val");
        assert_approx(v[0], 100.0, "1 m → 100 cm");
        assert_approx(v[1], 50.0, "0.5 m → 50 cm");
    }

    #[test]
    fn test_m_to_mm() {
        let df = make_df(&[1.0]);
        let conv = UnitConverter::new("val".into(), "m".into(), "mm".into());
        let result = conv.process(df).unwrap();
        let v = extract(&result, "val");
        assert_approx(v[0], 1000.0, "1 m → 1000 mm");
    }

    // ── Cross-family indirect conversions ──────────────────────────────────────

    #[test]
    fn test_km_to_nm() {
        // 1 km = 1000 m; 1 nm = 1852 m → 1 km = 1000/1852 ≈ 0.5400 nm
        let df = make_df(&[1.0, 1852.0]);
        let conv = UnitConverter::new("val".into(), "km".into(), "nm".into());
        let result = conv.process(df).unwrap();
        let v = extract(&result, "val");
        assert_approx(v[0], 1000.0 / 1852.0, "1 km → 0.5400 nm");
        assert_approx(v[1], 1000.0, "1852 km → 1000 nm");
    }

    #[test]
    fn test_kt_to_mph() {
        // 1 kt ≈ 1.15078 mph
        let df = make_df(&[1.0]);
        let conv = UnitConverter::new("val".into(), "kt".into(), "mph".into());
        let result = conv.process(df).unwrap();
        let v = extract(&result, "val");
        // factor = (1/1.943844) / (1/2.236936) = 2.236936 / 1.943844 ≈ 1.15078
        let expected = 2.236_936_f64 / 1.943_844_f64;
        assert_approx(v[0], expected, "1 kt → 1.15078 mph");
    }

    // ── Case insensitivity ─────────────────────────────────────────────────────

    #[test]
    fn test_case_insensitive_hpa_to_pa() {
        let df = make_df(&[1013.25]);
        let conv = UnitConverter::new("val".into(), "HPA".into(), "PA".into());
        let result = conv.process(df).unwrap();
        let v = extract(&result, "val");
        assert_approx(v[0], 101_325.0, "HPA → PA case insensitive");
    }

    #[test]
    fn test_case_insensitive_kmh_to_ms() {
        let df = make_df(&[3.6]);
        let conv = UnitConverter::new("val".into(), "Km/H".into(), "M/S".into());
        let result = conv.process(df).unwrap();
        let v = extract(&result, "val");
        assert_approx(v[0], 1.0, "Km/H → M/S case insensitive");
    }

    // ── Unknown pair fallthrough ───────────────────────────────────────────────

    #[test]
    fn test_unknown_pair_fallthrough_is_noop() {
        let df = make_df(&[42.0, 7.5]);
        let conv = UnitConverter::new("val".into(), "furlongs".into(), "fortnights".into());
        let result = conv.process(df).unwrap();
        let v = extract(&result, "val");
        assert!(
            (v[0] - 42.0).abs() < 1e-10,
            "Unknown units: value should be unchanged (got {})",
            v[0]
        );
        assert!(
            (v[1] - 7.5).abs() < 1e-10,
            "Unknown units: value should be unchanged (got {})",
            v[1]
        );
    }

    // ── Temperature regression ─────────────────────────────────────────────────

    #[test]
    fn test_regression_k_to_c() {
        let df = make_df(&[273.15, 373.15]);
        let conv = UnitConverter::new("val".into(), "k".into(), "c".into());
        let result = conv.process(df).unwrap();
        let v = extract(&result, "val");
        assert!((v[0] - 0.0).abs() < 1e-6, "273.15 K → 0 °C");
        assert!((v[1] - 100.0).abs() < 1e-6, "373.15 K → 100 °C");
    }

    #[test]
    fn test_regression_c_to_f() {
        let df = make_df(&[0.0, 100.0]);
        let conv = UnitConverter::new("val".into(), "c".into(), "f".into());
        let result = conv.process(df).unwrap();
        let v = extract(&result, "val");
        assert!((v[0] - 32.0).abs() < 1e-6, "0 °C → 32 °F");
        assert!((v[1] - 212.0).abs() < 1e-6, "100 °C → 212 °F");
    }

    // ── Pipeline batching ──────────────────────────────────────────────────────

    #[test]
    fn test_pipeline_batching_two_unit_converters() {
        use crate::postprocess::ProcessingPipeline;

        let df = df! {
            "pressure_pa" => [101_325.0_f64],
            "wind_ms"     => [10.0_f64],
        }
        .unwrap();

        let mut pipeline = ProcessingPipeline::new();
        pipeline.add_processor(Box::new(UnitConverter::new(
            "pressure_pa".into(),
            "pa".into(),
            "hpa".into(),
        )));
        pipeline.add_processor(Box::new(UnitConverter::new(
            "wind_ms".into(),
            "m/s".into(),
            "km/h".into(),
        )));

        let result = pipeline.execute(df).unwrap();

        let pres: Vec<f64> = result
            .column("pressure_pa")
            .unwrap()
            .f64()
            .unwrap()
            .into_iter()
            .map(|v| v.unwrap())
            .collect();
        let wind: Vec<f64> = result
            .column("wind_ms")
            .unwrap()
            .f64()
            .unwrap()
            .into_iter()
            .map(|v| v.unwrap())
            .collect();

        assert_approx(pres[0], 1013.25, "101325 Pa → 1013.25 hPa via pipeline");
        assert_approx(wind[0], 36.0, "10 m/s → 36 km/h via pipeline");
    }
}

#[cfg(test)]
mod post_process_error_display {
    use crate::postprocess::PostProcessError;
    use polars::prelude::PolarsError;

    #[test]
    fn test_error_display_output() {
        let col_not_found = PostProcessError::ColumnNotFound("my_col".to_string());
        let msg = col_not_found.to_string();
        assert!(
            msg.contains("my_col"),
            "ColumnNotFound display should contain column name, got: {}",
            msg
        );
        assert!(
            msg.contains("not found"),
            "ColumnNotFound display should say 'not found', got: {}",
            msg
        );

        let conversion_err = PostProcessError::ConversionError("bad value".to_string());
        let msg = conversion_err.to_string();
        assert!(
            msg.contains("Conversion error"),
            "ConversionError display should contain 'Conversion error', got: {}",
            msg
        );
        assert!(
            msg.contains("bad value"),
            "ConversionError display should contain the message, got: {}",
            msg
        );

        let config_err = PostProcessError::ConfigurationError("bad config".to_string());
        let msg = config_err.to_string();
        assert!(
            msg.contains("Configuration error"),
            "ConfigurationError display should contain 'Configuration error', got: {}",
            msg
        );
        assert!(
            msg.contains("bad config"),
            "ConfigurationError display should contain the message, got: {}",
            msg
        );

        let polars_err = PostProcessError::PolarsError(PolarsError::ColumnNotFound("x".into()));
        let msg = polars_err.to_string();
        assert!(
            msg.contains("Polars error"),
            "PolarsError display should contain 'Polars error', got: {}",
            msg
        );

        let processing_err = PostProcessError::ProcessingError("something went wrong".to_string());
        let msg = processing_err.to_string();
        assert!(
            msg.contains("Processing error"),
            "ProcessingError display should contain 'Processing error', got: {}",
            msg
        );
        assert!(
            msg.contains("something went wrong"),
            "ProcessingError display should contain the message, got: {}",
            msg
        );
    }
}

#[cfg(test)]
mod formula_parser_functions {
    use crate::postprocess::{FormulaApplier, PostProcessError, PostProcessor};
    use polars::prelude::*;
    use std::f64::consts;

    // ── Helpers ──────────────────────────────────────────────────────────────

    fn make_df(col_name: &str, values: &[f64]) -> DataFrame {
        df! { col_name => values }.unwrap()
    }

    fn make_df2(col_a: &str, a: &[f64], col_b: &str, b: &[f64]) -> DataFrame {
        df! { col_a => a, col_b => b }.unwrap()
    }

    fn extract_f64_opt(df: &DataFrame, col_name: &str) -> Vec<Option<f64>> {
        df.column(col_name)
            .unwrap()
            .f64()
            .unwrap()
            .into_iter()
            .collect()
    }

    fn extract_f64(df: &DataFrame, col_name: &str) -> Vec<f64> {
        extract_f64_opt(df, col_name)
            .into_iter()
            .map(|v| v.unwrap())
            .collect()
    }

    fn apply(formula: &str, df: DataFrame, src: &[&str]) -> DataFrame {
        FormulaApplier::new(
            "result".to_string(),
            formula.to_string(),
            src.iter().map(|s| s.to_string()).collect(),
        )
        .process(df)
        .unwrap()
    }

    fn apply_err(formula: &str, df: DataFrame, src: &[&str]) -> PostProcessError {
        FormulaApplier::new(
            "result".to_string(),
            formula.to_string(),
            src.iter().map(|s| s.to_string()).collect(),
        )
        .process(df)
        .unwrap_err()
    }

    fn assert_close(actual: f64, expected: f64, label: &str) {
        assert!(
            (actual - expected).abs() < 1e-6,
            "{}: expected {:.10}, got {:.10}",
            label,
            expected,
            actual
        );
    }

    // ── Unary: abs ───────────────────────────────────────────────────────────

    #[test]
    fn test_abs_basic() {
        let df = make_df("value", &[-5.0, 3.0, -1.0]);
        let result = apply("abs(value)", df, &["value"]);
        let v = extract_f64(&result, "result");
        assert_close(v[0], 5.0, "abs(-5) = 5");
        assert_close(v[1], 3.0, "abs(3) = 3");
        assert_close(v[2], 1.0, "abs(-1) = 1");
    }

    // ── Unary: sqrt ──────────────────────────────────────────────────────────

    #[test]
    fn test_sqrt_basic() {
        let df = make_df("value", &[4.0, 9.0, 16.0, 25.0]);
        let result = apply("sqrt(value)", df, &["value"]);
        let v = extract_f64(&result, "result");
        assert_close(v[0], 2.0, "sqrt(4) = 2");
        assert_close(v[1], 3.0, "sqrt(9) = 3");
        assert_close(v[2], 4.0, "sqrt(16) = 4");
        assert_close(v[3], 5.0, "sqrt(25) = 5");
    }

    // ── Unary: exp ───────────────────────────────────────────────────────────

    #[test]
    fn test_exp_basic() {
        let df = make_df("x", &[0.0, 1.0]);
        let result = apply("exp(x)", df, &["x"]);
        let v = extract_f64(&result, "result");
        assert_close(v[0], 1.0, "exp(0) = 1");
        assert_close(v[1], consts::E, "exp(1) = e");
    }

    // ── Unary: ln ────────────────────────────────────────────────────────────

    #[test]
    fn test_ln_basic() {
        let df = make_df("x", &[1.0, consts::E]);
        let result = apply("ln(x)", df, &["x"]);
        let v = extract_f64(&result, "result");
        assert_close(v[0], 0.0, "ln(1) = 0");
        assert_close(v[1], 1.0, "ln(e) = 1");
    }

    // ── Unary: log10 ─────────────────────────────────────────────────────────

    #[test]
    fn test_log10_basic() {
        let df = make_df("x", &[1.0, 100.0]);
        let result = apply("log10(x)", df, &["x"]);
        let v = extract_f64(&result, "result");
        assert_close(v[0], 0.0, "log10(1) = 0");
        assert_close(v[1], 2.0, "log10(100) = 2");
    }

    // ── Unary: sin ───────────────────────────────────────────────────────────

    #[test]
    fn test_sin_basic() {
        let df = make_df("x", &[0.0, consts::FRAC_PI_2]);
        let result = apply("sin(x)", df, &["x"]);
        let v = extract_f64(&result, "result");
        assert_close(v[0], 0.0, "sin(0) = 0");
        assert_close(v[1], 1.0, "sin(pi/2) = 1");
    }

    // ── Unary: cos ───────────────────────────────────────────────────────────

    #[test]
    fn test_cos_basic() {
        let df = make_df("x", &[0.0, consts::PI]);
        let result = apply("cos(x)", df, &["x"]);
        let v = extract_f64(&result, "result");
        assert_close(v[0], 1.0, "cos(0) = 1");
        assert_close(v[1], -1.0, "cos(pi) = -1");
    }

    // ── Unary: tan ───────────────────────────────────────────────────────────

    #[test]
    fn test_tan_basic() {
        let df = make_df("x", &[0.0]);
        let result = apply("tan(x)", df, &["x"]);
        let v = extract_f64(&result, "result");
        assert_close(v[0], 0.0, "tan(0) = 0");
    }

    // ── Unary: ceil ──────────────────────────────────────────────────────────

    #[test]
    fn test_ceil_basic() {
        let df = make_df("x", &[1.1, 2.9, -0.5]);
        let result = apply("ceil(x)", df, &["x"]);
        let v = extract_f64(&result, "result");
        assert_close(v[0], 2.0, "ceil(1.1) = 2");
        assert_close(v[1], 3.0, "ceil(2.9) = 3");
        assert_close(v[2], 0.0, "ceil(-0.5) = 0");
    }

    // ── Unary: floor ─────────────────────────────────────────────────────────

    #[test]
    fn test_floor_basic() {
        let df = make_df("x", &[1.9, 2.1, -0.5]);
        let result = apply("floor(x)", df, &["x"]);
        let v = extract_f64(&result, "result");
        assert_close(v[0], 1.0, "floor(1.9) = 1");
        assert_close(v[1], 2.0, "floor(2.1) = 2");
        assert_close(v[2], -1.0, "floor(-0.5) = -1");
    }

    // ── Unary: round ─────────────────────────────────────────────────────────

    #[test]
    fn test_round_basic() {
        // round(0) means round to nearest integer.
        // 1.4 -> 1, 1.5 -> 2 (half-away-from-zero), 2.5 -> 3
        let df = make_df("x", &[1.4, 1.5, 2.5]);
        let result = apply("round(x)", df, &["x"]);
        let v = extract_f64(&result, "result");
        assert_close(v[0], 1.0, "round(1.4) = 1");
        assert_close(v[1], 2.0, "round(1.5) = 2 (half-away-from-zero)");
        assert_close(v[2], 3.0, "round(2.5) = 3 (half-away-from-zero)");
    }

    // ── Binary: pow ──────────────────────────────────────────────────────────

    #[test]
    fn test_pow_basic() {
        let df = make_df("x", &[2.0, 3.0, 4.0]);
        let result = apply("pow(x, 2.0)", df, &["x"]);
        let v = extract_f64(&result, "result");
        assert_close(v[0], 4.0, "pow(2, 2) = 4");
        assert_close(v[1], 9.0, "pow(3, 2) = 9");
        assert_close(v[2], 16.0, "pow(4, 2) = 16");
    }

    // ── Binary: min ──────────────────────────────────────────────────────────

    #[test]
    fn test_min_basic() {
        let df = make_df2("a", &[1.0], "b", &[3.0]);
        let result = apply("min(a, b)", df, &["a", "b"]);
        let v = extract_f64(&result, "result");
        assert_close(v[0], 1.0, "min(1, 3) = 1");
    }

    #[test]
    fn test_min_second_wins() {
        let df = make_df2("a", &[5.0], "b", &[2.0]);
        let result = apply("min(a, b)", df, &["a", "b"]);
        let v = extract_f64(&result, "result");
        assert_close(v[0], 2.0, "min(5, 2) = 2");
    }

    // ── Binary: max ──────────────────────────────────────────────────────────

    #[test]
    fn test_max_basic() {
        let df = make_df2("a", &[1.0], "b", &[3.0]);
        let result = apply("max(a, b)", df, &["a", "b"]);
        let v = extract_f64(&result, "result");
        assert_close(v[0], 3.0, "max(1, 3) = 3");
    }

    // ── Binary: log ──────────────────────────────────────────────────────────

    #[test]
    fn test_log_base10() {
        // log(100, 10) = 2
        let df = make_df("x", &[100.0]);
        let result = apply("log(x, 10.0)", df, &["x"]);
        let v = extract_f64(&result, "result");
        assert_close(v[0], 2.0, "log(100, 10) = 2");
    }

    // ── Nested functions ─────────────────────────────────────────────────────

    #[test]
    fn test_nested_abs_minus_times_literal() {
        // abs(a - b) * 2.0: a=5, b=3 → abs(2) * 2 = 4
        let df = make_df2("a", &[5.0], "b", &[3.0]);
        let result = apply("abs(a - b) * 2.0", df, &["a", "b"]);
        let v = extract_f64(&result, "result");
        assert_close(v[0], 4.0, "abs(5-3)*2 = 4");
    }

    #[test]
    fn test_nested_sqrt_sum_of_squares_pythagorean() {
        // sqrt(a*a + b*b): a=3, b=4 → sqrt(9+16) = sqrt(25) = 5
        let df = make_df2("a", &[3.0], "b", &[4.0]);
        let result = apply("sqrt(a * a + b * b)", df, &["a", "b"]);
        let v = extract_f64(&result, "result");
        assert_close(v[0], 5.0, "sqrt(3^2 + 4^2) = 5 (Pythagorean triple)");
    }

    #[test]
    fn test_nested_pow_abs() {
        // pow(abs(value), 0.5): value=-4 → pow(4, 0.5) = 2
        let df = make_df("value", &[-4.0]);
        let result = apply("pow(abs(value), 0.5)", df, &["value"]);
        let v = extract_f64(&result, "result");
        assert_close(v[0], 2.0, "pow(abs(-4), 0.5) = 2");
    }

    // ── Functions in arithmetic expressions ──────────────────────────────────

    #[test]
    fn test_abs_plus_pow() {
        // abs(a) + pow(b, 2.0): a=-3, b=4 → 3 + 16 = 19
        let df = make_df2("a", &[-3.0], "b", &[4.0]);
        let result = apply("abs(a) + pow(b, 2.0)", df, &["a", "b"]);
        let v = extract_f64(&result, "result");
        assert_close(v[0], 19.0, "abs(-3) + pow(4, 2) = 19");
    }

    // ── Domain edge cases ────────────────────────────────────────────────────

    #[test]
    fn test_sqrt_negative_produces_nan() {
        let df = make_df("value", &[-1.0]);
        let result = apply("sqrt(value)", df, &["value"]);
        let v = extract_f64_opt(&result, "result");
        let first = v[0];
        assert!(
            first.is_none() || first.map(|x| x.is_nan()).unwrap_or(false),
            "sqrt(-1) should be NaN or null, got {:?}",
            first
        );
    }

    #[test]
    fn test_ln_zero_produces_neg_infinity() {
        let df = make_df("x", &[0.0]);
        let result = apply("ln(x)", df, &["x"]);
        let v = extract_f64_opt(&result, "result");
        // ln(0) = -infinity (IEEE 754)
        let val = v[0];
        assert!(
            val.map(|x| x.is_infinite() && x < 0.0).unwrap_or(false),
            "ln(0) should be -infinity, got {:?}",
            val
        );
    }

    // ── Case insensitivity ───────────────────────────────────────────────────

    #[test]
    fn test_case_insensitive_abs_uppercase() {
        let df = make_df("x", &[-7.0, 3.0]);
        let result = apply("ABS(x)", df, &["x"]);
        let v = extract_f64(&result, "result");
        assert_close(v[0], 7.0, "ABS(-7) = 7");
        assert_close(v[1], 3.0, "ABS(3) = 3");
    }

    #[test]
    fn test_case_insensitive_sqrt_mixed_case() {
        let df = make_df("x", &[9.0, 16.0]);
        let result = apply("Sqrt(x)", df, &["x"]);
        let v = extract_f64(&result, "result");
        assert_close(v[0], 3.0, "Sqrt(9) = 3");
        assert_close(v[1], 4.0, "Sqrt(16) = 4");
    }

    // ── Error cases ──────────────────────────────────────────────────────────

    #[test]
    fn test_unknown_function_error() {
        let df = make_df("x", &[1.0]);
        let err = apply_err("foobar(x)", df, &["x"]);
        let msg = err.to_string();
        assert!(
            msg.contains("Unknown function"),
            "Expected 'Unknown function' in error, got: {}",
            msg
        );
        assert!(
            msg.contains("foobar"),
            "Error should mention the function name, got: {}",
            msg
        );
    }

    #[test]
    fn test_wrong_arity_unary_too_many_args() {
        let df = make_df2("a", &[1.0], "b", &[2.0]);
        let err = apply_err("abs(a, b)", df, &["a", "b"]);
        let msg = err.to_string();
        assert!(
            msg.contains("expects") && msg.contains("1") && msg.contains("2"),
            "Expected arity error mentioning expected=1 and got=2, got: {}",
            msg
        );
    }

    #[test]
    fn test_wrong_arity_binary_too_few_args() {
        let df = make_df("x", &[2.0]);
        let err = apply_err("pow(x)", df, &["x"]);
        let msg = err.to_string();
        assert!(
            msg.contains("expects") && msg.contains("2") && msg.contains("1"),
            "Expected arity error mentioning expected=2 and got=1, got: {}",
            msg
        );
    }

    // ── Backward compatibility ────────────────────────────────────────────────

    #[test]
    fn test_backward_compat_sqrt_formula() {
        // The original sqrt(value) formula must continue to work.
        let df = df! { "value" => [4.0f64, 9.0, 16.0, 25.0] }.unwrap();
        let processor = FormulaApplier::new(
            "sqrt_value".to_string(),
            "sqrt(value)".to_string(),
            vec!["value".to_string()],
        );
        let result = processor.process(df).unwrap();
        let v: Vec<f64> = result
            .column("sqrt_value")
            .unwrap()
            .f64()
            .unwrap()
            .into_iter()
            .map(|v| v.unwrap())
            .collect();
        assert_close(v[0], 2.0, "sqrt(4) = 2");
        assert_close(v[3], 5.0, "sqrt(25) = 5");
    }

    #[test]
    fn test_backward_compat_arithmetic_formula() {
        let df = df! {
            "a" => [1.0f64, 2.0],
            "b" => [3.0f64, 4.0],
        }
        .unwrap();
        let processor = FormulaApplier::new(
            "result".to_string(),
            "a + b * 2.0".to_string(),
            vec!["a".to_string(), "b".to_string()],
        );
        let result = processor.process(df).unwrap();
        let v: Vec<f64> = result
            .column("result")
            .unwrap()
            .f64()
            .unwrap()
            .into_iter()
            .map(|v| v.unwrap())
            .collect();
        // a + b*2 = 1 + 6 = 7, 2 + 8 = 10
        assert_close(v[0], 7.0, "1 + 3*2 = 7");
        assert_close(v[1], 10.0, "2 + 4*2 = 10");
    }

    #[test]
    fn test_backward_compat_comparison_formula() {
        let df = df! {
            "a" => [1.0f64, 5.0, 3.0],
            "b" => [2.0f64, 3.0, 3.0],
        }
        .unwrap();
        let processor = FormulaApplier::new(
            "cmp".to_string(),
            "a < b".to_string(),
            vec!["a".to_string(), "b".to_string()],
        );
        let result = processor.process(df).unwrap();
        let values: Vec<bool> = result
            .column("cmp")
            .unwrap()
            .bool()
            .unwrap()
            .into_iter()
            .map(|v| v.unwrap())
            .collect();
        assert!(values[0], "1 < 2 is true");
        assert!(!values[1], "5 < 3 is false");
        assert!(!values[2], "3 < 3 is false");
    }
}
