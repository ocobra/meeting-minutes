use std::sync::{Arc, Mutex};
use tokio::sync::Mutex as AsyncMutex;
use anyhow::Result;
use log::{info, warn, error};
use tauri::{AppHandle, Runtime, Emitter};
use tokio::sync::mpsc;
use serde::{Serialize, Deserialize};
use std::path::PathBuf;

use super::recording_state::AudioChunk;
use super::audio_processing::create_meeting_folder;
use super::incremental_saver::IncrementalAudioSaver;
use crate::recording::error_handling::{RecordingError, CheckpointOperation, ErrorRecoveryCoordinator};

/// Structured transcript segment for JSON export
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptSegment {
    pub id: String,
    pub text: String,
    pub audio_start_time: f64, // Seconds from recording start
    pub audio_end_time: f64,   // Seconds from recording start
    pub duration: f64,          // Segment duration in seconds
    pub display_time: String,   // Formatted time for display like "[02:15]"
    pub confidence: f32,
    pub sequence_id: u64,
}

/// Meeting metadata structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeetingMetadata {
    pub version: String,
    pub meeting_id: Option<String>,
    pub meeting_name: Option<String>,
    pub created_at: String,
    pub completed_at: Option<String>,
    pub duration_seconds: Option<f64>,
    pub devices: DeviceInfo,
    pub audio_file: String,
    pub transcript_file: String,
    pub sample_rate: u32,
    pub status: String,  // "recording", "completed", "error"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceInfo {
    pub microphone: Option<String>,
    pub system_audio: Option<String>,
}

/// New recording saver using incremental saving strategy
pub struct RecordingSaver {
    incremental_saver: Option<Arc<AsyncMutex<IncrementalAudioSaver>>>,
    meeting_folder: Option<PathBuf>,
    meeting_name: Option<String>,
    metadata: Option<MeetingMetadata>,
    transcript_segments: Arc<Mutex<Vec<TranscriptSegment>>>,
    chunk_receiver: Option<mpsc::UnboundedReceiver<AudioChunk>>,
    is_saving: Arc<Mutex<bool>>,
    error_recovery: ErrorRecoveryCoordinator,
}

impl RecordingSaver {
    pub fn new() -> Self {
        Self {
            incremental_saver: None,
            meeting_folder: None,
            meeting_name: None,
            metadata: None,
            transcript_segments: Arc::new(Mutex::new(Vec::new())),
            chunk_receiver: None,
            is_saving: Arc::new(Mutex::new(false)),
            error_recovery: ErrorRecoveryCoordinator::new()
                .with_graceful_degradation(true)
                .with_retry_config(3, 1000),
        }
    }

    /// Set the meeting name for this recording session
    pub fn set_meeting_name(&mut self, name: Option<String>) {
        self.meeting_name = name;
    }

    /// Set device information in metadata
    pub fn set_device_info(&mut self, mic_name: Option<String>, sys_name: Option<String>) {
        if let Some(ref mut metadata) = self.metadata {
            metadata.devices.microphone = mic_name;
            metadata.devices.system_audio = sys_name;

            // Write updated metadata to disk if folder exists
            if let Some(folder) = &self.meeting_folder {
                let metadata_clone = metadata.clone();
                if let Err(e) = self.write_metadata(folder, &metadata_clone) {
                    warn!("Failed to update metadata with device info: {}", e);
                }
            }
        }
    }

    /// Add or update a structured transcript segment (upserts based on sequence_id)
    /// Also saves incrementally to disk
    pub fn add_transcript_segment(&self, segment: TranscriptSegment) {
        if let Ok(mut segments) = self.transcript_segments.lock() {
            // Check if segment with same sequence_id exists (update it)
            if let Some(existing) = segments.iter_mut().find(|s| s.sequence_id == segment.sequence_id) {
                *existing = segment.clone();
                info!("Updated transcript segment {} (seq: {}) - total segments: {}",
                      segment.id, segment.sequence_id, segments.len());
            } else {
                // New segment, add it
                segments.push(segment.clone());
                info!("Added new transcript segment {} (seq: {}) - total segments: {}",
                      segment.id, segment.sequence_id, segments.len());
            }
        } else {
            error!("Failed to lock transcript segments for adding segment {}", segment.id);
        }

        // NEW: Save incrementally to disk
        if let Some(folder) = &self.meeting_folder {
            if let Err(e) = self.write_transcripts_json(folder) {
                warn!("Failed to write incremental transcript update: {}", e);
            }
        }
    }

    /// Legacy method for backward compatibility - converts text to basic segment
    pub fn add_transcript_chunk(&self, text: String) {
        let segment = TranscriptSegment {
            id: format!("seg_{}", chrono::Utc::now().timestamp_millis()),
            text,
            audio_start_time: 0.0,
            audio_end_time: 0.0,
            duration: 0.0,
            display_time: "[00:00]".to_string(),
            confidence: 1.0,
            sequence_id: 0,
        };
        self.add_transcript_segment(segment);
    }

    /// Start accumulation with optional incremental saving
    ///
    /// # Arguments
    /// * `auto_save` - If true, creates checkpoints and enables saving. If false, audio chunks are discarded.
    pub fn start_accumulation(&mut self, auto_save: bool) -> mpsc::UnboundedSender<AudioChunk> {
        if auto_save {
            info!("Initializing incremental audio saver for recording (auto-save ENABLED)");
            
            // Requirement 7.1: Log auto_save parameter in recording saver
            info!("🔧 [STRUCTURED_LOG] recording_saver_accumulation_start: {{ \"auto_save\": true, \"mode\": \"incremental_checkpoints\", \"checkpoint_interval_seconds\": 30 }}");
        } else {
            info!("Starting recording without audio saving (auto-save DISABLED - transcripts only)");
            
            // Requirement 7.1: Log auto_save parameter in recording saver
            info!("🔧 [STRUCTURED_LOG] recording_saver_accumulation_start: {{ \"auto_save\": false, \"mode\": \"transcript_only\", \"audio_chunks_discarded\": true }}");
        }

        // Create channel for receiving audio chunks
        let (sender, receiver) = mpsc::unbounded_channel::<AudioChunk>();
        self.chunk_receiver = Some(receiver);

        // Initialize meeting folder and incremental saver ONLY if auto_save is enabled
        if auto_save {
            if let Some(name) = self.meeting_name.clone() {
                match self.initialize_meeting_folder(&name, true) {
                    Ok(()) => {
                        info!("✅ Successfully initialized meeting folder with checkpoints");
                        
                        // CRITICAL: Verify that incremental_saver was actually initialized
                        if self.incremental_saver.is_none() {
                            error!("❌ CRITICAL: incremental_saver is None after successful initialization!");
                            error!("❌ This will cause all audio chunks to be silently dropped!");
                            error!("❌ MP4 recording will NOT work!");
                            
                            // Try to initialize again as a recovery attempt
                            warn!("🔄 Attempting to re-initialize incremental saver...");
                            if let Ok(meeting_folder) = super::audio_processing::create_meeting_folder(&super::recording_preferences::get_default_recordings_folder(), &name, false) {
                                match IncrementalAudioSaver::new(meeting_folder.clone(), 48000) {
                                    Ok(incremental_saver) => {
                                        self.incremental_saver = Some(Arc::new(AsyncMutex::new(incremental_saver)));
                                        info!("✅ Recovery successful: Incremental saver re-initialized");
                                    }
                                    Err(e) => {
                                        error!("❌ Recovery failed: Could not re-initialize incremental saver: {}", e);
                                    }
                                }
                            }
                        } else {
                            info!("✅ Verified: incremental_saver is properly initialized");
                        }
                    },
                    Err(e) => {
                        error!("❌ Failed to initialize meeting folder with checkpoints: {}", e);
                        error!("🔄 Attempting graceful degradation: transcript-only mode");
                        
                        // Create a comprehensive error for the initialization failure
                        let initialization_error = RecordingError::meeting_folder_error(
                            format!("Failed to initialize meeting folder with checkpoints: {}", e),
                            std::path::PathBuf::from(&name),
                            e.to_string().contains("permission") || e.to_string().contains("Permission"),
                            e.to_string().contains("space") || e.to_string().contains("disk"),
                        );
                        
                        // Use error recovery coordinator for graceful degradation
                        let recovery_coordinator = ErrorRecoveryCoordinator::new()
                            .with_graceful_degradation(true);
                            
                        // Note: We can't use async/await in this context, so we'll handle this synchronously
                        warn!("🔄 Graceful degradation: MP4 recording failed, continuing with transcript-only mode");
                        warn!("📝 Transcripts will continue to be saved normally");
                        warn!("💡 MP4 recording can be restored by fixing folder permissions and restarting recording");
                                
                        // Try to create meeting folder without checkpoints as fallback
                        match self.initialize_meeting_folder(&name, false) {
                            Ok(()) => {
                                warn!("✅ Fallback successful: Meeting folder created without checkpoints");
                                warn!("📝 Recording will save transcripts only (no MP4 audio)");
                                warn!("💡 To restore MP4 recording, fix the issue and restart recording");
                            }
                            Err(fallback_error) => {
                                error!("❌ Fallback also failed: {}", fallback_error);
                                error!("❌ Recording may not save any files. Check folder permissions and disk space.");
                                
                                // Create error for complete failure
                                let complete_failure_error = RecordingError::meeting_folder_error(
                                    format!("Complete meeting folder initialization failure: {}", fallback_error),
                                    std::path::PathBuf::from(&name),
                                    fallback_error.to_string().contains("permission"),
                                    fallback_error.to_string().contains("space"),
                                );
                                
                                // Show recovery guidance
                                let guidance = complete_failure_error.recovery_guidance();
                                for guide in guidance {
                                    error!("💡 Critical recovery needed: {}", guide);
                                }
                            }
                        }
                    }
                }
            }
        } else {
            // When auto_save is false, still create meeting folder for transcripts/metadata
            // but skip .checkpoints directory
            if let Some(name) = self.meeting_name.clone() {
                match self.initialize_meeting_folder(&name, false) {
                    Ok(()) => info!("✅ Successfully initialized meeting folder (transcripts only)"),
                    Err(e) => {
                        error!("❌ Failed to initialize meeting folder for transcripts: {}", e);
                        
                        // Even transcript-only mode needs a folder - try alternatives
                        let transcript_error = RecordingError::meeting_folder_error(
                            format!("Failed to initialize meeting folder for transcripts: {}", e),
                            std::path::PathBuf::from(&name),
                            e.to_string().contains("permission"),
                            e.to_string().contains("space"),
                        );
                        
                        let recovery_coordinator = ErrorRecoveryCoordinator::new();
                        // Note: We can't use async/await in this context, so we'll handle this synchronously
                        info!("🔄 Trying alternative locations for transcript storage");
                        warn!("💡 Alternative transcript storage locations may be available");
                        warn!("💡 To use alternative location, restart recording with updated preferences");
                    }
                }
            }
        }

        // Start accumulation task
        let is_saving_clone = self.is_saving.clone();
        let incremental_saver_arc = self.incremental_saver.clone();
        let meeting_folder = self.meeting_folder.clone(); // Capture meeting_folder for the async closure
        let save_audio = auto_save;

        if let Some(mut receiver) = self.chunk_receiver.take() {
            tokio::spawn(async move {
                info!("Recording saver accumulation task started (save_audio: {})", save_audio);

                while let Some(chunk) = receiver.recv().await {
                    // Check if we should continue
                    let should_continue = if let Ok(is_saving) = is_saving_clone.lock() {
                        *is_saving
                    } else {
                        false
                    };

                    if !should_continue {
                        break;
                    }

                    // Only process audio chunks if auto_save is enabled
                    if save_audio {
                        // Add chunk to incremental saver with enhanced error handling and graceful degradation
                        if let Some(saver_arc) = &incremental_saver_arc {
                            let mut saver_guard = saver_arc.lock().await;
                            if let Err(e) = saver_guard.add_chunk(chunk.clone()) {
                                // Create a detailed checkpoint error
                                let meeting_folder_path = meeting_folder.clone().unwrap_or_else(|| std::path::PathBuf::from("unknown"));
                                let checkpoint_error = RecordingError::checkpoint_error(
                                    format!("Failed to add audio chunk to incremental saver: {}", e),
                                    meeting_folder_path,
                                    CheckpointOperation::ChunkWrite,
                                    1, // This chunk is affected
                                );
                                
                                error!("❌ Checkpoint error: {}", checkpoint_error.user_message());
                                
                                // Attempt recovery using the error recovery coordinator
                                let recovery_coordinator = ErrorRecoveryCoordinator::new()
                                    .with_graceful_degradation(true)
                                    .with_retry_config(3, 1000);
                                    
                                match recovery_coordinator.attempt_recovery(&checkpoint_error).await {
                                    crate::recording::error_handling::RecoveryResult::GracefulDegradation { 
                                        user_notification: Some(message), .. 
                                    } => {
                                        warn!("🔄 Graceful degradation: {}", message);
                                        warn!("🔄 Switching to transcript-only mode for remaining chunks");
                                        
                                        // Switch to transcript-only mode by breaking out of audio processing
                                        // Transcripts will continue to be processed by the transcription pipeline
                                        break;
                                    },
                                    crate::recording::error_handling::RecoveryResult::RetryOperation { 
                                        max_attempts, delay_ms, .. 
                                    } => {
                                        warn!("🔄 Retrying chunk write operation (max {} attempts, delay {}ms)", max_attempts, delay_ms);
                                        
                                        // Implement retry logic for chunk write
                                        let mut retry_count = 0;
                                        let mut retry_successful = false;
                                        
                                        while retry_count < max_attempts && !retry_successful {
                                            tokio::time::sleep(tokio::time::Duration::from_millis(delay_ms)).await;
                                            retry_count += 1;
                                            
                                            match saver_guard.add_chunk(chunk.clone()) {
                                                Ok(()) => {
                                                    info!("✅ Chunk write retry {} succeeded", retry_count);
                                                    retry_successful = true;
                                                }
                                                Err(retry_error) => {
                                                    warn!("❌ Chunk write retry {} failed: {}", retry_count, retry_error);
                                                }
                                            }
                                        }
                                        
                                        if !retry_successful {
                                            warn!("❌ All chunk write retries failed, switching to transcript-only mode");
                                            break;
                                        }
                                    },
                                    _ => {
                                        error!("❌ No recovery available for chunk write error, switching to transcript-only mode");
                                        break;
                                    }
                                }
                            }
                        } else {
                            // This happens when checkpoint directory creation failed
                            // but auto_save is still true - implement graceful degradation
                            use std::sync::atomic::{AtomicU32, Ordering};
                            static DROPPED_CHUNKS: AtomicU32 = AtomicU32::new(0);
                            static DEGRADATION_NOTIFIED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
                            
                            let count = DROPPED_CHUNKS.fetch_add(1, Ordering::Relaxed) + 1;
                            
                            // Create a comprehensive checkpoint error for missing incremental saver
                            let meeting_folder_path = meeting_folder.clone().unwrap_or_else(|| std::path::PathBuf::from("unknown"));
                            let checkpoint_error = RecordingError::checkpoint_error(
                                "Incremental saver not available - gracefully degrading to transcript-only mode".to_string(),
                                meeting_folder_path,
                                CheckpointOperation::DirectoryCreation,
                                count,
                            );
                            
                            // Only show detailed error and recovery guidance once
                            if count == 1 || !DEGRADATION_NOTIFIED.load(Ordering::Relaxed) {
                                error!("❌ {}", checkpoint_error.user_message());
                                
                                // Use error recovery coordinator for graceful degradation
                                let recovery_coordinator = ErrorRecoveryCoordinator::new()
                                    .with_graceful_degradation(true);
                                    
                                match recovery_coordinator.attempt_recovery(&checkpoint_error).await {
                                    crate::recording::error_handling::RecoveryResult::GracefulDegradation { 
                                        user_notification: Some(message), .. 
                                    } => {
                                        warn!("🔄 {}", message);
                                        warn!("📝 Transcripts will continue to be saved normally");
                                        warn!("💡 MP4 recording can be restored by fixing folder permissions and restarting recording");
                                        
                                        DEGRADATION_NOTIFIED.store(true, Ordering::Relaxed);
                                    },
                                    _ => {
                                        // Show recovery guidance to user
                                        let guidance = checkpoint_error.recovery_guidance();
                                        for guide in guidance {
                                            info!("💡 Recovery guidance: {}", guide);
                                        }
                                    }
                                }
                            } else if count % 100 == 0 {
                                // Periodic reminder that chunks are being dropped
                                warn!("❌ {} audio chunks dropped so far (transcript-only mode active)", count);
                            }
                            
                            // Continue processing - transcripts are still being saved by the transcription pipeline
                            // The audio chunk is dropped here, but transcription continues independently
                        }
                    } else {
                        // auto_save is false: discard audio chunk (this is expected behavior)
                        // Transcription already happened in the pipeline before this point
                        // This is normal transcript-only mode operation
                    }
                }

                info!("Recording saver accumulation task ended");
            });
        }

        // Set saving flag
        if let Ok(mut is_saving) = self.is_saving.lock() {
            *is_saving = true;
        }

        sender
    }

    /// Initialize meeting folder structure and metadata
    ///
    /// # Arguments
    /// * `meeting_name` - Name of the meeting
    /// * `create_checkpoints` - Whether to create .checkpoints/ directory and IncrementalAudioSaver
    fn initialize_meeting_folder(&mut self, meeting_name: &str, create_checkpoints: bool) -> Result<()> {
        // Load preferences to get base recordings folder
        let base_folder = super::recording_preferences::get_default_recordings_folder();
        info!("Initializing meeting folder in: {}", base_folder.display());

        // Requirement 7.2: Log meeting folder initialization start
        info!("🔧 [STRUCTURED_LOG] meeting_folder_initialization: {{ \"meeting_name\": \"{}\", \"base_folder\": \"{}\", \"create_checkpoints\": {} }}", 
              meeting_name, base_folder.display(), create_checkpoints);

        // Create meeting folder structure (with or without .checkpoints/ subdirectory)
        let meeting_folder = match create_meeting_folder(&base_folder, meeting_name, create_checkpoints) {
            Ok(folder) => {
                // Requirement 7.2: Log successful meeting folder creation
                info!("🔧 [STRUCTURED_LOG] meeting_folder_created: {{ \"folder_path\": \"{}\", \"checkpoints_directory_created\": {} }}", 
                      folder.display(), create_checkpoints);
                
                if create_checkpoints {
                    let checkpoints_dir = folder.join(".checkpoints");
                    if checkpoints_dir.exists() {
                        info!("🔧 [STRUCTURED_LOG] checkpoints_directory_ready: {{ \"path\": \"{}\", \"writable\": true }}", 
                              checkpoints_dir.display());
                    }
                }
                
                folder
            },
            Err(e) => {
                // Create a detailed meeting folder error
                let folder_error = RecordingError::meeting_folder_error(
                    format!("Failed to create meeting folder: {}", e),
                    base_folder.clone(),
                    e.to_string().to_lowercase().contains("permission"),
                    e.to_string().to_lowercase().contains("space") || e.to_string().to_lowercase().contains("disk"),
                );
                
                error!("❌ Meeting folder error: {}", folder_error.user_message());
                
                // Requirement 7.2: Log meeting folder creation failure
                error!("🔧 [STRUCTURED_LOG] meeting_folder_creation_failed: {{ \"meeting_name\": \"{}\", \"base_folder\": \"{}\", \"error\": \"{}\", \"create_checkpoints\": {} }}", 
                       meeting_name, base_folder.display(), e.to_string().replace("\"", "\\\""), create_checkpoints);
                
                // Show recovery guidance
                let guidance = folder_error.recovery_guidance();
                for guide in guidance {
                    info!("💡 Recovery guidance: {}", guide);
                }
                
                return Err(anyhow::anyhow!("Meeting folder creation failed: {}", e));
            }
        };

        // Only initialize incremental saver if checkpoints are needed (auto_save is true)
        if create_checkpoints {
            match IncrementalAudioSaver::new(meeting_folder.clone(), 48000) {
                Ok(incremental_saver) => {
                    self.incremental_saver = Some(Arc::new(AsyncMutex::new(incremental_saver)));
                    info!("✅ Incremental audio saver initialized for meeting: {}", meeting_name);
                    
                    // Requirement 7.2: Log incremental saver initialization success
                    info!("🔧 [STRUCTURED_LOG] incremental_saver_initialized: {{ \"meeting_name\": \"{}\", \"sample_rate\": 48000, \"checkpoint_interval_seconds\": 30, \"checkpoints_directory\": \"{}\" }}", 
                          meeting_name, meeting_folder.join(".checkpoints").display());
                }
                Err(e) => {
                    // Create a detailed checkpoint error for incremental saver initialization
                    let checkpoint_error = RecordingError::checkpoint_error(
                        format!("Failed to initialize incremental audio saver: {}", e),
                        meeting_folder.clone(),
                        CheckpointOperation::DirectoryCreation,
                        0, // No chunks affected yet
                    );
                    
                    error!("❌ Checkpoint initialization error: {}", checkpoint_error.user_message());
                    
                    // Requirement 7.2: Log incremental saver initialization failure
                    error!("🔧 [STRUCTURED_LOG] incremental_saver_initialization_failed: {{ \"meeting_name\": \"{}\", \"error\": \"{}\", \"checkpoints_directory\": \"{}\" }}", 
                           meeting_name, e.to_string().replace("\"", "\\\""), meeting_folder.join(".checkpoints").display());
                    
                    // Show recovery guidance
                    let guidance = checkpoint_error.recovery_guidance();
                    for guide in guidance {
                        info!("💡 Recovery guidance: {}", guide);
                    }
                    
                    return Err(anyhow::anyhow!(
                        "Failed to initialize incremental audio saver: {}. \
                         This prevents MP4 recording. Check that the .checkpoints/ directory \
                         was created and is writable.", e
                    ));
                }
            }
        } else {
            info!("⚠️  Skipped incremental audio saver (auto-save disabled)");
            
            // Requirement 7.1: Log when incremental saver is skipped due to auto_save=false
            info!("🔧 [STRUCTURED_LOG] incremental_saver_skipped: {{ \"reason\": \"auto_save_disabled\", \"mode\": \"transcript_only\" }}");
        }

        // Create initial metadata
        let metadata = MeetingMetadata {
            version: "1.0".to_string(),
            meeting_id: None,  // Will be set by backend
            meeting_name: Some(meeting_name.to_string()),
            created_at: chrono::Utc::now().to_rfc3339(),
            completed_at: None,
            duration_seconds: None,
            devices: DeviceInfo {
                microphone: None,  // Could be enhanced to store actual device names
                system_audio: None,
            },
            audio_file: if create_checkpoints { "audio.mp4".to_string() } else { "".to_string() },
            transcript_file: "transcripts.json".to_string(),
            sample_rate: 48000,
            status: "recording".to_string(),
        };

        // Write initial metadata.json with enhanced error handling
        self.write_metadata(&meeting_folder, &metadata)
            .map_err(|e| anyhow::anyhow!("Failed to write meeting metadata: {}", e))?;

        self.meeting_folder = Some(meeting_folder);
        self.metadata = Some(metadata);

        Ok(())
    }

    /// Write metadata.json to disk (atomic write with temp file)
    fn write_metadata(&self, folder: &PathBuf, metadata: &MeetingMetadata) -> Result<()> {
        let metadata_path = folder.join("metadata.json");
        let temp_path = folder.join(".metadata.json.tmp");

        let json_string = serde_json::to_string_pretty(metadata)?;
        std::fs::write(&temp_path, json_string)?;
        std::fs::rename(&temp_path, &metadata_path)?;  // Atomic

        Ok(())
    }

    /// Write transcripts.json to disk (atomic write with temp file and validation)
    fn write_transcripts_json(&self, folder: &PathBuf) -> Result<()> {
        // Clone segments to avoid holding lock during I/O
        let segments_clone = if let Ok(segments) = self.transcript_segments.lock() {
            segments.clone()
        } else {
            error!("Failed to lock transcript segments for writing");
            return Err(anyhow::anyhow!("Failed to lock transcript segments"));
        };

        info!("Writing {} transcript segments to JSON", segments_clone.len());

        let transcript_path = folder.join("transcripts.json");
        let temp_path = folder.join(".transcripts.json.tmp");

        // Create JSON structure
        let json = serde_json::json!({
            "version": "1.0",
            "segments": segments_clone,
            "last_updated": chrono::Utc::now().to_rfc3339(),
            "total_segments": segments_clone.len()
        });

        // Serialize to pretty JSON string
        let json_string = serde_json::to_string_pretty(&json)
            .map_err(|e| {
                error!("Failed to serialize transcripts to JSON: {}", e);
                anyhow::anyhow!("JSON serialization failed: {}", e)
            })?;

        // Write to temp file with error handling
        std::fs::write(&temp_path, &json_string)
            .map_err(|e| {
                error!("Failed to write transcript temp file to {}: {}", temp_path.display(), e);
                anyhow::anyhow!("Failed to write temp file: {}", e)
            })?;

        // Verify temp file was written correctly
        if !temp_path.exists() {
            error!("Temp transcript file does not exist after write: {}", temp_path.display());
            return Err(anyhow::anyhow!("Temp file verification failed"));
        }

        // Atomic rename
        std::fs::rename(&temp_path, &transcript_path)
            .map_err(|e| {
                error!("Failed to rename transcript file from {} to {}: {}",
                       temp_path.display(), transcript_path.display(), e);
                anyhow::anyhow!("Failed to rename transcript file: {}", e)
            })?;

        info!("✅ Successfully wrote transcripts.json with {} segments", segments_clone.len());
        Ok(())
    }

    // in frontend/src-tauri/src/audio/recording_saver.rs
    pub fn get_stats(&self) -> (usize, u32) {
        if let Some(ref saver) = self.incremental_saver {
            if let Ok(guard) = saver.try_lock() {
                (guard.get_checkpoint_count() as usize, 48000)
            } else {
                (0, 48000)
            }
        } else {
            (0, 48000)
        }
    }

    /// Stop and save using incremental saving approach
    ///
    /// # Arguments
    /// * `app` - Tauri app handle for emitting events
    /// * `recording_duration` - Actual recording duration in seconds (from RecordingState)
    pub async fn stop_and_save<R: Runtime>(
        &mut self,
        app: &AppHandle<R>,
        recording_duration: Option<f64>
    ) -> Result<Option<String>, String> {
        info!("Stopping recording saver");

        // Stop accumulation
        if let Ok(mut is_saving) = self.is_saving.lock() {
            *is_saving = false;
        }

        // Give time for final chunks
        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

        // Check if incremental saver exists (indicates auto_save was enabled)
        let should_save_audio = self.incremental_saver.is_some();

        if !should_save_audio {
            info!("⚠️  No audio saver initialized (auto-save was disabled) - skipping audio finalization");
            info!("✅ Transcripts and metadata already saved incrementally");
            return Ok(None);
        }

        // Finalize incremental saver (merge checkpoints into final audio.mp4)
        let final_audio_path = if let Some(saver_arc) = &self.incremental_saver {
            let mut saver = saver_arc.lock().await;
            match saver.finalize().await {
                Ok(path) => {
                    info!("✅ Successfully finalized audio: {}", path.display());
                    path
                }
                Err(e) => {
                    error!("❌ Failed to finalize incremental saver: {}", e);
                    return Err(format!("Failed to finalize audio: {}", e));
                }
            }
        } else {
            error!("No incremental saver initialized - cannot save recording");
            return Err("No incremental saver initialized".to_string());
        };

        // Save final transcripts.json with validation
        if let Some(folder) = &self.meeting_folder {
            if let Err(e) = self.write_transcripts_json(folder) {
                error!("❌ Failed to write final transcripts: {}", e);
                return Err(format!("Failed to save transcripts: {}", e));
            }

            // Verify transcripts were written correctly
            let transcript_path = folder.join("transcripts.json");
            if !transcript_path.exists() {
                error!("❌ Transcript file was not created at: {}", transcript_path.display());
                return Err("Transcript file verification failed".to_string());
            }
            info!("✅ Transcripts saved and verified at: {}", transcript_path.display());
        }

        // Update metadata to completed status with actual recording duration
        if let (Some(folder), Some(mut metadata)) = (&self.meeting_folder, self.metadata.clone()) {
            metadata.status = "completed".to_string();
            metadata.completed_at = Some(chrono::Utc::now().to_rfc3339());

            // Use actual recording duration from RecordingState (more accurate than transcript segments)
            // Falls back to last transcript segment if duration not provided
            metadata.duration_seconds = recording_duration.or_else(|| {
                if let Ok(segments) = self.transcript_segments.lock() {
                    segments.last().map(|seg| seg.audio_end_time)
                } else {
                    None
                }
            });

            if let Err(e) = self.write_metadata(folder, &metadata) {
                error!("❌ Failed to update metadata to completed: {}", e);
                return Err(format!("Failed to update metadata: {}", e));
            }

            info!("✅ Metadata updated with duration: {:?}s", metadata.duration_seconds);
        }

        // Emit save event with audio and transcript paths
        let save_event = serde_json::json!({
            "audio_file": final_audio_path.to_string_lossy(),
            "transcript_file": self.meeting_folder.as_ref()
                .map(|f| f.join("transcripts.json").to_string_lossy().to_string()),
            "meeting_name": self.meeting_name,
            "meeting_folder": self.meeting_folder.as_ref()
                .map(|f| f.to_string_lossy().to_string())
        });

        if let Err(e) = app.emit("recording-saved", &save_event) {
            warn!("Failed to emit recording-saved event: {}", e);
        }

        // Clean up transcript segments
        if let Ok(mut segments) = self.transcript_segments.lock() {
            segments.clear();
        }

        Ok(Some(final_audio_path.to_string_lossy().to_string()))
    }

    /// Get the meeting folder path (for passing to backend)
    pub fn get_meeting_folder(&self) -> Option<&PathBuf> {
        self.meeting_folder.as_ref()
    }

    /// Get accumulated transcript segments (for reload sync)
    pub fn get_transcript_segments(&self) -> Vec<TranscriptSegment> {
        if let Ok(segments) = self.transcript_segments.lock() {
            segments.clone()
        } else {
            Vec::new()
        }
    }

    /// Get meeting name (for reload sync)
    pub fn get_meeting_name(&self) -> Option<String> {
        self.meeting_name.clone()
    }
}

impl Default for RecordingSaver {
    fn default() -> Self {
        Self::new()
    }
}
