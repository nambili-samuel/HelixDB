use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Instant;

use get_routes::handler;
use helixdb::{
    helix_engine::graph_core::traversal::TraversalBuilder,
    helix_engine::graph_core::traversal_steps::{
        SourceTraversalSteps, TraversalBuilderMethods, TraversalMethods, TraversalSearchMethods,
        TraversalSteps, VectorTraversalSteps,
    },
    helix_engine::types::GraphError,
    helix_gateway::router::router::HandlerInput,
    node_matches, props,
    protocol::count::Count,
    protocol::remapping::ResponseRemapping,
    protocol::response::Response,
    protocol::traversal_value::TraversalValue,
    protocol::{
        filterable::Filterable, remapping::Remapping, return_values::ReturnValue, value::Value,
    },
};
use sonic_rs::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

// Node Schema: Record
#[derive(Serialize, Deserialize)]
struct Record {
    id: String,
    data: String,
}

#[handler]
pub fn delete_record(input: &HandlerInput, response: &mut Response) -> Result<(), GraphError> {
    #[derive(Serialize, Deserialize)]
    struct delete_recordData {
        id: String,
    }

    let data: delete_recordData = match sonic_rs::from_slice(&input.request.body) {
        Ok(data) => data,
        Err(err) => return Err(GraphError::from(err)),
    };

    let mut remapping_vals: RefCell<HashMap<String, ResponseRemapping>> =
        RefCell::new(HashMap::new());
    let db = Arc::clone(&input.graph.storage);
    let mut txn = db.graph_env.write_txn().unwrap();

    let mut return_vals: HashMap<String, ReturnValue> = HashMap::with_capacity(1);

    let mut tr = TraversalBuilder::new(Arc::clone(&db), TraversalValue::Empty);
    tr.v_from_id(&txn, &data.id);
    tr.drop(&mut txn);
    return_vals.insert("message".to_string(), ReturnValue::Empty);
    response.body = sonic_rs::to_vec(&return_vals).unwrap();

    txn.commit()?;
    Ok(())
}

#[handler]
pub fn update_record(input: &HandlerInput, response: &mut Response) -> Result<(), GraphError> {
    #[derive(Serialize, Deserialize)]
    struct update_recordData {
        id: String,
        data: String,
    }

    let data: update_recordData = match sonic_rs::from_slice(&input.request.body) {
        Ok(data) => data,
        Err(err) => return Err(GraphError::from(err)),
    };

    let mut remapping_vals: RefCell<HashMap<String, ResponseRemapping>> =
        RefCell::new(HashMap::new());
    let db = Arc::clone(&input.graph.storage);

    let mut txn = db.graph_env.write_txn().unwrap();

    let mut return_vals: HashMap<String, ReturnValue> = HashMap::with_capacity(1);

    let mut tr = TraversalBuilder::new(Arc::clone(&db), TraversalValue::Empty);
    tr.v_from_id(&txn, &data.id);
    tr.update_props(&mut txn, props! { "data".to_string() => data.data });
    let record = tr.finish()?;

    return_vals.insert(
        "record".to_string(),
        ReturnValue::from_traversal_value_array_with_mixin(record, remapping_vals.borrow_mut()),
    );
    response.body = sonic_rs::to_vec(&return_vals).unwrap();

    Ok(())
}

#[handler]
pub fn count_records(input: &HandlerInput, response: &mut Response) -> Result<(), GraphError> {
    let mut remapping_vals: RefCell<HashMap<String, ResponseRemapping>> =
        RefCell::new(HashMap::new());
    let db = Arc::clone(&input.graph.storage);
    let txn = db.graph_env.read_txn().unwrap();

    let mut return_vals: HashMap<String, ReturnValue> = HashMap::with_capacity(1);

    let mut tr = TraversalBuilder::new(Arc::clone(&db), TraversalValue::Empty);
    tr.v_from_types(&txn, &["Record"]);
    tr.count();
    let count = tr.finish()?;

    return_vals.insert(
        "count".to_string(),
        ReturnValue::from_traversal_value_array_with_mixin(count, remapping_vals.borrow_mut()),
    );
    response.body = sonic_rs::to_vec(&return_vals).unwrap();

    Ok(())
}

#[handler]
pub fn scan_records(input: &HandlerInput, response: &mut Response) -> Result<(), GraphError> {
    #[derive(Serialize, Deserialize)]
    struct scan_recordsData {
        limit: i32,
        offset: i32,
    }

    let data: scan_recordsData = match sonic_rs::from_slice(&input.request.body) {
        Ok(data) => data,
        Err(err) => return Err(GraphError::from(err)),
    };

    let mut remapping_vals: RefCell<HashMap<String, ResponseRemapping>> =
        RefCell::new(HashMap::new());
    let db = Arc::clone(&input.graph.storage);
    let txn = db.graph_env.read_txn().unwrap();

    let mut return_vals: HashMap<String, ReturnValue> = HashMap::with_capacity(1);

    let mut tr = TraversalBuilder::new(Arc::clone(&db), TraversalValue::Empty);
    tr.v_from_types(&txn, &["Record"]);
    tr.range(data.offset, data.limit);
    let records = tr.finish()?;

    return_vals.insert(
        "records".to_string(),
        ReturnValue::from_traversal_value_array_with_mixin(records, remapping_vals.borrow_mut()),
    );
    response.body = sonic_rs::to_vec(&return_vals).unwrap();

    Ok(())
}

#[handler]
pub fn create_record(input: &HandlerInput, response: &mut Response) -> Result<(), GraphError> {
    #[derive(Serialize, Deserialize)]
    struct create_recordData {
        id: String,
        data: JsonValue,
    }

    let data: create_recordData = match sonic_rs::from_slice(&input.request.body) {
        Ok(data) => data,
        Err(err) => return Err(GraphError::from(err)),
    };

    let mut remapping_vals: RefCell<HashMap<String, ResponseRemapping>> =
        RefCell::new(HashMap::new());
    let db = Arc::clone(&input.graph.storage);
    let mut txn = db.graph_env.write_txn().unwrap();

    let mut return_vals: HashMap<String, ReturnValue> = HashMap::with_capacity(1);

    let mut tr = TraversalBuilder::new(Arc::clone(&db), TraversalValue::Empty);
    let mut tr = TraversalBuilder::new(Arc::clone(&db), TraversalValue::Empty);
    tr.add_v(
        &mut txn,
        "Record",
        props! { "id".to_string() => data.id, "data".to_string() => data.data },
        None,
    );
    let record = tr.finish()?;

    return_vals.insert(
        "record".to_string(),
        ReturnValue::from_traversal_value_array_with_mixin(record, remapping_vals.borrow_mut()),
    );
    response.body = sonic_rs::to_vec(&return_vals).unwrap();

    txn.commit()?;
    Ok(())
}

#[handler]
pub fn read_record(input: &HandlerInput, response: &mut Response) -> Result<(), GraphError> {
    #[derive(Serialize, Deserialize)]
    struct read_recordData {
        id: String,
    }

    let data: read_recordData = match sonic_rs::from_slice(&input.request.body) {
        Ok(data) => data,
        Err(err) => return Err(GraphError::from(err)),
    };

    let mut remapping_vals: RefCell<HashMap<String, ResponseRemapping>> =
        RefCell::new(HashMap::new());
    let db = Arc::clone(&input.graph.storage);
    let txn = db.graph_env.read_txn().unwrap();

    let mut return_vals: HashMap<String, ReturnValue> = HashMap::with_capacity(1);

    let mut tr = TraversalBuilder::new(Arc::clone(&db), TraversalValue::Empty);
    tr.v_from_id(&txn, &data.id);
    let record = tr.finish()?;

    return_vals.insert(
        "record".to_string(),
        ReturnValue::from_traversal_value_array_with_mixin(record, remapping_vals.borrow_mut()),
    );
    response.body = sonic_rs::to_vec(&return_vals).unwrap();

    Ok(())
}

#[handler]
pub fn size(input: &HandlerInput, response: &mut Response) -> Result<(), GraphError> {
    let mut remapping_vals: RefCell<HashMap<String, ResponseRemapping>> =
        RefCell::new(HashMap::new());
    let db = Arc::clone(&input.graph.storage);
    let txn = db.graph_env.read_txn().unwrap();

    let mut return_vals: HashMap<String, ReturnValue> = HashMap::with_capacity(1);

    let mut tr = TraversalBuilder::new(Arc::clone(&db), TraversalValue::Empty);
    tr.count();
    let size = tr.finish()?;

    return_vals.insert(
        "size".to_string(),
        ReturnValue::from_traversal_value_array_with_mixin(size, remapping_vals.borrow_mut()),
    );
    response.body = sonic_rs::to_vec(&return_vals).unwrap();

    Ok(())
}

#[handler]
pub fn hnswsearch(input: &HandlerInput, response: &mut Response) -> Result<(), GraphError> {
    #[derive(Serialize, Deserialize)]
    struct hnswsearchData {
        query: Vec<f64>,
        k: i32,
    }

    let data: hnswsearchData = match sonic_rs::from_slice(&input.request.body) {
        Ok(data) => data,
        Err(err) => return Err(GraphError::from(err)),
    };

    let mut remapping_vals: RefCell<HashMap<String, ResponseRemapping>> =
        RefCell::new(HashMap::new());
    let db = Arc::clone(&input.graph.storage);
    let txn = db.graph_env.read_txn().unwrap();

    let mut return_vals: HashMap<String, ReturnValue> = HashMap::with_capacity(1);

    let mut tr = TraversalBuilder::new(Arc::clone(&db), TraversalValue::Empty);
    let mut tr = TraversalBuilder::new(Arc::clone(&db), TraversalValue::Empty);
    tr.vector_search(&txn, &data.query, data.k as usize);
    let res = tr.finish()?;

    return_vals.insert(
        "res".to_string(),
        ReturnValue::from_traversal_value_array_with_mixin(res, remapping_vals.borrow_mut()),
    );
    response.body = sonic_rs::to_vec(&return_vals).unwrap();

    Ok(())
}

#[handler]
pub fn hnswinsert(input: &HandlerInput, response: &mut Response) -> Result<(), GraphError> {
    #[derive(Serialize, Deserialize)]
    struct hnswinsertData {
        vector: Vec<f64>,
    }

    let data: hnswinsertData = match sonic_rs::from_slice(&input.request.body) {
        Ok(data) => data,
        Err(err) => return Err(GraphError::from(err)),
    };

    let mut remapping_vals: RefCell<HashMap<String, ResponseRemapping>> =
        RefCell::new(HashMap::new());
    let db = Arc::clone(&input.graph.storage);
    let mut txn = db.graph_env.write_txn().unwrap();

    let mut return_vals: HashMap<String, ReturnValue> = HashMap::with_capacity(1);

    let mut tr = TraversalBuilder::new(Arc::clone(&db), TraversalValue::Empty);
    tr.insert_vector(&mut txn, &data.vector);
    return_vals.insert("message".to_string(), ReturnValue::from("Success"));
    response.body = sonic_rs::to_vec(&return_vals).unwrap();

    txn.commit()?;
    Ok(())
}

#[handler]
pub fn hnswload(input: &HandlerInput, response: &mut Response) -> Result<(), GraphError> {
    #[derive(Serialize, Deserialize)]
    struct hnswloadData {
        vectors: Vec<Vec<f64>>,
    }

    let data: hnswloadData = match sonic_rs::from_slice(&input.request.body) {
        Ok(data) => data,
        Err(err) => return Err(GraphError::from(err)),
    };

    let mut remapping_vals: RefCell<HashMap<String, ResponseRemapping>> =
        RefCell::new(HashMap::new());
    let db = Arc::clone(&input.graph.storage);
    let mut txn = db.graph_env.write_txn().unwrap();

    let mut return_vals: HashMap<String, ReturnValue> = HashMap::with_capacity(1);

    let mut tr = TraversalBuilder::new(Arc::clone(&db), TraversalValue::Empty);
    let mut tr = TraversalBuilder::new(Arc::clone(&db), TraversalValue::Empty);
    for vec in data.vectors {
        tr.insert_vector(&mut txn, &vec);
    }
    let res = tr.finish()?;

    let mut tr = TraversalBuilder::new(Arc::clone(&db), TraversalValue::from(res.clone()));
    tr.for_each_node(&txn, |item, txn| {
        let id = item.check_property("ID");
        let id_remapping = Remapping::new(
            false,
            None,
            Some(match id {
                Some(value) => ReturnValue::from(value.clone()),
                None => {
                    return Err(GraphError::ConversionError(
                        "Property not found on id".to_string(),
                    ))
                }
            }),
        );
        remapping_vals.borrow_mut().insert(
            item.id.clone(),
            ResponseRemapping::new(HashMap::from([("ID".to_string(), id_remapping)]), false),
        );
        Ok(())
    });
    let return_val = tr.finish()?;
    return_vals.insert(
        "res".to_string(),
        ReturnValue::from_traversal_value_array_with_mixin(return_val, remapping_vals.borrow_mut()),
    );
    response.body = sonic_rs::to_vec(&return_vals).unwrap();

    txn.commit()?;
    Ok(())
}
