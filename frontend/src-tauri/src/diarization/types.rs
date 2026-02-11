// Core types for speaker diarization and identification

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// A continuous audio segment attributed to a single speaker
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpeakerSegment {
    /// Speaker label (e.g., "Speaker 1", "Speaker 2")
    pub speaker_label: String,
    /// Start timestamp in seconds
    pub start_time: f64,
    /// End timestamp in seconds
    pub end_time: f64,
    /// Confidence score (0.0-1.0)
    pub confidence: f32,
    /// Voice embedding vector (d-vector or x-vector)
    pub embedding: Vec<f32>,
}

/// Configuration for the diarization engine
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiarizationConfig {
    /// Minimum segment length in seconds
    pub min_segment_duration: f32,
    /// Sensitivity for speaker changes (0.0-1.0)
    pub speaker_change_threshold: f32,
    /// Embedding model name (e.g., "pyannote/embedding")
    pub embedding_model: String,
    /// Processing mode (real-time or batch)
    pub processing_mode: ProcessingMode,
    /// Privacy mode for model selection
    pub privacy_mode: PrivacyMode,
    /// Confidence threshold for name assignment (0.0-1.0)
    pub confidence_threshold: f32,
}

impl Default for DiarizationConfig {
    fn default() -> Self {
        Self {
            min_segment_duration: 1.0,
            speaker_change_threshold: 0.5,
            embedding_model: "pyannote/embedding".to_string(),
            processing_mode: ProcessingMode::Batch,
            privacy_mode: PrivacyMode::PreferExternal,
            confidence_threshold: 0.7,
        }
    }
}

/// Processing mode for diarization
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ProcessingMode {
    /// Real-time chunk-based processing for live meetings
    RealTime { chunk_size_ms: u32 },
    /// Batch processing for recorded meetings (higher accuracy)
    Batch,
}

impl Default for ProcessingMode {
    fn default() -> Self {
        Self::Batch
    }
}

/// Privacy mode for model selection
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum PrivacyMode {
    /// Never use external models (maximum privacy)
    LocalOnly,
    /// Use external models if available, fallback to local
    PreferExternal,
    /// Fail if external models unavailable
    ExternalOnly,
}

impl Default for PrivacyMode {
    fn default() -> Self {
        Self::PreferExternal
    }
}

/// Mapping between speaker label and identified name
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpeakerMapping {
    /// Meeting ID this mapping belongs to
    pub meeting_id: String,
    /// Speaker label (e.g., "Speaker 1")
    pub speaker_label: String,
    /// Identified speaker name (if found)
    pub speaker_name: Option<String>,
    /// Associated voice profile ID (if matched)
    pub voice_profile_id: Option<String>,
    /// Confidence score for this mapping (0.0-1.0)
    pub confidence: f32,
    /// Whether this mapping was manually set by user
    pub is_manual: bool,
}

/// Stored voice profile for known speakers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceProfile {
    /// Unique profile ID
    pub id: String,
    /// Speaker name
    pub name: String,
    /// SHA-256 hash of voice embedding (for privacy)
    pub embedding_hash: String,
    /// When this profile was created
    pub created_at: DateTime<Utc>,
    /// Last time this speaker was detected
    pub last_seen: DateTime<Utc>,
    /// Number of meetings this speaker participated in
    pub meeting_count: u32,
}

/// Enhanced transcript with speaker information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnhancedTranscript {
    /// Enhanced transcript segments
    pub segments: Vec<EnhancedSegment>,
    /// Speaker statistics for this meeting
    pub statistics: SpeakerStatistics,
}

/// A transcript segment with speaker information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnhancedSegment {
    /// Speaker name or label
    pub speaker_name: String,
    /// Transcript text
    pub text: String,
    /// Start timestamp in seconds
    pub start_time: f64,
    /// End timestamp in seconds
    pub end_time: f64,
    /// Confidence score (0.0-1.0)
    pub confidence: f32,
    /// Whether this segment has overlapping speech
    pub is_overlapping: bool,
}

/// Speaker statistics for a meeting
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpeakerStatistics {
    /// Per-speaker statistics
    pub speakers: Vec<SpeakerStats>,
    /// Total meeting duration in seconds
    pub total_duration: f64,
}

/// Statistics for a single speaker
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpeakerStats {
    /// Speaker name or label
    pub name: String,
    /// Total speaking time in seconds
    pub total_speaking_time: f64,
    /// Percentage of meeting time (0.0-100.0)
    pub percentage: f32,
    /// Number of speaking turns
    pub turn_count: u32,
    /// Average turn duration in seconds
    pub average_turn_duration: f64,
}

/// Transcript segment from transcription engine
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptSegment {
    /// Transcript text
    pub text: String,
    /// Start timestamp in seconds
    pub start_time: f64,
    /// End timestamp in seconds
    pub end_time: f64,
    /// Word-level timing information
    pub words: Vec<WordTiming>,
}

/// Word-level timing information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WordTiming {
    /// Word text
    pub word: String,
    /// Start timestamp in seconds
    pub start: f64,
    /// End timestamp in seconds
    pub end: f64,
    /// Confidence score (0.0-1.0)
    pub confidence: f32,
}

/// Result of speaker identification from LLM analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdentificationResult {
    /// Speaker label this identification is for
    pub speaker_label: String,
    /// Identified name (if found)
    pub identified_name: Option<String>,
    /// Confidence score (0.0-1.0)
    pub confidence: f32,
    /// Index of segment where name was found
    pub source_segment: usize,
}

/// Synchronized segment combining transcript and speaker info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SynchronizedSegment {
    /// Speaker label
    pub speaker_label: String,
    /// Speaker name (if identified)
    pub speaker_name: Option<String>,
    /// Transcript text
    pub text: String,
    /// Start timestamp in seconds
    pub start_time: f64,
    /// End timestamp in seconds
    pub end_time: f64,
    /// Word-level timing
    pub words: Vec<WordTiming>,
    /// Confidence score (0.0-1.0)
    pub confidence: f32,
}

/// Resource status for diarization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceStatus {
    /// Available memory in MB
    pub available_memory_mb: u64,
    /// Current CPU usage percentage (0.0-100.0)
    pub cpu_usage_percent: f32,
    /// Whether diarization can run with current resources
    pub can_run_diarization: bool,
    /// Recommended processing mode based on resources
    pub recommended_mode: Option<ProcessingMode>,
}

/// Resource usage estimate for diarization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceEstimate {
    /// Estimated memory usage in MB
    pub estimated_memory_mb: u64,
    /// Estimated CPU usage percentage (0.0-100.0)
    pub estimated_cpu_percent: f32,
    /// Estimated processing duration in seconds
    pub estimated_duration_seconds: f64,
}
