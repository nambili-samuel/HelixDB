use napi_derive::napi;
use napi::bindgen_prelude::*;
use crate::{HelixEmbedded, QueryInput};
use napi::JsUnknown;

#[napi]
pub struct HelixJS {
    db: HelixEmbedded,
}

#[napi]
impl HelixJS {
    #[napi(constructor)]
    pub fn new(user: String) -> napi::Result<HelixJS> {
        match HelixEmbedded::new(user) {
            Ok(db) => Ok(Self { db }),
            Err(e) => Err(napi::Error::from_reason(e.to_string()))
        }
    }

    #[napi]
    pub fn query(&self, query: String, params: Array) -> napi::Result<String> {
        let mut query_inputs: Vec<QueryInput> = Vec::new();
        
        for i in 0..params.len() {
            let param = params.get::<JsUnknown>(i)?.unwrap();
            let value_type = param.get_type()?;
            
            let input = match value_type {
                ValueType::String => {
                    let s: String = param.coerce_to_string()?.into_utf8()?.as_str()?.to_string();
                    QueryInput::StringValue{ value: s}
                },
                ValueType::Number => {
                    let n = param.coerce_to_number()?.get_double()?;
                    if n.fract() == 0.0 && n >= (i32::MIN as f64) && n <= (i32::MAX as f64) {
                        QueryInput::IntegerValue{ value: n as i32 }
                    } else {
                        QueryInput::FloatValue{ value: n}
                    }
                },
                ValueType::Boolean => {
                    let b = param.coerce_to_bool()?.get_value()?;
                    QueryInput::BooleanValue{ value: b }
                },
                _ => return Err(napi::Error::from_reason(format!(
                    "Unsupported parameter type: {:?}", value_type
                )))
            };
            
            query_inputs.push(QueryInput::from(input));
        }
        
        match self.db.query(query, query_inputs) {
            Ok(result) => Ok(result),
            Err(e) => Err(napi::Error::from_reason(e.to_string()))
        }
    }
}