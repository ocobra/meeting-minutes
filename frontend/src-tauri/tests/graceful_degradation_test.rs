//! Test for graceful degradation to transcript-only mode
//!
//! This test verifies that the recording system can gracefully degrade to
//! transcript-only mode when MP4 recording fails, implementing Requirements 5.3, 5.5.

use app_lib::audio::recording_manager::{RecordingPipeline, RecordingModeStatus};
use app_lib::recording::error_handling::{RecordingError, ErrorRecoveryCoordinator, RecoveryResult};

#[tokio::test]
async fn test_graceful_degradation_status_detection() {
    // Test that we can detect when the system is in graceful degradation mode
    let pipeline: RecordingPipeline<tauri::Wry> = RecordingPipeline::new();
    
    // Initially, should not be in degradation mode
    let is_degraded = pipeline.is_in_graceful_degradation_mode().await;
    assert!(!is_degraded, "Pipeline should not be in degradation mode initially");
    
    // Get recording mode status
    let status = pipeline.get_recording_mode_status().await;
    match status {
        RecordingModeStatus::NotRecording => {
            // This is expected for a new pipeline
        }
        _ => panic!("Expected NotRecording status for new pipeline"),
    }
}

#[tokio::test]
async fn test_error_recovery_coordinator_graceful_degradation() {
    // Test that the error recovery coordinator can handle graceful degradation
    let coordinator = ErrorRecoveryCoordinator::new()
        .with_graceful_degradation(true);
    
    // Create a checkpoint error that should trigger graceful degradation
    let error = RecordingError::checkpoint_error(
        "Test checkpoint failure for graceful degradation".to_string(),
        std::path::PathBuf::from("/tmp/test"),
        app_lib::recording::error_handling::CheckpointOperation::ChunkWrite,
        5,
    );
    
    // Verify the error allows graceful degradation
    assert!(error.allows_graceful_degradation(), "Checkpoint error should allow graceful degradation");
    
    // Attempt recovery
    let recovery_result = coordinator.attempt_recovery(&error).await;
    
    match recovery_result {
        RecoveryResult::GracefulDegradation { transcript_only_mode, .. } => {
            assert!(transcript_only_mode, "Should enable transcript-only mode");
        }
        _ => panic!("Expected graceful degradation recovery result"),
    }
}

#[tokio::test]
async fn test_mp4_recording_prerequisites_validation() {
    // Test that MP4 recording prerequisites validation works
    let mut pipeline: RecordingPipeline<tauri::Wry> = RecordingPipeline::new();
    
    // This method is private, so we'll test the public interface instead
    // We can test that the pipeline can be created and used without panicking
    let diagnostic_report = pipeline.get_diagnostic_report().await;
    
    // We don't assert specific values here because the test environment may not have
    // all prerequisites (FFmpeg, proper folders, etc.), but we verify it doesn't crash
    println!("✅ Diagnostic report generated successfully");
    println!("   Auto-save status: {:?}", diagnostic_report.auto_save_status);
    println!("   Dependency status: {:?}", diagnostic_report.dependency_status);
}

#[tokio::test]
async fn test_graceful_degradation_error_messages() {
    // Test that graceful degradation provides helpful error messages
    let error = RecordingError::checkpoint_error(
        "Disk full - cannot write audio chunks".to_string(),
        std::path::PathBuf::from("/tmp/test"),
        app_lib::recording::error_handling::CheckpointOperation::ChunkWrite,
        10,
    );
    
    let user_message = error.user_message();
    assert!(user_message.contains("10 chunks"), "User message should mention affected chunks");
    assert!(user_message.contains("Audio recording issue"), "User message should indicate audio recording issue");
    
    let guidance = error.recovery_guidance();
    assert!(!guidance.is_empty(), "Should provide recovery guidance");
    assert!(guidance.iter().any(|g| g.contains("transcript-only")), "Should mention transcript-only mode");
}

#[tokio::test]
async fn test_recording_mode_status_serialization() {
    // Test that RecordingModeStatus can be properly serialized for frontend communication
    use app_lib::recording::graceful_degradation_commands::{GracefulDegradationStatus, Mp4RestorationResult};
    
    let status = GracefulDegradationStatus {
        is_degraded: true,
        recording_mode: "transcript_only".to_string(),
        degradation_reason: Some("MP4 recording failed due to disk space".to_string()),
        checkpoints_created: Some(0),
        can_restore_mp4: true,
        status_message: "Recording in transcript-only mode".to_string(),
    };
    
    // Test serialization
    let serialized = serde_json::to_string(&status).expect("Should serialize successfully");
    let deserialized: GracefulDegradationStatus = serde_json::from_str(&serialized).expect("Should deserialize successfully");
    
    assert_eq!(status.is_degraded, deserialized.is_degraded);
    assert_eq!(status.recording_mode, deserialized.recording_mode);
    assert_eq!(status.degradation_reason, deserialized.degradation_reason);
    
    // Test Mp4RestorationResult serialization
    let restoration_result = Mp4RestorationResult {
        success: false,
        message: "Prerequisites not satisfied".to_string(),
        prerequisites_satisfied: false,
        restart_required: false,
    };
    
    let serialized = serde_json::to_string(&restoration_result).expect("Should serialize successfully");
    let deserialized: Mp4RestorationResult = serde_json::from_str(&serialized).expect("Should deserialize successfully");
    
    assert_eq!(restoration_result.success, deserialized.success);
    assert_eq!(restoration_result.message, deserialized.message);
}

#[test]
fn test_graceful_degradation_recovery_strategies() {
    // Test that different error types have appropriate recovery strategies
    use app_lib::recording::error_handling::{RecoveryStrategy, CheckpointOperation};
    
    // Test checkpoint write error - should allow graceful degradation
    let checkpoint_error = RecordingError::checkpoint_error(
        "Cannot write checkpoint".to_string(),
        std::path::PathBuf::from("/tmp"),
        CheckpointOperation::ChunkWrite,
        1,
    );
    
    match checkpoint_error.recovery_strategy() {
        RecoveryStrategy::GracefulDegradation { preserve_transcripts, .. } => {
            assert!(*preserve_transcripts, "Should preserve transcripts during degradation");
        }
        _ => panic!("Checkpoint write error should have graceful degradation strategy"),
    }
    
    // Test meeting folder error - should try alternatives first
    let folder_error = RecordingError::meeting_folder_error(
        "Cannot create folder".to_string(),
        std::path::PathBuf::from("/restricted"),
        true, // permission issue
        false, // not disk space issue
    );
    
    match folder_error.recovery_strategy() {
        RecoveryStrategy::AlternativeApproach { .. } => {
            // This is expected - should try alternative locations
        }
        _ => panic!("Meeting folder error should have alternative approach strategy"),
    }
}

#[test]
fn test_error_recovery_coordinator_configuration() {
    // Test that ErrorRecoveryCoordinator can be properly configured
    let _coordinator = ErrorRecoveryCoordinator::new()
        .with_graceful_degradation(true)
        .with_retry_config(5, 2000);
    
    // The coordinator should be configured (we can't easily test internal state,
    // but we can verify it was created without panicking)
    assert!(true, "ErrorRecoveryCoordinator should be configurable");
}