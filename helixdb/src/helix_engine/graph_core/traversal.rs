use crate::helix_engine::{
    graph_core::traversal_steps::{
        TraversalBuilderMethods, TraversalMethods, TraversalSearchMethods,
    },
    storage_core::{
        storage_core::HelixGraphStorage,
        storage_methods::{SearchMethods, StorageMethods},
    },
    types::GraphError,
    vector_core::{hnsw::HNSW, vector::HVector},
};
use crate::protocol::{
    count::Count,
    filterable::Filterable,
    items::{Edge, Node},
    traversal_value::TraversalValue,
    value::Value,
};
use core::panic;
use heed3::{Error, RoTxn, RwTxn, WithTls};
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use super::traversal_steps::{SourceTraversalSteps, TraversalSteps, VectorTraversalSteps};

pub struct TraversalBuilder {
    pub variables: HashMap<String, TraversalValue>,
    pub current_step: TraversalValue,
    pub storage: Arc<HelixGraphStorage>,
    pub error: Option<GraphError>,
}

impl TraversalBuilder {
    pub fn new(storage: Arc<HelixGraphStorage>, start_nodes: TraversalValue) -> Self {
        Self {
            variables: HashMap::new(),
            current_step: start_nodes,
            storage,
            error: None,
        }
    }

    pub fn check_is_valid_node_traversal(&self, function_name: &str) -> Result<(), GraphError> {
        match matches!(self.current_step, TraversalValue::NodeArray(_)) {
            true => Ok(()),
            false => Err(GraphError::TraversalError(format!(
                "The traversal step {:?}, is not a valid traversal from an edge.
                The current step should be a node.",
                function_name
            ))),
        }
    }

    pub fn check_is_valid_edge_traversal(&self, function_name: &str) -> Result<(), GraphError> {
        match matches!(self.current_step, TraversalValue::EdgeArray(_)) {
            true => Ok(()),
            false => Err(GraphError::TraversalError(format!(
                "The traversal step {:?}, is not a valid traversal from a node.
                The current step should be an edge",
                function_name
            ))),
        }
    }

    #[inline(always)]
    fn store_error(&mut self, err: GraphError) {
        if let GraphError::Empty = err {
            return;
        }
        if self.error.is_none() {
            self.error = Some(err);
        }
    }

    pub fn add_v_temp(
        &mut self,
        txn: &mut RwTxn,
        node_label: &str,
        props: Vec<(String, Value)>,
        secondary_indices: Option<&[String]>,
    ) -> Result<(), GraphError> {
        self.storage
            .create_node_(txn, node_label, props, secondary_indices)?;
        Ok(())
    }

    pub fn mut_self(&mut self) -> &mut Self {
        self
    }

    pub fn drop(&mut self, txn: &mut RwTxn) -> &mut Self {
        let mut e = GraphError::Empty;
        match &self.current_step {
            TraversalValue::NodeArray(nodes) => {
                for node in nodes {
                    match self.storage.drop_node(txn, &node.id) {
                        Ok(_) => {}
                        Err(err) => e = err,
                    }
                }
            }
            TraversalValue::EdgeArray(edges) => {
                for edge in edges {
                    match self.storage.drop_edge(txn, &edge.id) {
                        Ok(_) => {}
                        Err(err) => e = err,
                    }
                }
            }
            _ => {}
        }
        self.store_error(e);
        self
    }

    pub fn id(&mut self) -> &mut Self {
        match &self.current_step {
            TraversalValue::NodeArray(nodes) => match nodes.first() {
                Some(node) => {
                    self.current_step = TraversalValue::ValueArray(vec![(
                        "id".to_string(),
                        Value::from(node.id.clone()),
                    )]);
                }
                None => {
                    self.current_step = TraversalValue::Empty;
                }
            },
            TraversalValue::EdgeArray(edges) => match edges.first() {
                Some(edge) => {
                    self.current_step = TraversalValue::ValueArray(vec![(
                        "id".to_string(),
                        Value::from(edge.id.clone()),
                    )]);
                }
                None => {
                    self.current_step = TraversalValue::Empty;
                }
            },
            _ => {
                self.current_step = TraversalValue::Empty;
            }
        }
        self
    }
}

impl SourceTraversalSteps for TraversalBuilder {
    fn v(&mut self, txn: &RoTxn) -> &mut Self {
        match self.storage.get_all_nodes(txn) {
            Ok(nodes) => {
                self.current_step = TraversalValue::NodeArray(nodes);
            }
            Err(err) => match err {
                GraphError::Empty => {
                    self.current_step = TraversalValue::Empty;
                }
                _ => {
                    self.store_error(err);
                }
            },
        }
        self
    }

    fn e(&mut self, txn: &RoTxn) -> &mut Self {
        match self.storage.get_all_edges(txn) {
            Ok(edges) => {
                self.current_step = TraversalValue::EdgeArray(edges);
            }
            Err(err) => match err {
                GraphError::Empty => {
                    self.current_step = TraversalValue::Empty;
                }
                _ => {
                    self.store_error(err);
                }
            },
        }
        self
    }

    fn v_from_id(&mut self, txn: &RoTxn, node_id: &str) -> &mut Self {
        match self.storage.get_node(txn, node_id) {
            Ok(node) => {
                self.current_step = TraversalValue::from(node);
            }
            Err(err) => match err {
                GraphError::NodeNotFound => {
                    self.current_step = TraversalValue::Empty;
                }
                _ => {
                    self.store_error(err);
                }
            },
        }
        self
    }

    fn v_from_ids(&mut self, txn: &RoTxn, node_ids: &[&str]) -> &mut Self {
        let mut new_current = Vec::with_capacity(node_ids.len());
        for node_id in node_ids {
            match self.storage.get_node(txn, node_id) {
                Ok(node) => new_current.push(node),
                Err(err) => {
                    self.store_error(err);
                }
            }
        }
        if new_current.is_empty() {
            self.current_step = TraversalValue::Empty;
        } else {
            self.current_step = TraversalValue::NodeArray(new_current);
        }
        self
    }

    fn e_from_id(&mut self, txn: &RoTxn, edge_id: &str) -> &mut Self {
        match self.storage.get_edge(txn, edge_id) {
            Ok(edge) => {
                self.current_step = TraversalValue::from(edge);
            }
            Err(err) => match err {
                GraphError::EdgeNotFound => {
                    self.current_step = TraversalValue::Empty;
                }
                _ => {
                    self.store_error(err);
                }
            },
        }
        self
    }

    fn v_from_types(&mut self, txn: &RoTxn, node_labels: &[&str]) -> &mut Self {
        match self.storage.get_nodes_by_types(txn, node_labels) {
            Ok(nodes) => {
                self.current_step = TraversalValue::NodeArray(nodes);
            }
            Err(err) => match err {
                GraphError::NodeNotFound => {
                    self.current_step = TraversalValue::Empty;
                }
                _ => {
                    self.store_error(err);
                }
            },
        }
        self
    }

    fn v_from_secondary_index(&mut self, txn: &RoTxn, index: &str, value: &Value) -> &mut Self {
        match self.storage.get_node_by_secondary_index(txn, index, value) {
            Ok(node) => {
                self.current_step = TraversalValue::from(node);
            }
            Err(err) => match err {
                GraphError::NodeNotFound => {
                    self.current_step = TraversalValue::Empty;
                }
                _ => {
                    self.store_error(err);
                }
            },
        }
        self
    }

    fn add_v(
        &mut self,
        txn: &mut RwTxn,
        node_label: &str,
        props: Vec<(String, Value)>,
        secondary_indices: Option<&[String]>,
    ) -> &mut Self {
        match self
            .storage
            .create_node(txn, node_label, props, secondary_indices)
        {
            Ok(node) => {
                self.current_step = TraversalValue::from(node);
            }
            Err(err) => {
                self.store_error(err);
            }
        }
        self
    }

    fn add_e(
        &mut self,
        txn: &mut RwTxn,
        edge_label: &str,
        from_id: &str,
        to_id: &str,
        props: Vec<(String, Value)>,
    ) -> &mut Self {
        match self
            .storage
            .create_edge(txn, edge_label, from_id, to_id, props)
        {
            Ok(edge) => {
                self.current_step = TraversalValue::from(edge);
            }
            Err(err) => {
                self.store_error(err);
            }
        }
        self
    }
}

impl TraversalSteps for TraversalBuilder {
    fn out(&mut self, txn: &RoTxn, edge_label: &str) -> &mut Self {
        let mut e = GraphError::Empty;
        if let TraversalValue::NodeArray(nodes) = &self.current_step {
            let mut new_current = Vec::with_capacity(nodes.len());
            for node in nodes {
                match self.storage.get_out_nodes(txn, &node.id, edge_label) {
                    Ok(nodes) => match nodes.is_empty() {
                        false => new_current.extend(nodes),
                        true => continue,
                    },
                    Err(err) => e = err,
                }
            }
            if new_current.is_empty() {
                self.current_step = TraversalValue::Empty;
            } else {
                self.current_step = TraversalValue::NodeArray(new_current);
            }
        } else {
            self.current_step = TraversalValue::Empty;
        }
        self.store_error(e);
        self
    }

    fn out_e(&mut self, txn: &RoTxn, edge_label: &str) -> &mut Self {
        let mut e = GraphError::Empty;
        if let TraversalValue::NodeArray(nodes) = &self.current_step {
            let mut new_current = Vec::with_capacity(nodes.len());
            for node in nodes {
                match self.storage.get_out_edges(txn, &node.id, edge_label) {
                    Ok(edges) => match edges.is_empty() {
                        false => new_current.extend(edges),
                        true => continue,
                    },
                    Err(err) => e = err,
                }
            }
            if new_current.is_empty() {
                self.current_step = TraversalValue::Empty;
            } else {
                self.current_step = TraversalValue::EdgeArray(new_current);
            }
        } else {
            self.current_step = TraversalValue::Empty;
        }
        self.store_error(e);
        self
    }

    fn in_(&mut self, txn: &RoTxn, edge_label: &str) -> &mut Self {
        let mut e = GraphError::Empty;
        if let TraversalValue::NodeArray(nodes) = &self.current_step {
            let mut new_current = Vec::with_capacity(nodes.len());
            for node in nodes {
                match self.storage.get_in_nodes(txn, &node.id, edge_label) {
                    Ok(nodes) => match nodes.is_empty() {
                        false => new_current.extend(nodes),
                        true => continue,
                    },
                    Err(err) => e = err,
                }
            }
            if new_current.is_empty() {
                self.current_step = TraversalValue::Empty;
            } else {
                self.current_step = TraversalValue::NodeArray(new_current);
            }
        } else {
            self.current_step = TraversalValue::Empty;
        }
        self.store_error(e);
        self
    }

    fn in_e(&mut self, txn: &RoTxn, edge_label: &str) -> &mut Self {
        let mut e = GraphError::Empty;
        if let TraversalValue::NodeArray(nodes) = &self.current_step {
            let mut new_current = Vec::with_capacity(nodes.len());
            for node in nodes {
                match self.storage.get_in_edges(txn, &node.id, edge_label) {
                    Ok(edges) => match edges.is_empty() {
                        false => new_current.extend(edges),
                        true => continue,
                    },
                    Err(err) => e = err,
                }
            }
            if new_current.is_empty() {
                self.current_step = TraversalValue::Empty;
            } else {
                self.current_step = TraversalValue::EdgeArray(new_current);
            }
        } else {
            self.current_step = TraversalValue::Empty;
        }
        self.store_error(e);
        self
    }

    fn both_e(&mut self, txn: &RoTxn, edge_label: &str) -> &mut Self {
        let mut e = GraphError::Empty;
        if let TraversalValue::NodeArray(nodes) = &self.current_step {
            let mut new_current = Vec::with_capacity(nodes.len());
            for node in nodes {
                match self.storage.get_in_edges(txn, &node.id, edge_label) {
                    Ok(in_edges) => {
                        if !in_edges.is_empty() {
                            new_current.extend(in_edges);
                        }
                    }
                    Err(err) => e = err,
                }
                match self.storage.get_out_edges(txn, &node.id, edge_label) {
                    Ok(out_edges) => {
                        if !out_edges.is_empty() {
                            new_current.extend(out_edges);
                        }
                    }
                    Err(err) => e = err,
                }
            }
            if new_current.is_empty() {
                self.current_step = TraversalValue::Empty;
            } else {
                self.current_step = TraversalValue::EdgeArray(new_current);
            }
        } else {
            self.current_step = TraversalValue::Empty;
        }
        self.store_error(e);
        self
    }

    fn both(&mut self, txn: &RoTxn, edge_label: &str) -> &mut Self {
        let mut e = GraphError::Empty;
        if let TraversalValue::NodeArray(nodes) = &self.current_step {
            let mut new_current = Vec::with_capacity(nodes.len());
            for node in nodes {
                match self.storage.get_in_nodes(txn, &node.id, edge_label) {
                    Ok(in_nodes) => {
                        if !in_nodes.is_empty() {
                            new_current.extend(in_nodes);
                        }
                    }
                    Err(err) => e = err,
                }
                match self.storage.get_out_nodes(txn, &node.id, edge_label) {
                    Ok(out_nodes) => {
                        if !out_nodes.is_empty() {
                            new_current.extend(out_nodes);
                        }
                    }
                    Err(err) => e = err,
                }
            }
            if new_current.is_empty() {
                self.current_step = TraversalValue::Empty;
            } else {
                self.current_step = TraversalValue::NodeArray(new_current);
            }
        } else {
            self.current_step = TraversalValue::Empty;
        }
        self.store_error(e);
        self
    }

    fn out_v(&mut self, txn: &RoTxn) -> &mut Self {
        let mut e = GraphError::Empty;
        if let TraversalValue::EdgeArray(edges) = &self.current_step {
            let mut new_current = Vec::with_capacity(edges.len());
            for edge in edges {
                match self.storage.get_node(txn, &edge.from_node) {
                    Ok(node) => new_current.push(node),
                    Err(err) => e = err,
                }
            }
            if new_current.is_empty() {
                self.current_step = TraversalValue::Empty;
            } else {
                self.current_step = TraversalValue::NodeArray(new_current);
            }
        } else {
            self.current_step = TraversalValue::Empty;
        }
        self.store_error(e);
        self
    }

    fn in_v(&mut self, txn: &RoTxn) -> &mut Self {
        let mut e = GraphError::Empty;
        if let TraversalValue::EdgeArray(edges) = &self.current_step {
            let mut new_current = Vec::with_capacity(edges.len());
            for edge in edges {
                match self.storage.get_node(txn, &edge.to_node) {
                    Ok(node) => new_current.push(node),
                    Err(err) => e = err,
                }
            }
            if new_current.is_empty() {
                self.current_step = TraversalValue::Empty;
            } else {
                self.current_step = TraversalValue::NodeArray(new_current);
            }
        } else {
            self.current_step = TraversalValue::Empty;
        }
        self.store_error(e);
        self
    }

    fn both_v(&mut self, txn: &RoTxn) -> &mut Self {
        let mut e = GraphError::Empty;
        if let TraversalValue::EdgeArray(edges) = &self.current_step {
            let mut new_current = Vec::with_capacity(edges.len() * 2);
            for edge in edges {
                match self.storage.get_node(txn, &edge.from_node) {
                    Ok(node) => new_current.push(node),
                    Err(err) => e = err,
                }
                match self.storage.get_node(txn, &edge.to_node) {
                    Ok(node) => new_current.push(node),
                    Err(err) => e = err,
                }
            }
            if new_current.is_empty() {
                self.current_step = TraversalValue::Empty;
            } else {
                self.current_step = TraversalValue::NodeArray(new_current);
            }
        } else {
            self.current_step = TraversalValue::Empty;
        }
        self.store_error(e);
        self
    }

    fn mutual(&mut self, txn: &RoTxn, edge_label: &str) -> &mut Self {
        let mut e: GraphError = GraphError::Empty;

        if let TraversalValue::NodeArray(nodes) = &self.current_step {
            let result: Vec<Node> = nodes
                .iter()
                .filter_map(|node| {
                    let out = self.storage.get_out_nodes(txn, &node.id, edge_label);
                    let in_ = self.storage.get_in_nodes(txn, &node.id, edge_label);

                    match (out, in_) {
                        (Ok(out), Ok(in_)) => {
                            let in_set: HashSet<_> = in_.into_iter().map(|n| n.id).collect();
                            Some(
                                out.into_iter()
                                    .filter(|n| in_set.contains(&n.id))
                                    .collect::<Vec<_>>(),
                            )
                        }
                        (Err(err), _) | (_, Err(err)) => {
                            e = err;

                            None
                        }
                    }
                })
                .flatten()
                .collect();

            self.current_step = TraversalValue::NodeArray(result);
        } else {
            self.current_step = TraversalValue::Empty;
        }
        self.store_error(e);
        self
    }

    fn add_e_from(
        &mut self,
        txn: &mut RwTxn,
        edge_label: &str,
        from_id: &str,
        props: Vec<(String, Value)>,
    ) -> &mut Self {
        let mut e = GraphError::Empty;
        match &self.current_step {
            TraversalValue::NodeArray(nodes) => {
                let mut new_current = Vec::with_capacity(nodes.len());
                for node in nodes {
                    match self.storage.create_edge(
                        txn,
                        edge_label,
                        from_id,
                        &node.id,
                        props.clone(),
                    ) {
                        Ok(edge) => new_current.push(edge),
                        Err(err) => e = err,
                    }
                }
                self.current_step = TraversalValue::EdgeArray(new_current);
            }
            TraversalValue::Empty => {}
            _ => panic!("Invalid traversal step for add_e_from"),
        }
        self.store_error(e);
        self
    }

    fn add_e_to(
        &mut self,
        txn: &mut RwTxn,
        edge_label: &str,
        to_id: &str,
        props: Vec<(String, Value)>,
    ) -> &mut Self {
        let mut e = GraphError::Empty;
        match &self.current_step {
            TraversalValue::NodeArray(nodes) => {
                let mut new_current = Vec::with_capacity(nodes.len());
                for node in nodes {
                    match self
                        .storage
                        .create_edge(txn, edge_label, &node.id, to_id, props.clone())
                    {
                        Ok(edge) => new_current.push(edge),
                        Err(err) => e = err,
                    }
                }
                self.current_step = TraversalValue::EdgeArray(new_current);
            }
            TraversalValue::Empty => {}
            _ => panic!("Invalid traversal step for add_e_to"),
        }
        self.store_error(e);
        self
    }

    fn update_props(&mut self, txn: &mut RwTxn, props: Vec<(String, Value)>) -> &mut Self {
        let mut e = GraphError::Empty;
        match &self.current_step {
            TraversalValue::NodeArray(nodes) => {
                if nodes.len() > 1 {
                    panic!("Invalid traversal step for update_props");
                }
                let mut new_current = Vec::with_capacity(nodes.len());
                for node in nodes {
                    match self.storage.update_node(txn, &node.id, props.clone()) {
                        Ok(node) => new_current.push(node),
                        Err(err) => e = err,
                    }
                }
                self.current_step = TraversalValue::NodeArray(new_current);
            }
            TraversalValue::EdgeArray(edges) => {
                if edges.len() > 1 {
                    panic!("Invalid traversal step for update_props");
                }
                let mut new_current = Vec::with_capacity(edges.len());
                for edge in edges {
                    match self.storage.update_edge(txn, &edge.id, props.clone()) {
                        Ok(edge) => new_current.push(edge),
                        Err(err) => e = err,
                    }
                }
                self.current_step = TraversalValue::EdgeArray(new_current);
            }
            _ => panic!("Invalid traversal step for update_props"),
        }
        self.store_error(e);
        self
    }
}

impl TraversalMethods for TraversalBuilder {
    fn count(&mut self) -> &mut Self {
        self.current_step = TraversalValue::Count(Count::new(match &self.current_step {
            TraversalValue::NodeArray(nodes) => nodes.len(),
            TraversalValue::EdgeArray(edges) => edges.len(),
            TraversalValue::Empty => 0,
            _ => panic!("Invalid traversal step for count {:?}", &self.current_step),
        }));
        self
    }
    fn range(&mut self, start: i32, end: i32) -> &mut Self {
        let start = start as usize;
        let end = end as usize;
        match &self.current_step {
            TraversalValue::NodeArray(nodes) => {
                if nodes.len() == 0 {
                    self.current_step = TraversalValue::Empty;
                } else if nodes.len() < end {
                    self.current_step =
                        TraversalValue::NodeArray(nodes[start..nodes.len() - 1].to_vec());
                } else {
                    self.current_step = TraversalValue::NodeArray(nodes[start..end].to_vec());
                }
            }
            TraversalValue::EdgeArray(edges) => {
                if edges.len() == 0 {
                    self.current_step = TraversalValue::Empty;
                } else if edges.len() < end {
                    self.current_step =
                        TraversalValue::EdgeArray(edges[start..edges.len() - 1].to_vec());
                } else {
                    self.current_step = TraversalValue::EdgeArray(edges[start..end].to_vec());
                }
            }
            TraversalValue::Empty => {}
            _ => panic!("Invalid traversal step for range {:?}", &self.current_step),
        }
        self
    }

    // Then modify the filter function
    fn filter_nodes<F>(&mut self, txn: &RoTxn, predicate: F) -> &mut Self
    where
        F: Fn(&Node) -> Result<bool, GraphError>,
    {
        if let TraversalValue::NodeArray(nodes) = &mut self.current_step {
            nodes.retain(|node| predicate(node).unwrap());
        }
        self
    }

    fn filter_edges<F>(&mut self, txn: &RoTxn, predicate: F) -> &mut Self
    where
        F: Fn(&Edge) -> Result<bool, GraphError>,
    {
        if let TraversalValue::EdgeArray(edges) = &mut self.current_step {
            edges.retain(|edge| predicate(edge).unwrap());
        }
        self
    }

    fn get_properties(&mut self, txn: &RoTxn, keys: &Vec<String>) -> &mut Self {
        match &mut self.current_step {
            TraversalValue::NodeArray(nodes) => {
                let mut new_props = Vec::with_capacity(nodes.len() * keys.len());
                for node in nodes {
                    let vals = keys
                        .iter()
                        .map(|key| {
                            if let Some(value) = node.check_property(key) {
                                (key.clone(), value.clone())
                            } else {
                                (key.clone(), Value::Empty)
                            }
                        })
                        .collect::<Vec<_>>();
                    new_props.extend(vals);
                }
                self.current_step = TraversalValue::ValueArray(new_props);
            }
            TraversalValue::EdgeArray(edges) => {
                let mut new_props = Vec::with_capacity(edges.len() * keys.len());
                for edge in edges {
                    let vals = keys
                        .iter()
                        .map(|key| {
                            if let Some(value) = edge.check_property(key) {
                                (key.clone(), value.clone())
                            } else {
                                (key.clone(), Value::Empty)
                            }
                        })
                        .collect::<Vec<_>>();
                    new_props.extend(vals);
                }
                self.current_step = TraversalValue::ValueArray(new_props);
            }
            _ => panic!("Invalid traversal step for get_properties"),
        }
        self
    }

    fn map_nodes<F>(&mut self, txn: &RoTxn, map_fn: F) -> &mut Self
    where
        F: Fn(&Node) -> Result<Node, GraphError>,
    {
        if let TraversalValue::NodeArray(nodes) = &mut self.current_step {
            let new_nodes = nodes.iter().map(|node| map_fn(node).unwrap()).collect();
            self.current_step = TraversalValue::NodeArray(new_nodes);
        }
        self
    }

    fn map_edges<F>(&mut self, txn: &RoTxn, map_fn: F) -> &mut Self
    where
        F: Fn(&Edge) -> Result<Edge, GraphError>,
    {
        if let TraversalValue::EdgeArray(edges) = &mut self.current_step {
            let new_edges = edges.iter().map(|edge| map_fn(edge).unwrap()).collect();
            self.current_step = TraversalValue::EdgeArray(new_edges);
        }
        self
    }

    fn for_each_node<F>(&mut self, txn: &RoTxn, mut map_fn: F) -> &mut Self
    where
        F: FnMut(&Node, &RoTxn) -> Result<(), GraphError>,
    {
        if let TraversalValue::NodeArray(nodes) = &self.current_step {
            for node in nodes {
                match map_fn(node, txn) {
                    Ok(_) => (),
                    Err(err) => panic!("Error in for_each_node: {:?}", err),
                }
            }
        }
        self
    }

    fn for_each_node_mut<F>(&mut self, txn: &mut RwTxn, mut map_fn: F) -> &mut Self
    where
        F: FnMut(&Node, &mut RwTxn) -> Result<(), GraphError>,
    {
        if let TraversalValue::NodeArray(nodes) = &self.current_step {
            for node in nodes {
                map_fn(node, txn);
            }
        }
        self
    }

    fn for_each_edge<F>(&mut self, txn: &RoTxn, map_fn: F) -> &mut Self
    where
        F: Fn(&Edge) -> Result<(), GraphError>,
    {
        if let TraversalValue::EdgeArray(edges) = &self.current_step {
            for edge in edges {
                map_fn(edge).unwrap();
            }
        }
        self
    }
}

impl TraversalSearchMethods for TraversalBuilder {
    fn shortest_path_between(&mut self, txn: &RoTxn, from_id: &str, to_id: &str) -> &mut Self {
        let s = Arc::clone(&self.storage);
        let paths = {
            match s.shortest_path(txn, from_id, to_id) {
                Ok(paths) => paths,
                Err(err) => {
                    // self.store_error(err);
                    (vec![], vec![])
                }
            }
        };
        let mut v: Vec<(Vec<Node>, Vec<Edge>)> = Vec::with_capacity(1);
        v.push(paths);
        let new_current = TraversalValue::Paths(v);

        self.current_step = new_current;

        self
    }

    fn shortest_path_to(&mut self, txn: &RoTxn, to_id: &str) -> &mut Self {
        let mut paths = Vec::with_capacity(24);
        let nodes = match &self.current_step {
            TraversalValue::NodeArray(nodes) => nodes.clone(),
            _ => {
                println!(
                    "Invalid traversal step for shortest_path_to {:?}",
                    &self.current_step
                );
                unreachable!();
            }
        };
        for node in nodes {
            match self.storage.shortest_path(txn, &node.id, to_id) {
                Ok(path) => paths.push(path),
                Err(e) => self.store_error(e),
            }
        }
        self.current_step = TraversalValue::Paths(paths);
        self
    }

    fn shortest_path_from(&mut self, txn: &RoTxn, from_id: &str) -> &mut Self {
        let mut paths = Vec::with_capacity(24);
        let nodes = match &self.current_step {
            TraversalValue::NodeArray(nodes) => nodes.clone(),
            _ => unreachable!(),
        };
        for node in nodes {
            match self.storage.shortest_path(txn, from_id, &node.id) {
                Ok(path) => paths.push(path),
                Err(e) => self.store_error(e),
            }
        }
        self.current_step = TraversalValue::Paths(paths);
        self
    }

    fn shortest_mutual_path_from(&mut self, txn: &RoTxn, from_id: &str) -> &mut Self {
        let s = Arc::clone(&self.storage);
        let mut e = GraphError::Empty;
        let nodes = match &self.current_step {
            TraversalValue::NodeArray(nodes) => nodes.clone(),
            _ => {
                e = GraphError::TraversalError(format!(
                    "Invalid traversal step for shortest_mutual_path_between {:?}",
                    &self.current_step
                ));
                unreachable!();
            }
        };

        let mut paths = Vec::with_capacity(24);

        for node in nodes {
            match s.shortest_mutual_path(txn, from_id, &node.id) {
                Ok(path) => {
                    paths.push(path);
                }
                Err(err) => {
                    e = err;
                }
            }
        }
        self.store_error(e);
        self.current_step = TraversalValue::Paths(paths);

        self
    }

    fn shortest_mutual_path_to(&mut self, txn: &RoTxn, to_id: &str) -> &mut Self {
        let s = Arc::clone(&self.storage);
        let mut e = GraphError::Empty;
        let nodes = match &self.current_step {
            TraversalValue::NodeArray(nodes) => nodes,
            _ => {
                e = GraphError::TraversalError(format!(
                    "Invalid traversal step for shortest_mutual_path_between {:?}",
                    &self.current_step
                ));
                unreachable!();
            }
        };

        let mut paths = Vec::with_capacity(24);

        for node in nodes {
            match s.shortest_mutual_path(txn, &node.id, to_id) {
                Ok(path) => {
                    paths.push(path);
                }
                Err(err) => {
                    e = err;
                }
            }
        }
        self.store_error(e);
        self.current_step = TraversalValue::Paths(paths);

        self
    }
}

pub trait TransactionCommit {
    fn maybe_commit(self) -> Result<(), Error>;
}

// Implementation for RoTxn - does nothing
impl<'a> TransactionCommit for RoTxn<'a, WithTls> {
    fn maybe_commit(self) -> Result<(), Error> {
        // Read-only transaction, nothing to commit
        Ok(())
    }
}

// Implementation for RwTxn - performs commit
impl<'a> TransactionCommit for RwTxn<'a> {
    fn maybe_commit(self) -> Result<(), Error> {
        self.commit()
    }
}

impl TraversalBuilderMethods for TraversalBuilder {
    fn result<T>(self, txn: T) -> Result<TraversalValue, GraphError>
    where
        T: TransactionCommit,
    {
        if let Some(err) = self.error {
            return Err(err);
        }

        // Will commit for RwTxn, do nothing for RoTxn
        txn.maybe_commit()
            .map_err(|e| GraphError::from(e.to_string()))?;

        Ok(self.current_step)
    }

    fn finish(self) -> Result<TraversalValue, GraphError> {
        if let Some(err) = self.error {
            return Err(err);
        }
        Ok(self.current_step)
    }

    fn execute(self) -> Result<(), GraphError> {
        if let Some(err) = self.error {
            return Err(err);
        }

        Ok(())
    }
}

impl VectorTraversalSteps for TraversalBuilder {
    fn vector_search(&mut self, txn: &RoTxn, query_vector: &[f64], k: usize) -> &mut Self {
        let result = match self.storage.vectors.search(txn, query_vector, k) {
            Ok(result) => result,
            Err(err) => {
                self.store_error(GraphError::from(err));
                return self;
            }
        };
        self.current_step = TraversalValue::VectorArray(result);
        self
    }

    fn insert_vector(&mut self, txn: &mut RwTxn, vector: &[f64]) -> &mut Self {
        self.storage.vectors.insert(txn, vector, None).unwrap();
        self
    }

    fn delete_vector(&mut self, txn: &mut RwTxn, vector_id: &str) -> &mut Self {
        self
    }

    fn update_vector(&mut self, txn: &mut RwTxn, vector_id: &str, vector: &[f64]) -> &mut Self {
        self
    }
}
