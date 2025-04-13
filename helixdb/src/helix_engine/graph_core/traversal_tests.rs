use std::sync::Arc;

use crate::helix_engine::{
    graph_core::traversal_steps::{
        SourceTraversalSteps, TraversalBuilderMethods, TraversalSearchMethods,
    },
    storage_core::{storage_core::HelixGraphStorage, storage_methods::StorageMethods},
    types::GraphError,
};
use crate::props;
use crate::protocol::{
    filterable::Filterable,
    items::{Edge, Node},
    traversal_value::TraversalValue,
    value::Value,
};
use tempfile::TempDir;

use super::{traversal::TraversalBuilder, traversal_steps::{TraversalMethods, TraversalSteps}};

fn setup_test_db() -> (Arc<HelixGraphStorage>, TempDir) {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().to_str().unwrap();
    let storage = HelixGraphStorage::new(db_path, super::config::Config::default()).unwrap();
    (Arc::new(storage), temp_dir)
}

#[test]
fn test_v() {
    let (storage, _temp_dir) = setup_test_db();
    let mut txn = storage.graph_env.write_txn().unwrap();

    let person1 = storage
        .create_node(&mut txn, "person", props!(), None)
        .unwrap();
    let person2 = storage
        .create_node(&mut txn, "person", props!(), None)
        .unwrap();
    let thing = storage
        .create_node(&mut txn, "thing", props!(), None)
        .unwrap();
    txn.commit().unwrap();

    let txn = storage.graph_env.read_txn().unwrap();
    let mut traversal = TraversalBuilder::new(Arc::clone(&storage), TraversalValue::Empty);
    traversal.v(&txn);
    // Check that the node array contains all nodes
    match &traversal.current_step {
        TraversalValue::NodeArray(nodes) => {
            assert_eq!(nodes.len(), 3);

            let node_ids: Vec<String> = nodes.iter().map(|n| n.id.clone()).collect();
            let node_labels: Vec<String> = nodes.iter().map(|n| n.label.clone()).collect();

            assert!(node_ids.contains(&person1.id));
            assert!(node_ids.contains(&person2.id));
            assert!(node_ids.contains(&thing.id));

            assert_eq!(node_labels.iter().filter(|&l| l == "person").count(), 2);
            assert_eq!(node_labels.iter().filter(|&l| l == "thing").count(), 1);
        }
        _ => panic!("Expected NodeArray value {:?}", &traversal.current_step),
    }
}

#[test]
fn test_e() {
    let (storage, _temp_dir) = setup_test_db();
    let mut txn = storage.graph_env.write_txn().unwrap();

    // Graph Structure:
    // (person1)-[knows]->(person2)
    //         \-[likes]->(person3)
    // (person2)-[follows]->(person3)

    let person1 = storage
        .create_node(&mut txn, "person", props!(), None)
        .unwrap();
    let person2 = storage
        .create_node(&mut txn, "person", props!(), None)
        .unwrap();
    let person3 = storage
        .create_node(&mut txn, "person", props!(), None)
        .unwrap();

    let knows_edge = storage
        .create_edge(&mut txn, "knows", &person1.id, &person2.id, props!())
        .unwrap();
    let likes_edge = storage
        .create_edge(&mut txn, "likes", &person1.id, &person3.id, props!())
        .unwrap();
    let follows_edge = storage
        .create_edge(&mut txn, "follows", &person2.id, &person3.id, props!())
        .unwrap();

    txn.commit().unwrap();

    let txn = storage.graph_env.read_txn().unwrap();
    let mut traversal = TraversalBuilder::new(Arc::clone(&storage), TraversalValue::Empty);
    traversal.e(&txn);

    // Check that the edge array contains the three edges
    match &traversal.current_step {
        TraversalValue::EdgeArray(edges) => {
            assert_eq!(edges.len(), 3);

            let edge_ids: Vec<String> = edges.iter().map(|e| e.id.clone()).collect();
            let edge_labels: Vec<String> = edges.iter().map(|e| e.label.clone()).collect();

            assert!(edge_ids.contains(&knows_edge.id));
            assert!(edge_ids.contains(&likes_edge.id));
            assert!(edge_ids.contains(&follows_edge.id));

            assert!(edge_labels.contains(&"knows".to_string()));
            assert!(edge_labels.contains(&"likes".to_string()));
            assert!(edge_labels.contains(&"follows".to_string()));

            for edge in edges {
                match edge.label.as_str() {
                    "knows" => {
                        assert_eq!(edge.from_node, person1.id);
                        assert_eq!(edge.to_node, person2.id);
                    }
                    "likes" => {
                        assert_eq!(edge.from_node, person1.id);
                        assert_eq!(edge.to_node, person3.id);
                    }
                    "follows" => {
                        assert_eq!(edge.from_node, person2.id);
                        assert_eq!(edge.to_node, person3.id);
                    }
                    _ => panic!("Unexpected edge label"),
                }
            }
        }
        _ => panic!("Expected EdgeArray value"),
    }
}

#[test]
fn test_v_empty_graph() {
    let (storage, _temp_dir) = setup_test_db();

    let txn = storage.graph_env.read_txn().unwrap();
    let mut traversal = TraversalBuilder::new(Arc::clone(&storage), TraversalValue::Empty);
    traversal.v(&txn);

    // Check that the node array is empty
    match &traversal.current_step {
        TraversalValue::NodeArray(nodes) => {
            assert_eq!(nodes.len(), 0);
        }
        _ => panic!("Expected NodeArray value"),
    }
}

#[test]
fn test_e_empty_graph() {
    let (storage, _temp_dir) = setup_test_db();

    let txn = storage.graph_env.read_txn().unwrap();
    let mut traversal = TraversalBuilder::new(Arc::clone(&storage), TraversalValue::Empty);
    traversal.e(&txn);

    // Check that the edge array is empty
    match &traversal.current_step {
        TraversalValue::EdgeArray(edges) => {
            assert_eq!(edges.len(), 0);
        }
        _ => panic!("Expected EdgeArray value"),
    }
    txn.commit().unwrap();
}

#[test]
fn test_v_nodes_without_edges() {
    let (storage, _temp_dir) = setup_test_db();
    let mut txn = storage.graph_env.write_txn().unwrap();

    let person1 = storage
        .create_node(&mut txn, "person", props!(), None)
        .unwrap();
    let person2 = storage
        .create_node(&mut txn, "person", props!(), None)
        .unwrap();

    txn.commit().unwrap();
    let txn = storage.graph_env.read_txn().unwrap();
    let mut traversal = TraversalBuilder::new(Arc::clone(&storage), TraversalValue::Empty);
    traversal.v(&txn);

    // Check that the node array contains the two nodes
    match &traversal.current_step {
        TraversalValue::NodeArray(nodes) => {
            assert_eq!(nodes.len(), 2);
            let node_ids: Vec<String> = nodes.iter().map(|n| n.id.clone()).collect();
            assert!(node_ids.contains(&person1.id));
            assert!(node_ids.contains(&person2.id));
        }
        _ => panic!("Expected NodeArray value"),
    }
}

#[test]
fn test_add_v() {
    let (storage, _temp_dir) = setup_test_db();

    let mut txn = storage.graph_env.write_txn().unwrap();
    let mut traversal = TraversalBuilder::new(Arc::clone(&storage), TraversalValue::Empty);

    traversal.add_v(&mut txn, "person", props! {}, None);

    match &traversal.current_step {
        TraversalValue::NodeArray(node) => {
            assert_eq!(node.first().unwrap().label, "person");
        }
        _ => panic!("Expected SingleNode value"),
    }

    // Now txn is free of borrows
    // (If you dropped txn above, you would need to reinitialize it; so in practice, this pattern
    //  is only used if the borrow is the only problem.)

    // If we haven’t dropped txn, ensure no borrows exist before commit
    txn.commit().unwrap();
}

#[test]
fn test_add_e() {
    let (storage, _temp_dir) = setup_test_db();
    let mut txn = storage.graph_env.write_txn().unwrap();

    let node1 = storage
        .create_node(&mut txn, "person", props!(), None)
        .unwrap();
    let node2 = storage
        .create_node(&mut txn, "person", props!(), None)
        .unwrap();

    txn.commit().unwrap();
    let mut txn = storage.graph_env.write_txn().unwrap();
    let mut traversal = TraversalBuilder::new(Arc::clone(&storage), TraversalValue::Empty);
    traversal.add_e(&mut txn, "knows", &node1.id, &node2.id, props!());
    let result = traversal.result(txn).unwrap();
    // Check that the current step contains a single edge
    match &result {
        TraversalValue::EdgeArray(edges) => {
            assert_eq!(edges.len(), 1);
            assert_eq!(edges[0].label, "knows");
            assert_eq!(edges[0].from_node, node1.id);
            assert_eq!(edges[0].to_node, node2.id);
        }
        _ => panic!("Expected SingleEdge value"),
    }
}

#[test]
fn test_out() {
    let (storage, _temp_dir) = setup_test_db();
    let mut txn = storage.graph_env.write_txn().unwrap();

    // Create graph: (person1)-[knows]->(person2)-[knows]->(person3)
    let person1 = storage
        .create_node(&mut txn, "person", props!(), None)
        .unwrap();
    let person2 = storage
        .create_node(&mut txn, "person", props!(), None)
        .unwrap();
    let person3 = storage
        .create_node(&mut txn, "person", props!(), None)
        .unwrap();

    storage
        .create_edge(&mut txn, "knows", &person1.id, &person2.id, props!())
        .unwrap();
    storage
        .create_edge(&mut txn, "knows", &person2.id, &person3.id, props!())
        .unwrap();

    txn.commit().unwrap();
    let txn = storage.graph_env.read_txn().unwrap();
    let mut traversal =
        TraversalBuilder::new(Arc::clone(&storage), TraversalValue::from(person1.clone()));
    // Traverse from person1 to person2
    traversal.out(&txn, "knows");

    // Check that current step is at person2
    match &traversal.current_step {
        TraversalValue::NodeArray(nodes) => {
            assert_eq!(nodes.len(), 1);
            assert_eq!(nodes[0].id, person2.id);
        }
        _ => panic!("Expected NodeArray value"),
    }
}

#[test]
fn test_out_e() {
    let (storage, _temp_dir) = setup_test_db();
    let mut txn = storage.graph_env.write_txn().unwrap();

    // Create graph: (person1)-[knows]->(person2)
    let person1 = storage
        .create_node(&mut txn, "person", props!(), None)
        .unwrap();
    let person2 = storage
        .create_node(&mut txn, "person", props!(), None)
        .unwrap();

    let edge = storage
        .create_edge(&mut txn, "knows", &person1.id, &person2.id, props!())
        .unwrap();

    txn.commit().unwrap();
    let txn = storage.graph_env.read_txn().unwrap();
    let mut traversal =
        TraversalBuilder::new(Arc::clone(&storage), TraversalValue::from(person1.clone()));
    // Traverse from person1 to person2
    traversal.out_e(&txn, "knows");

    // Check that current step is at the edge between person1 and person2
    match &traversal.current_step {
        TraversalValue::EdgeArray(edges) => {
            assert_eq!(edges.len(), 1);
            assert_eq!(edges[0].id, edge.id);
            assert_eq!(edges[0].label, "knows");
        }
        _ => panic!("Expected EdgeArray value"),
    }
}

#[test]
fn test_in() {
    let (storage, _temp_dir) = setup_test_db();
    let mut txn = storage.graph_env.write_txn().unwrap();

    // Create graph: (person1)-[knows]->(person2)
    let person1 = storage
        .create_node(&mut txn, "person", props!(), None)
        .unwrap();
    let person2 = storage
        .create_node(&mut txn, "person", props!(), None)
        .unwrap();

    storage
        .create_edge(&mut txn, "knows", &person1.id, &person2.id, props!())
        .unwrap();
    txn.commit().unwrap();

    let txn = storage.graph_env.read_txn().unwrap();
    let mut traversal =
        TraversalBuilder::new(Arc::clone(&storage), TraversalValue::from(person2.clone()));
    // Traverse from person2 to person1
    traversal.in_(&txn, "knows");

    // Check that current step is at person1
    match &traversal.current_step {
        TraversalValue::NodeArray(nodes) => {
            assert_eq!(nodes.len(), 1);
            assert_eq!(nodes[0].id, person1.id);
        }
        _ => panic!("Expected NodeArray value"),
    }
}

#[test]
fn test_in_e() {
    let (storage, _temp_dir) = setup_test_db();
    let mut txn = storage.graph_env.write_txn().unwrap();

    // Create test graph: (person1)-[knows]->(person2)
    let person1 = storage
        .create_node(&mut txn, "person", props!(), None)
        .unwrap();
    let person2 = storage
        .create_node(&mut txn, "person", props!(), None)
        .unwrap();

    let edge = storage
        .create_edge(&mut txn, "knows", &person1.id, &person2.id, props!())
        .unwrap();

    txn.commit().unwrap();
    let txn = storage.graph_env.read_txn().unwrap();
    let mut traversal =
        TraversalBuilder::new(Arc::clone(&storage), TraversalValue::from(person2.clone()));
    // Traverse from person2 to person1
    traversal.in_e(&txn, "knows");

    // Check that current step is at the edge between person1 and person2
    match &traversal.current_step {
        TraversalValue::EdgeArray(edges) => {
            assert_eq!(edges.len(), 1);
            assert_eq!(edges[0].id, edge.id);
            assert_eq!(edges[0].label, "knows");
        }
        _ => panic!("Expected EdgeArray value"),
    }
}

#[test]
fn test_traversal_validation() {
    let (storage, _temp_dir) = setup_test_db();
    let mut txn = storage.graph_env.write_txn().unwrap();

    let node1 = storage
        .create_node(&mut txn, "person", props!(), None)
        .unwrap();
    let node2 = storage
        .create_node(&mut txn, "person", props!(), None)
        .unwrap();
    let edge = storage
        .create_edge(&mut txn, "knows", &node1.id, &node2.id, props!())
        .unwrap();
    txn.commit().unwrap();
    let txn = storage.graph_env.read_txn().unwrap();
    let mut traversal = TraversalBuilder::new(Arc::clone(&storage), TraversalValue::Empty);
    traversal.current_step = TraversalValue::from(edge);

    assert!(traversal.check_is_valid_node_traversal("test").is_err());

    traversal.current_step = TraversalValue::from(node1);
    assert!(traversal.check_is_valid_edge_traversal("test").is_err());
}

#[test]
fn test_complex_traversal() {
    let (storage, _temp_dir) = setup_test_db();
    let mut txn = storage.graph_env.write_txn().unwrap();

    // Graph structure:
    // (person1)-[knows]->(person2)-[likes]->(person3)
    //     ^                                     |
    //     |                                     |
    //     +-------<------[follows]------<-------+

    let person1 = storage
        .create_node(&mut txn, "person", props!(), None)
        .unwrap();
    let person2 = storage
        .create_node(&mut txn, "person", props!(), None)
        .unwrap();
    let person3 = storage
        .create_node(&mut txn, "person", props!(), None)
        .unwrap();

    storage
        .create_edge(&mut txn, "knows", &person1.id, &person2.id, props!())
        .unwrap();
    storage
        .create_edge(&mut txn, "likes", &person2.id, &person3.id, props!())
        .unwrap();
    storage
        .create_edge(&mut txn, "follows", &person3.id, &person1.id, props!())
        .unwrap();

    txn.commit().unwrap();

    let txn = storage.graph_env.read_txn().unwrap();
    let mut traversal =
        TraversalBuilder::new(Arc::clone(&storage), TraversalValue::from(person1.clone()));
    // Traverse from person1 to person2
    traversal.out(&txn, "knows");

    // Check that current step is at person2
    match &traversal.current_step {
        TraversalValue::NodeArray(nodes) => {
            assert_eq!(nodes.len(), 1);
            assert_eq!(nodes[0].id, person2.id);
        }
        _ => panic!("Expected NodeArray value"),
    }

    // Traverse from person2 to person3
    traversal.out(&txn, "likes");

    // Check that current step is at person3
    match &traversal.current_step {
        TraversalValue::NodeArray(nodes) => {
            assert_eq!(nodes.len(), 1);
            assert_eq!(nodes[0].id, person3.id);
        }
        _ => panic!("Expected NodeArray value"),
    }

    // Traverse from person3 to person1
    traversal.out(&txn, "follows");

    // Check that current step is at person1
    match &traversal.current_step {
        TraversalValue::NodeArray(nodes) => {
            assert_eq!(nodes.len(), 1);
            assert_eq!(nodes[0].id, person1.id);
        }
        _ => panic!("Expected NodeArray value"),
    }
}

#[test]
fn test_count_single_node() {
    let (storage, _temp_dir) = setup_test_db();
    let mut txn = storage.graph_env.write_txn().unwrap();
    let person = storage
        .create_node(&mut txn, "person", props!(), None)
        .unwrap();
    txn.commit().unwrap();
    let txn = storage.graph_env.read_txn().unwrap();
    let mut traversal = TraversalBuilder::new(Arc::clone(&storage), TraversalValue::from(person));
    if let TraversalValue::Count(count) = &traversal.count().current_step {
        assert_eq!(count.value(), 1);
    } else {
        panic!("Expected Count value");
    }
}

#[test]
fn test_count_node_array() {
    let (storage, _temp_dir) = setup_test_db();
    let mut txn = storage.graph_env.write_txn().unwrap();
    let _ = storage
        .create_node(&mut txn, "person", props!(), None)
        .unwrap();
    let _ = storage
        .create_node(&mut txn, "person", props!(), None)
        .unwrap();
    let _ = storage
        .create_node(&mut txn, "person", props!(), None)
        .unwrap();

    txn.commit().unwrap();
    let txn = storage.graph_env.read_txn().unwrap();
    let mut traversal = TraversalBuilder::new(Arc::clone(&storage), TraversalValue::Empty);
    traversal.v(&txn); // Get all nodes
    if let TraversalValue::Count(count) = &traversal.count().current_step {
        assert_eq!(count.value(), 3);
    } else {
        panic!("Expected Count value");
    }
}

#[test]
fn test_count_mixed_steps() {
    let (storage, _temp_dir) = setup_test_db();
    let mut txn = storage.graph_env.write_txn().unwrap();

    // Create a graph with multiple paths
    let person1 = storage
        .create_node(&mut txn, "person", props!(), None)
        .unwrap();
    let person2 = storage
        .create_node(&mut txn, "person", props!(), None)
        .unwrap();
    let person3 = storage
        .create_node(&mut txn, "person", props!(), None)
        .unwrap();

    storage
        .create_edge(&mut txn, "knows", &person1.id, &person2.id, props!())
        .unwrap();
    storage
        .create_edge(&mut txn, "knows", &person1.id, &person3.id, props!())
        .unwrap();
    txn.commit().unwrap();
    println!(
        "person1: {:?},\nperson2: {:?},\nperson3: {:?}",
        person1, person2, person3
    );

    let txn = storage.graph_env.read_txn().unwrap();
    let mut traversal =
        TraversalBuilder::new(Arc::clone(&storage), TraversalValue::from(person1.clone()));
    traversal.out(&txn, "knows"); // Should have 2 nodes (person2 and person3)


    if let TraversalValue::Count(count) = &traversal.count().current_step {
        assert_eq!(count.value(), 2);
    } else {
        panic!("Expected Count value");
    }
}

#[test]
fn test_range_subset() {
    let (storage, _temp_dir) = setup_test_db();
    let mut txn = storage.graph_env.write_txn().unwrap();

    // Create multiple nodes
    let _: Vec<Node> = (0..5)
        .map(|_| {
            storage
                .create_node(&mut txn, "person", props!(), None)
                .unwrap()
        })
        .collect();

    txn.commit().unwrap();
    let txn = storage.graph_env.read_txn().unwrap();
    let mut traversal = TraversalBuilder::new(Arc::clone(&storage), TraversalValue::Empty);
    traversal.v(&txn); // Get all nodes
    traversal.range(1, 3); // Take nodes at index 1 and 2

    if let TraversalValue::Count(count) = &traversal.count().current_step {
        assert_eq!(count.value(), 2);
    } else {
        panic!("Expected Count value");
    }
}

#[test]
fn test_range_chaining() {
    let (storage, _temp_dir) = setup_test_db();
    let mut txn = storage.graph_env.write_txn().unwrap();

    // Create graph: (p1)-[knows]->(p2)-[knows]->(p3)-[knows]->(p4)-[knows]->(p5)
    let nodes: Vec<Node> = (0..5)
        .map(|i| {
            storage
                .create_node(&mut txn, "person", props! { "name" => i }, None)
                .unwrap()
        })
        .collect();

    // Create edges connecting nodes sequentially
    for i in 0..4 {
        storage
            .create_edge(&mut txn, "knows", &nodes[i].id, &nodes[i + 1].id, props!())
            .unwrap();
    }

    storage
        .create_edge(&mut txn, "knows", &nodes[4].id, &nodes[0].id, props!())
        .unwrap();

    txn.commit().unwrap();
    let txn = storage.graph_env.read_txn().unwrap();
    let mut traversal = TraversalBuilder::new(Arc::clone(&storage), TraversalValue::Empty);
    traversal.v(&txn); // Get all nodes
    traversal.range(0, 3); // Take first 3 nodes
    traversal.out(&txn, "knows"); // Get their outgoing nodes

    if let TraversalValue::Count(count) = &traversal.count().current_step {
        assert_eq!(count.value(), 3);
    } else {
        panic!("Expected Count value");
    }
}

#[test]
fn test_range_empty() {
    let (storage, _temp_dir) = setup_test_db();
    let mut txn = storage.graph_env.write_txn().unwrap();
    txn.commit().unwrap();
    let txn = storage.graph_env.read_txn().unwrap();
    let mut traversal = TraversalBuilder::new(Arc::clone(&storage), TraversalValue::Empty);
    traversal.v(&txn);
    traversal.range(0, 0);
    if let TraversalValue::Count(count) = &traversal.count().current_step {
        assert_eq!(count.value(), 0);
    } else {
        panic!("Expected Count value");
    }
}

#[test]
fn test_count_empty() {
    let (storage, _temp_dir) = setup_test_db();
    let mut txn = storage.graph_env.write_txn().unwrap();
    txn.commit().unwrap();
    let txn = storage.graph_env.read_txn().unwrap();
    let mut traversal = TraversalBuilder::new(Arc::clone(&storage), TraversalValue::Empty);
    if let TraversalValue::Count(count) = &traversal.count().current_step {
        assert_eq!(count.value(), 0);
    } else {
        panic!("Expected Count value");
    }
}

#[test]
fn test_v_from_id() {
    let (storage, _temp_dir) = setup_test_db();
    let mut txn = storage.graph_env.write_txn().unwrap();

    // Create a test node
    let person = storage
        .create_node(&mut txn, "person", props!(), None)
        .unwrap();
    let node_id = person.id.clone();

    txn.commit().unwrap();
    let txn = storage.graph_env.read_txn().unwrap();
    let mut traversal = TraversalBuilder::new(Arc::clone(&storage), TraversalValue::Empty);
    traversal.v_from_id(&txn, &node_id);
    // Check that the current step contains the correct single node
    match &traversal.current_step {
        TraversalValue::NodeArray(nodes) => {
            assert_eq!(nodes.len(), 1);
            assert_eq!(nodes[0].id, node_id);
            assert_eq!(nodes[0].label, "person");
        }
        _ => panic!("Expected SingleNode value"),
    }
}

#[test]
fn test_v_from_id_with_traversal() {
    let (storage, _temp_dir) = setup_test_db();
    let mut txn = storage.graph_env.write_txn().unwrap();

    // Create test graph: (person1)-[knows]->(person2)
    let person1 = storage
        .create_node(&mut txn, "person", props!(), None)
        .unwrap();
    let person2 = storage
        .create_node(&mut txn, "person", props!(), None)
        .unwrap();
    storage
        .create_edge(&mut txn, "knows", &person1.id, &person2.id, props!())
        .unwrap();

    txn.commit().unwrap();
    let txn = storage.graph_env.read_txn().unwrap();
    let mut traversal = TraversalBuilder::new(Arc::clone(&storage), TraversalValue::Empty);
    traversal.v_from_id(&txn, &person1.id).out(&txn, "knows");

    // Check that traversal reaches person2
    match &traversal.current_step {
        TraversalValue::NodeArray(nodes) => {
            assert_eq!(nodes.len(), 1);
            assert_eq!(nodes[0].id, person2.id);
        }
        _ => panic!("Expected NodeArray value"),
    }
}

#[test]
fn test_e_from_id() {
    let (storage, _temp_dir) = setup_test_db();
    let mut txn = storage.graph_env.write_txn().unwrap();

    // Create test graph and edge
    let person1 = storage
        .create_node(&mut txn, "person", props!(), None)
        .unwrap();
    let person2 = storage
        .create_node(&mut txn, "person", props!(), None)
        .unwrap();
    let edge = storage
        .create_edge(&mut txn, "knows", &person1.id, &person2.id, props!())
        .unwrap();
    let edge_id = edge.id.clone();

    txn.commit().unwrap();
    let txn = storage.graph_env.read_txn().unwrap();
    let mut traversal = TraversalBuilder::new(Arc::clone(&storage), TraversalValue::Empty);
    traversal.e_from_id(&txn, &edge_id);

    // Check that the current step contains the correct single edge
    match &traversal.current_step {
        TraversalValue::EdgeArray(edges) => {
            assert_eq!(edges.len(), 1);
            assert_eq!(edges[0].id, edge_id);
            assert_eq!(edges[0].label, "knows");
            assert_eq!(edges[0].from_node, person1.id);
            assert_eq!(edges[0].to_node, person2.id);
        }
        _ => panic!("Expected SingleEdge value"),
    }
}

#[test]
fn test_v_from_id_nonexistent() {
    let (storage, _temp_dir) = setup_test_db();
    let txn = storage.graph_env.read_txn().unwrap();
    let mut traversal = TraversalBuilder::new(Arc::clone(&storage), TraversalValue::Empty);
    traversal.v_from_id(&txn, "nonexistent_id");
    let result = traversal.finish().unwrap();
    assert!(result.is_empty());
}

#[test]
fn test_e_from_id_nonexistent() {
    let (storage, _temp_dir) = setup_test_db();
    let txn = storage.graph_env.read_txn().unwrap();
    let mut traversal = TraversalBuilder::new(Arc::clone(&storage), TraversalValue::Empty);
    traversal.e_from_id(&txn, "nonexistent_id");
    let result = traversal.finish().unwrap();
    assert!(result.is_empty());
}

#[test]
fn test_v_from_id_chain_operations() {
    let (storage, _temp_dir) = setup_test_db();
    let mut txn = storage.graph_env.write_txn().unwrap();

    // Create test graph: (person1)-[knows]->(person2)-[likes]->(person3)
    let person1 = storage
        .create_node(&mut txn, "person", props!(), None)
        .unwrap();
    let person2 = storage
        .create_node(&mut txn, "person", props!(), None)
        .unwrap();
    let person3 = storage
        .create_node(&mut txn, "person", props!(), None)
        .unwrap();

    storage
        .create_edge(&mut txn, "knows", &person1.id, &person2.id, props!())
        .unwrap();
    storage
        .create_edge(&mut txn, "likes", &person2.id, &person3.id, props!())
        .unwrap();

    txn.commit().unwrap();
    let txn = storage.graph_env.read_txn().unwrap();
    let mut traversal = TraversalBuilder::new(Arc::clone(&storage), TraversalValue::Empty);
    traversal
        .v_from_id(&txn, &person1.id)
        .out(&txn, "knows")
        .out(&txn, "likes");

    // Check that the chain of traversals reaches person3
    match &traversal.current_step {
        TraversalValue::NodeArray(nodes) => {
            assert_eq!(nodes.len(), 1);
            assert_eq!(nodes[0].id, person3.id);
        }
        _ => panic!("Expected NodeArray value"),
    }
}

#[test]
fn test_e_from_id_chain_operations() {
    let (storage, _temp_dir) = setup_test_db();
    let mut txn = storage.graph_env.write_txn().unwrap();

    // Create test graph and edges
    let person1 = storage
        .create_node(&mut txn, "person", props!(), None)
        .unwrap();
    let person2 = storage
        .create_node(&mut txn, "person", props!(), None)
        .unwrap();
    let edge = storage
        .create_edge(&mut txn, "knows", &person1.id, &person2.id, props!())
        .unwrap();

    txn.commit().unwrap();
    let txn = storage.graph_env.read_txn().unwrap();
    let mut traversal = TraversalBuilder::new(Arc::clone(&storage), TraversalValue::Empty);
    let count_before = traversal.e_from_id(&txn, &edge.id).count();

    if let TraversalValue::Count(count) = &count_before.current_step {
        assert_eq!(count.value(), 1);
    } else {
        panic!("Expected Count value");
    }
}

#[test]
fn test_filter_nodes() {
    let (storage, _temp_dir) = setup_test_db();
    let mut txn = storage.graph_env.write_txn().unwrap();

    // Create nodes with different properties
    let _ = storage
        .create_node(&mut txn, "person", props! { "age" => 25 }, None)
        .unwrap();
    let _ = storage
        .create_node(&mut txn, "person", props! { "age" => 30 }, None)
        .unwrap();
    let person3 = storage
        .create_node(&mut txn, "person", props! { "age" => 35 }, None)
        .unwrap();

    txn.commit().unwrap();
    let txn = storage.graph_env.read_txn().unwrap();
    let mut traversal = TraversalBuilder::new(Arc::clone(&storage), TraversalValue::Empty);
    traversal.v(&txn);

    // Filter nodes with age > 30
    traversal.filter_nodes(&txn, |val| {
        if let Some(value) = val.check_property("age") {
            match value {
                Value::Float(age) => Ok(*age > 30.0),
                Value::Integer(age) => Ok(*age > 30),

                _ => Err(GraphError::TraversalError("Invalid type".to_string())),
            }
        } else {
            Err(GraphError::TraversalError("No age property".to_string()))
        }
    });
    match &traversal.current_step {
        TraversalValue::NodeArray(nodes) => {
            assert_eq!(nodes.len(), 1);
            assert_eq!(nodes[0].id, person3.id);
        }
        _ => panic!("Expected Node value"),
    }

    if let TraversalValue::Count(count) = &traversal.count().current_step {
        assert_eq!(count.value(), 1);
    } else {
        panic!("Expected Count value");
    }
}

#[test]
fn test_filter_macro_single_argument() {
    let (storage, _temp_dir) = setup_test_db();
    let mut txn = storage.graph_env.write_txn().unwrap();

    let _ = storage
        .create_node(&mut txn, "person", props! { "name" => "Alice" }, None)
        .unwrap();
    let _ = storage
        .create_node(&mut txn, "person", props! { "name" => "Bob" }, None)
        .unwrap();

    fn has_name(val: &Node) -> Result<bool, GraphError> {
        return Ok(val.check_property("name").is_some());
    }

    txn.commit().unwrap();
    let txn = storage.graph_env.read_txn().unwrap();
    let mut traversal = TraversalBuilder::new(Arc::clone(&storage), TraversalValue::Empty);
    traversal.v(&txn).filter_nodes(&txn, has_name);

    if let TraversalValue::Count(count) = &traversal.count().current_step {
        assert_eq!(count.value(), 2);
    } else {
        panic!("Expected Count value");
    }
}

#[test]
fn test_filter_macro_multiple_arguments() {
    let (storage, _temp_dir) = setup_test_db();
    let mut txn = storage.graph_env.write_txn().unwrap();

    let _ = storage
        .create_node(&mut txn, "person", props! { "age" => 25 }, None)
        .unwrap();
    let person2 = storage
        .create_node(&mut txn, "person", props! { "age" => 30 }, None)
        .unwrap();

    fn age_greater_than(val: &Node, min_age: i32) -> Result<bool, GraphError> {
        if let Some(value) = val.check_property("age") {
            match value {
                Value::Float(age) => Ok(*age > min_age as f64),
                Value::Integer(age) => Ok(*age > min_age),
                _ => Err(GraphError::TraversalError("Invalid type".to_string())),
            }
        } else {
            Err(GraphError::TraversalError("Invalid node".to_string()))
        }
    }

    txn.commit().unwrap();
    let txn = storage.graph_env.read_txn().unwrap();
    let mut traversal = TraversalBuilder::new(Arc::clone(&storage), TraversalValue::Empty);
    traversal.v(&txn);
    traversal.filter_nodes(&txn, |node| age_greater_than(node, 27));

    match &traversal.current_step {
        TraversalValue::NodeArray(nodes) => {
            assert_eq!(nodes.len(), 1);
            assert_eq!(nodes[0].id, person2.id);
        }
        _ => panic!("Expected Node value"),
    }

    if let TraversalValue::Count(count) = &traversal.count().current_step {
        assert_eq!(count.value(), 1);
    } else {
        panic!("Expected Count value");
    }
}

#[test]
fn test_filter_edges() {
    let (storage, _temp_dir) = setup_test_db();
    let mut txn = storage.graph_env.write_txn().unwrap();

    let person1 = storage
        .create_node(&mut txn, "person", props!(), None)
        .unwrap();
    let person2 = storage
        .create_node(&mut txn, "person", props!(), None)
        .unwrap();

    let _ = storage
        .create_edge(
            &mut txn,
            "knows",
            &person1.id,
            &person2.id,
            props! { "since" => 2020 },
        )
        .unwrap();
    let edge2 = storage
        .create_edge(
            &mut txn,
            "knows",
            &person2.id,
            &person1.id,
            props! { "since" => 2022 },
        )
        .unwrap();

    txn.commit().unwrap();
    let txn = storage.graph_env.read_txn().unwrap();
    let mut traversal = TraversalBuilder::new(Arc::clone(&storage), TraversalValue::Empty);
    traversal.e(&txn);

    fn recent_edge(val: &Edge, year: i32) -> Result<bool, GraphError> {
        if let Some(value) = val.check_property("since") {
            match value {
                Value::Integer(since) => return Ok(*since > year),
                Value::Float(since) => return Ok(*since > year as f64),
                _ => return Err(GraphError::TraversalError("Invalid type".to_string())),
            }
        }
        Err(GraphError::TraversalError("Invalid edge".to_string()))
    }

    traversal.filter_edges(&txn, |edge| recent_edge(edge, 2021));

    match &traversal.current_step {
        // TraversalValue::SingleEdge(edge) => {
        //     assert_eq!(edge.id, edge2.id);
        // }
        TraversalValue::EdgeArray(edges) => {
            assert_eq!(edges.len(), 1);
            assert_eq!(edges[0].id, edge2.id);
        }
        _ => panic!("Expected Edge value"),
    }

    if let TraversalValue::Count(count) = &traversal.count().current_step {
        assert_eq!(count.value(), 1);
    } else {
        panic!("Expected Count value");
    }
}

#[test]
fn test_filter_empty_result() {
    let (storage, _temp_dir) = setup_test_db();
    let mut txn = storage.graph_env.write_txn().unwrap();

    let _ = storage
        .create_node(&mut txn, "person", props! { "age" => 25 }, None)
        .unwrap();

    txn.commit().unwrap();
    let txn = storage.graph_env.read_txn().unwrap();
    let mut traversal = TraversalBuilder::new(Arc::clone(&storage), TraversalValue::Empty);
    traversal.v(&txn);

    // Filter with a condition that no nodes satisfy
    traversal.filter_nodes(&txn, |val| {
        if let Some(value) = val.check_property("age") {
            match value {
                Value::Integer(age) => return Ok(*age > 100),
                Value::Float(age) => return Ok(*age > 100.0),
                _ => return Err(GraphError::TraversalError("Invalid type".to_string())),
            }
        }
        Err(GraphError::TraversalError("Invalid node".to_string()))
    });
    if let TraversalValue::NodeArray(nodes) = &traversal.current_step {
        assert!(nodes.is_empty());
    } else {
        panic!("Expected NodeArray value");
    }

    if let TraversalValue::Count(count) = &traversal.count().current_step {
        assert_eq!(count.value(), 0);
    } else {
        panic!("Expected Count value");
    }
}

#[test]
fn test_filter_chain() {
    let (storage, _temp_dir) = setup_test_db();
    let mut txn = storage.graph_env.write_txn().unwrap();

    let _ = storage
        .create_node(
            &mut txn,
            "person",
            props! { "age" => 25, "name" => "Alice" },
            None,
        )
        .unwrap();
    let person2 = storage
        .create_node(
            &mut txn,
            "person",
            props! { "age" => 30, "name" => "Bob" },
            None,
        )
        .unwrap();
    let _ = storage
        .create_node(&mut txn, "person", props! { "age" => 35 }, None)
        .unwrap();

    txn.commit().unwrap();
    let txn = storage.graph_env.read_txn().unwrap();
    let mut traversal = TraversalBuilder::new(Arc::clone(&storage), TraversalValue::Empty);
    traversal.v(&txn);

    fn has_name(val: &Node) -> Result<bool, GraphError> {
        return Ok(val.check_property("name").is_some());
    }

    fn age_greater_than(val: &Node, min_age: i32) -> Result<bool, GraphError> {
        if let Some(value) = val.check_property("age") {
            match value {
                Value::Float(age) => Ok(*age > min_age as f64),
                Value::Integer(age) => Ok(*age > min_age),
                _ => Err(GraphError::TraversalError("Invalid type".to_string())),
            }
        } else {
            Err(GraphError::TraversalError("Invalid node".to_string()))
        }
    }

    traversal
        .filter_nodes(&txn, has_name)
        .filter_nodes(&txn, |val| age_greater_than(val, 27));

    match &traversal.current_step {
        TraversalValue::NodeArray(nodes) => {
            assert_eq!(nodes.len(), 1);
            assert_eq!(nodes[0].id, person2.id);
        }
        _ => panic!("Expected Node value"),
    }

    if let TraversalValue::Count(count) = &traversal.count().current_step {
        assert_eq!(count.value(), 1);
    } else {
        panic!("Expected Count value");
    }
}

#[test]
fn test_in_v() {
    let (storage, _temp_dir) = setup_test_db();
    let mut txn = storage.graph_env.write_txn().unwrap();

    let person1 = storage
        .create_node(&mut txn, "person", props!(), None)
        .unwrap();
    let person2 = storage
        .create_node(&mut txn, "person", props!(), None)
        .unwrap();

    let edge = storage
        .create_edge(&mut txn, "knows", &person1.id, &person2.id, props!())
        .unwrap();

    txn.commit().unwrap();
    let txn = storage.graph_env.read_txn().unwrap();
    let mut traversal = TraversalBuilder::new(Arc::clone(&storage), TraversalValue::Empty);
    traversal.e_from_id(&txn, &edge.id).in_v(&txn);

    match &traversal.current_step {
        TraversalValue::NodeArray(nodes) => {
            assert_eq!(nodes.len(), 1);
            assert_eq!(nodes[0].id, person2.id);
        }
        _ => panic!("Expected SingleNode value"),
    }
}

#[test]
fn test_out_v() {
    let (storage, _temp_dir) = setup_test_db();
    let mut txn = storage.graph_env.write_txn().unwrap();

    let person1 = storage
        .create_node(&mut txn, "person", props!(), None)
        .unwrap();
    let person2 = storage
        .create_node(&mut txn, "person", props!(), None)
        .unwrap();

    let edge = storage
        .create_edge(&mut txn, "knows", &person1.id, &person2.id, props!())
        .unwrap();

    txn.commit().unwrap();
    let txn = storage.graph_env.read_txn().unwrap();
    let mut traversal = TraversalBuilder::new(Arc::clone(&storage), TraversalValue::Empty);
    traversal.e_from_id(&txn, &edge.id).out_v(&txn);

    match &traversal.current_step {
        TraversalValue::NodeArray(nodes) => {
            assert_eq!(nodes.len(), 1);
            assert_eq!(nodes[0].id, person1.id);
        }
        _ => panic!("Expected SingleNode value"),
    }
}

#[test]
fn test_both() {
    let (storage, _temp_dir) = setup_test_db();
    let mut txn = storage.graph_env.write_txn().unwrap();

    let person1 = storage
        .create_node(&mut txn, "person", props!(), None)
        .unwrap();
    let person2 = storage
        .create_node(&mut txn, "person", props!(), None)
        .unwrap();
    let person3 = storage
        .create_node(&mut txn, "person", props!(), None)
        .unwrap();

    storage
        .create_edge(&mut txn, "knows", &person1.id, &person2.id, props!())
        .unwrap();
    storage
        .create_edge(&mut txn, "knows", &person2.id, &person3.id, props!())
        .unwrap();
    storage
        .create_edge(&mut txn, "knows", &person3.id, &person2.id, props!())
        .unwrap();
    txn.commit().unwrap();
    let txn = storage.graph_env.read_txn().unwrap();
    let mut traversal =
        TraversalBuilder::new(Arc::clone(&storage), TraversalValue::from(person2.clone()));
    traversal.both(&txn, "knows");

    let nds = match_node_array(&traversal.current_step);
    let nodes = nds.iter().map(|n| n.id.clone()).collect::<Vec<String>>();

    assert_eq!(nodes.len(), 3);
    assert!(nodes.contains(&person1.id));
    assert!(nodes.contains(&person3.id));
}

#[test]
fn test_both_e() {
    let (storage, _temp_dir) = setup_test_db();
    let mut txn = storage.graph_env.write_txn().unwrap();
    let db = Arc::clone(&storage);
    let person1 = storage
        .create_node(&mut txn, "person", props!(), None)
        .unwrap();
    let person2 = storage
        .create_node(&mut txn, "person", props!(), None)
        .unwrap();

    let edge1 = storage
        .create_edge(&mut txn, "knows", &person1.id, &person2.id, props!())
        .unwrap();
    let edge2 = storage
        .create_edge(&mut txn, "likes", &person2.id, &person1.id, props!())
        .unwrap();

    txn.commit().unwrap();
    let txn = storage.graph_env.read_txn().unwrap();
    let mut traversal =
        TraversalBuilder::new(Arc::clone(&storage), TraversalValue::from(person2.clone()));
    traversal.both_e(&txn, "knows");

    match &traversal.current_step {
        TraversalValue::EdgeArray(edges) => {
            assert_eq!(edges.len(), 1);
            assert_eq!(edges[0].id, edge1.id);
        }
        _ => panic!("Expected EdgeArray value"),
    }

    let mut traversal =
        TraversalBuilder::new(Arc::clone(&storage), TraversalValue::from(person2.clone()));
    traversal.both_e(&txn, "likes");

    match &traversal.current_step {
        TraversalValue::EdgeArray(edges) => {
            assert_eq!(edges.len(), 1);
            assert_eq!(edges[0].id, edge2.id);
        }
        _ => panic!("Expected EdgeArray value"),
    }
}

#[test]
fn test_both_v() {
    let (storage, _temp_dir) = setup_test_db();
    let mut txn = storage.graph_env.write_txn().unwrap();

    let person1 = storage
        .create_node(&mut txn, "person", props!(), None)
        .unwrap();
    let person2 = storage
        .create_node(&mut txn, "person", props!(), None)
        .unwrap();

    let edge = storage
        .create_edge(&mut txn, "knows", &person1.id, &person2.id, props!())
        .unwrap();

    txn.commit().unwrap();
    let txn = storage.graph_env.read_txn().unwrap();
    let mut traversal = TraversalBuilder::new(Arc::clone(&storage), TraversalValue::Empty);
    traversal.e_from_id(&txn, &edge.id).both_v(&txn);

    match &traversal.current_step {
        TraversalValue::NodeArray(nodes) => {
            assert_eq!(nodes.len(), 2);
            let node_ids: Vec<String> = nodes.iter().map(|n| n.id.clone()).collect();
            assert!(node_ids.contains(&person1.id));
            assert!(node_ids.contains(&person2.id));
        }
        _ => panic!("Expected NodeArray value"),
    }
}

fn match_node_array(value: &TraversalValue) -> Vec<Node> {
    match value {
        TraversalValue::NodeArray(nodes) => nodes.clone(),
        _ => vec![],
    }
}

#[test]
fn test_shortest_mutual_path() {
    let (storage, _temp_dir) = setup_test_db();
    let mut txn = storage.graph_env.write_txn().unwrap();

    // Create a complex network of mutual and one-way connections
    // Mutual: Alice <-> Bob <-> Charlie <-> David
    // One-way: Alice -> Eve -> David
    let users: Vec<Node> = vec!["alice", "bob", "charlie", "dave", "eve"]
        .iter()
        .map(|name| {
            storage
                .create_node(&mut txn, "person", props! { "name" => *name }, None)
                .unwrap()
        })
        .collect();

    for (i, j) in [(0, 1), (1, 2), (2, 3)].iter() {
        storage
            .create_edge(&mut txn, "knows", &users[*i].id, &users[*j].id, props!())
            .unwrap();
        storage
            .create_edge(&mut txn, "knows", &users[*j].id, &users[*i].id, props!())
            .unwrap();
    }

    storage
        .create_edge(&mut txn, "knows", &users[0].id, &users[4].id, props!())
        .unwrap();
    storage
        .create_edge(&mut txn, "knows", &users[4].id, &users[3].id, props!())
        .unwrap();

    txn.commit().unwrap();

    let txn = storage.graph_env.read_txn().unwrap();
    let mut tr =
        TraversalBuilder::new(Arc::clone(&storage), TraversalValue::from(users[0].clone()));
    tr.shortest_mutual_path_to(&txn, &users[3].id);

    let result = tr.result(txn);
    let paths = match result.unwrap() {
        TraversalValue::Paths(paths) => paths,
        _ => {
            panic!("Expected PathArray value")
        }
    };

    assert_eq!(paths.len(), 1);
    let (nodes, edges) = &paths[0];

    assert_eq!(nodes.len(), 4);
    assert_eq!(edges.len(), 3);
    assert_eq!(nodes[0].id, users[3].id); // David
    assert_eq!(nodes[1].id, users[2].id); // Charlie
    assert_eq!(nodes[2].id, users[1].id); // Bob
    assert_eq!(nodes[3].id, users[0].id); // Alice
}
