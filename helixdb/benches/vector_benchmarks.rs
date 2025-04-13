use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use helixdb::helix_engine::vector_core::{
    vector::HVector,
    vector_core::{HNSWConfig, VectorCore},
};
use polars::prelude::*;

use std::collections::HashSet;
use heed3::{EnvOpenOptions, Env};
use rand::{
    rngs::StdRng,
    SeedableRng,
    Rng,
    prelude::SliceRandom,
};
use std::time::Duration;
use tempfile::TempDir;

/*
 * things to benchmark:
 * - speed
 * - memory
 * - (putting precision in vector_core_tests)
 */

fn setup_temp_env() -> (Env, TempDir) {
    let temp_dir = TempDir::new().unwrap();
    let path = temp_dir.path().to_str().unwrap();

    let env = unsafe {
        EnvOpenOptions::new()
            .map_size(1024 * 1024 * 1024)
            .max_dbs(10)
            .open(path)
            .unwrap()
    };

    (env, temp_dir)
}

fn generate_random_vectors(count: usize, dim: usize, seed: u64) -> Vec<(String, Vec<f64>)> {
    let mut rng = StdRng::seed_from_u64(seed);
    let mut vectors = Vec::with_capacity(count);

    for i in 0..count {
        let id = format!("vec_{}", i);
        let data: Vec<f64> = (0..dim).map(|_| rng.random_range(-1.0..1.0)).collect();
        vectors.push((id, data));
    }

    vectors
}

fn bench_vector_insertion(c: &mut Criterion) {
    let mut group = c.benchmark_group("vector_insertion");
    group.measurement_time(Duration::from_secs(20));
    group.sample_size(10);

    for &dim in &[128, 1024, 4096, 8192] {
        let vectors_per_iter = 10;

        let id = BenchmarkId::new(format!("insert_{}vecs", vectors_per_iter), dim);
        group.bench_with_input(id, &dim, |b, &dim| {
            eprintln!("Benchmarking insertion of {} vectors with {} dimensions", vectors_per_iter, dim);

            b.iter_with_setup(
                || {
                    let (env, _temp_dir) = setup_temp_env();
                    let mut txn = env.write_txn().unwrap();
                    let hnsw = VectorCore::new(&env, &mut txn, HNSWConfig::new(100)).unwrap();
                    txn.commit().unwrap();

                    let vectors = generate_random_vectors(100, dim, 42);
                    (env, hnsw, vectors)
                },
                |(env, hnsw, vectors)| {
                    let mut txn = env.write_txn().unwrap();
                    for (_id, data) in vectors.iter().take(vectors_per_iter) {
                        hnsw.insert(&mut txn, data, None).unwrap();
                    }
                    txn.commit().unwrap();
                },
            );
        });
    }

    group.finish();
}

fn bench_vector_search(c: &mut Criterion) {
    let mut group = c.benchmark_group("vector_search");
    group.sample_size(20);

    for &dim in &[128, 1024, 4096, 8192] {
        let index_size = 1000;
        let queries_per_iter = 10;

        let id = BenchmarkId::new(format!("search_{}q_{}idx", queries_per_iter, index_size), dim);
        group.bench_with_input(id, &dim, |b, &dim| {
            eprintln!("benchmarking {} queries against index of {} vectors with {} dimensions",
                     queries_per_iter, index_size, dim);

            let (env, _temp_dir) = setup_temp_env();
            let mut txn = env.write_txn().unwrap();
            let hnsw = VectorCore::new(&env, &mut txn, dim, Some(HNSWConfig::optimized(index_size)), None).unwrap();
            let vectors = generate_random_vectors(index_size, dim, 42);
            eprintln!("building index with {} vectors of {} dimensions...", vectors.len(), dim);
            for (_id, data) in &vectors {
                hnsw.insert(&mut txn, data).unwrap();
            }
            txn.commit().unwrap();
            eprintln!("index built successfully");

            let query_vectors = generate_random_vectors(queries_per_iter, dim, 1);
            b.iter(|| {
                let txn = env.read_txn().unwrap();
                for (_, data) in &query_vectors {
                    let results = hnsw.search(&txn, &data, 10).unwrap();
                    black_box(results);
                }
            });
        });
    }

    group.finish();
}

// TODO: bench memory usage

criterion_group!(
    benches,
    bench_vector_search,
    bench_vector_insertion,
);

criterion_main!(benches);
