use std::collections::HashMap;

use crate::protocol::{
    items::{Edge, Node},
    value::Value,
};

#[derive(Debug, Clone)]
pub enum FilterableType {
    Node,
    Edge,
    Vector,
}

use super::return_values::ReturnValue;

/// Trait for types that can be filtered based on their properties.
/// Implemented by both Node and Edge types.
pub trait Filterable<'a> {
    fn type_name(&'a self) -> FilterableType;

    fn id(&'a self) -> &'a str;

    fn label(&'a self) -> &'a str;

    fn from_node(&'a self) -> String;
    fn to_node(&'a self) -> String;

    fn properties(self) -> HashMap<String, Value>;

    fn properties_mut(&'a mut self) -> &'a mut HashMap<String, Value>;

    fn properties_ref(&'a self) -> &'a HashMap<String, Value>;

    fn check_property(&'a self, key: &str) -> Option<&'a Value>;

    fn find_property(
        &'a self,
        key: &str,
        secondary_properties: &'a HashMap<String, ReturnValue>,
        property: &'a mut ReturnValue,
    ) -> Option<&'a ReturnValue>;
}

impl<'a> Filterable<'a> for Node {
    #[inline(always)]
    fn type_name(&'a self) -> FilterableType {
        FilterableType::Node
    }

    #[inline(always)]
    fn id(&'a self) -> &'a str {
        &self.id
    }

    #[inline(always)]
    fn label(&'a self) -> &'a str {
        &self.label
    }

    #[inline(always)]
    fn from_node(&'a self) -> String {
        unreachable!()
    }

    #[inline(always)]
    fn to_node(&'a self) -> String {
        unreachable!()
    }

    #[inline(always)]
    fn properties(self) -> HashMap<String, Value> {
        self.properties
    }

    #[inline(always)]
    fn properties_ref(&'a self) -> &'a HashMap<String, Value> {
        &self.properties
    }

    #[inline(always)]
    fn properties_mut(&'a mut self) -> &'a mut HashMap<String, Value> {
        &mut self.properties
    }

    #[inline(always)]
    fn check_property(&'a self, key: &str) -> Option<&'a Value> {
        self.properties.get(key)
    }

    #[inline(always)]
    fn find_property(
        &'a self,
        key: &str,
        secondary_properties: &'a HashMap<String, ReturnValue>,
        property: &'a mut ReturnValue,
    ) -> Option<&'a ReturnValue> {
        match self.properties.get(key) {
            Some(value) => {
                property.clone_from(&ReturnValue::Value(value.clone()));
                Some(property)
            }
            None => secondary_properties.get(key),
        }
    }
}

impl<'a> Filterable<'a> for Edge {
    #[inline(always)]
    fn type_name(&'a self) -> FilterableType {
        FilterableType::Edge
    }

    #[inline(always)]
    fn id(&'a self) -> &'a str {
        &self.id
    }

    #[inline(always)]
    fn label(&'a self) -> &'a str {
        &self.label
    }

    #[inline(always)]
    fn from_node(&'a self) -> String {
        self.from_node.clone()
    }

    #[inline(always)]
    fn to_node(&'a self) -> String {
        self.to_node.clone()
    }

    #[inline(always)]
    fn properties(self) -> HashMap<String, Value> {
        self.properties
    }

    #[inline(always)]
    fn properties_ref(&'a self) -> &'a HashMap<String, Value> {
        &self.properties
    }

    #[inline(always)]
    fn properties_mut(&'a mut self) -> &'a mut HashMap<String, Value> {
        &mut self.properties
    }

    #[inline(always)]
    fn check_property(&'a self, key: &str) -> Option<&'a Value> {
        self.properties.get(key)
    }

    #[inline(always)]
    fn find_property(
        &'a self,
        key: &str,
        secondary_properties: &'a HashMap<String, ReturnValue>,
        property: &'a mut ReturnValue,
    ) -> Option<&'a ReturnValue> {
        match self.properties.get(key) {
            Some(value) => {
                property.clone_from(&ReturnValue::Value(value.clone()));
                Some(property)
            }
            None => secondary_properties.get(key),
        }
    }
}
