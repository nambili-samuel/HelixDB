use heed3::{RoTxn, RwTxn};
use crate::helix_engine::types::VectorError;
use crate::helix_engine::vector_core::vector::HVector;

pub trait HNSW {
    /// Search for the k nearest neighbors of a query vector
    ///
    /// # Arguments
    ///
    /// * `txn` - The transaction to use
    /// * `query` - The query vector
    /// * `k` - The number of nearest neighbors to search for
    ///
    /// # Returns
    ///
    /// A vector of tuples containing the id and distance of the nearest neighbors
    fn search(&self, txn: &RoTxn, query: &[f64], k: usize) -> Result<Vec<HVector>, VectorError>;

    /// Insert a new vector into the index
    ///
    /// # Arguments
    ///
    /// * `txn` - The transaction to use
    /// * `id` - The id of the vector
    /// * `data` - The vector data
    ///
    /// # Returns
    ///
    /// An HVector of the data inserted
    fn insert(&self, txn: &mut RwTxn, data: &[f64], nid: Option<String>) -> Result<HVector, VectorError>;

    /// Load a full hnsw index with all vectors at once
    ///
    /// # Arguments
    ///
    /// * `txn` - The transaction to use
    /// * `id` - The id of the vector
    /// * `data` - A Vec of all the vectors to insert
    ///
    /// # Returns
    ///
    /// An emtpy tuple
    fn load(&self, txn: &mut RwTxn, data: Vec<&[f64]>) -> Result<(), VectorError>;

    /// Get all vectors from the index
    ///
    /// # Arguments
    ///
    /// * `txn` - The read-only transaction to use for retrieving vectors
    ///
    /// # Returns
    ///
    /// A `Result` containing a `Vec` of `HVector` if successful
    fn get_all_vectors(&self, txn: &RoTxn) -> Result<Vec<HVector>, VectorError>;

    /// Get all vectors from the index at a specific level
    ///
    /// # Arguments
    ///
    /// * `txn` - The read-only transaction to use for retrieving vectors
    /// * `level` - A usize for which level to get all vectors from
    ///
    /// # Returns
    ///
    /// A `Result` containing a `Vec` of `HVector` if successful
    fn get_all_vectors_at_level(&self, txn: &RoTxn, level: usize) -> Result<Vec<HVector>, VectorError>;

    // Get the number of vectors in the hnsw index
    //
    // # Returns
    //
    // A `usize` of the number of vecs
    //fn get_num_of_vecs(&self) -> usize;
}