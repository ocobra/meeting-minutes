// Error types for speaker diarization and identification

use thiserror::Error;

/// Errors that can occur during speaker diarization and identification
#[derive(Debug, Error)]
pub enum DiarizationError {
    /// Audio processing errors
    #[error("Audio processing error: {0}")]
    AudioProcessingError(String),

    /// Model loading errors
    #[error("Model load error: {0}")]
    ModelLoadError(String),

    /// Speaker identification errors
    #[error("Identification error: {0}")]
    IdentificationError(String),

    /// Database errors
    #[error("Database error: {0}")]
    DatabaseError(String),

    /// Resource constraint errors
    #[error("Resource constraint error: {0}")]
    ResourceConstraintError(String),

    /// Resource monitoring errors
    #[error("Resource error: {0}")]
    ResourceError(String),

    /// Invalid configuration errors
    #[error("Invalid configuration: {0}")]
    InvalidConfiguration(String),

    /// Network/connectivity errors
    #[error("Network error: {0}")]
    NetworkError(String),

    /// External API errors
    #[error("External API error: {0}")]
    ExternalApiError(String),

    /// Embedding processing errors
    #[error("Embedding error: {0}")]
    EmbeddingError(String),

    /// Synchronization errors
    #[error("Synchronization error: {0}")]
    SynchronizationError(String),

    /// Export errors
    #[error("Export error: {0}")]
    ExportError(String),

    /// Consent required errors
    #[error("Consent required: {0}")]
    ConsentRequired(String),

    /// IO errors
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    /// Serialization errors
    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    /// Other errors
    #[error("Other error: {0}")]
    Other(String),
}

/// Result type for diarization operations
pub type Result<T> = std::result::Result<T, DiarizationError>;
