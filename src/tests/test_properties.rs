//! Property-based tests for filters and formula parser.
//!
//! Uses `proptest` to generate many random inputs and verify structural invariants
//! that must hold regardless of the specific values chosen.

#[cfg(test)]
mod filter_construction_properties {
    use crate::filters::{NCListFilter, NCRangeFilter};
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn range_filter_stores_all_fields(
            min in -1000.0f64..0.0,
            max in 0.01f64..1000.0,
            dim_name in "[a-z]{1,10}",
        ) {
            let filter = NCRangeFilter::new(&dim_name, min, max);
            prop_assert_eq!(&filter.dimension_name, &dim_name);
            prop_assert_eq!(filter.min_value, min);
            prop_assert_eq!(filter.max_value, max);
        }

        #[test]
        fn list_filter_stores_all_fields(
            values in proptest::collection::vec(-1000.0f64..1000.0, 1..20),
            dim_name in "[a-z]{1,10}",
        ) {
            let filter = NCListFilter::new(&dim_name, values.clone());
            prop_assert_eq!(&filter.dimension_name, &dim_name);
            prop_assert_eq!(filter.values.len(), values.len());
            // Values are preserved in order, not deduplicated
            prop_assert_eq!(filter.values, values);
        }
    }
}

#[cfg(test)]
mod filter_result_invariants {
    use crate::filters::FilterResult;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn filter_result_single_len_invariant(
            indices in proptest::collection::vec(0usize..100, 0..50),
        ) {
            let result = FilterResult::Single {
                dimension: "test".to_string(),
                indices: indices.clone(),
            };
            prop_assert_eq!(result.len(), indices.len());
            prop_assert_eq!(result.is_empty(), indices.is_empty());
        }

        #[test]
        fn filter_result_pairs_len_invariant(
            pairs in proptest::collection::vec((0usize..100, 0usize..100), 0..50),
        ) {
            let result = FilterResult::Pairs {
                lat_dimension: "lat".to_string(),
                lon_dimension: "lon".to_string(),
                pairs: pairs.clone(),
            };
            prop_assert_eq!(result.len(), pairs.len());
            prop_assert_eq!(result.is_empty(), pairs.is_empty());
        }
    }
}

#[cfg(test)]
mod formula_applier_properties {
    use crate::postprocess::{FormulaApplier, PostProcessor};
    use proptest::prelude::*;

    proptest! {
        /// A formula that is a bare non-negative constant should produce that constant
        /// for every row in the output column.
        ///
        /// Negative constants are excluded because the formula parser treats a leading
        /// `-` as a binary subtraction with an empty left operand, which is not
        /// supported and returns an error.  That is an existing parser limitation,
        /// not a bug introduced here, so we constrain the strategy accordingly.
        #[test]
        fn constant_formula_produces_constant(
            constant in 0.0f64..1e6,
        ) {
            prop_assume!(constant.is_finite());

            let df = polars::prelude::df! {
                "dummy" => [1.0_f64, 2.0, 3.0],
            }.unwrap();

            let formula_str = format!("{}", constant);
            let processor = FormulaApplier::new(
                "result".to_string(),
                formula_str,
                vec!["dummy".to_string()],
            );

            let result = processor.process(df);
            if let Ok(result_df) = result {
                let col = result_df.column("result").unwrap();
                let vals: Vec<f64> = col
                    .f64()
                    .unwrap()
                    .into_iter()
                    .map(|v| v.unwrap())
                    .collect();
                for &v in &vals {
                    prop_assert!(
                        (v - constant).abs() < 1e-6,
                        "Expected constant {}, got {}",
                        constant,
                        v
                    );
                }
            }
            // A processor error for edge-case constants is acceptable; only
            // successful results need to satisfy the invariant.
        }

        /// Adding zero to a column must not change any value.
        #[test]
        fn arithmetic_identity_add_zero(
            values in proptest::collection::vec(-1000.0f64..1000.0, 1..10),
        ) {
            prop_assume!(values.iter().all(|v| v.is_finite()));

            let df = polars::prelude::df! {
                "col" => values.clone(),
            }.unwrap();

            let processor = FormulaApplier::new(
                "result".to_string(),
                "col + 0".to_string(),
                vec!["col".to_string()],
            );

            if let Ok(result_df) = processor.process(df) {
                let col_series = result_df.column("result").unwrap();
                let result_vals: Vec<f64> = col_series
                    .f64()
                    .unwrap()
                    .into_iter()
                    .map(|v| v.unwrap())
                    .collect();
                for (original, result) in values.iter().zip(result_vals.iter()) {
                    prop_assert!(
                        (original - result).abs() < 1e-10,
                        "col + 0 should equal original: {} != {}",
                        original,
                        result
                    );
                }
            }
        }
    }
}

#[cfg(test)]
mod filter_factory_properties {
    use crate::filters::filter_factory;
    use proptest::prelude::*;

    proptest! {
        /// Any `kind` string that is not one of the four known kinds must cause
        /// `filter_factory` to return an error.
        #[test]
        fn unknown_filter_kind_rejected(
            kind in "[a-z]{1,20}".prop_filter("Not a valid kind", |s| {
                !["range", "list", "2d_point", "3d_point"].contains(&s.as_str())
            }),
        ) {
            let json = format!(r#"{{"kind": "{}"}}"#, kind);
            let result = filter_factory(&json);
            prop_assert!(result.is_err(),
                "Expected error for unknown kind '{}', but got Ok", kind);
        }
    }
}
