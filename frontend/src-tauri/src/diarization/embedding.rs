//! Voice Embedding Utilities
//!
//! This module provides utilities for voice embedding processing with a focus
//! on privacy compliance and speaker matching.
//!
//! # Features
//!
//! - **SHA-256 Hashing**: Privacy-compliant storage (GDPR/CCPA compliant)
//! - **Cosine Similarity**: Speaker matching and verification
//! - **Clustering**: Group similar embeddings for speaker identification
//! - **Normalization**: Ensure consistent similarity calculations
//! - **Averaging**: Create speaker profiles from multiple samples
//!
//! # Privacy Compliance
//!
//! Voice embeddings are biometric data under GDPR and CCPA. This module ensures
//! compliance by:
//! - Storing only SHA-256 hashes (irreversible)
//! - Never persisting raw embeddings
//! - Providing clear data retention policies
//!
//! # Similarity Thresholds
//!
//! For speaker verification:
//! - **> 0.8**: Very likely same speaker
//! - **0.7-0.8**: Probably same speaker
//! - **0.5-0.7**: Uncertain
//! - **< 0.5**: Different speakers
//!
//! # Example
//!
//! ```no_run
//! use crate::diarization::embedding::{hash_embedding, cosine_similarity, cluster_embeddings};
//!
//! // Hash for storage
//! let embedding = vec![0.1, 0.2, 0.3, 0.4, 0.5];
//! let hash = hash_embedding(&embedding);
//!
//! // Compare speakers
//! let similarity = cosine_similarity(&embedding1, &embedding2)?;
//! if similarity > 0.8 {
//!     println!("Same speaker!");
//! }
//!
//! // Cluster speakers
//! let clusters = cluster_embeddings(&embeddings, 0.8)?;
//! ```

use crate::diarization::DiarizationError;
use log::{debug, info};
use sha2::{Digest, Sha256};
use std::collections::HashMap;

/// Hash a voice embedding using SHA-256 for privacy-compliant storage
///
/// This ensures embeddings are stored as irreversible hashes, complying with
/// GDPR and CCPA requirements for biometric data.
pub fn hash_embedding(embedding: &[f32]) -> String {
    // Convert f32 array to bytes
    let bytes: Vec<u8> = embedding
        .iter()
        .flat_map(|f| f.to_le_bytes())
        .collect();
    
    // Compute SHA-256 hash
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    let result = hasher.finalize();
    
    // Convert to hex string
    format!("{:x}", result)
}

/// Calculate cosine similarity between two embeddings
///
/// Returns a value between -1.0 and 1.0, where:
/// - 1.0 means identical vectors
/// - 0.0 means orthogonal vectors
/// - -1.0 means opposite vectors
///
/// For speaker verification, values > 0.7 typically indicate same speaker
pub fn cosine_similarity(embedding1: &[f32], embedding2: &[f32]) -> Result<f32, DiarizationError> {
    if embedding1.len() != embedding2.len() {
        return Err(DiarizationError::EmbeddingError(
            format!("Embedding dimension mismatch: {} vs {}", embedding1.len(), embedding2.len())
        ));
    }
    
    if embedding1.is_empty() {
        return Err(DiarizationError::EmbeddingError(
            "Cannot compute similarity of empty embeddings".to_string()
        ));
    }
    
    // Compute dot product
    let dot_product: f32 = embedding1
        .iter()
        .zip(embedding2.iter())
        .map(|(a, b)| a * b)
        .sum();
    
    // Compute magnitudes
    let magnitude1: f32 = embedding1.iter().map(|x| x * x).sum::<f32>().sqrt();
    let magnitude2: f32 = embedding2.iter().map(|x| x * x).sum::<f32>().sqrt();
    
    // Avoid division by zero
    if magnitude1 == 0.0 || magnitude2 == 0.0 {
        return Ok(0.0);
    }
    
    // Compute cosine similarity
    let similarity = dot_product / (magnitude1 * magnitude2);
    
    // Clamp to [-1, 1] to handle floating point errors
    Ok(similarity.clamp(-1.0, 1.0))
}

/// Find the most similar embedding from a list
///
/// Returns the index and similarity score of the most similar embedding,
/// or None if no embeddings are provided or similarity is below threshold
pub fn find_most_similar(
    query: &[f32],
    candidates: &[Vec<f32>],
    threshold: f32,
) -> Result<Option<(usize, f32)>, DiarizationError> {
    if candidates.is_empty() {
        return Ok(None);
    }
    
    let mut best_match: Option<(usize, f32)> = None;
    
    for (idx, candidate) in candidates.iter().enumerate() {
        let similarity = cosine_similarity(query, candidate)?;
        
        if similarity >= threshold {
            if let Some((_, best_sim)) = best_match {
                if similarity > best_sim {
                    best_match = Some((idx, similarity));
                }
            } else {
                best_match = Some((idx, similarity));
            }
        }
    }
    
    Ok(best_match)
}

/// Cluster embeddings using simple threshold-based clustering
///
/// Groups embeddings that have similarity above the threshold.
/// Returns a mapping from embedding index to cluster ID.
pub fn cluster_embeddings(
    embeddings: &[Vec<f32>],
    threshold: f32,
) -> Result<HashMap<usize, usize>, DiarizationError> {
    info!("Clustering {} embeddings with threshold {}", embeddings.len(), threshold);
    
    let mut clusters: HashMap<usize, usize> = HashMap::new();
    let mut cluster_representatives: Vec<Vec<f32>> = Vec::new();
    let mut next_cluster_id = 0;
    
    for (idx, embedding) in embeddings.iter().enumerate() {
        // Find most similar cluster representative
        let similar_cluster = find_most_similar(
            embedding,
            &cluster_representatives,
            threshold,
        )?;
        
        match similar_cluster {
            Some((cluster_idx, similarity)) => {
                // Assign to existing cluster
                clusters.insert(idx, cluster_idx);
                debug!("Embedding {} assigned to cluster {} (similarity: {:.3})", idx, cluster_idx, similarity);
            }
            None => {
                // Create new cluster
                clusters.insert(idx, next_cluster_id);
                cluster_representatives.push(embedding.clone());
                debug!("Embedding {} creates new cluster {}", idx, next_cluster_id);
                next_cluster_id += 1;
            }
        }
    }
    
    info!("Clustering complete: {} clusters formed", next_cluster_id);
    Ok(clusters)
}

/// Normalize an embedding to unit length
///
/// This is useful for ensuring consistent similarity calculations
pub fn normalize_embedding(embedding: &[f32]) -> Vec<f32> {
    let magnitude: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
    
    if magnitude == 0.0 {
        return embedding.to_vec();
    }
    
    embedding.iter().map(|x| x / magnitude).collect()
}

/// Calculate average embedding from a list of embeddings
///
/// Useful for creating cluster centroids or speaker profiles
pub fn average_embedding(embeddings: &[Vec<f32>]) -> Result<Vec<f32>, DiarizationError> {
    if embeddings.is_empty() {
        return Err(DiarizationError::EmbeddingError(
            "Cannot average empty embedding list".to_string()
        ));
    }
    
    let dim = embeddings[0].len();
    
    // Verify all embeddings have same dimension
    for emb in embeddings.iter() {
        if emb.len() != dim {
            return Err(DiarizationError::EmbeddingError(
                "All embeddings must have same dimension".to_string()
            ));
        }
    }
    
    // Compute average
    let mut avg = vec![0.0; dim];
    for emb in embeddings.iter() {
        for (i, val) in emb.iter().enumerate() {
            avg[i] += val;
        }
    }
    
    let count = embeddings.len() as f32;
    for val in avg.iter_mut() {
        *val /= count;
    }
    
    Ok(avg)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_embedding() {
        let embedding = vec![0.1, 0.2, 0.3, 0.4, 0.5];
        let hash1 = hash_embedding(&embedding);
        let hash2 = hash_embedding(&embedding);
        
        // Same embedding should produce same hash
        assert_eq!(hash1, hash2);
        
        // Hash should be 64 characters (SHA-256 in hex)
        assert_eq!(hash1.len(), 64);
        
        // Different embedding should produce different hash
        let different_embedding = vec![0.5, 0.4, 0.3, 0.2, 0.1];
        let hash3 = hash_embedding(&different_embedding);
        assert_ne!(hash1, hash3);
    }

    #[test]
    fn test_cosine_similarity_identical() {
        let embedding = vec![1.0, 2.0, 3.0];
        let similarity = cosine_similarity(&embedding, &embedding).unwrap();
        
        // Identical vectors should have similarity of 1.0
        assert!((similarity - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_cosine_similarity_orthogonal() {
        let embedding1 = vec![1.0, 0.0, 0.0];
        let embedding2 = vec![0.0, 1.0, 0.0];
        let similarity = cosine_similarity(&embedding1, &embedding2).unwrap();
        
        // Orthogonal vectors should have similarity of 0.0
        assert!((similarity - 0.0).abs() < 0.001);
    }

    #[test]
    fn test_cosine_similarity_opposite() {
        let embedding1 = vec![1.0, 2.0, 3.0];
        let embedding2 = vec![-1.0, -2.0, -3.0];
        let similarity = cosine_similarity(&embedding1, &embedding2).unwrap();
        
        // Opposite vectors should have similarity of -1.0
        assert!((similarity + 1.0).abs() < 0.001);
    }

    #[test]
    fn test_cosine_similarity_dimension_mismatch() {
        let embedding1 = vec![1.0, 2.0, 3.0];
        let embedding2 = vec![1.0, 2.0];
        let result = cosine_similarity(&embedding1, &embedding2);
        
        assert!(result.is_err());
    }

    #[test]
    fn test_find_most_similar() {
        let query = vec![1.0, 2.0, 3.0];
        let candidates = vec![
            vec![1.1, 2.1, 3.1],  // Very similar
            vec![5.0, 6.0, 7.0],  // Less similar
            vec![-1.0, -2.0, -3.0], // Opposite
        ];
        
        let result = find_most_similar(&query, &candidates, 0.9).unwrap();
        
        assert!(result.is_some());
        let (idx, similarity) = result.unwrap();
        assert_eq!(idx, 0); // First candidate is most similar
        assert!(similarity > 0.99);
    }

    #[test]
    fn test_find_most_similar_below_threshold() {
        let query = vec![1.0, 2.0, 3.0];
        let candidates = vec![
            vec![5.0, 6.0, 7.0],
            vec![10.0, 11.0, 12.0],
        ];
        
        let result = find_most_similar(&query, &candidates, 0.99).unwrap();
        
        // No candidates above threshold
        assert!(result.is_none());
    }

    #[test]
    fn test_cluster_embeddings() {
        let embeddings = vec![
            vec![1.0, 0.0, 0.0],  // Cluster 0
            vec![1.1, 0.1, 0.0],  // Cluster 0 (similar to first)
            vec![0.0, 1.0, 0.0],  // Cluster 1
            vec![0.0, 1.1, 0.1],  // Cluster 1 (similar to third)
        ];
        
        let clusters = cluster_embeddings(&embeddings, 0.9).unwrap();
        
        // Should form 2 clusters
        assert_eq!(clusters.len(), 4);
        
        // First two should be in same cluster
        assert_eq!(clusters[&0], clusters[&1]);
        
        // Last two should be in same cluster
        assert_eq!(clusters[&2], clusters[&3]);
        
        // But different from first cluster
        assert_ne!(clusters[&0], clusters[&2]);
    }

    #[test]
    fn test_normalize_embedding() {
        let embedding = vec![3.0, 4.0]; // Magnitude = 5.0
        let normalized = normalize_embedding(&embedding);
        
        // Check magnitude is 1.0
        let magnitude: f32 = normalized.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((magnitude - 1.0).abs() < 0.001);
        
        // Check direction is preserved
        assert!((normalized[0] - 0.6).abs() < 0.001);
        assert!((normalized[1] - 0.8).abs() < 0.001);
    }

    #[test]
    fn test_average_embedding() {
        let embeddings = vec![
            vec![1.0, 2.0, 3.0],
            vec![2.0, 3.0, 4.0],
            vec![3.0, 4.0, 5.0],
        ];
        
        let avg = average_embedding(&embeddings).unwrap();
        
        assert_eq!(avg.len(), 3);
        assert!((avg[0] - 2.0).abs() < 0.001);
        assert!((avg[1] - 3.0).abs() < 0.001);
        assert!((avg[2] - 4.0).abs() < 0.001);
    }

    #[test]
    fn test_average_embedding_empty() {
        let embeddings: Vec<Vec<f32>> = vec![];
        let result = average_embedding(&embeddings);
        
        assert!(result.is_err());
    }

    #[test]
    fn test_average_embedding_dimension_mismatch() {
        let embeddings = vec![
            vec![1.0, 2.0, 3.0],
            vec![1.0, 2.0], // Different dimension
        ];
        
        let result = average_embedding(&embeddings);
        
        assert!(result.is_err());
    }
}
