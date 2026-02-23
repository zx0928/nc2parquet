use criterion::{BatchSize, Criterion, criterion_group, criterion_main};
use nc2parquet::input::{FilterConfig, JobConfig, Point2DParams, RangeParams};
use std::path::PathBuf;

fn fixture_path(name: &str) -> String {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("examples/data")
        .join(name)
        .to_string_lossy()
        .to_string()
}

fn bench_extraction(c: &mut Criterion) {
    let mut group = c.benchmark_group("extraction");

    // 2D no filter — simple_xy.nc, variable "data"
    group.bench_function("simple_xy_no_filter", |b| {
        b.iter_batched(
            || {
                let dir = tempfile::TempDir::new().unwrap();
                let output = dir.path().join("out.parquet").to_string_lossy().to_string();
                let config = JobConfig {
                    nc_key: fixture_path("simple_xy.nc"),
                    variable_name: "data".to_string(),
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

    // 4D no filter — pres_temp_4D.nc: time(2)*level(2)*latitude(6)*longitude(12) = 288 rows
    group.bench_function("pres_temp_4d_no_filter", |b| {
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

    // 4D with range filter on latitude (30–45 → 4 out of 6 values retained)
    group.bench_function("pres_temp_4d_range_filter", |b| {
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
                            max_value: 45.0,
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

    // 4D with 2D point filter (lat/lon pair selection — NC3DPointFilter would fail on this file
    // because it has no time coordinate variable, so we use NC2DPointFilter here)
    group.bench_function("pres_temp_4d_point2d_filter", |b| {
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

criterion_group!(benches, bench_extraction);
criterion_main!(benches);
