use serde::{
    de::{DeserializeSeed, VariantAccess, Visitor},
    Deserializer, Serializer,
};
use sonic_rs::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::{collections::HashMap, fmt};

/// A flexible value type that can represent various property values in nodes and edges.
/// Handles both JSON and binary serialisation formats via custom implementaions of the Serialize and Deserialize traits.
#[derive(Clone, PartialEq, Debug)]
pub enum Value {
    String(String),
    Float(f64),
    Integer(i32),
    Boolean(bool),
    Array(Vec<Value>),
    Object(HashMap<String, Value>),
    Empty,
}

/// Custom serialisation implementation for Value that removes enum variant names in JSON
/// whilst preserving them for binary formats like bincode.
impl Serialize for Value {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if serializer.is_human_readable() {
            match self {
                Value::String(s) => s.serialize(serializer),
                Value::Float(f) => f.serialize(serializer),
                Value::Integer(i) => i.serialize(serializer),
                Value::Boolean(b) => b.serialize(serializer),
                Value::Array(arr) => {
                    use serde::ser::SerializeSeq;
                    let mut seq = serializer.serialize_seq(Some(arr.len()))?;
                    for value in arr {
                        seq.serialize_element(&value)?;
                    }
                    seq.end()
                }
                Value::Object(obj) => {
                    use serde::ser::SerializeMap;
                    let mut map = serializer.serialize_map(Some(obj.len()))?;
                    for (k, v) in obj {
                        map.serialize_entry(k, v)?;
                    }
                    map.end()
                }
                Value::Empty => serializer.serialize_none(),
            }
        } else {
            match self {
                Value::String(s) => serializer.serialize_newtype_variant("Value", 0, "String", s),
                Value::Float(f) => serializer.serialize_newtype_variant("Value", 1, "Float", f),
                Value::Integer(i) => serializer.serialize_newtype_variant("Value", 2, "Integer", i),
                Value::Boolean(b) => serializer.serialize_newtype_variant("Value", 3, "Boolean", b),
                Value::Array(a) => serializer.serialize_newtype_variant("Value", 4, "Array", a),
                Value::Object(obj) => serializer.serialize_newtype_variant("Value", 5, "Object", obj),
                Value::Empty => serializer.serialize_unit_variant("Value", 6, "Empty"),
            }
        }
    }
}

/// Custom deserialisation implementation for Value that handles both JSON and binary formats.
/// For JSON, parses raw values directly.
/// For binary formats like bincode, reconstructs the full enum structure.
impl<'de> Deserialize<'de> for Value {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        /// Visitor implementation that handles conversion of raw values into Value enum variants.
        /// Supports both direct value parsing for JSON and enum variant parsing for binary formats.
        struct ValueVisitor;

        impl<'de> Visitor<'de> for ValueVisitor {
            type Value = Value;

            #[inline]
            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a string, number, boolean, array, null, or Value enum")
            }

            #[inline]
            fn visit_str<E>(self, value: &str) -> Result<Value, E>
            where
                E: serde::de::Error,
            {
                Ok(Value::String(value.to_owned()))
            }

            #[inline]
            fn visit_string<E>(self, value: String) -> Result<Value, E>
            where
                E: serde::de::Error,
            {
                Ok(Value::String(value))
            }

            #[inline]
            fn visit_i32<E>(self, value: i32) -> Result<Value, E>
            where
                E: serde::de::Error,
            {
                Ok(Value::Integer(value))
            }

            #[inline]
            fn visit_f64<E>(self, value: f64) -> Result<Value, E>
            where
                E: serde::de::Error,
            {
                Ok(Value::Float(value))
            }

            #[inline]
            fn visit_bool<E>(self, value: bool) -> Result<Value, E>
            where
                E: serde::de::Error,
            {
                Ok(Value::Boolean(value))
            }

            #[inline]
            fn visit_none<E>(self) -> Result<Value, E>
            where
                E: serde::de::Error,
            {
                Ok(Value::Empty)
            }

            /// Handles array values by recursively deserialising each element
            fn visit_seq<A>(self, mut seq: A) -> Result<Value, A::Error>
            where
                A: serde::de::SeqAccess<'de>,
            {
                let mut values = Vec::new();
                while let Some(value) = seq.next_element()? {
                    values.push(value);
                }
                Ok(Value::Array(values))
            }

            /// Handles binary format deserialisation using numeric indices to identify variants
            /// Maps indices 0-5 to corresponding Value enum variants
            fn visit_enum<A>(self, data: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::EnumAccess<'de>,
            {
                let (variant_idx, variant_data) = data.variant_seed(VariantIdxDeserializer)?;
                match variant_idx {
                    0 => Ok(Value::String(variant_data.newtype_variant()?)),
                    1 => Ok(Value::Float(variant_data.newtype_variant()?)),
                    2 => Ok(Value::Integer(variant_data.newtype_variant()?)),
                    3 => Ok(Value::Boolean(variant_data.newtype_variant()?)),
                    4 => Ok(Value::Array(variant_data.newtype_variant()?)),
                    5 => {
                        variant_data.unit_variant()?;
                        Ok(Value::Empty)
                    }
                    _ => Err(serde::de::Error::invalid_value(
                        serde::de::Unexpected::Unsigned(variant_idx as u64),
                        &"variant index 0 through 5",
                    )),
                }
            }
        }

        /// Helper deserialiser for handling numeric variant indices in binary format
        struct VariantIdxDeserializer;

        impl<'de> DeserializeSeed<'de> for VariantIdxDeserializer {
            type Value = u32;
            #[inline]
            fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
            where
                D: Deserializer<'de>,
            {
                deserializer.deserialize_u32(self)
            }
        }

        impl<'de> Visitor<'de> for VariantIdxDeserializer {
            type Value = u32;

            #[inline]
            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("variant index")
            }

            #[inline]
            fn visit_u32<E>(self, v: u32) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(v)
            }
        }
        // Choose deserialisation strategy based on format
        if deserializer.is_human_readable() {
            // For JSON, accept any value type
            deserializer.deserialize_any(ValueVisitor)
        } else {
            // For binary, use enum variant indices
            deserializer.deserialize_enum(
                "Value",
                &["String", "Float", "Integer", "Boolean", "Array", "Empty"],
                ValueVisitor,
            )
        }
    }
}

/// Module for custom serialisation of property hashmaps
/// Ensures consistent handling of Value enum serialisation within property maps
pub mod properties_format {
    use super::*;

    #[inline]
    pub fn serialize<S>(
        properties: &HashMap<String, Value>,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(Some(properties.len()))?;
        for (k, v) in properties {
            map.serialize_entry(k, v)?;
        }
        map.end()
    }

    #[inline]
    pub fn deserialize<'de, D>(deserializer: D) -> Result<HashMap<String, Value>, D::Error>
    where
        D: Deserializer<'de>,
    {
        HashMap::deserialize(deserializer)
    }
}

impl From<&str> for Value {
    #[inline]
    fn from(s: &str) -> Self {
        Value::String(s.trim_matches('"').to_string())
    }
}

impl From<String> for Value {
    #[inline]
    fn from(s: String) -> Self {
        Value::String(s.trim_matches('"').to_string())
    }
}

impl From<i32> for Value {
    #[inline]
    fn from(i: i32) -> Self {
        Value::Integer(i)
    }
}

impl From<bool> for Value {
    #[inline]
    fn from(b: bool) -> Self {
        Value::Boolean(b)
    }
}

impl From<f64> for Value {
    #[inline]
    fn from(f: f64) -> Self {
        Value::Float(f)
    }
}

impl From<Vec<Value>> for Value {
    #[inline]
    fn from(v: Vec<Value>) -> Self {
        Value::Array(v)
    }
}

impl From<Value> for String {
    #[inline]
    fn from(v: Value) -> Self {
        match v {
            Value::String(s) => s,
            _ => panic!("Value is not a string"),
        }
    }
}
impl From<JsonValue> for Value {
    #[inline]
    fn from(v: JsonValue) -> Self {
        match v {
            JsonValue::String(s) => Value::String(s),
            JsonValue::Number(n) =>{
                if n.is_u64() {
                    Value::Integer(n.as_u64().unwrap() as i32)
                } else if n.is_i64() {
                    Value::Integer(n.as_i64().unwrap() as i32)
                } else {
                    Value::Float(n.as_f64().unwrap())
                }
            },
            JsonValue::Bool(b) => Value::Boolean(b),
            JsonValue::Array(a) => Value::Array(a.into_iter().map(|v| v.into()).collect()),
            JsonValue::Object(o) => Value::Object(o.into_iter().map(|(k, v)| (k, v.into())).collect()),
            JsonValue::Null => Value::Empty,
        }
    }
}