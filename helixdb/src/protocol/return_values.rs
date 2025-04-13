use super::count::Count;
use super::filterable::{Filterable, FilterableType};
use super::items::{Edge, Node};
use super::remapping::{Remapping, ResponseRemapping};
use super::traversal_value::TraversalValue;
use super::value::{properties_format, Value};
use serde::{
    de::{DeserializeSeed, VariantAccess, Visitor},
    Deserializer, Serializer,
};
use sonic_rs::{Deserialize, Serialize};
use std::cell::RefMut;
use std::{collections::HashMap, fmt};

/// A return value enum that represents different possible outputs from graph operations.
/// Can contain traversal results, counts, boolean flags, or empty values.
#[derive(Deserialize, Debug, Clone, PartialEq)]
pub enum ReturnValue {
    Array(Vec<ReturnValue>),
    Object(HashMap<String, ReturnValue>),
    Value(Value),
    Empty,
}

impl Serialize for ReturnValue {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        match self {
            ReturnValue::Value(value) => value.serialize(serializer),
            ReturnValue::Object(object) => object.serialize(serializer),
            ReturnValue::Array(array) => array.serialize(serializer),
            ReturnValue::Empty => serializer.serialize_none(),
        }
    }
}

impl From<Value> for ReturnValue {
    fn from(value: Value) -> Self {
        ReturnValue::Value(value)
    }
}

impl From<&Value> for ReturnValue {
    fn from(value: &Value) -> Self {
        ReturnValue::Value(value.clone())
    }
}

impl From<Count> for ReturnValue {
    fn from(count: Count) -> Self {
        ReturnValue::Value(Value::Integer(count.value() as i32))
    }
}

impl From<String> for ReturnValue {
    fn from(string: String) -> Self {
        ReturnValue::Value(Value::String(string))
    }
}

impl From<bool> for ReturnValue {
    fn from(boolean: bool) -> Self {
        ReturnValue::Value(Value::Boolean(boolean))
    }
}

impl From<&str> for ReturnValue {
    fn from(string: &str) -> Self {
        ReturnValue::Value(Value::String(string.to_string()))
    }
}

impl From<i32> for ReturnValue {
    fn from(integer: i32) -> Self {
        ReturnValue::Value(Value::Integer(integer))
    }
}

impl From<f64> for ReturnValue {
    fn from(float: f64) -> Self {
        ReturnValue::Value(Value::Float(float))
    }
}

impl<I> From<I> for ReturnValue
where
    for<'a> I: Filterable<'a> + Clone,
{
    #[inline]
    fn from(item: I) -> Self {
        let mut properties = match item.type_name() {
            FilterableType::Node => {
                HashMap::with_capacity(Node::NUM_PROPERTIES + item.properties_ref().len())
            }
            FilterableType::Edge => {
                let mut properties =
                    HashMap::with_capacity(Edge::NUM_PROPERTIES + item.properties_ref().len());
                properties.insert("from_node".to_string(), ReturnValue::from(item.from_node()));
                properties.insert("to_node".to_string(), ReturnValue::from(item.to_node()));
                properties
            }
            FilterableType::Vector => {
                let mut properties = item.clone().properties();
                let mut return_value = HashMap::new();
                let data = match properties.remove("data") {
                    Some(value) => value,
                    None => {
                        eprintln!("No data found in vector");
                        return ReturnValue::Empty;
                    }
                };
                return_value.insert("data".to_string(), ReturnValue::from(data));
                return_value
            }
        };
        properties.insert("id".to_string(), ReturnValue::from(item.id().to_string()));
        properties.insert(
            "label".to_string(),
            ReturnValue::from(item.label().to_string()),
        );
        properties.extend(
            item.properties()
                .into_iter()
                .map(|(k, v)| (k, ReturnValue::from(v))),
        );
        ReturnValue::Object(properties)
    }
}

impl Default for ReturnValue {
    fn default() -> Self {
        ReturnValue::Object(HashMap::new())
    }
}

impl ReturnValue {
    #[inline]
    #[allow(unused_attributes)]
    #[ignore = "No use for this function yet, however, I believe it may be useful in the future so I'm keeping it here"]
    pub fn from_properties(properties: HashMap<String, Value>) -> Self {
        ReturnValue::Object(
            properties
                .into_iter()
                .map(|(k, v)| (k, ReturnValue::Value(v)))
                .collect(),
        )
    }

    #[inline(always)]
    fn process_items_with_mixin<T>(
        items: Vec<T>,
        mut mixin: RefMut<HashMap<String, ResponseRemapping>>,
    ) -> ReturnValue
    where
        for<'a> T: Filterable<'a> + Clone,
    {
        ReturnValue::Array(
            items
                .into_iter()
                .map(|item| {
                    let id = item.id().to_string();
                    if let Some(m) = mixin.get_mut(&id) {
                        if m.should_spread {
                            ReturnValue::from(item).mixin_remapping(&mut m.remappings)
                        } else {
                            ReturnValue::default().mixin_remapping(&mut m.remappings)
                        }
                    } else {
                        ReturnValue::from(item)
                    }
                })
                .collect(),
        )
    }

    #[inline]
    pub fn from_traversal_value_array_with_mixin(
        traversal_value: TraversalValue,
        mixin: RefMut<HashMap<String, ResponseRemapping>>,
    ) -> Self {
        match traversal_value {
            TraversalValue::VectorArray(vectors) => {
                ReturnValue::process_items_with_mixin(vectors, mixin)
            }
            TraversalValue::NodeArray(nodes) => ReturnValue::process_items_with_mixin(nodes, mixin),
            TraversalValue::EdgeArray(edges) => ReturnValue::process_items_with_mixin(edges, mixin),
            TraversalValue::ValueArray(values) => ReturnValue::Empty,
            TraversalValue::Count(count) => ReturnValue::from(count),
            TraversalValue::Empty => ReturnValue::Empty,
            _ => {
                println!("not working");
                unreachable!()
            }
        }
    }

    #[inline(always)]
    #[allow(unused_attributes)]
    #[ignore = "No use for this function yet, however, I believe it may be useful in the future so I'm keeping it here"]
    pub fn mixin(self, other: ReturnValue) -> Self {
        match (self, other) {
            (ReturnValue::Object(mut a), ReturnValue::Object(b)) => {
                a.extend(b);
                ReturnValue::Object(a)
            }
            _ => unreachable!(),
        }
    }

    /// Mixin a remapping to a return value.
    ///
    /// This function takes a hashmap of `Remappings` and mixes them into the return value
    ///
    /// - If the mapping is an exclude, then the key is removed from the return value
    /// - If the mapping is a remapping from an old value to a new value, then the key
    ///     is replaced with the new name and the value is the new value
    /// - If the mapping is a new mapping, then the key is added to the return value
    ///     and the value is the new value
    /// - Otherwise, the key is left unchanged and the value is the original value
    ///
    /// Basic usage:
    ///
    /// ```rust
    /// use helixdb::protocol::{ReturnValue, Remapping};
    /// use std::collections::HashMap;
    ///
    /// let remappings = HashMap::new();
    /// remappings.insert(
    ///     "old_key".to_string(),
    ///     Remapping::new(
    ///         Some("new_key".to_string()),
    ///         ReturnValue::from("new_value".to_string())
    ///     )
    /// );
    ///
    /// let return_value = ReturnValue::from("old_value".to_string());
    /// let return_value = return_value.mixin_remapping(remappings);
    ///
    /// assert_eq!(
    ///     return_value.get("new_key".to_string()),
    ///     Some(&ReturnValue::from("new_value".to_string()))
    /// );
    /// ```
    #[inline(always)]
    pub fn mixin_remapping(self, remappings: &mut HashMap<String, Remapping>) -> Self {
        match self {
            ReturnValue::Object(mut a) => {
                remappings.into_iter().for_each(|(k, v)| {
                    if v.exclude {
                        let _ = a.remove(k);
                    } else if let Some(new_name) = &v.new_name {
                        if let Some(value) = a.remove(k) { 
                            a.insert(new_name.clone(), value);
                        }
                    } else {
                        a.insert(k.clone(), v.return_value.clone());
                    }
                });
                ReturnValue::Object(a)
            }
            _ => unreachable!(),
        }
    }

    #[inline(always)]
    #[allow(unused_attributes)]
    #[ignore = "No use for this function yet, however, I believe it may be useful in the future so I'm keeping it here"]
    pub fn mixin_other<I>(&self, item: I, mut secondary_properties: ResponseRemapping) -> Self
    where
        for<'a> I: Filterable<'a> + Clone,
    {
        let mut return_val = ReturnValue::default();
        if !secondary_properties.should_spread {
            match item.type_name() {
                FilterableType::Node => {
                    return_val = ReturnValue::from(item);
                }
                FilterableType::Edge => {
                    return_val = ReturnValue::from(item);
                }
                FilterableType::Vector => {
                    return_val = ReturnValue::from(item);
                }
            }
        }
        return_val = return_val.mixin_remapping(&mut secondary_properties.remappings);
        return_val
    }
}
