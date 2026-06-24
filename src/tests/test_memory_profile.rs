/// DHAT heap-profiling integration test for the full nc2parquet pipeline.
///
/// Run with:
///
/// ```text
/// cargo test --features dhat-heap --lib -- memory_profile::dhat_profile --nocapture
/// ```
///
/// DHAT writes `dhat-heap.json` to the current directory upon test completion.
/// Visualise with: <https://nnethercote.github.io/dh_view/dh_view.html>
#[cfg(feature = "dhat-heap")]
mod memory_profile {
    use crate::input::JobConfig;
    use crate::test_helpers::get_test_data_path;

    #[test]
    fn dhat_profile() {
        let _profiler = dhat::Profiler::new_heap();

        let temp_dir = tempfile::tempdir().expect("failed to create temp dir");
        let output_path = temp_dir.path().join("profiled_output.parquet");

        let config = JobConfig {
            nc_key: get_test_data_path("pres_temp_4D.nc")
                .to_string_lossy()
                .to_string(),
            variable_name: "temperature".to_string(),
            variable_names: None,
            merge_variable_names: None,
            parquet_key: output_path.to_string_lossy().to_string(),
            filters: vec![],
            postprocessing: None,
            output: None,
        };

        crate::process_netcdf_job(&config)
            .expect("process_netcdf_job must succeed for memory profile test");

        assert!(
            output_path.exists(),
            "profiled output parquet file must be created"
        );
    }
}
