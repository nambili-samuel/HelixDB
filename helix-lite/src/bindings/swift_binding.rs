use std::sync::Arc;
use crate::HelixEmbedded;
use crate::GraphError;
use crate::HelixLiteError;
use crate::QueryInput;


pub struct Helix {
    db: HelixEmbedded,
}

impl Helix {
    pub fn new(path: String) -> Result<Self, HelixLiteError> {
        Ok(Self {
            db: HelixEmbedded::new(path)?
        })
    }

    pub fn query(&self, query_id: String, inputs: Vec<QueryInput>) -> Result<String, HelixLiteError> {
        self.db.query(query_id, inputs)
    }
}