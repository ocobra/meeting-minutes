//! # Speaker Diarization and Identification Module
//!
//! This module implements speaker diarization (detecting who is speaking when)
//! and speaker identification (extracting speaker names from meeting introductions).
//!
//! ## Features
//!
//! - **Automatic Speaker Detection**: Segments audio by speaker using voice embeddings
//! - **Name Extraction**: Identifies speaker names from transcript introductions
//! - **Voice Profiles**: Stores and matches speakers across multiple meetings
//! - **Privacy-First**: Local processing with optional external model support
//! - **Graceful Degradation**: Transcripts available even when diarization fails
//!
//! ## Architecture
//!
//! The system consists of several interconnected components:
//!
//! 1. **Model Router**: Chooses between external/cloud and local models
//! 2. **Diarization Engine**: Segments audio by speaker using voice embeddings
//! 3. **Identification Service**: Extracts speaker names from transcripts using LLM
//! 4. **Speaker Mapper**: Maps speaker labels to identified names
//! 5. **Synchronization Layer**: Aligns diarization with transcription timestamps
//! 6. **Transcript Enhancer**: Formats transcripts with speaker information
//! 7. **Profile Manager**: Manages voice profile CRUD operations
//! 8. **Confidence Scorer**: Calculates and applies confidence thresholds
//! 9. **Error Recovery**: Handles errors with retry logic and fallback
//! 10. **Resource Monitor**: Monitors system resources for adaptive processing
//!
//! ## Usage Example
//!
//! ```rust,no_run
//! use meetily::diarization::{
//!     DiarizationConfig, ProcessingMode, PrivacyMode,
//!     ProfileManager, ProfileManagerConfig,
//! };
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Configure diarization
//! let config = DiarizationConfig {
//!     processing_mode: ProcessingMode::Batch,
//!     privacy_mode: PrivacyMode::PreferExternal,
//!     confidence_threshold: 0.7,
//!     ..Default::default()
//! };
//!
//! // Create voice profile manager
//! let db = sqlx::SqlitePool::connect("sqlite::memory:").await?;
//! let profile_config = ProfileManagerConfig::default();
//! let profile_manager = ProfileManager::new(db, profile_config);
//!
//! // Create a voice profile
//! let embedding = vec![0.1, 0.2, 0.3, 0.4];
//! let profile = profile_manager
//!     .create_profile("John Doe".to_string(), embedding, true)
//!     .await?;
//!
//! println!("Created profile: {}", profile.name);
//! # Ok(())
//! # }
//! ```
//!
//! ## Build Requirements
//!
//! **CRITICAL**: All builds MUST use the `vulkan` feature flag for GPU acceleration:
//!
//! ```bash
//! cargo build --features vulkan
//! cargo test --features vulkan
//! ```
//!
//! ## Privacy Modes
//!
//! - **LocalOnly**: Never use external models (maximum privacy)
//! - **PreferExternal**: Use external if available, fallback to local (recommended)
//! - **ExternalOnly**: Fail if external unavailable (maximum accuracy)
//!
//! ## Documentation
//!
//! - User Guide: See `USER_GUIDE.md` for end-user documentation
//! - Developer Guide: See `DEVELOPER_GUIDE.md` for architecture and API details
//! - Design Document: See `.kiro/specs/speaker-diarization-and-identification/design.md`
//!
//! ## Testing
//!
//! Run all tests with:
//! ```bash
//! cargo test --features vulkan --lib diarization
//! ```
//!
//! Current test coverage: 96 tests passing

pub mod types;
pub mod errors;
pub mod router;
pub mod engine;
pub mod identification;
pub mod mapper;
pub mod enhancer;
pub mod sync;
pub mod resource_monitor;
pub mod embedding;
pub mod export;
pub mod profile_manager;
pub mod confidence;
pub mod error_recovery;
pub mod commands;

// Re-export commonly used types
pub use types::{
    SpeakerSegment, DiarizationConfig, ProcessingMode, PrivacyMode,
    SpeakerMapping, VoiceProfile, EnhancedTranscript, SpeakerStatistics,
};
pub use errors::DiarizationError;
pub use embedding::{hash_embedding, cosine_similarity, cluster_embeddings};
pub use export::{export_transcript, ExportFormat};
pub use profile_manager::{ProfileManager, ProfileManagerConfig, EnrollmentSession};
pub use confidence::{ConfidenceScorer, ConfidenceConfig};
pub use error_recovery::{ErrorRecoveryCoordinator, ErrorRecoveryConfig};
