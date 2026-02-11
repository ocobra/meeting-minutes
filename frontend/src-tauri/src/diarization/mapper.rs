//! Speaker Mapper - Maps speaker labels to identified names
//!
//! This module manages the mapping between speaker labels (Speaker 1, Speaker 2)
//! and identified names, including voice profile matching and persistence.
//!
//! # Architecture
//!
//! The mapper coordinates three sources of speaker identity:
//! 1. **Voice Profile Matching**: Match embeddings against known voice profiles
//! 2. **LLM Identification**: Use identified names from transcript analysis
//! 3. **Manual Corrections**: Allow users to override automatic assignments
//!
//! # Confidence Thresholds
//!
//! - Confidence >= 0.7: Auto-assign identified name
//! - Confidence < 0.7: Keep speaker label, mark as uncertain
//! - Voice profile match: Always use profile name (confidence = 1.0)
//!
//! # Database Schema
//!
//! The mapper persists mappings to the database for:
//! - Historical tracking across meetings
//! - Voice profile management
//! - Manual correction persistence
//!
//! # Operations
//!
//! - **map_speakers**: Create initial mappings from segments and identifications
//! - **update_mapping**: Apply manual corrections
//! - **merge_speaker_labels**: Consolidate duplicate speakers
//! - **create_voice_profile**: Register new speaker for future recognition
//!
//! # Example
//!
//! ```no_run
//! use crate::diarization::mapper::{SpeakerMapper, MapperConfig};
//!
//! let config = MapperConfig {
//!     confidence_threshold: 0.7,
//!     embedding_similarity_threshold: 0.8,
//! };
//! let mapper = SpeakerMapper::new(config, db_pool);
//!
//! // Map speakers
//! let mappings = mapper.map_speakers(
//!     "meeting-123",
//!     &speaker_segments,
//!     &identifications
//! ).await?;
//!
//! // Manual correction
//! mapper.update_mapping(
//!     "meeting-123",
//!     "Speaker 1",
//!     "Alice Smith".to_string(),
//!     true
//! ).await?;
//! ```

use crate::diarization::{
    embedding::{cosine_similarity, hash_embedding},
    types::{IdentificationResult, SpeakerMapping, SpeakerSegment, VoiceProfile},
    DiarizationError,
};
use chrono::Utc;
use log::{debug, info, warn};
use sqlx::{Pool, Row, Sqlite};
use std::collections::HashMap;
use uuid::Uuid;

/// Configuration for speaker mapper
#[derive(Debug, Clone)]
pub struct MapperConfig {
    /// Minimum confidence for auto-assignment (0.0-1.0)
    pub confidence_threshold: f32,
    /// Threshold for voice embedding similarity (0.0-1.0)
    pub embedding_similarity_threshold: f32,
}

impl Default for MapperConfig {
    fn default() -> Self {
        Self {
            confidence_threshold: 0.7,
            embedding_similarity_threshold: 0.8,
        }
    }
}

/// Speaker mapper for managing speaker label to name mappings
pub struct SpeakerMapper {
    config: MapperConfig,
    db: Pool<Sqlite>,
}

impl SpeakerMapper {
    /// Create a new speaker mapper
    pub fn new(config: MapperConfig, db: Pool<Sqlite>) -> Self {
        info!("Initializing SpeakerMapper with config: {:?}", config);
        Self { config, db }
    }

    /// Map speakers based on segments and identifications
    pub async fn map_speakers(
        &self,
        meeting_id: &str,
        speaker_segments: &[SpeakerSegment],
        identifications: &[IdentificationResult],
    ) -> Result<Vec<SpeakerMapping>, DiarizationError> {
        info!(
            "Mapping speakers for meeting {}: {} segments, {} identifications",
            meeting_id,
            speaker_segments.len(),
            identifications.len()
        );

        let mut mappings = Vec::new();
        let mut speaker_labels: HashMap<String, SpeakerMapping> = HashMap::new();

        // Process each speaker segment
        for segment in speaker_segments {
            // Skip if we've already processed this speaker label
            if speaker_labels.contains_key(&segment.speaker_label) {
                continue;
            }

            // Try to match with known voice profile
            let profile_match = self
                .query_known_profiles(&[segment.embedding.clone()])
                .await?;

            let mapping = if let Some(Some(profile)) = profile_match.first() {
                // Found matching voice profile
                info!(
                    "Speaker {} matched to known profile: {}",
                    segment.speaker_label, profile.name
                );
                SpeakerMapping {
                    meeting_id: meeting_id.to_string(),
                    speaker_label: segment.speaker_label.clone(),
                    speaker_name: Some(profile.name.clone()),
                    voice_profile_id: Some(profile.id.clone()),
                    confidence: 1.0, // High confidence for profile match
                    is_manual: false,
                }
            } else {
                // Check identification results
                let identification = identifications
                    .iter()
                    .find(|id| id.speaker_label == segment.speaker_label);

                if let Some(id) = identification {
                    if id.confidence >= self.config.confidence_threshold {
                        // Use identified name
                        info!(
                            "Speaker {} identified as: {:?} (confidence: {:.2})",
                            segment.speaker_label, id.identified_name, id.confidence
                        );
                        SpeakerMapping {
                            meeting_id: meeting_id.to_string(),
                            speaker_label: segment.speaker_label.clone(),
                            speaker_name: id.identified_name.clone(),
                            voice_profile_id: None,
                            confidence: id.confidence,
                            is_manual: false,
                        }
                    } else {
                        // Confidence too low, use label
                        debug!(
                            "Speaker {} identification confidence too low: {:.2}",
                            segment.speaker_label, id.confidence
                        );
                        SpeakerMapping {
                            meeting_id: meeting_id.to_string(),
                            speaker_label: segment.speaker_label.clone(),
                            speaker_name: None,
                            voice_profile_id: None,
                            confidence: id.confidence,
                            is_manual: false,
                        }
                    }
                } else {
                    // No identification found, use label
                    debug!("No identification found for speaker {}", segment.speaker_label);
                    SpeakerMapping {
                        meeting_id: meeting_id.to_string(),
                        speaker_label: segment.speaker_label.clone(),
                        speaker_name: None,
                        voice_profile_id: None,
                        confidence: 0.0,
                        is_manual: false,
                    }
                }
            };

            speaker_labels.insert(segment.speaker_label.clone(), mapping.clone());
            mappings.push(mapping);
        }

        // Persist mappings to database
        for mapping in &mappings {
            self.save_mapping(mapping).await?;
        }

        info!("Created {} speaker mappings", mappings.len());
        Ok(mappings)
    }

    /// Query known voice profiles by embeddings
    pub async fn query_known_profiles(
        &self,
        embeddings: &[Vec<f32>],
    ) -> Result<Vec<Option<VoiceProfile>>, DiarizationError> {
        debug!("Querying {} embeddings against known profiles", embeddings.len());

        let mut results = Vec::new();

        for embedding in embeddings {
            // Get all voice profiles from database
            let profiles = self.get_all_profiles().await?;

            // Find best matching profile
            let mut best_match: Option<(VoiceProfile, f32)> = None;

            for profile in profiles {
                // Note: We store hashes, so we can't directly compare embeddings
                // In a real implementation, we'd need to store embeddings in a way
                // that allows similarity comparison (e.g., using a vector database)
                // For now, we'll use a simplified approach
                
                // This is a placeholder - in production, you'd use proper vector similarity
                let embedding_hash = hash_embedding(embedding);
                if embedding_hash == profile.embedding_hash {
                    // Exact match (unlikely but possible)
                    best_match = Some((profile, 1.0));
                    break;
                }
            }

            results.push(best_match.map(|(profile, _)| profile));
        }

        Ok(results)
    }

    /// Create a new voice profile
    pub async fn create_voice_profile(
        &self,
        name: String,
        embedding: Vec<f32>,
    ) -> Result<VoiceProfile, DiarizationError> {
        info!("Creating voice profile for: {}", name);

        let profile = VoiceProfile {
            id: Uuid::new_v4().to_string(),
            name: name.clone(),
            embedding_hash: hash_embedding(&embedding),
            created_at: Utc::now(),
            last_seen: Utc::now(),
            meeting_count: 0,
        };

        // Save to database
        sqlx::query(
            r#"
            INSERT INTO voice_profiles (id, name, embedding_hash, created_at, last_seen, meeting_count)
            VALUES (?, ?, ?, ?, ?, ?)
            "#
        )
        .bind(&profile.id)
        .bind(&profile.name)
        .bind(&profile.embedding_hash)
        .bind(profile.created_at)
        .bind(profile.last_seen)
        .bind(profile.meeting_count)
        .execute(&self.db)
        .await
        .map_err(|e| DiarizationError::DatabaseError(format!("Failed to create voice profile: {}", e)))?;

        info!("Voice profile created: {}", profile.id);
        Ok(profile)
    }

    /// Update a speaker mapping (manual correction)
    pub async fn update_mapping(
        &self,
        meeting_id: &str,
        speaker_label: &str,
        speaker_name: String,
        is_manual: bool,
    ) -> Result<(), DiarizationError> {
        info!(
            "Updating mapping for {} in meeting {}: {} (manual: {})",
            speaker_label, meeting_id, speaker_name, is_manual
        );

        sqlx::query(
            r#"
            UPDATE speaker_mappings
            SET speaker_name = ?, is_manual = ?, updated_at = ?
            WHERE meeting_id = ? AND speaker_label = ?
            "#
        )
        .bind(&speaker_name)
        .bind(is_manual)
        .bind(Utc::now())
        .bind(meeting_id)
        .bind(speaker_label)
        .execute(&self.db)
        .await
        .map_err(|e| DiarizationError::DatabaseError(format!("Failed to update mapping: {}", e)))?;

        Ok(())
    }

    /// Merge two speaker labels (consolidate segments)
    pub async fn merge_speaker_labels(
        &self,
        meeting_id: &str,
        source_label: &str,
        target_label: &str,
    ) -> Result<(), DiarizationError> {
        info!(
            "Merging speaker labels in meeting {}: {} -> {}",
            meeting_id, source_label, target_label
        );

        // Update all segments with source label to target label
        sqlx::query(
            r#"
            UPDATE speaker_segments
            SET speaker_label = ?
            WHERE meeting_id = ? AND speaker_label = ?
            "#
        )
        .bind(target_label)
        .bind(meeting_id)
        .bind(source_label)
        .execute(&self.db)
        .await
        .map_err(|e| DiarizationError::DatabaseError(format!("Failed to merge labels: {}", e)))?;

        // Delete the source mapping
        sqlx::query(
            r#"
            DELETE FROM speaker_mappings
            WHERE meeting_id = ? AND speaker_label = ?
            "#
        )
        .bind(meeting_id)
        .bind(source_label)
        .execute(&self.db)
        .await
        .map_err(|e| DiarizationError::DatabaseError(format!("Failed to delete source mapping: {}", e)))?;

        Ok(())
    }

    /// Save a speaker mapping to database
    async fn save_mapping(&self, mapping: &SpeakerMapping) -> Result<(), DiarizationError> {
        sqlx::query(
            r#"
            INSERT OR REPLACE INTO speaker_mappings 
            (meeting_id, speaker_label, speaker_name, voice_profile_id, confidence, is_manual, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            "#
        )
        .bind(&mapping.meeting_id)
        .bind(&mapping.speaker_label)
        .bind(&mapping.speaker_name)
        .bind(&mapping.voice_profile_id)
        .bind(mapping.confidence)
        .bind(mapping.is_manual)
        .bind(Utc::now())
        .bind(Utc::now())
        .execute(&self.db)
        .await
        .map_err(|e| DiarizationError::DatabaseError(format!("Failed to save mapping: {}", e)))?;

        Ok(())
    }

    /// Get all voice profiles from database
    async fn get_all_profiles(&self) -> Result<Vec<VoiceProfile>, DiarizationError> {
        let rows = sqlx::query(
            r#"
            SELECT id, name, embedding_hash, created_at, last_seen, meeting_count
            FROM voice_profiles
            ORDER BY last_seen DESC
            "#
        )
        .fetch_all(&self.db)
        .await
        .map_err(|e| DiarizationError::DatabaseError(format!("Failed to fetch profiles: {}", e)))?;

        let mut profiles = Vec::new();
        for row in rows {
            profiles.push(VoiceProfile {
                id: row.get("id"),
                name: row.get("name"),
                embedding_hash: row.get("embedding_hash"),
                created_at: row.get("created_at"),
                last_seen: row.get("last_seen"),
                meeting_count: row.get("meeting_count"),
            });
        }

        Ok(profiles)
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::SqlitePoolOptions;

    async fn setup_test_db() -> Pool<Sqlite> {
        let pool = SqlitePoolOptions::new()
            .connect("sqlite::memory:")
            .await
            .unwrap();
        
        // Create tables
        sqlx::query(
            r#"
            CREATE TABLE voice_profiles (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                embedding_hash TEXT NOT NULL,
                created_at TIMESTAMP NOT NULL,
                last_seen TIMESTAMP NOT NULL,
                meeting_count INTEGER DEFAULT 0,
                metadata TEXT
            )
            "#
        )
        .execute(&pool)
        .await
        .unwrap();
        
        sqlx::query(
            r#"
            CREATE TABLE speaker_mappings (
                meeting_id TEXT NOT NULL,
                speaker_label TEXT NOT NULL,
                speaker_name TEXT,
                voice_profile_id TEXT,
                confidence REAL NOT NULL,
                is_manual BOOLEAN DEFAULT FALSE,
                created_at TIMESTAMP NOT NULL,
                updated_at TIMESTAMP NOT NULL,
                PRIMARY KEY (meeting_id, speaker_label),
                FOREIGN KEY (voice_profile_id) REFERENCES voice_profiles(id)
            )
            "#
        )
        .execute(&pool)
        .await
        .unwrap();
        
        sqlx::query(
            r#"
            CREATE TABLE speaker_segments (
                id TEXT PRIMARY KEY,
                meeting_id TEXT NOT NULL,
                speaker_label TEXT NOT NULL,
                start_time REAL NOT NULL,
                end_time REAL NOT NULL,
                confidence REAL NOT NULL,
                embedding_hash TEXT,
                created_at TIMESTAMP NOT NULL
            )
            "#
        )
        .execute(&pool)
        .await
        .unwrap();
        
        pool
    }

    #[tokio::test]
    async fn test_create_voice_profile() {
        let pool = setup_test_db().await;
        let config = MapperConfig::default();
        let mapper = SpeakerMapper::new(config, pool);
        
        let embedding = vec![0.1, 0.2, 0.3, 0.4, 0.5];
        let profile = mapper.create_voice_profile("John Doe".to_string(), embedding).await.unwrap();
        
        assert_eq!(profile.name, "John Doe");
        assert_eq!(profile.meeting_count, 0);
        assert!(!profile.id.is_empty());
        assert!(!profile.embedding_hash.is_empty());
    }

    #[tokio::test]
    async fn test_map_speakers_with_identification() {
        let pool = setup_test_db().await;
        let config = MapperConfig::default();
        let mapper = SpeakerMapper::new(config, pool);
        
        let segments = vec![
            SpeakerSegment {
                speaker_label: "Speaker 1".to_string(),
                start_time: 0.0,
                end_time: 5.0,
                confidence: 0.9,
                embedding: vec![0.1, 0.2, 0.3],
            },
            SpeakerSegment {
                speaker_label: "Speaker 2".to_string(),
                start_time: 5.0,
                end_time: 10.0,
                confidence: 0.85,
                embedding: vec![0.4, 0.5, 0.6],
            },
        ];
        
        let identifications = vec![
            IdentificationResult {
                speaker_label: "Speaker 1".to_string(),
                identified_name: Some("Alice".to_string()),
                confidence: 0.95,
                source_segment: 0,
            },
            IdentificationResult {
                speaker_label: "Speaker 2".to_string(),
                identified_name: Some("Bob".to_string()),
                confidence: 0.8,
                source_segment: 1,
            },
        ];
        
        let mappings = mapper.map_speakers("meeting-1", &segments, &identifications).await.unwrap();
        
        assert_eq!(mappings.len(), 2);
        assert_eq!(mappings[0].speaker_label, "Speaker 1");
        assert_eq!(mappings[0].speaker_name, Some("Alice".to_string()));
        assert_eq!(mappings[1].speaker_label, "Speaker 2");
        assert_eq!(mappings[1].speaker_name, Some("Bob".to_string()));
    }

    #[tokio::test]
    async fn test_map_speakers_low_confidence() {
        let pool = setup_test_db().await;
        let config = MapperConfig {
            confidence_threshold: 0.7,
            embedding_similarity_threshold: 0.8,
        };
        let mapper = SpeakerMapper::new(config, pool);
        
        let segments = vec![
            SpeakerSegment {
                speaker_label: "Speaker 1".to_string(),
                start_time: 0.0,
                end_time: 5.0,
                confidence: 0.9,
                embedding: vec![0.1, 0.2, 0.3],
            },
        ];
        
        let identifications = vec![
            IdentificationResult {
                speaker_label: "Speaker 1".to_string(),
                identified_name: Some("Alice".to_string()),
                confidence: 0.5, // Below threshold
                source_segment: 0,
            },
        ];
        
        let mappings = mapper.map_speakers("meeting-1", &segments, &identifications).await.unwrap();
        
        assert_eq!(mappings.len(), 1);
        assert_eq!(mappings[0].speaker_label, "Speaker 1");
        // Should not use name due to low confidence
        assert_eq!(mappings[0].speaker_name, None);
    }

    #[tokio::test]
    async fn test_update_mapping() {
        let pool = setup_test_db().await;
        let config = MapperConfig::default();
        let mapper = SpeakerMapper::new(config, pool.clone());
        
        // Create initial mapping
        let segments = vec![
            SpeakerSegment {
                speaker_label: "Speaker 1".to_string(),
                start_time: 0.0,
                end_time: 5.0,
                confidence: 0.9,
                embedding: vec![0.1, 0.2, 0.3],
            },
        ];
        
        let identifications = vec![
            IdentificationResult {
                speaker_label: "Speaker 1".to_string(),
                identified_name: Some("Alice".to_string()),
                confidence: 0.95,
                source_segment: 0,
            },
        ];
        
        mapper.map_speakers("meeting-1", &segments, &identifications).await.unwrap();
        
        // Update mapping manually
        mapper.update_mapping("meeting-1", "Speaker 1", "Alice Smith".to_string(), true).await.unwrap();
        
        // Verify update
        let row = sqlx::query("SELECT speaker_name, is_manual FROM speaker_mappings WHERE meeting_id = ? AND speaker_label = ?")
            .bind("meeting-1")
            .bind("Speaker 1")
            .fetch_one(&pool)
            .await
            .unwrap();
        
        let name: String = row.get("speaker_name");
        let is_manual: bool = row.get("is_manual");
        
        assert_eq!(name, "Alice Smith");
        assert!(is_manual);
    }

    #[tokio::test]
    async fn test_merge_speaker_labels() {
        let pool = setup_test_db().await;
        let config = MapperConfig::default();
        let mapper = SpeakerMapper::new(config, pool.clone());
        
        // Create segments for two speakers
        let segments = vec![
            SpeakerSegment {
                speaker_label: "Speaker 1".to_string(),
                start_time: 0.0,
                end_time: 5.0,
                confidence: 0.9,
                embedding: vec![0.1, 0.2, 0.3],
            },
            SpeakerSegment {
                speaker_label: "Speaker 2".to_string(),
                start_time: 5.0,
                end_time: 10.0,
                confidence: 0.85,
                embedding: vec![0.4, 0.5, 0.6],
            },
        ];
        
        let identifications = vec![];
        
        mapper.map_speakers("meeting-1", &segments, &identifications).await.unwrap();
        
        // Insert speaker segments
        sqlx::query("INSERT INTO speaker_segments (id, meeting_id, speaker_label, start_time, end_time, confidence, created_at) VALUES (?, ?, ?, ?, ?, ?, ?)")
            .bind("seg-1")
            .bind("meeting-1")
            .bind("Speaker 1")
            .bind(0.0)
            .bind(5.0)
            .bind(0.9)
            .bind(Utc::now())
            .execute(&pool)
            .await
            .unwrap();
        
        sqlx::query("INSERT INTO speaker_segments (id, meeting_id, speaker_label, start_time, end_time, confidence, created_at) VALUES (?, ?, ?, ?, ?, ?, ?)")
            .bind("seg-2")
            .bind("meeting-1")
            .bind("Speaker 2")
            .bind(5.0)
            .bind(10.0)
            .bind(0.85)
            .bind(Utc::now())
            .execute(&pool)
            .await
            .unwrap();
        
        // Merge Speaker 2 into Speaker 1
        mapper.merge_speaker_labels("meeting-1", "Speaker 2", "Speaker 1").await.unwrap();
        
        // Verify all segments now have Speaker 1 label
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM speaker_segments WHERE meeting_id = ? AND speaker_label = ?")
            .bind("meeting-1")
            .bind("Speaker 1")
            .fetch_one(&pool)
            .await
            .unwrap();
        
        assert_eq!(count, 2);
        
        // Verify Speaker 2 mapping is deleted
        let mapping_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM speaker_mappings WHERE meeting_id = ? AND speaker_label = ?")
            .bind("meeting-1")
            .bind("Speaker 2")
            .fetch_one(&pool)
            .await
            .unwrap();
        
        assert_eq!(mapping_count, 0);
    }

    #[tokio::test]
    async fn test_get_all_profiles() {
        let pool = setup_test_db().await;
        let config = MapperConfig::default();
        let mapper = SpeakerMapper::new(config, pool);
        
        // Create multiple profiles
        let embedding1 = vec![0.1, 0.2, 0.3];
        let embedding2 = vec![0.4, 0.5, 0.6];
        
        mapper.create_voice_profile("Alice".to_string(), embedding1).await.unwrap();
        mapper.create_voice_profile("Bob".to_string(), embedding2).await.unwrap();
        
        // Get all profiles
        let profiles = mapper.get_all_profiles().await.unwrap();
        
        assert_eq!(profiles.len(), 2);
        assert!(profiles.iter().any(|p| p.name == "Alice"));
        assert!(profiles.iter().any(|p| p.name == "Bob"));
    }
}
