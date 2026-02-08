//! Property-based testing strategies for recording system
//!
//! This module provides custom proptest strategies for generating test data:
//! - Preference files (valid, corrupted, missing fields)
//! - Recording states (various configurations)
//! - Error conditions (filesystem, FFmpeg, permissions)
//!
//! These strategies are used across multiple property tests to ensure
//! comprehensive coverage of the recording system's behavior.

use proptest::prelude::*;
use serde_json::{json, Value};
use std::path::PathBuf;

/// Strategy for generating valid auto_save boolean values
pub fn auto_save_strategy() -> impl Strategy<Value = bool> {
    prop::bool::ANY
}

/// Strategy for generating meeting names (including edge cases)
pub fn meeting_name_strategy() -> impl Strategy<Value = String> {
    prop_oneof![
        // Normal names
        "[a-zA-Z0-9 ]{5,30}".prop_map(|s| s.trim().to_string()),
        // Names with special characters
        Just("Meeting/with/slashes".to_string()),
        Just("Meeting:with:colons".to_string()),
        Just("Meeting<with>brackets".to_string()),
        Just("Meeting|with|pipes".to_string()),
        // Edge cases
        Just("".to_string()),
        Just("   ".to_string()),
        Just("A".to_string()),
        Just("Very Long Meeting Name That Exceeds Normal Length Limits And Contains Many Words".to_string()),
    ]
}

/// Strategy for generating valid recording preferences JSON
pub fn valid_preferences_strategy() -> impl Strategy<Value = Value> {
    (auto_save_strategy(), meeting_name_strategy()).prop_map(|(auto_save, meeting_name)| {
        json!({
            "auto_save": auto_save,
            "meeting_name": meeting_name,
            "sample_rate": 48000,
            "channels": 2,
        })
    })
}

/// Strategy for generating corrupted preference files
pub fn corrupted_preferences_strategy() -> impl Strategy<Value = Value> {
    prop_oneof![
        // Missing auto_save field
        Just(json!({
            "meeting_name": "Test Meeting",
            "sample_rate": 48000,
        })),
        // Wrong type for auto_save
        Just(json!({
            "auto_save": "true",  // String instead of boolean
            "meeting_name": "Test Meeting",
        })),
        // Null auto_save
        Just(json!({
            "auto_save": null,
            "meeting_name": "Test Meeting",
        })),
        // Empty object
        Just(json!({})),
        // Invalid JSON structure
        Just(json!("not an object")),
    ]
}

/// Strategy for generating preference file states
pub fn preference_file_state_strategy() -> impl Strategy<Value = PreferenceFileState> {
    prop_oneof![
        valid_preferences_strategy().prop_map(PreferenceFileState::Valid),
        corrupted_preferences_strategy().prop_map(PreferenceFileState::Corrupted),
        Just(PreferenceFileState::Missing),
        Just(PreferenceFileState::Unreadable),
    ]
}

/// Represents different states a preference file can be in
#[derive(Debug, Clone)]
pub enum PreferenceFileState {
    Valid(Value),
    Corrupted(Value),
    Missing,
    Unreadable,
}

/// Strategy for generating recording configurations
pub fn recording_config_strategy() -> impl Strategy<Value = RecordingConfig> {
    (
        auto_save_strategy(),
        meeting_name_strategy(),
        prop::option::of(1000u64..100000u64), // Optional sample_rate
    )
        .prop_map(|(auto_save, meeting_name, sample_rate)| RecordingConfig {
            auto_save,
            meeting_name,
            sample_rate: sample_rate.unwrap_or(48000),
        })
}

/// Configuration for a recording session
#[derive(Debug, Clone)]
pub struct RecordingConfig {
    pub auto_save: bool,
    pub meeting_name: String,
    pub sample_rate: u64,
}

/// Strategy for generating filesystem error conditions
pub fn filesystem_error_strategy() -> impl Strategy<Value = FilesystemError> {
    prop_oneof![
        Just(FilesystemError::PermissionDenied),
        Just(FilesystemError::DiskFull),
        Just(FilesystemError::PathTooLong),
        Just(FilesystemError::InvalidPath),
        Just(FilesystemError::ReadOnly),
    ]
}

/// Types of filesystem errors that can occur
#[derive(Debug, Clone, PartialEq)]
pub enum FilesystemError {
    PermissionDenied,
    DiskFull,
    PathTooLong,
    InvalidPath,
    ReadOnly,
}

/// Strategy for generating FFmpeg error conditions
pub fn ffmpeg_error_strategy() -> impl Strategy<Value = FFmpegError> {
    prop_oneof![
        Just(FFmpegError::NotFound),
        Just(FFmpegError::InvalidVersion),
        Just(FFmpegError::ExecutionFailed),
        Just(FFmpegError::CodecNotSupported),
        Just(FFmpegError::OutputError),
    ]
}

/// Types of FFmpeg errors that can occur
#[derive(Debug, Clone, PartialEq)]
pub enum FFmpegError {
    NotFound,
    InvalidVersion,
    ExecutionFailed,
    CodecNotSupported,
    OutputError,
}

/// Strategy for generating audio chunk sizes (in bytes)
pub fn audio_chunk_size_strategy() -> impl Strategy<Value = usize> {
    prop_oneof![
        // Normal chunk sizes (1KB to 1MB)
        1024usize..1_048_576usize,
        // Edge cases
        Just(0),
        Just(1),
        Just(10_485_760), // 10MB - very large chunk
    ]
}

/// Strategy for generating sample rates
pub fn sample_rate_strategy() -> impl Strategy<Value = u32> {
    prop_oneof![
        // Common sample rates
        Just(8000),
        Just(16000),
        Just(22050),
        Just(44100),
        Just(48000),
        Just(96000),
        // Edge cases
        Just(1),
        Just(1000),
        Just(192000),
    ]
}

/// Strategy for generating directory paths (valid and invalid)
pub fn directory_path_strategy() -> impl Strategy<Value = PathBuf> {
    prop_oneof![
        // Valid paths
        "[a-zA-Z0-9_-]{1,20}".prop_map(|s| PathBuf::from(format!("/tmp/{}", s))),
        // Paths with special characters
        Just(PathBuf::from("/tmp/path with spaces")),
        Just(PathBuf::from("/tmp/path/with/many/levels")),
        // Edge cases
        Just(PathBuf::from("")),
        Just(PathBuf::from("/")),
        Just(PathBuf::from("/tmp/very/long/path/that/exceeds/normal/limits/and/contains/many/directories")),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    proptest! {
        #[test]
        fn test_auto_save_strategy_generates_booleans(auto_save in auto_save_strategy()) {
            // Just verify it generates booleans
            assert!(auto_save == true || auto_save == false);
        }

        #[test]
        fn test_meeting_name_strategy_generates_strings(name in meeting_name_strategy()) {
            // Verify it generates strings (can be empty or contain special chars)
            assert!(name.len() <= 200); // Reasonable upper bound
        }

        #[test]
        fn test_valid_preferences_has_auto_save(prefs in valid_preferences_strategy()) {
            // Valid preferences should have auto_save field
            assert!(prefs.get("auto_save").is_some());
        }

        #[test]
        fn test_sample_rate_strategy_generates_valid_rates(rate in sample_rate_strategy()) {
            // Sample rates should be positive
            assert!(rate > 0);
        }

        #[test]
        fn test_audio_chunk_size_strategy_generates_sizes(size in audio_chunk_size_strategy()) {
            // Chunk sizes should be reasonable (0 to 10MB)
            assert!(size <= 10_485_760);
        }
    }
}
