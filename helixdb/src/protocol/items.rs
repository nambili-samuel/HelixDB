use super::count::Count;
use serde::{
    de::{DeserializeSeed, VariantAccess, Visitor},
    Deserializer, Serializer,
};
use sonic_rs::{Deserialize, Serialize};
use std::{collections::HashMap, fmt, hash::Hash};
use super::value::{properties_format, Value};
use super::traversal_value::TraversalValue;



/// A node in the graph containing an ID, label, and property map.
/// Properties are serialised without enum variant names in JSON format.
#[derive(Clone, Serialize, Deserialize, PartialEq)]
pub struct Node {
    pub id: String,
    pub label: String,
    #[serde(with = "properties_format")]
    pub properties: HashMap<String, Value>,
}


impl Node {
    pub const NUM_PROPERTIES: usize = 2;
    pub fn new(label: &str, properties: Vec<(String, Value)>) -> Self {
        Self { id: "".to_string(), label: label.to_string(), properties: HashMap::from_iter(properties) }
    }
}


impl std::fmt::Display for Node {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{{ id: {}, label: {}, properties: {:?} }}",
            self.id, self.label, self.properties
        )
    }
}

impl std::fmt::Debug for Node {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{{ id: {}, label: {}, properties: {:?} }}",
            self.id, self.label, self.properties
        )
    }
}

/// An edge in the graph connecting two nodes with an ID, label, and property map.
/// Properties are serialised without enum variant names in JSON format.
#[derive(Serialize, Deserialize, Clone, PartialEq)]
pub struct Edge {
    pub id: String,
    pub label: String,
    pub from_node: String,
    pub to_node: String,
    #[serde(with = "properties_format")]
    pub properties: HashMap<String, Value>,
}

impl Edge {
    pub const NUM_PROPERTIES: usize = 4;
}

impl std::fmt::Display for Edge {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{{ id: {}, label: {}, from_node: {}, to_node: {}, properties: {:?} }}",
            self.id, self.label, self.from_node, self.to_node, self.properties
        )
    }
}

impl std::fmt::Debug for Edge {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{{ id: {}, label: {}, from_node: {}, to_node: {}, properties: {:?} }}",
            self.id, self.label, self.from_node, self.to_node, self.properties
        )
    }
}
