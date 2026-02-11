//! Voice Profile Manager - Manages voice profiles for speaker identification
//!
//! This module handles CRUD operations for voice profiles with privacy compliance
//! and consent management.
//!
//! # Features
//!
//! - **Profile Creation**: Create voice profiles with consent workflow
//! - **Profile Retrieval**: Query profiles by ID or list all profiles
//! - **Profile Deletion**: Delete profiles with cascade cleanup
//! - **Auto-Deletion**: Automatic cleanup based on retention policy (default: 90 days)
//! - **Enrollment Tracking**: Track voice sample collection sessions
//!
//! # Privacy Compliance
//!
//! Voice profiles are biometric data under GDPR and CCPA. This module ensures:
//! - Explicit consent required before profile creation
//! - Embeddings stored as SHA-256 hashes only
//! - Automatic deletion after retention period
//! - Clear data retention policies
//! - User control over profile deletion
//!
//! # Enrollment Workflow
//!
//! 1. User provides consent
//! 2. Create enrollment session
//! 3. Collect voice samples (multiple recordings recommended)
//! 4. Create voice profile from averaged embeddings
//! 5. Link profile to enrollment session for audit trail
//!
//! # Example
//!
//! ```no_run
//! use crate::diarization::profile_manager::{ProfileManager, ProfileManagerConfig};
//!
//! let config = ProfileManagerConfig {
//!     auto_delete_after_days: Some(90),
//!     require_consent: true,
//! };
//! let manager = ProfileManager::new(config, db_pool);
//!
//! // Create profile (requires consent)
//! let profile = manager.create_profile(
//!     "Alice Smith".to_string(),
//!     embedding,
//!     true // consent_given
//! ).await?;
//!
//! // Query profiles
//! let profiles = manager.list_profiles().await?;
//!
//! // Auto-delete old profiles
//! manager.auto_delete_old_profiles().await?;
//! ```

use crate::diarization::{
    embedding::hash_embedding,
    types::{VoiceProfile},
    DiarizationError,
};
use chrono::{DateTime, Utc, Duration};
use sqlx::{Row, SqlitePool};
use uuid::Uuid;
use log::{debug, info};

/// Configuration for voice profile management
#[derive(Debug, Clone)]
pub struct ProfileManagerConfig {
    /// Auto-delete profiles after this many days (None = never delete)
    pub auto_delete_after_days: Option<u32>,
    /// Require explicit consent before creating profiles
    pub require_consent: bool,
}

impl Default for ProfileManagerConfig {
    fn default() -> Self {
        Self {
            auto_delete_after_days: Some(90), // 90 days default
            require_consent: true,
        }
    }
}

/// Enrollment session for tracking voice profile creation
#[derive(Debug, Clone)]
pub struct EnrollmentSession {
    pub id: String,
    pub voice_profile_id: String,
    pub audio_duration_seconds: f64,
    pub sample_count: u32,
    pub created_at: DateTime<Utc>,
}

/// Voice profile manager for CRUD operations
pub struct ProfileManager {
    db: SqlitePool,
    config: ProfileManagerConfig,
}

impl ProfileManager {
    /// Create a new profile manager
    pub fn new(db: SqlitePool, config: ProfileManagerConfig) -> Self {
        Self { db, config }
    }

    /// Create a new voice profile
    /// 
    /// This creates a voice profile with the given name and embedding.
    /// The embedding is stored as a SHA-256 hash for privacy.
    pub async fn create_profile(
        &self,
        name: String,
        embedding: Vec<f32>,
        consent_given: bool,
    ) -> Result<VoiceProfile, DiarizationError> {
        // Check consent requirement
        if self.config.require_consent && !consent_given {
            return Err(DiarizationError::ConsentRequired(
                "User consent required to create voice profile".to_string()
            ));
        }

        let id = Uuid::new_v4().to_string();
        let embedding_hash = hash_embedding(&embedding);
        let now = Utc::now();

        sqlx::query(
            "INSERT INTO voice_profiles (id, name, embedding_hash, created_at, last_seen, meeting_count)
             VALUES (?, ?, ?, ?, ?, ?)"
        )
        .bind(&id)
        .bind(&name)
        .bind(&embedding_hash)
        .bind(now)
        .bind(now)
        .bind(0)
        .execute(&self.db)
        .await
        .map_err(|e| DiarizationError::DatabaseError(format!("Failed to create profile: {}", e)))?;

        info!("Created voice profile: {} ({})", name, id);

        Ok(VoiceProfile {
            id,
            name,
            embedding_hash,
            created_at: now,
            last_seen: now,
            meeting_count: 0,
        })
    }

    /// Get a voice profile by ID
    pub async fn get_profile(&self, profile_id: &str) -> Result<Option<VoiceProfile>, DiarizationError> {
        let row = sqlx::query(
            "SELECT id, name, embedding_hash, created_at, last_seen, meeting_count
             FROM voice_profiles
             WHERE id = ?"
        )
        .bind(profile_id)
        .fetch_optional(&self.db)
        .await
        .map_err(|e| DiarizationError::DatabaseError(format!("Failed to get profile: {}", e)))?;

        Ok(row.map(|r| VoiceProfile {
            id: r.get("id"),
            name: r.get("name"),
            embedding_hash: r.get("embedding_hash"),
            created_at: r.get("created_at"),
            last_seen: r.get("last_seen"),
            meeting_count: r.get("meeting_count"),
        }))
    }

    /// Get all voice profiles
    pub async fn list_profiles(&self) -> Result<Vec<VoiceProfile>, DiarizationError> {
        let rows = sqlx::query(
            "SELECT id, name, embedding_hash, created_at, last_seen, meeting_count
             FROM voice_profiles
             ORDER BY last_seen DESC"
        )
        .fetch_all(&self.db)
        .await
        .map_err(|e| DiarizationError::DatabaseError(format!("Failed to list profiles: {}", e)))?;

        Ok(rows.into_iter().map(|r| VoiceProfile {
            id: r.get("id"),
            name: r.get("name"),
            embedding_hash: r.get("embedding_hash"),
            created_at: r.get("created_at"),
            last_seen: r.get("last_seen"),
            meeting_count: r.get("meeting_count"),
        }).collect())
    }

    /// Update profile last seen timestamp
    pub async fn update_last_seen(&self, profile_id: &str) -> Result<(), DiarizationError> {
        let now = Utc::now();
        
        sqlx::query(
            "UPDATE voice_profiles
             SET last_seen = ?, meeting_count = meeting_count + 1
             WHERE id = ?"
        )
        .bind(now)
        .bind(profile_id)
        .execute(&self.db)
        .await
        .map_err(|e| DiarizationError::DatabaseError(format!("Failed to update last seen: {}", e)))?;

        debug!("Updated last seen for profile: {}", profile_id);
        Ok(())
    }

    /// Delete a voice profile
    pub async fn delete_profile(&self, profile_id: &str) -> Result<(), DiarizationError> {
        // Delete enrollment sessions first (foreign key constraint)
        sqlx::query("DELETE FROM enrollment_sessions WHERE voice_profile_id = ?")
            .bind(profile_id)
            .execute(&self.db)
            .await
            .map_err(|e| DiarizationError::DatabaseError(format!("Failed to delete enrollment sessions: {}", e)))?;

        // Delete speaker mappings
        sqlx::query("DELETE FROM speaker_mappings WHERE voice_profile_id = ?")
            .bind(profile_id)
            .execute(&self.db)
            .await
            .map_err(|e| DiarizationError::DatabaseError(format!("Failed to delete speaker mappings: {}", e)))?;

        // Delete the profile
        sqlx::query("DELETE FROM voice_profiles WHERE id = ?")
            .bind(profile_id)
            .execute(&self.db)
            .await
            .map_err(|e| DiarizationError::DatabaseError(format!("Failed to delete profile: {}", e)))?;

        info!("Deleted voice profile: {}", profile_id);
        Ok(())
    }

    /// Delete all voice profiles
    pub async fn delete_all_profiles(&self) -> Result<u64, DiarizationError> {
        // Delete all enrollment sessions
        sqlx::query("DELETE FROM enrollment_sessions")
            .execute(&self.db)
            .await
            .map_err(|e| DiarizationError::DatabaseError(format!("Failed to delete enrollment sessions: {}", e)))?;

        // Delete all speaker mappings
        sqlx::query("DELETE FROM speaker_mappings")
            .execute(&self.db)
            .await
            .map_err(|e| DiarizationError::DatabaseError(format!("Failed to delete speaker mappings: {}", e)))?;

        // Delete all profiles
        let result = sqlx::query("DELETE FROM voice_profiles")
            .execute(&self.db)
            .await
            .map_err(|e| DiarizationError::DatabaseError(format!("Failed to delete profiles: {}", e)))?;

        let count = result.rows_affected();
        info!("Deleted {} voice profiles", count);
        Ok(count)
    }

    /// Auto-delete old profiles based on retention policy
    pub async fn auto_delete_old_profiles(&self) -> Result<u64, DiarizationError> {
        let days = match self.config.auto_delete_after_days {
            Some(d) => d,
            None => {
                debug!("Auto-deletion disabled");
                return Ok(0);
            }
        };

        let cutoff_date = Utc::now() - Duration::days(days as i64);

        // Find profiles to delete
        let rows = sqlx::query(
            "SELECT id FROM voice_profiles WHERE last_seen < ?"
        )
        .bind(cutoff_date)
        .fetch_all(&self.db)
        .await
        .map_err(|e| DiarizationError::DatabaseError(format!("Failed to find old profiles: {}", e)))?;

        let mut deleted_count = 0;
        for row in rows {
            let profile_id: String = row.get("id");
            if let Err(e) = self.delete_profile(&profile_id).await {
                log::error!("Failed to delete old profile {}: {}", profile_id, e);
            } else {
                deleted_count += 1;
            }
        }

        if deleted_count > 0 {
            info!("Auto-deleted {} old voice profiles (older than {} days)", deleted_count, days);
        }

        Ok(deleted_count)
    }

    /// Create an enrollment session
    pub async fn create_enrollment_session(
        &self,
        voice_profile_id: String,
        audio_duration_seconds: f64,
        sample_count: u32,
    ) -> Result<EnrollmentSession, DiarizationError> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now();

        sqlx::query(
            "INSERT INTO enrollment_sessions (id, voice_profile_id, audio_duration_seconds, sample_count, created_at)
             VALUES (?, ?, ?, ?, ?)"
        )
        .bind(&id)
        .bind(&voice_profile_id)
        .bind(audio_duration_seconds)
        .bind(sample_count)
        .bind(now)
        .execute(&self.db)
        .await
        .map_err(|e| DiarizationError::DatabaseError(format!("Failed to create enrollment session: {}", e)))?;

        debug!("Created enrollment session: {} for profile: {}", id, voice_profile_id);

        Ok(EnrollmentSession {
            id,
            voice_profile_id,
            audio_duration_seconds,
            sample_count,
            created_at: now,
        })
    }

    /// Get enrollment sessions for a profile
    pub async fn get_enrollment_sessions(
        &self,
        voice_profile_id: &str,
    ) -> Result<Vec<EnrollmentSession>, DiarizationError> {
        let rows = sqlx::query(
            "SELECT id, voice_profile_id, audio_duration_seconds, sample_count, created_at
             FROM enrollment_sessions
             WHERE voice_profile_id = ?
             ORDER BY created_at DESC"
        )
        .bind(voice_profile_id)
        .fetch_all(&self.db)
        .await
        .map_err(|e| DiarizationError::DatabaseError(format!("Failed to get enrollment sessions: {}", e)))?;

        Ok(rows.into_iter().map(|r| EnrollmentSession {
            id: r.get("id"),
            voice_profile_id: r.get("voice_profile_id"),
            audio_duration_seconds: r.get("audio_duration_seconds"),
            sample_count: r.get("sample_count"),
            created_at: r.get("created_at"),
        }).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::SqlitePoolOptions;

    async fn setup_test_db() -> SqlitePool {
        let pool = SqlitePoolOptions::new()
            .connect("sqlite::memory:")
            .await
            .expect("Failed to create test database");

        // Create tables
        sqlx::query(
            "CREATE TABLE voice_profiles (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                embedding_hash TEXT NOT NULL,
                created_at TIMESTAMP NOT NULL,
                last_seen TIMESTAMP NOT NULL,
                meeting_count INTEGER DEFAULT 0,
                metadata TEXT
            )"
        )
        .execute(&pool)
        .await
        .expect("Failed to create voice_profiles table");

        sqlx::query(
            "CREATE TABLE speaker_mappings (
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
            )"
        )
        .execute(&pool)
        .await
        .expect("Failed to create speaker_mappings table");

        sqlx::query(
            "CREATE TABLE enrollment_sessions (
                id TEXT PRIMARY KEY,
                voice_profile_id TEXT NOT NULL,
                audio_duration_seconds REAL NOT NULL,
                sample_count INTEGER NOT NULL,
                created_at TIMESTAMP NOT NULL,
                FOREIGN KEY (voice_profile_id) REFERENCES voice_profiles(id)
            )"
        )
        .execute(&pool)
        .await
        .expect("Failed to create enrollment_sessions table");

        pool
    }

    #[tokio::test]
    async fn test_create_profile() {
        let db = setup_test_db().await;
        let config = ProfileManagerConfig::default();
        let manager = ProfileManager::new(db, config);

        let embedding = vec![0.1, 0.2, 0.3, 0.4];
        let profile = manager.create_profile("John Doe".to_string(), embedding, true)
            .await
            .expect("Failed to create profile");

        assert_eq!(profile.name, "John Doe");
        assert_eq!(profile.meeting_count, 0);
        assert!(!profile.embedding_hash.is_empty());
    }

    #[tokio::test]
    async fn test_create_profile_without_consent() {
        let db = setup_test_db().await;
        let config = ProfileManagerConfig {
            require_consent: true,
            ..Default::default()
        };
        let manager = ProfileManager::new(db, config);

        let embedding = vec![0.1, 0.2, 0.3, 0.4];
        let result = manager.create_profile("John Doe".to_string(), embedding, false).await;

        assert!(result.is_err());
        match result {
            Err(DiarizationError::ConsentRequired(_)) => {},
            _ => panic!("Expected ConsentRequired error"),
        }
    }

    #[tokio::test]
    async fn test_get_profile() {
        let db = setup_test_db().await;
        let config = ProfileManagerConfig::default();
        let manager = ProfileManager::new(db, config);

        let embedding = vec![0.1, 0.2, 0.3, 0.4];
        let created = manager.create_profile("Jane Smith".to_string(), embedding, true)
            .await
            .expect("Failed to create profile");

        let retrieved = manager.get_profile(&created.id)
            .await
            .expect("Failed to get profile")
            .expect("Profile not found");

        assert_eq!(retrieved.id, created.id);
        assert_eq!(retrieved.name, "Jane Smith");
    }

    #[tokio::test]
    async fn test_list_profiles() {
        let db = setup_test_db().await;
        let config = ProfileManagerConfig::default();
        let manager = ProfileManager::new(db, config);

        let embedding1 = vec![0.1, 0.2, 0.3, 0.4];
        let embedding2 = vec![0.5, 0.6, 0.7, 0.8];

        manager.create_profile("Alice".to_string(), embedding1, true)
            .await
            .expect("Failed to create profile 1");
        manager.create_profile("Bob".to_string(), embedding2, true)
            .await
            .expect("Failed to create profile 2");

        let profiles = manager.list_profiles()
            .await
            .expect("Failed to list profiles");

        assert_eq!(profiles.len(), 2);
    }

    #[tokio::test]
    async fn test_update_last_seen() {
        let db = setup_test_db().await;
        let config = ProfileManagerConfig::default();
        let manager = ProfileManager::new(db, config);

        let embedding = vec![0.1, 0.2, 0.3, 0.4];
        let profile = manager.create_profile("Charlie".to_string(), embedding, true)
            .await
            .expect("Failed to create profile");

        let original_last_seen = profile.last_seen;
        let original_count = profile.meeting_count;

        // Wait a bit to ensure timestamp changes
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        manager.update_last_seen(&profile.id)
            .await
            .expect("Failed to update last seen");

        let updated = manager.get_profile(&profile.id)
            .await
            .expect("Failed to get profile")
            .expect("Profile not found");

        assert!(updated.last_seen > original_last_seen);
        assert_eq!(updated.meeting_count, original_count + 1);
    }

    #[tokio::test]
    async fn test_delete_profile() {
        let db = setup_test_db().await;
        let config = ProfileManagerConfig::default();
        let manager = ProfileManager::new(db, config);

        let embedding = vec![0.1, 0.2, 0.3, 0.4];
        let profile = manager.create_profile("David".to_string(), embedding, true)
            .await
            .expect("Failed to create profile");

        manager.delete_profile(&profile.id)
            .await
            .expect("Failed to delete profile");

        let retrieved = manager.get_profile(&profile.id)
            .await
            .expect("Failed to get profile");

        assert!(retrieved.is_none());
    }

    #[tokio::test]
    async fn test_delete_all_profiles() {
        let db = setup_test_db().await;
        let config = ProfileManagerConfig::default();
        let manager = ProfileManager::new(db, config);

        let embedding1 = vec![0.1, 0.2, 0.3, 0.4];
        let embedding2 = vec![0.5, 0.6, 0.7, 0.8];

        manager.create_profile("Eve".to_string(), embedding1, true)
            .await
            .expect("Failed to create profile 1");
        manager.create_profile("Frank".to_string(), embedding2, true)
            .await
            .expect("Failed to create profile 2");

        let count = manager.delete_all_profiles()
            .await
            .expect("Failed to delete all profiles");

        assert_eq!(count, 2);

        let profiles = manager.list_profiles()
            .await
            .expect("Failed to list profiles");

        assert_eq!(profiles.len(), 0);
    }

    #[tokio::test]
    async fn test_auto_delete_old_profiles() {
        let db = setup_test_db().await;
        let config = ProfileManagerConfig {
            auto_delete_after_days: Some(1), // 1 day
            ..Default::default()
        };
        let manager = ProfileManager::new(db, config);

        let embedding = vec![0.1, 0.2, 0.3, 0.4];
        let profile = manager.create_profile("Grace".to_string(), embedding, true)
            .await
            .expect("Failed to create profile");

        // Manually set last_seen to 2 days ago
        let two_days_ago = Utc::now() - Duration::days(2);
        sqlx::query("UPDATE voice_profiles SET last_seen = ? WHERE id = ?")
            .bind(two_days_ago)
            .bind(&profile.id)
            .execute(&manager.db)
            .await
            .expect("Failed to update last_seen");

        let deleted = manager.auto_delete_old_profiles()
            .await
            .expect("Failed to auto-delete");

        assert_eq!(deleted, 1);

        let retrieved = manager.get_profile(&profile.id)
            .await
            .expect("Failed to get profile");

        assert!(retrieved.is_none());
    }

    #[tokio::test]
    async fn test_create_enrollment_session() {
        let db = setup_test_db().await;
        let config = ProfileManagerConfig::default();
        let manager = ProfileManager::new(db, config);

        let embedding = vec![0.1, 0.2, 0.3, 0.4];
        let profile = manager.create_profile("Henry".to_string(), embedding, true)
            .await
            .expect("Failed to create profile");

        let session = manager.create_enrollment_session(
            profile.id.clone(),
            15.0,
            3,
        )
        .await
        .expect("Failed to create enrollment session");

        assert_eq!(session.voice_profile_id, profile.id);
        assert_eq!(session.audio_duration_seconds, 15.0);
        assert_eq!(session.sample_count, 3);
    }

    #[tokio::test]
    async fn test_get_enrollment_sessions() {
        let db = setup_test_db().await;
        let config = ProfileManagerConfig::default();
        let manager = ProfileManager::new(db, config);

        let embedding = vec![0.1, 0.2, 0.3, 0.4];
        let profile = manager.create_profile("Iris".to_string(), embedding, true)
            .await
            .expect("Failed to create profile");

        manager.create_enrollment_session(profile.id.clone(), 15.0, 3)
            .await
            .expect("Failed to create session 1");
        manager.create_enrollment_session(profile.id.clone(), 20.0, 4)
            .await
            .expect("Failed to create session 2");

        let sessions = manager.get_enrollment_sessions(&profile.id)
            .await
            .expect("Failed to get sessions");

        assert_eq!(sessions.len(), 2);
    }
}
