use criterion::{BatchSize, BenchmarkId, Criterion, criterion_group, criterion_main};
use nc2parquet::postprocess::{
    AggregationOp, Aggregator, ColumnRenamer, DateTimeConverter, FormulaApplier, PostProcessor,
    ProcessingPipeline, TimeUnit, UnitConverter,
};
use polars::prelude::*;
use std::collections::HashMap;

fn make_df(n: usize) -> DataFrame {
    let temp: Vec<f64> = (0..n).map(|i| 273.15 + (i as f64 * 0.1)).collect();
    let pressure: Vec<f64> = (0..n).map(|i| 1013.25 - (i as f64 * 0.01)).collect();
    let time_offset: Vec<f64> = (0..n).map(|i| i as f64).collect();
    df!(
        "temperature" => &temp,
        "pressure" => &pressure,
        "time_offset" => &time_offset,
    )
    .unwrap()
}

fn make_group_df(n: usize) -> DataFrame {
    // Use string group keys with a small cardinality (10 groups).
    // NOTE: group-by with Mean on large DataFrames triggers the Polars streaming engine
    // which has an unimplemented branch in 0.51.0. We use Sum instead (which uses the
    // hash-based path that is fully implemented even in release builds).
    let group: Vec<String> = (0..n).map(|i| format!("group_{}", i % 10)).collect();
    let temp: Vec<f64> = (0..n).map(|i| 273.15 + (i as f64 * 0.1)).collect();
    df!(
        "group" => &group,
        "temperature" => &temp,
    )
    .unwrap()
}

fn bench_postprocess(c: &mut Criterion) {
    let sizes = [1_000usize, 10_000, 100_000];

    // --- UnitConverter: Kelvin -> Celsius ---
    {
        let mut group = c.benchmark_group("postprocess/unit_convert");
        for &size in &sizes {
            group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, &size| {
                let converter = UnitConverter::new(
                    "temperature".to_string(),
                    "kelvin".to_string(),
                    "celsius".to_string(),
                );
                b.iter_batched(
                    || make_df(size),
                    |df| converter.process(df).unwrap(),
                    BatchSize::SmallInput,
                )
            });
        }
        group.finish();
    }

    // --- ColumnRenamer ---
    {
        let mut group = c.benchmark_group("postprocess/column_rename");
        for &size in &sizes {
            group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, &size| {
                let mut mappings = HashMap::new();
                mappings.insert("temperature".to_string(), "temp_celsius".to_string());
                mappings.insert("pressure".to_string(), "pres_hpa".to_string());
                let renamer = ColumnRenamer::new(mappings);
                b.iter_batched(
                    || make_df(size),
                    |df| renamer.process(df).unwrap(),
                    BatchSize::SmallInput,
                )
            });
        }
        group.finish();
    }

    // --- DateTimeConverter ---
    {
        let mut group = c.benchmark_group("postprocess/datetime_convert");
        for &size in &sizes {
            group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, &size| {
                let base: chrono::DateTime<chrono::Utc> = "1900-01-01T00:00:00Z".parse().unwrap();
                let converter =
                    DateTimeConverter::new("time_offset".to_string(), base, TimeUnit::Hours);
                b.iter_batched(
                    || make_df(size),
                    |df| converter.process(df).unwrap(),
                    BatchSize::SmallInput,
                )
            });
        }
        group.finish();
    }

    // --- FormulaApplier: arithmetic expression ---
    {
        let mut group = c.benchmark_group("postprocess/formula_arithmetic");
        for &size in &sizes {
            group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, &size| {
                let applier = FormulaApplier::new(
                    "temp_celsius".to_string(),
                    "temperature - 273.15".to_string(),
                    vec!["temperature".to_string()],
                );
                b.iter_batched(
                    || make_df(size),
                    |df| applier.process(df).unwrap(),
                    BatchSize::SmallInput,
                )
            });
        }
        group.finish();
    }

    // --- Aggregator: group-by with sum ---
    // Uses AggregationOp::Sum via the hash-based path, which is fully implemented in
    // Polars 0.51.0 release builds. AggregationOp::Mean triggers the streaming engine
    // on large DataFrames, which has an unimplemented branch in this version.
    {
        let mut group = c.benchmark_group("postprocess/aggregate_groupby");
        for &size in &sizes {
            group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, &size| {
                let mut aggs = HashMap::new();
                aggs.insert("temperature".to_string(), AggregationOp::Sum);
                let aggregator = Aggregator::new(vec!["group".to_string()], aggs);
                b.iter_batched(
                    || make_group_df(size),
                    |df| aggregator.process(df).unwrap(),
                    BatchSize::SmallInput,
                )
            });
        }
        group.finish();
    }

    // --- ProcessingPipeline: multi-step pipeline (rename + unit convert) ---
    {
        let mut group = c.benchmark_group("postprocess/pipeline_multi_step");
        for &size in &sizes {
            group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, &size| {
                b.iter_batched(
                    || {
                        let df = make_df(size);
                        let mut mappings = HashMap::new();
                        mappings.insert("temperature".to_string(), "temp_k".to_string());
                        let mut pipeline = ProcessingPipeline::new();
                        pipeline.add_processor(Box::new(ColumnRenamer::new(mappings)));
                        pipeline.add_processor(Box::new(UnitConverter::new(
                            "temp_k".to_string(),
                            "kelvin".to_string(),
                            "celsius".to_string(),
                        )));
                        (df, pipeline)
                    },
                    |(df, mut pipeline)| pipeline.execute(df).unwrap(),
                    BatchSize::SmallInput,
                )
            });
        }
        group.finish();
    }

    // --- ProcessingPipeline: batched lazy execution (two independent UnitConverters) ---
    // This benchmark exercises the batching path where two UnitConverter processors
    // operating on disjoint columns are fused into a single `.collect()` call.
    {
        let mut group = c.benchmark_group("postprocess/pipeline_batched_lazy");
        for &size in &sizes {
            group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, &size| {
                b.iter_batched(
                    || {
                        let df = make_df(size);
                        let mut pipeline = ProcessingPipeline::new();
                        // temperature and pressure are independent — both UnitConverters
                        // will be batched into a single collect() by the pipeline executor.
                        pipeline.add_processor(Box::new(UnitConverter::new(
                            "temperature".to_string(),
                            "kelvin".to_string(),
                            "celsius".to_string(),
                        )));
                        pipeline.add_processor(Box::new(UnitConverter::with_conversion_factor(
                            "pressure".to_string(),
                            "hpa".to_string(),
                            "pa".to_string(),
                            100.0,
                        )));
                        (df, pipeline)
                    },
                    |(df, mut pipeline)| pipeline.execute(df).unwrap(),
                    BatchSize::SmallInput,
                )
            });
        }
        group.finish();
    }
}

criterion_group!(benches, bench_postprocess);
criterion_main!(benches);
