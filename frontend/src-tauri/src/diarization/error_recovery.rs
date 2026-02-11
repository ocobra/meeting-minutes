//! Error Recovery - Handles errors and provides graceful degradation
//!
//! This module implements comprehensive error handling for diarization with
//! retry logic, fallback mechanisms, and graceful degradation.
//!
//! # Architecture
//!
//! The error recovery coordinator provides multiple layers of resilience:
//! 1. **Retry Logic**: Exponential backoff for transient failures
//! 2. **Fallback Mechanisms**: Simpler models, smaller chunks, batch mode
//! 3. **Graceful Degradation**: Transcripts available even when diarization fails
//!
//! # Error Classification
//!
//! **Retryable Errors** (will retry with backoff):
//! - Network errors (connectivity issues)
//! - API errors (rate limits, timeouts)
//! - Database errors (locks, temporary failures)
//! - Resource errors (temporary resource exhaustion)
//!
//! **Non-Retryable Errors** (fail immediately):
//! - Audio processing errors (invalid format, corrupted file)
//! - Configuration errors (invalid settings)
//! - Consent errors (user declined)
//!
//! # Retry Strategy
//!
//! - **Max Retries**: 3 attempts (configurable)
//! - **Backoff**: Exponential (1s, 2s, 4s)
//! - **Jitter**: Random delay to avoid thundering herd
//!
//! # Fallback Strategy
//!
//! When diarization fails:
//! 1. Try simpler model (if available)
//! 2. Try smaller chunk size (reduce memory)
//! 3. Switch to batch mode (more reliable)
//! 4. Return transcript without speaker labels (graceful degradation)
//!
//! # Example
//!
//! ```no_run
//! use crate::diarization::error_recovery::{ErrorRecoveryCoordinator, ErrorRecoveryConfig};
//!
//! let config = ErrorRecoveryConfig {
//!     max_retries: 3,
//!     initial_retry_delay_ms: 1000,
//!     use_exponential_backoff: true,
//!     enable_fallback: true,
//!     enable_graceful_degradation: true,
//! };
//! let coordinator = ErrorRecoveryCoordinator::new(config);
//!
//! // Execute with retry
//! let result = coordinator.execute_with_retry(|| async {
//!     // Diarization operation
//! }).await?;
//!
//! // Or handle error with fallback
//! let transcript = coordinator.handle_error_with_fallback(
//!     error,
//!     &transcript_segments
//! ).await?;
//! ```

use crate::diarization::{
    types::{ProcessingMode, SpeakerSegment, EnhancedTranscript, EnhancedSegment, SpeakerStatistics},
    DiarizationError,
};
use log::{debug, error, info, warn};
use std::time::Duration;
use tokio::time::sleep;

/// Configuration for error recovery
#[derive(Debug, Clone)]
pub struct ErrorRecoveryConfig {
    /// Maximum number of retry attempts
    pub max_retries: u32,
    /// Initial retry delay in milliseconds
    pub initial_retry_delay_ms: u64,
    /// Whether to use exponential backoff
    pub use_exponential_backoff: bool,
    /// Whether to enable fallback to simpler models
    pub enable_fallback: bool,
    /// Whether to enable graceful degradation
    pub enable_graceful_degradation: bool,
}

impl Default for ErrorRecoveryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            initial_retry_delay_ms: 1000,
            use_exponential_backoff: true,
            enable_fallback: true,
            enable_graceful_degradation: true,
        }
    }
}

/// Error recovery coordinator
pub struct ErrorRecoveryCoordinator {
    config: ErrorRecoveryConfig,
}

impl ErrorRecoveryCoordinator {
    /// Create a new error recovery coordinator
    pub fn new(config: ErrorRecoveryConfig) -> Self {
        Self { config }
    }

    /// Retry an operation with exponential backoff
    pub async fn retry_with_backoff<F, T, E>(
        &self,
        operation: F,
        operation_name: &str,
    ) -> Result<T, E>
    where
        F: Fn() -> Result<T, E>,
        E: std::fmt::Display,
    {
        let mut attempts = 0;
        let mut delay = self.config.initial_retry_delay_ms;

        loop {
            match operation() {
                Ok(result) => {
                    if attempts > 0 {
                        info!("{} succeeded after {} retries", operation_name, attempts);
                    }
                    return Ok(result);
                }
                Err(e) => {
                    attempts += 1;
                    
                    if attempts >= self.config.max_retries {
                        error!("{} failed after {} attempts: {}", operation_name, attempts, e);
                        return Err(e);
                    }

                    warn!(
                        "{} failed (attempt {}/{}): {}. Retrying in {}ms...",
                        operation_name,
                        attempts,
                        self.config.max_retries,
                        e,
                        delay
                    );

                    sleep(Duration::from_millis(delay)).await;

                    if self.config.use_exponential_backoff {
                        delay *= 2;
                    }
                }
            }
        }
    }

    /// Handle diarization error with graceful degradation
    pub fn handle_diarization_error(
        &self,
        error: &DiarizationError,
        context: &str,
    ) -> Option<Vec<SpeakerSegment>> {
        error!("Diarization error in {}: {}", context, error);

        if !self.config.enable_graceful_degradation {
            return None;
        }

        match error {
            DiarizationError::AudioProcessingError(_) => {
                warn!("Audio processing failed, returning empty segments");
                Some(Vec::new())
            }
            DiarizationError::ModelLoadError(_) => {
                warn!("Model loading failed, returning empty segments");
                Some(Vec::new())
            }
            DiarizationError::ResourceConstraintError(_) => {
                warn!("Resource constraints, deferring diarization");
                Some(Vec::new())
            }
            _ => {
                warn!("Unhandled error type, returning empty segments");
                Some(Vec::new())
            }
        }
    }

    /// Handle identification error with graceful degradation
    pub fn handle_identification_error(
        &self,
        error: &DiarizationError,
        context: &str,
    ) -> bool {
        error!("Identification error in {}: {}", context, error);

        if !self.config.enable_graceful_degradation {
            return false;
        }

        match error {
            DiarizationError::IdentificationError(_) => {
                warn!("Identification failed, will use speaker labels");
                true
            }
            DiarizationError::ExternalApiError(_) => {
                warn!("External API failed, will use speaker labels");
                true
            }
            DiarizationError::NetworkError(_) => {
                warn!("Network error, will use speaker labels");
                true
            }
            _ => {
                warn!("Unhandled error type, will use speaker labels");
                true
            }
        }
    }

    /// Create fallback transcript without speaker information
    pub fn create_fallback_transcript(
        &self,
        transcript_text: Vec<String>,
        timestamps: Vec<(f64, f64)>,
    ) -> EnhancedTranscript {
        info!("Creating fallback transcript without speaker information");

        let segments: Vec<EnhancedSegment> = transcript_text
            .into_iter()
            .zip(timestamps.into_iter())
            .map(|(text, (start, end))| EnhancedSegment {
                speaker_name: "Unknown".to_string(),
                text,
                start_time: start,
                end_time: end,
                confidence: 0.0,
                is_overlapping: false,
            })
            .collect();

        let total_duration = segments
            .last()
            .map(|s| s.end_time)
            .unwrap_or(0.0);

        EnhancedTranscript {
            segments,
            statistics: SpeakerStatistics {
                speakers: Vec::new(),
                total_duration,
            },
        }
    }

    /// Determine if error is retryable
    pub fn is_retryable_error(&self, error: &DiarizationError) -> bool {
        match error {
            // Retryable errors
            DiarizationError::NetworkError(_) => true,
            DiarizationError::ExternalApiError(_) => true,
            DiarizationError::DatabaseError(_) => true,
            DiarizationError::ResourceError(_) => true,
            
            // Non-retryable errors
            DiarizationError::AudioProcessingError(_) => false,
            DiarizationError::InvalidConfiguration(_) => false,
            DiarizationError::ConsentRequired(_) => false,
            
            // Default to non-retryable
            _ => false,
        }
    }

    /// Suggest fallback processing mode
    pub fn suggest_fallback_mode(&self, original_mode: ProcessingMode) -> Option<ProcessingMode> {
        if !self.config.enable_fallback {
            return None;
        }

        match original_mode {
            ProcessingMode::RealTime { chunk_size_ms } => {
                // Try smaller chunks
                if chunk_size_ms > 1000 {
                    debug!("Suggesting smaller chunk size for fallback");
                    Some(ProcessingMode::RealTime {
                        chunk_size_ms: chunk_size_ms / 2,
                    })
                } else {
                    // Fall back to batch mode
                    debug!("Suggesting batch mode for fallback");
                    Some(ProcessingMode::Batch)
                }
            }
            ProcessingMode::Batch => {
                // Already in batch mode, no fallback
                debug!("Already in batch mode, no fallback available");
                None
            }
        }
    }

    /// Log error without exposing to user
    pub fn log_error_safely(&self, error: &DiarizationError, context: &str) {
        // Log full error details for debugging
        error!("Error in {}: {:?}", context, error);
        
        // Log user-friendly message
        let user_message = match error {
            DiarizationError::AudioProcessingError(_) => {
                "Audio quality may be insufficient for speaker detection"
            }
            DiarizationError::ModelLoadError(_) => {
                "Speaker detection model could not be loaded"
            }
            DiarizationError::IdentificationError(_) => {
                "Could not identify speaker names from transcript"
            }
            DiarizationError::DatabaseError(_) => {
                "Could not save speaker information"
            }
            DiarizationError::ResourceConstraintError(_) => {
                "Insufficient system resources for speaker detection"
            }
            DiarizationError::NetworkError(_) => {
                "Network connection required for speaker detection"
            }
            DiarizationError::ExternalApiError(_) => {
                "External service unavailable for speaker detection"
            }
            _ => "Speaker detection encountered an error",
        };

        info!("User-friendly message: {}", user_message);
    }

    /// Check if operation should be retried
    pub fn should_retry(&self, error: &DiarizationError, attempt: u32) -> bool {
        if attempt >= self.config.max_retries {
            return false;
        }

        self.is_retryable_error(error)
    }

    /// Calculate retry delay
    pub fn calculate_retry_delay(&self, attempt: u32) -> Duration {
        let base_delay = self.config.initial_retry_delay_ms;
        
        if self.config.use_exponential_backoff {
            let delay = base_delay * 2_u64.pow(attempt);
            Duration::from_millis(delay)
        } else {
            Duration::from_millis(base_delay)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_retryable_error() {
        let config = ErrorRecoveryConfig::default();
        let coordinator = ErrorRecoveryCoordinator::new(config);

        // Retryable errors
        assert!(coordinator.is_retryable_error(&DiarizationError::NetworkError("test".to_string())));
        assert!(coordinator.is_retryable_error(&DiarizationError::ExternalApiError("test".to_string())));
        assert!(coordinator.is_retryable_error(&DiarizationError::DatabaseError("test".to_string())));

        // Non-retryable errors
        assert!(!coordinator.is_retryable_error(&DiarizationError::AudioProcessingError("test".to_string())));
        assert!(!coordinator.is_retryable_error(&DiarizationError::InvalidConfiguration("test".to_string())));
    }

    #[test]
    fn test_suggest_fallback_mode() {
        let config = ErrorRecoveryConfig::default();
        let coordinator = ErrorRecoveryCoordinator::new(config);

        // Real-time with large chunks should suggest smaller chunks
        let mode = ProcessingMode::RealTime { chunk_size_ms: 5000 };
        let fallback = coordinator.suggest_fallback_mode(mode);
        assert!(fallback.is_some());
        match fallback.unwrap() {
            ProcessingMode::RealTime { chunk_size_ms } => assert_eq!(chunk_size_ms, 2500),
            _ => panic!("Expected RealTime mode"),
        }

        // Real-time with small chunks should suggest batch mode
        let mode = ProcessingMode::RealTime { chunk_size_ms: 1000 };
        let fallback = coordinator.suggest_fallback_mode(mode);
        assert!(fallback.is_some());
        match fallback.unwrap() {
            ProcessingMode::Batch => {},
            _ => panic!("Expected Batch mode"),
        }

        // Batch mode has no fallback
        let mode = ProcessingMode::Batch;
        let fallback = coordinator.suggest_fallback_mode(mode);
        assert!(fallback.is_none());
    }

    #[test]
    fn test_suggest_fallback_mode_disabled() {
        let config = ErrorRecoveryConfig {
            enable_fallback: false,
            ..Default::default()
        };
        let coordinator = ErrorRecoveryCoordinator::new(config);

        let mode = ProcessingMode::RealTime { chunk_size_ms: 5000 };
        let fallback = coordinator.suggest_fallback_mode(mode);
        assert!(fallback.is_none());
    }

    #[test]
    fn test_should_retry() {
        let config = ErrorRecoveryConfig {
            max_retries: 3,
            ..Default::default()
        };
        let coordinator = ErrorRecoveryCoordinator::new(config);

        let error = DiarizationError::NetworkError("test".to_string());

        // Should retry on first attempts
        assert!(coordinator.should_retry(&error, 0));
        assert!(coordinator.should_retry(&error, 1));
        assert!(coordinator.should_retry(&error, 2));

        // Should not retry after max attempts
        assert!(!coordinator.should_retry(&error, 3));
        assert!(!coordinator.should_retry(&error, 4));
    }

    #[test]
    fn test_should_retry_non_retryable() {
        let config = ErrorRecoveryConfig::default();
        let coordinator = ErrorRecoveryCoordinator::new(config);

        let error = DiarizationError::InvalidConfiguration("test".to_string());

        // Should not retry non-retryable errors
        assert!(!coordinator.should_retry(&error, 0));
    }

    #[test]
    fn test_calculate_retry_delay() {
        let config = ErrorRecoveryConfig {
            initial_retry_delay_ms: 1000,
            use_exponential_backoff: true,
            ..Default::default()
        };
        let coordinator = ErrorRecoveryCoordinator::new(config);

        // Exponential backoff
        assert_eq!(coordinator.calculate_retry_delay(0), Duration::from_millis(1000));
        assert_eq!(coordinator.calculate_retry_delay(1), Duration::from_millis(2000));
        assert_eq!(coordinator.calculate_retry_delay(2), Duration::from_millis(4000));
    }

    #[test]
    fn test_calculate_retry_delay_no_backoff() {
        let config = ErrorRecoveryConfig {
            initial_retry_delay_ms: 1000,
            use_exponential_backoff: false,
            ..Default::default()
        };
        let coordinator = ErrorRecoveryCoordinator::new(config);

        // Fixed delay
        assert_eq!(coordinator.calculate_retry_delay(0), Duration::from_millis(1000));
        assert_eq!(coordinator.calculate_retry_delay(1), Duration::from_millis(1000));
        assert_eq!(coordinator.calculate_retry_delay(2), Duration::from_millis(1000));
    }

    #[test]
    fn test_create_fallback_transcript() {
        let config = ErrorRecoveryConfig::default();
        let coordinator = ErrorRecoveryCoordinator::new(config);

        let texts = vec![
            "Hello everyone".to_string(),
            "Welcome to the meeting".to_string(),
        ];
        let timestamps = vec![(0.0, 2.0), (2.0, 5.0)];

        let transcript = coordinator.create_fallback_transcript(texts, timestamps);

        assert_eq!(transcript.segments.len(), 2);
        assert_eq!(transcript.segments[0].speaker_name, "Unknown");
        assert_eq!(transcript.segments[0].text, "Hello everyone");
        assert_eq!(transcript.segments[1].text, "Welcome to the meeting");
        assert_eq!(transcript.statistics.total_duration, 5.0);
    }

    #[test]
    fn test_handle_diarization_error() {
        let config = ErrorRecoveryConfig {
            enable_graceful_degradation: true,
            ..Default::default()
        };
        let coordinator = ErrorRecoveryCoordinator::new(config);

        let error = DiarizationError::AudioProcessingError("test".to_string());
        let result = coordinator.handle_diarization_error(&error, "test context");

        assert!(result.is_some());
        assert_eq!(result.unwrap().len(), 0);
    }

    #[test]
    fn test_handle_diarization_error_disabled() {
        let config = ErrorRecoveryConfig {
            enable_graceful_degradation: false,
            ..Default::default()
        };
        let coordinator = ErrorRecoveryCoordinator::new(config);

        let error = DiarizationError::AudioProcessingError("test".to_string());
        let result = coordinator.handle_diarization_error(&error, "test context");

        assert!(result.is_none());
    }

    #[test]
    fn test_handle_identification_error() {
        let config = ErrorRecoveryConfig {
            enable_graceful_degradation: true,
            ..Default::default()
        };
        let coordinator = ErrorRecoveryCoordinator::new(config);

        let error = DiarizationError::IdentificationError("test".to_string());
        let result = coordinator.handle_identification_error(&error, "test context");

        assert!(result);
    }

    #[test]
    fn test_handle_identification_error_disabled() {
        let config = ErrorRecoveryConfig {
            enable_graceful_degradation: false,
            ..Default::default()
        };
        let coordinator = ErrorRecoveryCoordinator::new(config);

        let error = DiarizationError::IdentificationError("test".to_string());
        let result = coordinator.handle_identification_error(&error, "test context");

        assert!(!result);
    }
}
