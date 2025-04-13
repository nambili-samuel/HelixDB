use criterion::{black_box, criterion_group, criterion_main, Criterion};
use helixdb::helix_engine::{
    graph_core::{
        traversal::TraversalBuilder,
        traversal_steps::{SourceTraversalSteps, TraversalBuilderMethods, TraversalSteps},
    },
    storage_core::{storage_core::HelixGraphStorage, storage_methods::StorageMethods},
};
use helixdb::props;
use helixdb::protocol::traversal_value::TraversalValue;
use rand::Rng;
use std::{sync::Arc, time::Duration};
use tempfile::TempDir;

fn create_test_graph(
    size: usize,
    edges_per_node: usize,
) -> (Arc<HelixGraphStorage>, TempDir, Vec<String>) {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().to_str().unwrap();
    let storage = HelixGraphStorage::new(db_path, None).unwrap();
    let mut node_ids = Vec::with_capacity(size);

    let mut txn = storage.graph_env.write_txn().unwrap();
    for _ in 0..size {
        let node = storage
            .create_node(&mut txn, "person", props!(), None)
            .unwrap();
        node_ids.push(node.id);
    }

    let mut rng = rand::thread_rng();
    for from_id in &node_ids {
        for _ in 0..edges_per_node {
            let to_index = rng.gen_range(0..size);
            let to_id = &node_ids[to_index];

            if from_id != to_id {
                storage
                    .create_edge(&mut txn, "knows", from_id, to_id, props!())
                    .unwrap();
            }
        }
    }
    txn.commit().unwrap();

    (Arc::new(storage), temp_dir, node_ids)
}

fn bench_graph_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("graph_operations");
    group.measurement_time(Duration::from_secs(10));
    group.sample_size(10);

    for size in [100, 1000, 10000].iter() {
        let edges_per_node = 5;

        // Benchmark node creation
        group.bench_function(format!("create_nodes_{}", size), |b| {
            b.iter(|| {
                let (storage, _temp_dir, _) = create_test_graph(*size, 0);
                black_box(storage)
            });
        });

        // Benchmark edge creation
        group.bench_function(format!("create_edges_{}", size), |b| {
            b.iter(|| {
                let (storage, _temp_dir, _) = create_test_graph(*size, edges_per_node);
                black_box(storage)
            });
        });

        let (storage, _temp_dir, node_ids) = create_test_graph(*size, edges_per_node);
        let txn = storage.graph_env.write_txn().unwrap();
        let start_node = storage.get_node(&txn, &node_ids[0]).unwrap();
        txn.commit().unwrap();

        // Benchmark simple traversals
        group.bench_function(format!("out_traversal_{}", size), |b| {
            b.iter(|| {
                let txn = storage.graph_env.read_txn().unwrap();
                let mut traversal = TraversalBuilder::new(
                    Arc::clone(&storage),
                    TraversalValue::NodeArray(vec![start_node.clone()]),
                );
                traversal.out(&txn, "knows");
                black_box(traversal)
            });
        });

        // Benchmark chained traversals
        group.bench_function(format!("chained_traversal_{}", size), |b| {
            b.iter(|| {
                let txn = storage.graph_env.read_txn().unwrap();
                let mut traversal = TraversalBuilder::new(
                    Arc::clone(&storage),
                    TraversalValue::NodeArray(vec![start_node.clone()]),
                );
                traversal
                    .out(&txn, "knows")
                    .out(&txn, "knows")
                    .out(&txn, "knows");
                black_box(traversal)
            });
        });

        // Benchmark full graph scan
        group.bench_function(format!("full_graph_scan_{}", size), |b| {
            b.iter(|| {
                let txn = storage.graph_env.read_txn().unwrap();
                let mut traversal =
                    TraversalBuilder::new(Arc::clone(&storage), TraversalValue::Empty);
                traversal.v(&txn);
                black_box(traversal)
            });
        });
    }

    group.finish();
}

fn bench_complex_queries(c: &mut Criterion) {
    let mut group = c.benchmark_group("complex_queries");
    group.measurement_time(Duration::from_secs(10));
    group.sample_size(10);

    let size = 100000;
    let edges_per_node = 10;
    let (storage, _temp_dir, node_ids) = create_test_graph(size, edges_per_node);

    let txn = storage.graph_env.read_txn().unwrap();
    let start_node = storage.get_node(&txn, &node_ids[0]).unwrap();
    txn.commit().unwrap();
    // Benchmark two hop query
    group.bench_function("two_hops_100000", |b| {
        b.iter(|| {
            let txn = storage.graph_env.read_txn().unwrap();
            let mut traversal = TraversalBuilder::new(
                Arc::clone(&storage),
                TraversalValue::NodeArray(vec![start_node.clone()]),
            );
            traversal.out(&txn, "knows").out(&txn, "knows");
            black_box(traversal)
        });
    });

    // Benchmark circular traversal
    group.bench_function("circular_traversal_100000", |b| {
        b.iter(|| {
            let txn = storage.graph_env.read_txn().unwrap();

            let mut traversal = TraversalBuilder::new(
                Arc::clone(&storage),
                TraversalValue::NodeArray(vec![start_node.clone()]),
            );
            traversal
                .out(&txn, "knows")
                .out(&txn, "knows")
                .out(&txn, "knows")
                .in_(&txn, "knows");
            black_box(traversal)
        });
    });

    group.finish();
}

criterion_group!(benches, bench_graph_operations, bench_complex_queries);
criterion_main!(benches);
