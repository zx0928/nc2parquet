use criterion::{BatchSize, Criterion, criterion_group, criterion_main};
use nc2parquet::input::{FilterConfig, JobConfig, ListParams, Point2DParams, RangeParams};
use std::path::PathBuf;

fn fixture_path(name: &str) -> String {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("examples/data")
        .join(name)
        .to_string_lossy()
        .to_string()
}

fn bench_combinations(c: &mut Criterion) {
    let mut group = c.benchmark_group("combinations");

    // Unfiltered 4D: time(2) * level(2) * latitude(6) * longitude(12) = 288 combinations
    group.bench_function("unfiltered_4d_288_combos", |b| {
        b.iter_batched(
            || {
                let dir = tempfile::TempDir::new().unwrap();
                let output = dir.path().join("out.parquet").to_string_lossy().to_string();
                let config = JobConfig {
                    nc_key: fixture_path("pres_temp_4D.nc"),
                    variable_name: "temperature".to_string(),
                    parquet_key: output,
                    filters: vec![],
                    postprocessing: None,
                };
                (dir, config)
            },
            |(_dir, config)| nc2parquet::process_netcdf_job(&config).unwrap(),
            BatchSize::SmallInput,
        )
    });

    // Range filter reducing latitude from 6→3 (30, 35, 40):
    // time(2) * level(2) * latitude(3) * longitude(12) = 144 combinations
    group.bench_function("range_lat_6_to_3_144_combos", |b| {
        b.iter_batched(
            || {
                let dir = tempfile::TempDir::new().unwrap();
                let output = dir.path().join("out.parquet").to_string_lossy().to_string();
                let config = JobConfig {
                    nc_key: fixture_path("pres_temp_4D.nc"),
                    variable_name: "temperature".to_string(),
                    parquet_key: output,
                    filters: vec![FilterConfig::Range {
                        params: RangeParams {
                            dimension_name: "latitude".to_string(),
                            min_value: 30.0,
                            max_value: 40.0,
                        },
                    }],
                    postprocessing: None,
                };
                (dir, config)
            },
            |(_dir, config)| nc2parquet::process_netcdf_job(&config).unwrap(),
            BatchSize::SmallInput,
        )
    });

    // Range filter on latitude + list filter on longitude (very selective):
    // time(2) * level(2) * latitude(2=[30,35]) * longitude(2=[-120,-85]) = 16 combinations
    group.bench_function("range_lat_and_list_lon_16_combos", |b| {
        b.iter_batched(
            || {
                let dir = tempfile::TempDir::new().unwrap();
                let output = dir.path().join("out.parquet").to_string_lossy().to_string();
                let config = JobConfig {
                    nc_key: fixture_path("pres_temp_4D.nc"),
                    variable_name: "temperature".to_string(),
                    parquet_key: output,
                    filters: vec![
                        FilterConfig::Range {
                            params: RangeParams {
                                dimension_name: "latitude".to_string(),
                                min_value: 30.0,
                                max_value: 35.0,
                            },
                        },
                        FilterConfig::List {
                            params: ListParams {
                                dimension_name: "longitude".to_string(),
                                values: vec![-120.0, -85.0],
                            },
                        },
                    ],
                    postprocessing: None,
                };
                (dir, config)
            },
            |(_dir, config)| nc2parquet::process_netcdf_job(&config).unwrap(),
            BatchSize::SmallInput,
        )
    });

    // 2D point filter: selects specific lat/lon pairs, all time and level steps.
    // 2 pairs * time(2) * level(2) = 8 combinations (highly selective)
    group.bench_function("point2d_filter_8_combos", |b| {
        b.iter_batched(
            || {
                let dir = tempfile::TempDir::new().unwrap();
                let output = dir.path().join("out.parquet").to_string_lossy().to_string();
                let config = JobConfig {
                    nc_key: fixture_path("pres_temp_4D.nc"),
                    variable_name: "temperature".to_string(),
                    parquet_key: output,
                    filters: vec![FilterConfig::Point2D {
                        params: Point2DParams {
                            lat_dimension_name: "latitude".to_string(),
                            lon_dimension_name: "longitude".to_string(),
                            points: vec![(30.0, -120.0), (40.0, -100.0)],
                            tolerance: 1.0,
                        },
                    }],
                    postprocessing: None,
                };
                (dir, config)
            },
            |(_dir, config)| nc2parquet::process_netcdf_job(&config).unwrap(),
            BatchSize::SmallInput,
        )
    });

    group.finish();
}

criterion_group!(benches, bench_combinations);
criterion_main!(benches);
