use criterion::{Criterion, criterion_group, criterion_main};
use nc2parquet::filters::{NC2DPointFilter, NCFilter, NCListFilter, NCRangeFilter};
use std::path::PathBuf;

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("examples/data")
        .join(name)
}

fn bench_filters(c: &mut Criterion) {
    // pres_temp_4D.nc: time(2), level(2), latitude(6=[25,30,35,40,45,50]),
    //                  longitude(12=[-125,-120,...,-70])
    let path = fixture_path("pres_temp_4D.nc");
    let file = netcdf::open(&path).expect("Failed to open pres_temp_4D.nc fixture");

    let mut group = c.benchmark_group("filters");

    // Range filter on latitude: selects indices 1..=4 (30.0–45.0), 4 of 6 values
    group.bench_function("range_latitude_30_45", |b| {
        let filter = NCRangeFilter::new("latitude", 30.0, 45.0);
        b.iter(|| filter.apply(&file).unwrap())
    });

    // Range filter on longitude: selects a subset of the 12 longitude values
    group.bench_function("range_longitude_120_90", |b| {
        let filter = NCRangeFilter::new("longitude", -120.0, -90.0);
        b.iter(|| filter.apply(&file).unwrap())
    });

    // List filter on latitude: discrete values — matches 3 of 6 latitude values
    group.bench_function("list_latitude_25_35_50", |b| {
        let filter = NCListFilter::new("latitude", vec![25.0, 35.0, 50.0]);
        b.iter(|| filter.apply(&file).unwrap())
    });

    // List filter on longitude: matches 2 of 12 longitude values
    group.bench_function("list_longitude_two_values", |b| {
        let filter = NCListFilter::new("longitude", vec![-120.0, -85.0]);
        b.iter(|| filter.apply(&file).unwrap())
    });

    // 2D point filter on lat/lon: selects grid cells within 1.0 degree of 2 target points
    group.bench_function("point2d_two_locations", |b| {
        let filter = NC2DPointFilter::new(
            "latitude",
            "longitude",
            vec![(30.0, -120.0), (45.0, -85.0)],
            1.0,
        );
        b.iter(|| filter.apply(&file).unwrap())
    });

    // 2D point filter with tight tolerance: only exact matches (tolerance = 0.1)
    group.bench_function("point2d_tight_tolerance", |b| {
        let filter = NC2DPointFilter::new(
            "latitude",
            "longitude",
            vec![(25.0, -125.0), (50.0, -70.0)],
            0.1,
        );
        b.iter(|| filter.apply(&file).unwrap())
    });

    group.finish();

    file.close().expect("Failed to close fixture file");
}

criterion_group!(benches, bench_filters);
criterion_main!(benches);
