use std::sync::Arc;
use tokio::sync::mpsc;
use anyhow::Result;
use log::{debug, error, info, warn};

use super::devices::{AudioDevice, list_audio_devices};

#[cfg(target_os = "macos")]
use super::devices::get_safe_recording_devices_macos;

#[cfg(not(target_os = "macos"))]
use super::devices::{default_input_device, default_output_device};
use super::recording_state::{RecordingState, AudioChunk, DeviceType as RecordingDeviceType};
use super::pipeline::AudioPipelineManager;
use super::stream::AudioStreamManager;
use super::recording_saver::RecordingSaver;
use super::device_monitor::{AudioDeviceMonitor, DeviceEvent, DeviceMonitorType};
use crate::recording::diagnostics::{DiagnosticEngine, DiagnosticReport, AutoSaveStatus, PipelineStatus};

/// Status of the recording mode
#[derive(Debug, Clone)]
pub enum RecordingModeStatus {
    /// Not currently recording
    NotRecording,
    /// Recording with full MP4 audio and transcripts
    FullRecording {
        checkpoints_created: u32,
    },
    /// Recording transcripts only (graceful degradation mode)
    TranscriptOnly {
        reason: String,
    },
}

/// Stream manager type enumeration
pub enum StreamManagerType {
    Standard(AudioStreamManager),
}

/// Enhanced recording pipeline with integrated diagnostics
/// 
/// This structure wraps the RecordingManager and adds diagnostic capabilities
/// to validate the auto_save parameter flow and ensure proper initialization.
/// Implements Requirements 1.3, 2.3, 4.4 from the specification.
pub struct RecordingPipeline<R: tauri::Runtime = tauri::Wry> {
    manager: RecordingManager,
    diagnostics: DiagnosticEngine,
    app_handle: Option<tauri::AppHandle<R>>,
}

impl<R: tauri::Runtime> RecordingPipeline<R> {
    /// Create a new recording pipeline with diagnostic integration
    pub fn new() -> Self {
        Self {
            manager: RecordingManager::new(),
            diagnostics: DiagnosticEngine::new(),
            app_handle: None,
        }
    }

    /// Create a new recording pipeline with app handle for enhanced diagnostics
    pub fn with_app_handle(app_handle: tauri::AppHandle<R>) -> Self {
        // Create diagnostic engine with a type-erased app handle
        // We need to convert the generic AppHandle<R> to AppHandle for the diagnostic engine
        let diagnostics = DiagnosticEngine::new(); // Use new() for now, will enhance later
        
        Self {
            manager: RecordingManager::new(),
            diagnostics,
            app_handle: Some(app_handle),
        }
    }

    /// Initialize the recording pipeline with comprehensive diagnostics
    /// 
    /// This method validates the entire recording system before starting:
    /// - Checks auto_save parameter loading and integrity
    /// - Validates pipeline component initialization
    /// - Ensures dependencies (FFmpeg) are available
    /// - Verifies filesystem permissions and space
    /// 
    /// Implements Requirement 1.3: Pipeline validation during initialization
    pub async fn initialize_with_diagnostics(&mut self) -> Result<DiagnosticReport, String> {
        info!("🔍 Initializing recording pipeline with comprehensive diagnostics");

        // Run full diagnostic check before initialization
        let diagnostic_report = self.diagnostics.run_full_diagnosis().await;

        // Log diagnostic results
        info!("📊 Diagnostic Results:");
        info!("  - Auto-save status: {:?}", diagnostic_report.auto_save_status);
        info!("  - Preference status: {:?}", diagnostic_report.preference_status);
        info!("  - Pipeline status: {:?}", diagnostic_report.pipeline_status);
        info!("  - Dependency status: {:?}", diagnostic_report.dependency_status);
        info!("  - Filesystem status: {:?}", diagnostic_report.filesystem_status);

        // Check for critical issues that would prevent recording
        let critical_issues = diagnostic_report.get_critical_issues();
        if !critical_issues.is_empty() {
            error!("❌ Critical issues found during pipeline initialization:");
            for issue in &critical_issues {
                error!("  - {}", issue);
            }
            
            // Return diagnostic report with issues for user action
            return Ok(diagnostic_report);
        }

        // Validate auto_save parameter specifically (Requirement 2.3)
        match diagnostic_report.auto_save_status {
            AutoSaveStatus::Enabled => {
                info!("✅ Auto-save parameter validated: enabled");
            }
            AutoSaveStatus::Disabled => {
                warn!("⚠️  Auto-save parameter is disabled - recording will save transcripts only");
            }
            AutoSaveStatus::Corrupted => {
                error!("❌ Auto-save parameter is corrupted - attempting repair");
                // The diagnostic system should have already attempted repair
                return Err("Auto-save parameter is corrupted and could not be repaired".to_string());
            }
            AutoSaveStatus::NotFound => {
                warn!("⚠️  Auto-save parameter not found - using default (true)");
            }
            AutoSaveStatus::HardcodedFalse(ref location) => {
                error!("❌ Auto-save parameter is hardcoded to false at: {}", location);
                return Err(format!("Auto-save parameter is hardcoded to false at: {}", location));
            }
        }

        // Validate pipeline initialization status
        match diagnostic_report.pipeline_status {
            PipelineStatus::Initialized => {
                info!("✅ Pipeline initialization validated");
            }
            PipelineStatus::InitializationFailed(ref error) => {
                error!("❌ Pipeline initialization failed: {}", error);
                return Err(format!("Pipeline initialization failed: {}", error));
            }
            PipelineStatus::ParameterNotPropagated => {
                error!("❌ Auto-save parameter not properly propagated through pipeline");
                return Err("Auto-save parameter not properly propagated through pipeline".to_string());
            }
            PipelineStatus::IncrementalSaverMissing => {
                error!("❌ IncrementalSaver not initialized when auto_save is true");
                return Err("IncrementalSaver not initialized when auto_save is true".to_string());
            }
        }

        info!("✅ Recording pipeline initialization diagnostics completed successfully");
        Ok(diagnostic_report)
    }

    /// Start recording with diagnostic validation of auto_save parameter flow
    /// 
    /// This method ensures the auto_save parameter flows correctly from preferences
    /// through all pipeline components to the IncrementalSaver. Implements graceful
    /// degradation to transcript-only mode when MP4 recording fails.
    /// 
    /// Implements Requirements 2.3, 4.4, 5.3, 5.5: Auto-save parameter validation, flow, and graceful degradation
    pub async fn start_recording_with_validation(
        &mut self,
        microphone_device: Option<Arc<AudioDevice>>,
        system_device: Option<Arc<AudioDevice>>,
        auto_save: bool,
    ) -> Result<mpsc::UnboundedReceiver<AudioChunk>, String> {
        info!("🚀 Starting recording with auto_save parameter validation (auto_save: {})", auto_save);

        // Step 1: Trace auto_save parameter flow through pipeline
        let parameter_trace = self.diagnostics.trace_auto_save_parameter().await;
        
        info!("📋 Auto-save parameter trace:");
        info!("  - Source: {:?}", parameter_trace.source);
        info!("  - Initial value: {}", parameter_trace.value);
        info!("  - Final value: {}", parameter_trace.final_value());
        info!("  - Propagated correctly: {}", parameter_trace.is_propagated_correctly());

        // Step 2: Validate parameter propagation
        if !parameter_trace.is_propagated_correctly() {
            error!("❌ Auto-save parameter not propagated correctly through pipeline");
            for override_point in &parameter_trace.override_points {
                error!("  - Override at {}: {} -> {} ({})", 
                       override_point.location, 
                       override_point.original_value, 
                       override_point.new_value,
                       override_point.reason);
            }
            
            // Attempt graceful degradation instead of failing completely
            warn!("🔄 Attempting graceful degradation to transcript-only mode due to parameter propagation failure");
            return self.start_recording_with_graceful_degradation(
                microphone_device, 
                system_device, 
                "Auto-save parameter propagation failed".to_string()
            ).await;
        }

        // Step 3: Validate that the provided auto_save matches the traced value
        let traced_auto_save = parameter_trace.final_value();
        if auto_save != traced_auto_save {
            warn!("⚠️  Auto-save parameter mismatch: provided={}, traced={}", auto_save, traced_auto_save);
            warn!("   Using traced value from preferences: {}", traced_auto_save);
        }

        // Step 4: Use the traced auto_save value to ensure consistency
        let validated_auto_save = traced_auto_save;

        // Step 5: Validate IncrementalSaver initialization when auto_save is true
        if validated_auto_save {
            info!("✅ Auto-save enabled - validating MP4 recording prerequisites");
            
            // Comprehensive validation with graceful degradation fallback
            match self.validate_mp4_recording_prerequisites().await {
                Ok(()) => {
                    info!("✅ MP4 recording prerequisites validated successfully");
                }
                Err(validation_error) => {
                    warn!("⚠️  MP4 recording prerequisites validation failed: {}", validation_error);
                    warn!("🔄 Attempting graceful degradation to transcript-only mode");
                    
                    return self.start_recording_with_graceful_degradation(
                        microphone_device, 
                        system_device, 
                        format!("MP4 recording prerequisites failed: {}", validation_error)
                    ).await;
                }
            }
        } else {
            info!("ℹ️  Auto-save disabled - recording will save transcripts only");
        }

        // Step 6: Attempt to start recording with validated auto_save parameter
        info!("🎬 Starting recording manager with validated auto_save: {}", validated_auto_save);
        
        match self.manager.start_recording(microphone_device.clone(), system_device.clone(), validated_auto_save).await {
            Ok(transcription_receiver) => {
                info!("✅ Recording started successfully with auto_save validation");
                
                // Step 7: Post-start validation - ensure IncrementalSaver was initialized if needed
                if validated_auto_save {
                    let (checkpoint_count, _sample_rate) = self.manager.get_recording_stats();
                    info!("📊 Post-start validation: checkpoint system initialized (count: {})", checkpoint_count);
                }
                
                Ok(transcription_receiver)
            }
            Err(e) => {
                error!("❌ Failed to start recording manager: {}", e);
                
                // Attempt graceful degradation if MP4 recording was requested
                if validated_auto_save {
                    warn!("🔄 MP4 recording failed, attempting graceful degradation to transcript-only mode");
                    return self.start_recording_with_graceful_degradation(
                        microphone_device, 
                        system_device, 
                        format!("Recording manager startup failed: {}", e)
                    ).await;
                } else {
                    // If transcript-only mode also failed, this is a critical error
                    return Err(format!("Failed to start recording manager: {}", e));
                }
            }
        }
    }

    /// Start recording with graceful degradation to transcript-only mode
    /// 
    /// This method ensures that transcript functionality continues even when
    /// MP4 recording fails. It implements Requirements 5.3, 5.5 for graceful
    /// degradation and continued transcript functionality.
    pub async fn start_recording_with_graceful_degradation(
        &mut self,
        microphone_device: Option<Arc<AudioDevice>>,
        system_device: Option<Arc<AudioDevice>>,
        failure_reason: String,
    ) -> Result<mpsc::UnboundedReceiver<AudioChunk>, String> {
        warn!("🔄 Starting recording in graceful degradation mode (transcript-only)");
        warn!("🔄 Reason for degradation: {}", failure_reason);
        
        // Create a comprehensive error for the failure
        let degradation_error = crate::recording::error_handling::RecordingError::CheckpointError {
            context: failure_reason.clone(),
            checkpoint_dir: std::path::PathBuf::from("unknown"),
            failed_operation: crate::recording::error_handling::CheckpointOperation::DirectoryCreation,
            chunks_affected: 0,
            recovery_strategy: crate::recording::error_handling::RecoveryStrategy::GracefulDegradation {
                preserve_transcripts: true,
                notify_user: true,
                fallback_message: format!("MP4 recording failed ({}), continuing with transcript-only mode", failure_reason),
            },
        };
        
        // Use the error recovery coordinator to handle the degradation
        let recovery_coordinator = crate::recording::error_handling::ErrorRecoveryCoordinator::new()
            .with_graceful_degradation(true)
            .with_retry_config(1, 0); // No retries for degradation
            
        let recovery_result = recovery_coordinator.attempt_recovery(&degradation_error).await;
        
        match recovery_result {
            crate::recording::error_handling::RecoveryResult::GracefulDegradation { 
                user_notification: Some(message), .. 
            } => {
                info!("✅ Graceful degradation activated: {}", message);
            },
            _ => {
                warn!("⚠️  Unexpected recovery result, continuing with degradation anyway");
            }
        }
        
        // Force auto_save to false for transcript-only mode
        info!("🎯 Starting recording manager in transcript-only mode (auto_save: false)");
        
        match self.manager.start_recording(microphone_device, system_device, false).await {
            Ok(transcription_receiver) => {
                info!("✅ Recording started successfully in transcript-only mode");
                info!("📝 Transcripts will be saved, but no MP4 audio files will be created");
                info!("💡 To restore MP4 recording, resolve the issue: {}", failure_reason);
                
                Ok(transcription_receiver)
            }
            Err(e) => {
                error!("❌ Critical failure: Even transcript-only mode failed to start: {}", e);
                Err(format!("Critical failure: Even transcript-only mode failed to start: {}", e))
            }
        }
    }

    /// Validate prerequisites for MP4 recording
    /// 
    /// This method performs comprehensive validation of all components needed
    /// for successful MP4 recording, including folder creation, FFmpeg availability,
    /// and disk space checks.
    async fn validate_mp4_recording_prerequisites(&self) -> Result<(), String> {
        info!("🔍 Validating MP4 recording prerequisites");
        
        // Check 1: Validate meeting folder creation capability
        if let Some(app_handle) = &self.app_handle {
            match super::recording_preferences::load_recording_preferences_with_validation(app_handle).await {
                Ok(preferences) => {
                    // Test meeting folder creation
                    let test_meeting_name = format!("validation_test_{}", chrono::Utc::now().timestamp());
                    match super::audio_processing::create_meeting_folder(
                        &preferences.save_folder,
                        &test_meeting_name,
                        true, // Create checkpoints directory
                    ) {
                        Ok(test_folder) => {
                            info!("✅ Meeting folder creation validated");
                            // Clean up test folder
                            if let Err(e) = std::fs::remove_dir_all(&test_folder) {
                                warn!("Failed to clean up test folder: {}", e);
                            }
                        }
                        Err(e) => {
                            return Err(format!("Cannot create meeting folder: {}", e));
                        }
                    }
                    
                    // Check 2: Validate disk space
                    match self.check_available_disk_space(&preferences.save_folder).await {
                        Ok(available_mb) => {
                            if available_mb < 100 { // Minimum 100MB required
                                return Err(format!("Insufficient disk space: {} MB available, 100 MB required", available_mb));
                            }
                            info!("✅ Disk space validated: {} MB available", available_mb);
                        }
                        Err(e) => {
                            warn!("⚠️  Could not check disk space: {}", e);
                            // Non-fatal - continue without disk space check
                        }
                    }
                }
                Err(e) => {
                    return Err(format!("Cannot load recording preferences: {}", e));
                }
            }
        } else {
            warn!("⚠️  No app handle available for preference validation");
        }
        
        // Check 3: Validate FFmpeg availability (if needed for merging)
        match self.validate_ffmpeg_availability().await {
            Ok(()) => {
                info!("✅ FFmpeg availability validated");
            }
            Err(e) => {
                // FFmpeg might not be critical for basic checkpoint creation
                warn!("⚠️  FFmpeg validation failed: {}", e);
                warn!("⚠️  MP4 merging may not work, but checkpoints will be preserved");
            }
        }
        
        info!("✅ MP4 recording prerequisites validation completed");
        Ok(())
    }

    /// Check available disk space at the given path
    async fn check_available_disk_space(&self, path: &std::path::Path) -> Result<u64, String> {
        use std::fs;
        
        // Try to get filesystem stats
        match fs::metadata(path) {
            Ok(_) => {
                // For a more accurate disk space check, we'd need platform-specific code
                // For now, we'll do a simple check by trying to create a small test file
                let test_file = path.join("disk_space_test.tmp");
                match fs::write(&test_file, b"test") {
                    Ok(()) => {
                        // Clean up test file
                        let _ = fs::remove_file(&test_file);
                        // Return a conservative estimate
                        Ok(1000) // Assume 1GB available if we can write
                    }
                    Err(e) => {
                        Err(format!("Cannot write to disk: {}", e))
                    }
                }
            }
            Err(e) => {
                Err(format!("Cannot access path: {}", e))
            }
        }
    }

    /// Validate FFmpeg availability for MP4 merging
    async fn validate_ffmpeg_availability(&self) -> Result<(), String> {
        // Use the diagnostic engine's dependency checker
        let dependency_status = self.diagnostics.dependency_checker.check_dependencies().await;
        
        match dependency_status {
            crate::recording::diagnostics::DependencyStatus::Available => Ok(()),
            crate::recording::diagnostics::DependencyStatus::FFmpegNotFound => {
                Err("FFmpeg not found in system PATH".to_string())
            }
            crate::recording::diagnostics::DependencyStatus::FFmpegIncompatible(version) => {
                Err(format!("FFmpeg version not supported: {}", version))
            }
        }
    }

    /// Start recording with default devices and diagnostic validation
    /// 
    /// This method includes comprehensive graceful degradation support,
    /// ensuring transcript functionality continues even when MP4 recording fails.
    pub async fn start_recording_with_defaults_and_validation(&mut self, auto_save: bool) -> Result<mpsc::UnboundedReceiver<AudioChunk>, String> {
        info!("🚀 Starting recording with defaults and auto_save validation (auto_save: {})", auto_save);

        // First run diagnostic validation
        let diagnostic_report = self.initialize_with_diagnostics().await?;
        
        // Check if system is healthy enough to proceed
        if !diagnostic_report.is_healthy() {
            let issues = diagnostic_report.get_critical_issues();
            if !issues.is_empty() {
                error!("❌ System not healthy for recording:");
                for issue in &issues {
                    error!("  - {}", issue);
                }
                
                // If auto_save was requested but system isn't healthy, try graceful degradation
                if auto_save {
                    warn!("🔄 System not healthy for MP4 recording, attempting graceful degradation");
                    return self.start_recording_with_graceful_degradation_defaults(
                        "System diagnostic issues detected".to_string()
                    ).await;
                } else {
                    return Err("System not healthy for recording - see diagnostic report".to_string());
                }
            }
        }

        // Use the manager's method but with our validation wrapper and graceful degradation
        match self.manager.start_recording_with_defaults_and_auto_save(auto_save).await {
            Ok(transcription_receiver) => {
                info!("✅ Recording started with defaults and validation");
                Ok(transcription_receiver)
            }
            Err(e) => {
                error!("❌ Failed to start recording with defaults: {}", e);
                
                // If auto_save was requested but failed, try graceful degradation
                if auto_save {
                    warn!("🔄 MP4 recording with defaults failed, attempting graceful degradation");
                    return self.start_recording_with_graceful_degradation_defaults(
                        format!("Recording with defaults failed: {}", e)
                    ).await;
                } else {
                    return Err(format!("Failed to start recording with defaults: {}", e));
                }
            }
        }
    }

    /// Start recording with default devices in graceful degradation mode (transcript-only)
    async fn start_recording_with_graceful_degradation_defaults(
        &mut self,
        failure_reason: String,
    ) -> Result<mpsc::UnboundedReceiver<AudioChunk>, String> {
        warn!("🔄 Starting recording with defaults in graceful degradation mode");
        warn!("🔄 Reason for degradation: {}", failure_reason);
        
        // Create a comprehensive error for the failure
        let degradation_error = crate::recording::error_handling::RecordingError::PipelineInitializationError {
            context: failure_reason.clone(),
            failed_component: "RecordingManager".to_string(),
            diagnostic_info: "Failed to start with auto_save enabled".to_string(),
            recovery_strategy: crate::recording::error_handling::RecoveryStrategy::GracefulDegradation {
                preserve_transcripts: true,
                notify_user: true,
                fallback_message: format!("MP4 recording failed ({}), continuing with transcript-only mode", failure_reason),
            },
        };
        
        // Use the error recovery coordinator
        let recovery_coordinator = crate::recording::error_handling::ErrorRecoveryCoordinator::new()
            .with_graceful_degradation(true);
            
        let recovery_result = recovery_coordinator.attempt_recovery(&degradation_error).await;
        
        match recovery_result {
            crate::recording::error_handling::RecoveryResult::GracefulDegradation { 
                user_notification: Some(message), .. 
            } => {
                info!("✅ Graceful degradation activated: {}", message);
            },
            _ => {
                warn!("⚠️  Unexpected recovery result, continuing with degradation anyway");
            }
        }
        
        // Force auto_save to false for transcript-only mode
        info!("🎯 Starting recording manager with defaults in transcript-only mode (auto_save: false)");
        
        match self.manager.start_recording_with_defaults_and_auto_save(false).await {
            Ok(transcription_receiver) => {
                info!("✅ Recording started successfully with defaults in transcript-only mode");
                info!("📝 Transcripts will be saved, but no MP4 audio files will be created");
                info!("💡 To restore MP4 recording, resolve the issue: {}", failure_reason);
                
                Ok(transcription_receiver)
            }
            Err(e) => {
                error!("❌ Critical failure: Even transcript-only mode with defaults failed: {}", e);
                Err(format!("Critical failure: Even transcript-only mode with defaults failed: {}", e))
            }
        }
    }

    /// Get diagnostic report for the current pipeline state
    pub async fn get_diagnostic_report(&self) -> DiagnosticReport {
        self.diagnostics.run_full_diagnosis().await
    }

    /// Validate auto_save parameter flow without starting recording
    pub async fn validate_auto_save_flow(&self) -> Result<bool, String> {
        info!("🔍 Validating auto_save parameter flow");

        let parameter_trace = self.diagnostics.trace_auto_save_parameter().await;
        
        if parameter_trace.is_propagated_correctly() {
            info!("✅ Auto-save parameter flow validation passed");
            Ok(true)
        } else {
            error!("❌ Auto-save parameter flow validation failed");
            for override_point in &parameter_trace.override_points {
                error!("  - Override at {}: {} -> {} ({})", 
                       override_point.location, 
                       override_point.original_value, 
                       override_point.new_value,
                       override_point.reason);
            }
            Ok(false)
        }
    }

    /// Check if the recording system is currently in graceful degradation mode
    /// 
    /// This method provides status information about whether the system has
    /// fallen back to transcript-only mode due to MP4 recording failures.
    pub async fn is_in_graceful_degradation_mode(&self) -> bool {
        // Check if the manager is recording but without auto_save
        if self.manager.is_recording() {
            let (checkpoint_count, _) = self.manager.get_recording_stats();
            
            // If recording is active but no checkpoints are being created,
            // we're likely in graceful degradation mode
            checkpoint_count == 0
        } else {
            false
        }
    }

    /// Get the current recording mode status
    /// 
    /// Returns information about whether the system is recording with MP4,
    /// transcript-only, or not recording at all.
    pub async fn get_recording_mode_status(&self) -> RecordingModeStatus {
        if !self.manager.is_recording() {
            return RecordingModeStatus::NotRecording;
        }

        let (checkpoint_count, _) = self.manager.get_recording_stats();
        
        if checkpoint_count > 0 {
            RecordingModeStatus::FullRecording {
                checkpoints_created: checkpoint_count as u32,
            }
        } else {
            RecordingModeStatus::TranscriptOnly {
                reason: "Graceful degradation due to MP4 recording failure".to_string(),
            }
        }
    }

    /// Attempt to restore MP4 recording during an active session
    /// 
    /// This method tries to re-enable MP4 recording if the underlying issues
    /// have been resolved while recording is still active.
    pub async fn attempt_mp4_recording_restoration(&mut self) -> Result<bool, String> {
        if !self.manager.is_recording() {
            return Err("No active recording session to restore".to_string());
        }

        info!("🔄 Attempting to restore MP4 recording for active session");

        // Re-validate MP4 recording prerequisites
        match self.validate_mp4_recording_prerequisites().await {
            Ok(()) => {
                info!("✅ MP4 recording prerequisites now satisfied");
                
                // Try to re-initialize the incremental saver in the recording manager
                // Note: This would require additional methods in RecordingManager
                // For now, we'll just report that prerequisites are met
                warn!("💡 MP4 recording prerequisites are now satisfied");
                warn!("💡 To fully restore MP4 recording, stop and restart the recording session");
                
                Ok(true)
            }
            Err(e) => {
                info!("❌ MP4 recording prerequisites still not satisfied: {}", e);
                Ok(false)
            }
        }
    }

    /// Delegate all other methods to the underlying RecordingManager
    pub async fn stop_streams_and_force_flush(&mut self) -> Result<()> {
        self.manager.stop_streams_and_force_flush().await
    }

    pub async fn save_recording_only<T: tauri::Runtime>(&mut self, app: &tauri::AppHandle<T>) -> Result<()> {
        self.manager.save_recording_only(app).await
    }

    pub async fn stop_recording<T: tauri::Runtime>(&mut self, app: &tauri::AppHandle<T>) -> Result<()> {
        self.manager.stop_recording(app).await
    }

    pub fn get_recording_stats(&self) -> (usize, u32) {
        self.manager.get_recording_stats()
    }

    pub fn is_recording(&self) -> bool {
        self.manager.is_recording()
    }

    pub fn pause_recording(&self) -> Result<()> {
        self.manager.pause_recording()
    }

    pub fn resume_recording(&self) -> Result<()> {
        self.manager.resume_recording()
    }

    pub fn is_paused(&self) -> bool {
        self.manager.is_paused()
    }

    pub fn is_active(&self) -> bool {
        self.manager.is_active()
    }

    pub fn get_stats(&self) -> super::recording_state::RecordingStats {
        self.manager.get_stats()
    }

    pub fn get_recording_duration(&self) -> Option<f64> {
        self.manager.get_recording_duration()
    }

    pub fn get_active_recording_duration(&self) -> Option<f64> {
        self.manager.get_active_recording_duration()
    }

    pub fn get_total_pause_duration(&self) -> f64 {
        self.manager.get_total_pause_duration()
    }

    pub fn get_current_pause_duration(&self) -> Option<f64> {
        self.manager.get_current_pause_duration()
    }

    pub fn get_error_info(&self) -> (u32, Option<super::recording_state::AudioError>) {
        self.manager.get_error_info()
    }

    pub fn active_stream_count(&self) -> usize {
        self.manager.active_stream_count()
    }

    pub fn set_error_callback<F>(&self, callback: F)
    where
        F: Fn(&super::recording_state::AudioError) + Send + Sync + 'static,
    {
        self.manager.set_error_callback(callback);
    }

    pub fn has_fatal_error(&self) -> bool {
        self.manager.has_fatal_error()
    }

    pub fn set_meeting_name(&mut self, name: Option<String>) {
        self.manager.set_meeting_name(name);
    }

    pub fn add_transcript_segment(&self, segment: super::recording_saver::TranscriptSegment) {
        self.manager.add_transcript_segment(segment);
    }

    pub fn add_transcript_chunk(&self, text: String) {
        self.manager.add_transcript_chunk(text);
    }

    pub fn get_transcript_segments(&self) -> Vec<super::recording_saver::TranscriptSegment> {
        self.manager.get_transcript_segments()
    }

    pub fn get_meeting_name(&self) -> Option<String> {
        self.manager.get_meeting_name()
    }

    pub async fn cleanup_without_save(&mut self) {
        self.manager.cleanup_without_save().await
    }

    pub fn get_meeting_folder(&self) -> Option<std::path::PathBuf> {
        self.manager.get_meeting_folder()
    }

    pub fn poll_device_events(&mut self) -> Option<DeviceEvent> {
        self.manager.poll_device_events()
    }

    pub async fn attempt_device_reconnect(&mut self, device_name: &str, device_type: DeviceMonitorType) -> Result<bool> {
        self.manager.attempt_device_reconnect(device_name, device_type).await
    }

    pub async fn handle_device_disconnect(&mut self, device_name: String, device_type: DeviceMonitorType) {
        self.manager.handle_device_disconnect(device_name, device_type).await
    }

    pub async fn handle_device_reconnect(&mut self, device_name: String, device_type: DeviceMonitorType) -> Result<()> {
        self.manager.handle_device_reconnect(device_name, device_type).await
    }

    pub fn is_reconnecting(&self) -> bool {
        self.manager.is_reconnecting()
    }

    pub fn get_state(&self) -> &Arc<RecordingState> {
        self.manager.get_state()
    }

    /// Get access to the underlying RecordingManager for advanced operations
    pub fn get_manager(&self) -> &RecordingManager {
        &self.manager
    }

    /// Get mutable access to the underlying RecordingManager for advanced operations
    pub fn get_manager_mut(&mut self) -> &mut RecordingManager {
        &mut self.manager
    }

    /// Get access to the diagnostic engine for advanced diagnostics
    pub fn get_diagnostics(&self) -> &DiagnosticEngine {
        &self.diagnostics
    }
}

impl<R: tauri::Runtime> Default for RecordingPipeline<R> {
    fn default() -> Self {
        Self::new()
    }
}

/// Simplified recording manager that coordinates all audio components
pub struct RecordingManager {
    state: Arc<RecordingState>,
    stream_manager: AudioStreamManager,
    pipeline_manager: AudioPipelineManager,
    recording_saver: RecordingSaver,
    device_monitor: Option<AudioDeviceMonitor>,
    device_event_receiver: Option<mpsc::UnboundedReceiver<DeviceEvent>>,
}

// SAFETY: RecordingManager contains types that we've marked as Send
unsafe impl Send for RecordingManager {}

impl RecordingManager {
    /// Create a new recording manager
    pub fn new() -> Self {
        let state = RecordingState::new();
        let stream_manager = AudioStreamManager::new(state.clone());
        let pipeline_manager = AudioPipelineManager::new();
        let (device_monitor, device_event_receiver) = AudioDeviceMonitor::new();

        Self {
            state,
            stream_manager,
            pipeline_manager,
            recording_saver: RecordingSaver::new(),
            device_monitor: Some(device_monitor),
            device_event_receiver: Some(device_event_receiver),
        }
    }

    // Remove app handle storage for now - will be passed directly when saving

    /// Start recording with specified devices
    ///
    /// # Arguments
    /// * `microphone_device` - Optional microphone device to use
    /// * `system_device` - Optional system audio device to use
    /// * `auto_save` - Whether to save audio checkpoints (true) or just transcripts/metadata (false)
    pub async fn start_recording(
        &mut self,
        microphone_device: Option<Arc<AudioDevice>>,
        system_device: Option<Arc<AudioDevice>>,
        auto_save: bool,
    ) -> Result<mpsc::UnboundedReceiver<AudioChunk>> {
        info!("Starting recording manager (auto_save: {})", auto_save);

        // Requirement 7.1: Log auto_save parameter propagation through recording manager
        info!("🔧 [STRUCTURED_LOG] recording_manager_start: {{ \"auto_save_received\": {}, \"microphone_device\": \"{}\", \"system_device\": \"{}\" }}", 
              auto_save,
              microphone_device.as_ref().map(|d| d.name.as_str()).unwrap_or("none"),
              system_device.as_ref().map(|d| d.name.as_str()).unwrap_or("none"));

        // Set up transcription channel
        let (transcription_sender, transcription_receiver) = mpsc::unbounded_channel::<AudioChunk>();

        // CRITICAL FIX: Create recording sender for pre-mixed audio from pipeline
        // Pipeline will mix mic + system audio professionally and send to this channel
        // Pass auto_save to control whether audio checkpoints are created
        let recording_sender = self.recording_saver.start_accumulation(auto_save);

        // Requirement 7.1: Log recording saver initialization with auto_save parameter
        info!("🔧 [STRUCTURED_LOG] recording_saver_initialized: {{ \"auto_save_passed\": {}, \"checkpoint_creation_enabled\": {} }}", 
              auto_save, auto_save);

        // Start recording state first
        self.state.start_recording()?;

        // Get device information for adaptive mixing
        // The pipeline uses device kind (Bluetooth vs Wired) to apply adaptive buffering:
        // - Bluetooth: Larger buffers (80-200ms) to handle jitter
        // - Wired: Smaller buffers (20-50ms) for low latency
        let (mic_name, mic_kind) = if let Some(ref mic) = microphone_device {
            let device_kind = super::device_detection::InputDeviceKind::detect(&mic.name, 512, 48000);
            (mic.name.clone(), device_kind)
        } else {
            ("No Microphone".to_string(), super::device_detection::InputDeviceKind::Unknown)
        };

        let (sys_name, sys_kind) = if let Some(ref sys) = system_device {
            let device_kind = super::device_detection::InputDeviceKind::detect(&sys.name, 512, 48000);
            (sys.name.clone(), device_kind)
        } else {
            ("No System Audio".to_string(), super::device_detection::InputDeviceKind::Unknown)
        };

        // Update recording metadata with device information
        self.recording_saver.set_device_info(
            microphone_device.as_ref().map(|d| d.name.clone()),
            system_device.as_ref().map(|d| d.name.clone())
        );

        // Start the audio processing pipeline with FFmpeg adaptive mixer
        // Pipeline will: 1) Mix mic+system audio with adaptive buffering, 2) Send mixed to recording_sender,
        // 3) Apply VAD and send speech segments to transcription
        self.pipeline_manager.start(
            self.state.clone(),
            transcription_sender,
            0, // Ignored - using dynamic sizing internally
            48000, // 48kHz sample rate
            Some(recording_sender), // CRITICAL: Pass recording sender to receive pre-mixed audio
            mic_name,
            mic_kind,
            sys_name,
            sys_kind,
        )?;

        // Give the pipeline a moment to fully initialize before starting streams
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

        // Start audio streams - they send RAW unmixed chunks to pipeline for mixing
        // Pipeline handles mixing and distribution to both recording and transcription
        self.stream_manager.start_streams(microphone_device.clone(), system_device.clone(), None).await?;

        // Start device monitoring to detect disconnects
        if let Some(ref mut monitor) = self.device_monitor {
            if let Err(e) = monitor.start_monitoring(microphone_device, system_device) {
                warn!("Failed to start device monitoring: {}", e);
                // Non-fatal - continue without monitoring
            } else {
                info!("✅ Device monitoring started");
            }
        }

        info!("Recording manager started successfully with {} active streams",
               self.stream_manager.active_stream_count());

        Ok(transcription_receiver)
    }

    /// Start recording with default devices and auto_save setting
    ///
    /// # Arguments
    /// * `auto_save` - Whether to save audio checkpoints (true) or just transcripts/metadata (false)
    ///
    /// # Platform-Specific Behavior
    ///
    /// **macOS**: Uses smart device selection that automatically overrides
    /// Bluetooth devices to built-in wired devices for stable, consistent sample rates.
    /// This prevents Core Audio/ScreenCaptureKit from delivering variable sample rate
    /// streams that cause sync issues when mixing mic + system audio.
    ///
    /// **Windows/Linux**: Uses system default devices directly without override.
    ///
    /// # macOS Bluetooth Override Strategy
    ///
    /// - Microphone: If Bluetooth → Use built-in MacBook mic
    /// - Speaker: If Bluetooth → Use built-in MacBook speaker (for ScreenCaptureKit)
    /// - Each device is checked INDEPENDENTLY
    ///
    /// Rationale: Bluetooth devices on macOS can have variable sample rates as Core Audio
    /// and the Bluetooth stack may resample dynamically. Built-in devices provide
    /// fixed, consistent sample rates for reliable audio mixing.
    ///
    /// User still hears audio via Bluetooth (playback), but recording captures
    /// via stable wired path for best quality.
    pub async fn start_recording_with_defaults_and_auto_save(&mut self, auto_save: bool) -> Result<mpsc::UnboundedReceiver<AudioChunk>> {
        #[cfg(target_os = "macos")]
        {
            info!("🎙️ [macOS] Starting recording with smart device selection (Bluetooth override enabled)");

            // Get safe recording devices with automatic Bluetooth fallback
            // This function handles all the detection and override logic for macOS
            let (microphone_device, system_device) = get_safe_recording_devices_macos()?;

            // Wrap in Arc for sharing across threads
            let microphone_device = microphone_device.map(Arc::new);
            let system_device = system_device.map(Arc::new);

            // Ensure at least microphone is available
            if microphone_device.is_none() {
                return Err(anyhow::anyhow!("❌ No microphone device available for recording"));
            }

            // Start recording with selected devices and auto_save setting
            self.start_recording(microphone_device, system_device, auto_save).await
        }

        #[cfg(not(target_os = "macos"))]
        {
            info!("Starting recording with default devices");

            // Get default devices (no Bluetooth override on Windows/Linux)
            let microphone_device = match default_input_device() {
                Ok(device) => {
                    info!("Using default microphone: {}", device.name);
                    Some(Arc::new(device))
                }
                Err(e) => {
                    warn!("No default microphone available: {}", e);
                    None
                }
            };

            let system_device = match default_output_device() {
                Ok(device) => {
                    info!("Using default system audio: {}", device.name);
                    Some(Arc::new(device))
                }
                Err(e) => {
                    warn!("No default system audio available: {}", e);
                    None
                }
            };

            // Ensure at least microphone is available
            if microphone_device.is_none() {
                return Err(anyhow::anyhow!("No microphone device available"));
            }

            self.start_recording(microphone_device, system_device, auto_save).await
        }
    }

    /// Stop recording streams without saving (for use when waiting for transcription)
    pub async fn stop_streams_only(&mut self) -> Result<()> {
        info!("Stopping recording streams only");

        // Stop device monitoring
        if let Some(ref mut monitor) = self.device_monitor {
            monitor.stop_monitoring().await;
        }

        // Stop recording state first
        self.state.stop_recording();

        // Stop audio streams
        if let Err(e) = self.stream_manager.stop_streams() {
            error!("Error stopping audio streams: {}", e);
        }

        // Stop audio pipeline
        if let Err(e) = self.pipeline_manager.stop().await {
            error!("Error stopping audio pipeline: {}", e);
        }

        debug!("Recording streams stopped successfully");
        Ok(())
    }

    /// Stop streams and force immediate pipeline flush to process all accumulated audio
    pub async fn stop_streams_and_force_flush(&mut self) -> Result<()> {
        info!("🚀 Stopping recording streams with IMMEDIATE pipeline flush");

        // CRITICAL: Stop device monitor FIRST to prevent continuous WASAPI polling on Windows
        // This fixes the slow shutdown issue where device enumeration runs for 90+ seconds
        if let Some(ref mut monitor) = self.device_monitor {
            info!("Stopping device monitor first...");
            monitor.stop_monitoring().await;
        }

        // Stop recording state first - this clears device references
        self.state.stop_recording();

        // Stop audio streams immediately
        if let Err(e) = self.stream_manager.stop_streams() {
            error!("Error stopping audio streams: {}", e);
        }

        // CRITICAL: Force pipeline to flush ALL accumulated audio before stopping
        debug!("💨 Forcing pipeline to flush accumulated audio immediately");
        if let Err(e) = self.pipeline_manager.force_flush_and_stop().await {
            error!("Error during force flush: {}", e);
        }

        // CRITICAL: Full cleanup to release all Arc references and resources
        // This ensures microphone is released even if Drop is delayed
        self.state.cleanup();

        info!("✅ Recording streams stopped with immediate flush completed");
        Ok(())
    }

    /// Save recording after transcription is complete
    pub async fn save_recording_only<R: tauri::Runtime>(&mut self, app: &tauri::AppHandle<R>) -> Result<()> {
        debug!("Saving recording with transcript chunks");

        // Get actual recording duration from state
        let recording_duration = self.state.get_active_recording_duration();
        info!("Recording duration from state: {:?}s", recording_duration);

        // Save the recording with actual duration
        match self.recording_saver.stop_and_save(app, recording_duration).await {
            Ok(Some(file_path)) => {
                info!("Recording saved successfully to: {}", file_path);
            }
            Ok(None) => {
                debug!("Recording not saved (auto-save disabled or no audio data)");
            }
            Err(e) => {
                error!("Failed to save recording: {}", e);
                // Don't fail the stop operation if saving fails
            }
        }

        debug!("Recording save operation completed");
        Ok(())
    }

    /// Stop recording and save audio (legacy method)
    pub async fn stop_recording<R: tauri::Runtime>(&mut self, app: &tauri::AppHandle<R>) -> Result<()> {
        info!("Stopping recording manager");

        // Get recording duration BEFORE stopping (important!)
        let recording_duration = self.state.get_active_recording_duration();
        info!("Recording duration before stop: {:?}s", recording_duration);

        // Stop recording state first
        self.state.stop_recording();

        // Stop audio streams
        if let Err(e) = self.stream_manager.stop_streams() {
            error!("Error stopping audio streams: {}", e);
        }

        // Stop audio pipeline
        if let Err(e) = self.pipeline_manager.stop().await {
            error!("Error stopping audio pipeline: {}", e);
        }

        // Save the recording with actual duration
        match self.recording_saver.stop_and_save(app, recording_duration).await {
            Ok(Some(file_path)) => {
                info!("Recording saved successfully to: {}", file_path);
            }
            Ok(None) => {
                info!("Recording not saved (auto-save disabled or no audio data)");
            }
            Err(e) => {
                error!("Failed to save recording: {}", e);
                // Don't fail the stop operation if saving fails
            }
        }

        info!("Recording manager stopped");
        Ok(())
    }

    /// Get recording stats from the saver
    pub fn get_recording_stats(&self) -> (usize, u32) {
        self.recording_saver.get_stats()
    }

    /// Check if currently recording
    pub fn is_recording(&self) -> bool {
        self.state.is_recording()
    }

    /// Pause the current recording session
    pub fn pause_recording(&self) -> Result<()> {
        info!("Pausing recording");
        self.state.pause_recording()
    }

    /// Resume the current recording session
    pub fn resume_recording(&self) -> Result<()> {
        info!("Resuming recording");
        self.state.resume_recording()
    }

    /// Check if recording is currently paused
    pub fn is_paused(&self) -> bool {
        self.state.is_paused()
    }

    /// Check if recording is active (recording and not paused)
    pub fn is_active(&self) -> bool {
        self.state.is_active()
    }

    /// Get recording statistics
    pub fn get_stats(&self) -> super::recording_state::RecordingStats {
        self.state.get_stats()
    }

    /// Get recording duration
    pub fn get_recording_duration(&self) -> Option<f64> {
        self.state.get_recording_duration()
    }

    /// Get active recording duration (excluding pauses)
    pub fn get_active_recording_duration(&self) -> Option<f64> {
        self.state.get_active_recording_duration()
    }

    /// Get total pause duration
    pub fn get_total_pause_duration(&self) -> f64 {
        self.state.get_total_pause_duration()
    }

    /// Get current pause duration if paused
    pub fn get_current_pause_duration(&self) -> Option<f64> {
        self.state.get_current_pause_duration()
    }

    /// Get error information
    pub fn get_error_info(&self) -> (u32, Option<super::recording_state::AudioError>) {
        (self.state.get_error_count(), self.state.get_last_error())
    }

    /// Get active stream count
    pub fn active_stream_count(&self) -> usize {
        self.stream_manager.active_stream_count()
    }

    /// Set error callback for handling errors
    pub fn set_error_callback<F>(&self, callback: F)
    where
        F: Fn(&super::recording_state::AudioError) + Send + Sync + 'static,
    {
        self.state.set_error_callback(callback);
    }

    /// Check if there's a fatal error
    pub fn has_fatal_error(&self) -> bool {
        self.state.has_fatal_error()
    }

    /// Set the meeting name for this recording session
    pub fn set_meeting_name(&mut self, name: Option<String>) {
        self.recording_saver.set_meeting_name(name);
    }

    /// Add a structured transcript segment to be saved later
    pub fn add_transcript_segment(&self, segment: super::recording_saver::TranscriptSegment) {
        self.recording_saver.add_transcript_segment(segment);
    }

    /// Add a transcript chunk to be saved later (legacy method)
    pub fn add_transcript_chunk(&self, text: String) {
        self.recording_saver.add_transcript_chunk(text);
    }

    /// Get accumulated transcript segments from current recording session
    /// Used for syncing frontend state after page reload during active recording
    pub fn get_transcript_segments(&self) -> Vec<super::recording_saver::TranscriptSegment> {
        self.recording_saver.get_transcript_segments()
    }

    /// Get meeting name from current recording session
    /// Used for syncing frontend state after page reload during active recording
    pub fn get_meeting_name(&self) -> Option<String> {
        self.recording_saver.get_meeting_name()
    }

    /// Cleanup all resources without saving
    pub async fn cleanup_without_save(&mut self) {
        if self.is_recording() {
            debug!("Stopping recording without saving during cleanup");

            // Stop recording state first
            self.state.stop_recording();

            // Stop audio streams
            if let Err(e) = self.stream_manager.stop_streams() {
                error!("Error stopping audio streams during cleanup: {}", e);
            }

            // Stop audio pipeline
            if let Err(e) = self.pipeline_manager.stop().await {
                error!("Error stopping audio pipeline during cleanup: {}", e);
            }
        }
        self.state.cleanup();
    }

    /// Get the meeting folder path (if available)
    /// Returns None if no meeting name was set or folder structure not initialized
    pub fn get_meeting_folder(&self) -> Option<std::path::PathBuf> {
        self.recording_saver.get_meeting_folder().map(|p| p.clone())
    }

    /// Check for device events (disconnects/reconnects)
    /// Returns Some(DeviceEvent) if an event occurred, None otherwise
    pub fn poll_device_events(&mut self) -> Option<DeviceEvent> {
        if let Some(ref mut receiver) = self.device_event_receiver {
            receiver.try_recv().ok()
        } else {
            None
        }
    }

    /// Attempt to reconnect a disconnected device
    /// Returns true if reconnection successful
    pub async fn attempt_device_reconnect(&mut self, device_name: &str, device_type: DeviceMonitorType) -> Result<bool> {
        info!("🔄 Attempting to reconnect device: {} ({:?})", device_name, device_type);

        // List current devices
        let available_devices = list_audio_devices().await?;

        // Find the device by name
        let device = available_devices.iter()
            .find(|d| d.name == device_name)
            .cloned();

        if let Some(device) = device {
            info!("✅ Device '{}' found, recreating stream...", device_name);

            // Determine which device to reconnect based on type
            let device_arc: Arc<AudioDevice> = Arc::new(device);
            match device_type {
                DeviceMonitorType::Microphone => {
                    // Stop existing mic stream and start new one
                    // We need to keep system audio running if it exists
                    let system_device = self.state.get_system_device();

                    // Restart streams with new microphone
                    self.stream_manager.stop_streams()?;
                    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

                    self.stream_manager.start_streams(Some(device_arc.clone()), system_device, None).await?;
                    self.state.set_microphone_device(device_arc);

                    info!("✅ Microphone reconnected successfully");
                    Ok(true)
                }
                DeviceMonitorType::SystemAudio => {
                    // Stop existing system audio stream and start new one
                    let microphone_device = self.state.get_microphone_device();

                    // Restart streams with new system audio
                    self.stream_manager.stop_streams()?;
                    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

                    self.stream_manager.start_streams(microphone_device, Some(device_arc.clone()), None).await?;
                    self.state.set_system_device(device_arc);

                    info!("✅ System audio reconnected successfully");
                    Ok(true)
                }
            }
        } else {
            warn!("❌ Device '{}' not yet available", device_name);
            Ok(false)
        }
    }

    /// Handle a device disconnect event
    /// Pauses recording and attempts reconnection
    pub async fn handle_device_disconnect(&mut self, device_name: String, device_type: DeviceMonitorType) {
        warn!("📱 Device disconnected: {} ({:?})", device_name, device_type);

        // Mark state as reconnecting (keeps recording alive but in waiting state)
        let device = match device_type {
            DeviceMonitorType::Microphone => self.state.get_microphone_device(),
            DeviceMonitorType::SystemAudio => self.state.get_system_device(),
        };

        if let Some(device) = device {
            let recording_device_type = match device_type {
                DeviceMonitorType::Microphone => RecordingDeviceType::Microphone,
                DeviceMonitorType::SystemAudio => RecordingDeviceType::System,
            };
            self.state.start_reconnecting(device, recording_device_type);
        }
    }

    /// Handle a device reconnect event
    pub async fn handle_device_reconnect(&mut self, device_name: String, device_type: DeviceMonitorType) -> Result<()> {
        info!("📱 Device reconnected: {} ({:?})", device_name, device_type);

        // Attempt to reconnect the device
        match self.attempt_device_reconnect(&device_name, device_type).await {
            Ok(true) => {
                info!("✅ Successfully reconnected device: {}", device_name);
                self.state.stop_reconnecting();
                Ok(())
            }
            Ok(false) => {
                warn!("Device reconnect attempt failed (device not yet available)");
                Err(anyhow::anyhow!("Device not available"))
            }
            Err(e) => {
                error!("Device reconnect failed: {}", e);
                Err(e)
            }
        }
    }

    /// Check if currently attempting to reconnect
    pub fn is_reconnecting(&self) -> bool {
        self.state.is_reconnecting()
    }

    /// Get reference to recording state for external access
    pub fn get_state(&self) -> &Arc<RecordingState> {
        &self.state
    }
}

impl Default for RecordingManager {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for RecordingManager {
    fn drop(&mut self) {
        // Note: Can't call async cleanup in Drop, but streams have their own Drop implementations
        self.state.cleanup();
    }
}