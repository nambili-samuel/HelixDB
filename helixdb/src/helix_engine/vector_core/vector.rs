use crate::{helix_engine::types::VectorError, protocol::{filterable::{Filterable, FilterableType}, return_values::ReturnValue, value::Value}};
use serde::{Deserialize, Serialize};
use std::{cmp::Ordering, collections::HashMap};

#[repr(C, align(16))]
#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub struct HVector {
    id: String,
    pub is_deleted: bool,
    pub level: usize,
    pub distance: Option<f64>,
    data: Vec<f64>,
}

impl Eq for HVector {}

impl PartialOrd for HVector {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        other.distance.partial_cmp(&self.distance)
    }
}

impl Ord for HVector {
    fn cmp(&self, other: &Self) -> Ordering {
        self.partial_cmp(other).unwrap_or(Ordering::Equal)
    }
}

pub trait DistanceCalc {
    // TODO: make this cosine similarity
    fn distance(from: &HVector, to: &HVector) -> f64;
}

impl DistanceCalc for HVector {
    #[inline(always)]
    #[cfg(feature = "euclidean")]
    fn distance(from: &HVector, to: &HVector) -> f64 {
        if from.len() == to.len() {
            #[cfg(target_arch = "aarch64")]
            unsafe {
                return from.simd_distance_unchecked(to);
            }
            #[cfg(not(target_arch = "aarch64"))]
            return from.scalar_distance(to);
        }
        from.scalar_distance(to)
    }

    #[inline(always)]
    #[cfg(feature = "cosine")]
    fn distance(from: &HVector, to: &HVector) -> f64 {
        from.cosine_similarity(to)
    }
}

impl HVector {
    #[inline(always)]
    pub fn new(id: String, data: Vec<f64>) -> Self {
        HVector {
            id,
            is_deleted: false,
            level: 0,
            data,
            distance: None,
        }
    }

    #[inline(always)]
    pub fn from_slice(id: String, level: usize, data: Vec<f64>) -> Self {
        HVector {
            id,
            is_deleted: false,
            level,
            data,
            distance: None,
        }
    }

    #[inline(always)]
    pub fn get_data(&self) -> &[f64] {
        &self.data
    }

    #[inline(always)]
    pub fn get_id(&self) -> &str {
        &self.id
    }

    #[inline(always)]
    pub fn get_level(&self) -> usize {
        self.level
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let size = self.data.len() * std::mem::size_of::<f64>();
        let mut bytes = Vec::with_capacity(size);
        for &value in &self.data {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        bytes
    }

    pub fn from_bytes(id: String, level: usize, bytes: &[u8]) -> Result<Self, VectorError> {
        if bytes.len() % std::mem::size_of::<f64>() != 0 {
            return Err(VectorError::InvalidVectorData);
        }

        let mut data = Vec::with_capacity(bytes.len() / std::mem::size_of::<f64>());
        let chunks = bytes.chunks_exact(std::mem::size_of::<f64>());

        for chunk in chunks {
            let value = f64::from_le_bytes(chunk.try_into().unwrap());
            data.push(value);
        }

        Ok(HVector {
            id,
            is_deleted: false,
            level,
            data,
            distance: None,
        })
    }

    #[inline(always)]
    pub fn len(&self) -> usize {
        self.data.len()
    }

    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    #[inline(always)]
    pub fn distance_to(&self, other: &HVector) -> f64 {
        HVector::distance(self, other)
    }

    #[inline(always)]
    pub fn set_distance(&mut self, distance: f64) {
        self.distance = Some(distance);
    }

    #[inline(always)]
    pub fn get_distance(&self) -> f64 {
        match self.distance {
            Some(distance) => distance,
            None => panic!("Distance is not set for vector: {}", self.get_id()),
        }
    }

    #[cfg(target_arch = "aarch64")]
    #[inline(always)]
    unsafe fn simd_distance_unchecked(&self, other: &HVector) -> f64 {
        use std::arch::aarch64::{vaddvq_f64, vld1q_f64, vmulq_f64, vsubq_f64};

        let mut sum = 0.0;
        let n = self.len();
        let mut i = 0;

        while i + 2 <= n {
            let a = vld1q_f64(self.data[i..].as_ptr());
            let b = vld1q_f64(other.data[i..].as_ptr());
            let diff = vsubq_f64(a, b);
            let squared = vmulq_f64(diff, diff);
            sum += vaddvq_f64(squared);
            i += 2;
        }

        while i < n {
            let diff = self.data[i] - other.data[i];
            sum += diff * diff;
            i += 1;
        }

        sum.sqrt()
    }

    #[inline(always)]
    #[cfg(feature = "euclidean")]
    fn scalar_distance(&self, other: &HVector) -> f64 {
        self.data
            .iter()
            .zip(other.data.iter())
            .map(|(&a, &b)| (a - b).powi(2))
            .sum::<f64>()
            .sqrt()
    }

    #[inline(always)]
    #[cfg(feature = "cosine")]
    fn cosine_similarity(&self, other: &HVector) -> f64 {
        let len = self.data.len();
        debug_assert_eq!(len, other.data.len(), "Vectors must have the same length");

        #[cfg(target_feature = "avx2")]
        {
            return self.cosine_similarity_avx2(other);
        }

        let mut dot_product = 0.0;
        let mut magnitude_a = 0.0;
        let mut magnitude_b = 0.0;

        const CHUNK_SIZE: usize = 8;
        let chunks = len / CHUNK_SIZE;
        let remainder = len % CHUNK_SIZE;

        for i in 0..chunks {
            let offset = i * CHUNK_SIZE;
            let a_chunk = &self.data[offset..offset + CHUNK_SIZE];
            let b_chunk = &other.data[offset..offset + CHUNK_SIZE];

            let mut local_dot = 0.0;
            let mut local_mag_a = 0.0;
            let mut local_mag_b = 0.0;

            for j in 0..CHUNK_SIZE {
                let a_val = a_chunk[j];
                let b_val = b_chunk[j];
                local_dot += a_val * b_val;
                local_mag_a += a_val * a_val;
                local_mag_b += b_val * b_val;
            }

            dot_product += local_dot;
            magnitude_a += local_mag_a;
            magnitude_b += local_mag_b;
        }

        let remainder_offset = chunks * CHUNK_SIZE;
        for i in 0..remainder {
            let a_val = self.data[remainder_offset + i];
            let b_val = other.data[remainder_offset + i];
            dot_product += a_val * b_val;
            magnitude_a += a_val * a_val;
            magnitude_b += b_val * b_val;
        }

        dot_product / (magnitude_a.sqrt() * magnitude_b.sqrt())
    }

    // SIMD implementation using AVX2 (256-bit vectors)
    #[cfg(target_feature = "avx2")]
    #[inline(always)]
    fn cosine_similarity_avx2(a: &[f64], b: &[f64]) -> f64 {
        use std::arch::x86_64::*;

        let len = a.len();
        let chunks = len / 4; // AVX2 processes 4 f64 values at once

        unsafe {
            let mut dot_product = _mm256_setzero_pd();
            let mut magnitude_a = _mm256_setzero_pd();
            let mut magnitude_b = _mm256_setzero_pd();

            for i in 0..chunks {
                let offset = i * 4;

                // Load data - handle unaligned data
                let a_chunk = _mm256_loadu_pd(&a[offset]);
                let b_chunk = _mm256_loadu_pd(&b[offset]);

                // Calculate dot product and magnitudes in parallel
                dot_product = _mm256_add_pd(dot_product, _mm256_mul_pd(a_chunk, b_chunk));
                magnitude_a = _mm256_add_pd(magnitude_a, _mm256_mul_pd(a_chunk, a_chunk));
                magnitude_b = _mm256_add_pd(magnitude_b, _mm256_mul_pd(b_chunk, b_chunk));
            }

            // Horizontal sum of 4 doubles in each vector
            let dot_sum = horizontal_sum_pd(dot_product);
            let mag_a_sum = horizontal_sum_pd(magnitude_a);
            let mag_b_sum = horizontal_sum_pd(magnitude_b);

            // Handle remainder elements
            let mut dot_remainder = 0.0;
            let mut mag_a_remainder = 0.0;
            let mut mag_b_remainder = 0.0;

            let remainder_offset = chunks * 4;
            for i in remainder_offset..len {
                let a_val = a[i];
                let b_val = b[i];
                dot_remainder += a_val * b_val;
                mag_a_remainder += a_val * a_val;
                mag_b_remainder += b_val * b_val;
            }

            // Combine SIMD and scalar results
            let dot_product_total = dot_sum + dot_remainder;
            let magnitude_a_total = (mag_a_sum + mag_a_remainder).sqrt();
            let magnitude_b_total = (mag_b_sum + mag_b_remainder).sqrt();

            dot_product_total / (magnitude_a_total * magnitude_b_total)
        }
    }

    // Helper function to sum the 4 doubles in an AVX2 vector
    #[cfg(target_feature = "avx2")]
    #[inline(always)]
    unsafe fn horizontal_sum_pd(__v: __m256d) -> f64 {
        use std::arch::x86_64::*;

        // Extract the high 128 bits and add to the low 128 bits
        let sum_hi_lo = _mm_add_pd(_mm256_castpd256_pd128(__v), _mm256_extractf128_pd(__v, 1));

        // Add the high 64 bits to the low 64 bits
        let sum = _mm_add_sd(sum_hi_lo, _mm_unpackhi_pd(sum_hi_lo, sum_hi_lo));

        // Extract the low 64 bits as a scalar
        _mm_cvtsd_f64(sum)
    }
}

#[cfg(test)]
mod vector_tests {
    use super::*;

    #[test]
    fn test_hvector_new() {
        let data = vec![1.0, 2.0, 3.0];
        let vector = HVector::new("test".to_string(), data);
        assert_eq!(vector.get_data(), &[1.0, 2.0, 3.0]);
    }

    #[test]
    fn test_hvector_from_slice() {
        let data = [1.0, 2.0, 3.0];
        let vector = HVector::from_slice("test".to_string(), 0, data.to_vec());
        assert_eq!(vector.get_data(), &[1.0, 2.0, 3.0]);
    }

    #[test]
    fn test_hvector_distance() {
        let v1 = HVector::new("test".to_string(), vec![1.0, 0.0]);
        let v2 = HVector::new("test".to_string(), vec![0.0, 1.0]);
        let distance = HVector::distance(&v1, &v2);
        assert!((distance - 2.0_f64.sqrt()).abs() < 1e-10);
    }

    #[test]
    fn test_hvector_distance_zero() {
        let v1 = HVector::new("test".to_string(), vec![1.0, 2.0, 3.0]);
        let v2 = HVector::new("test".to_string(), vec![1.0, 2.0, 3.0]);
        let distance = HVector::distance(&v1, &v2);
        assert!(distance.abs() < 1e-10);
    }

    #[test]
    fn test_hvector_distance_to() {
        let v1 = HVector::new("test".to_string(), vec![0.0, 0.0]);
        let v2 = HVector::new("test".to_string(), vec![3.0, 4.0]);
        let distance = v1.distance_to(&v2);
        assert!((distance - 5.0).abs() < 1e-10);
    }

    #[test]
    fn test_bytes_roundtrip() {
        let original = HVector::new("test".to_string(), vec![1.0, 2.0, 3.0]);
        let bytes = original.to_bytes();
        let reconstructed = HVector::from_bytes("test".to_string(), 0, &bytes).unwrap();
        assert_eq!(original.get_data(), reconstructed.get_data());
    }

    #[test]
    fn test_hvector_len() {
        let data = vec![1.0, 2.0, 3.0, 4.0];
        let vector = HVector::new("test".to_string(), data);
        assert_eq!(vector.len(), 4);
    }

    #[test]
    fn test_hvector_is_empty() {
        let empty_vector = HVector::new("test".to_string(), vec![]);
        let non_empty_vector = HVector::new("test".to_string(), vec![1.0, 2.0]);

        assert!(empty_vector.is_empty());
        assert!(!non_empty_vector.is_empty());
    }

    #[test]
    fn test_hvector_distance_different_dimensions() {
        let v1 = HVector::new("test".to_string(), vec![1.0, 2.0, 3.0]);
        let v2 = HVector::new("test".to_string(), vec![1.0, 2.0, 3.0, 4.0]);
        let distance = HVector::distance(&v1, &v2);
        assert!(distance.is_finite());
    }

    #[test]
    fn test_hvector_large_values() {
        let v1 = HVector::new("test".to_string(), vec![1e6, 2e6]);
        let v2 = HVector::new("test".to_string(), vec![1e6, 2e6]);
        let distance = HVector::distance(&v1, &v2);
        assert!(distance.abs() < 1e-10);
    }

    #[test]
    fn test_hvector_negative_values() {
        let v1 = HVector::new("test".to_string(), vec![-1.0, -2.0]);
        let v2 = HVector::new("test".to_string(), vec![1.0, 2.0]);
        let distance = HVector::distance(&v1, &v2);
        assert!((distance - (20.0_f64).sqrt()).abs() < 1e-10);
    }

    #[test]
    fn test_hvector_cosine_similarity() {
        let v1 = HVector::new("test".to_string(), vec![1.0, 2.0, 3.0]);
        let v2 = HVector::new("test".to_string(), vec![4.0, 5.0, 6.0]);
        let similarity = v1.cosine_similarity(&v2);
        assert!((similarity - 0.9746318461970762).abs() < 1e-10);
    }
}


impl<'a> Filterable<'a> for HVector {
    fn type_name(&'a self) -> FilterableType {
        FilterableType::Vector
    }

    fn id(&self) -> &str {
        &self.id
    }

    fn label(&self) -> &str {
        "vector"
    }

    fn from_node(&self) -> String {
        "vector".to_string()
    }

    fn to_node(&self) -> String {
        "vector".to_string()
    }

    fn properties(self) -> HashMap<String, Value> {
        let mut properties = HashMap::new();
        properties.insert("data".to_string(), Value::Array(self.data.iter().map(|f| Value::Float(*f)).collect()));
        properties
    }

    fn properties_mut(&mut self) -> &mut HashMap<String, Value> {
        unreachable!()
    }

    fn properties_ref(&self) -> &HashMap<String, Value> {
        unreachable!()
    }

    fn check_property(&self, _key: &str) -> Option<&Value> {
        unreachable!()
    }

    fn find_property(&self, _key: &str, _secondary_properties: &HashMap<String, ReturnValue>, _property: &mut ReturnValue) -> Option<&ReturnValue> {
        unreachable!()
    }   
}
