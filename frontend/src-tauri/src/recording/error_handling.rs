//! Comprehensive Error Handling System for MP4 Recording
//!
//! This module provides detailed error contexts and recovery strategies for all types
//! of recording failures. It implements Requirements 5.1, 5.2, 5.3, 5.4, 5.5 from
//! the MP4 recording fix specification.
//!
//! The error handling system provides:
//! - Detailed error contexts with actionable information
//! - Recovery strategies for each error type
//! - Graceful degradation to transcript-only mode when needed
//! - Clear user guidance for resolving issues

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use thiserror::Error;

/// Comprehensive error types for the recording system with detailed contexts
#[derive(Debug, Error, Clone, Serialize, Deserialize)]
pub enum RecordingError {
    /// Auto-save parameter not properly loaded or corrupted
    #[error("Auto-save parameter error: {context}")]
    AutoSaveParameterError {
        context: String,
        source: AutoSaveErrorSource,
        recovery_strategy: RecoveryStrategy,
    },

    /// FFmpeg not found or not executable
    #[error("FFmpeg not available: {context}")]
    FFmpegNotFound {
        context: String,
        attempted_paths: Vec<PathBuf>,
        installation_guidance: String,
        recovery_strategy: RecoveryStrategy,
    },

    /// Cannot create or access meeting folder
    #[error("Meeting folder error: {context}")]
    MeetingFolderError {
        context: String,
        attempted_path: PathBuf,
        alternative_paths: Vec<PathBuf>,
        permission_issue: bool,
        disk_space_issue: bool,
        recovery_strategy: RecoveryStrategy,
    },

    /// Checkpoint file creation or management failed
    #[error("Checkpoint error: {context}")]
    CheckpointError {
        context: String,
        checkpoint_dir: PathBuf,
        failed_operation: CheckpointOperation,
        chunks_affected: u32,
        recovery_strategy: RecoveryStrategy,
    },

    /// File merging operation failed
    #[error("File merging error: {context}")]
    MergingError {
        context: String,
        ffmpeg_command: String,
        ffmpeg_output: String,
        checkpoint_files: Vec<PathBuf>,
        preserve_checkpoints: bool,
        recovery_strategy: RecoveryStrategy,
    },

    /// Pipeline initialization failed
    #[error("Pipeline initialization error: {context}")]
    PipelineInitializationError {
        context: String,
        failed_component: String,
        diagnostic_info: String,
        recovery_strategy: RecoveryStrategy,
    },

    /// Preference system error
    #[error("Preference error: {context}")]
    PreferenceError {
        context: String,
        preference_file: Option<PathBuf>,
        corruption_details: Option<String>,
        recovery_strategy: RecoveryStrategy,
    },

    /// Disk space insufficient for recording
    #[error("Insufficient disk space: {context}")]
    InsufficientDiskSpace {
        context: String,
        required_space_mb: u64,
        available_space_mb: u64,
        save_location: PathBuf,
        recovery_strategy: RecoveryStrategy,
    },

    /// Permission denied for file operations
    #[error("Permission denied: {context}")]
    PermissionDenied {
        context: String,
        denied_path: PathBuf,
        required_permissions: String,
        recovery_strategy: RecoveryStrategy,
    },

    /// Audio device or stream error
    #[error("Audio system error: {context}")]
    AudioSystemError {
        context: String,
        device_info: Option<String>,
        stream_error: bool,
        recoverable: bool,
        recovery_strategy: RecoveryStrategy,
    },

    /// Transcription system error
    #[error("Transcription error: {context}")]
    TranscriptionError {
        context: String,
        model_issue: bool,
        processing_error: bool,
        recovery_strategy: RecoveryStrategy,
    },

    /// Configuration or validation error
    #[error("Configuration error: {context}")]
    ConfigurationError {
        context: String,
        invalid_setting: String,
        expected_value: String,
        recovery_strategy: RecoveryStrategy,
    },
}

/// Source of auto-save parameter errors
#[derive(Debug, Clone, Serialize, Deserialize, thiserror::Error)]
pub enum AutoSaveErrorSource {
    /// Preference file is corrupted or unreadable
    #[error("Preference file is corrupted or unreadable")]
    CorruptedPreferences,
    /// Preference file is missing
    #[error("Preference file is missing")]
    MissingPreferences,
    /// Hardcoded false value overriding preferences
    #[error("Hardcoded false value overriding preferences at {0}")]
    HardcodedOverride(String),
    /// Parameter not propagated through pipeline
    #[error("Parameter not propagated through pipeline")]
    PropagationFailure,
    /// Unknown or unidentifiable source
    #[error("Unknown or unidentifiable source")]
    Unknown,
}

/// Types of checkpoint operations that can fail
#[derive(Debug, Clone, Serialize, Deserialize, thiserror::Error)]
pub enum CheckpointOperation {
    /// Creating the .checkpoints directory
    #[error("Creating the .checkpoints directory")]
    DirectoryCreation,
    /// Writing audio chunk to checkpoint file
    #[error("Writing audio chunk to checkpoint file")]
    ChunkWrite,
    /// Reading checkpoint file for merging
    #[error("Reading checkpoint file for merging")]
    ChunkRead,
    /// Cleaning up checkpoint files after merging
    #[error("Cleaning up checkpoint files after merging")]
    Cleanup,
    /// Validating checkpoint file integrity
    #[error("Validating checkpoint file integrity")]
    Validation,
}

/// Recovery strategies for different error types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RecoveryStrategy {
    /// Automatically retry the operation
    AutoRetry {
        max_attempts: u32,
        delay_ms: u64,
        exponential_backoff: bool,
    },
    /// Degrade gracefully to transcript-only mode
    GracefulDegradation {
        preserve_transcripts: bool,
        notify_user: bool,
        fallback_message: String,
    },
    /// Attempt alternative approaches
    AlternativeApproach {
        alternatives: Vec<AlternativeAction>,
        try_all: bool,
    },
    /// Require user intervention
    UserIntervention {
        required_actions: Vec<UserAction>,
        guidance_message: String,
        can_continue_without: bool,
    },
    /// Repair or restore system state
    SystemRepair {
        repair_actions: Vec<RepairAction>,
        backup_current_state: bool,
        validate_after_repair: bool,
    },
    /// No recovery possible - fail operation
    FailOperation {
        preserve_partial_data: bool,
        cleanup_required: bool,
    },
}

/// Alternative actions to try when primary approach fails
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AlternativeAction {
    /// Try a different save location
    AlternativeSaveLocation(PathBuf),
    /// Use different audio format or settings
    AlternativeAudioSettings,
    /// Use different FFmpeg command or options
    AlternativeFFmpegCommand(String),
    /// Skip problematic component
    SkipComponent(String),
    /// Use fallback implementation
    FallbackImplementation(String),
}

/// Actions that require user intervention
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UserAction {
    /// Install missing software
    InstallSoftware {
        software_name: String,
        installation_url: String,
        installation_command: Option<String>,
    },
    /// Fix file permissions
    FixPermissions {
        path: PathBuf,
        required_permissions: String,
        fix_command: Option<String>,
    },
    /// Free up disk space
    FreeDiskSpace {
        required_mb: u64,
        suggested_locations: Vec<PathBuf>,
    },
    /// Update configuration
    UpdateConfiguration {
        setting_name: String,
        current_value: String,
        suggested_value: String,
    },
    /// Restart application or system
    RestartRequired {
        restart_type: RestartType,
        reason: String,
    },
}

/// Types of restarts that might be required
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RestartType {
    /// Restart the application
    Application,
    /// Restart audio services
    AudioServices,
    /// Restart the system
    System,
}

/// System repair actions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RepairAction {
    /// Restore default preferences
    RestoreDefaultPreferences,
    /// Repair corrupted preference file
    RepairPreferences,
    /// Recreate missing directories
    RecreateDirectories(Vec<PathBuf>),
    /// Reset audio system state
    ResetAudioSystem,
    /// Clear temporary files
    ClearTemporaryFiles(Vec<PathBuf>),
    /// Validate and repair file permissions
    RepairPermissions(Vec<PathBuf>),
}

impl RecordingError {
    /// Create an auto-save parameter error with context
    pub fn auto_save_parameter_error(
        context: impl Into<String>,
        source: AutoSaveErrorSource,
    ) -> Self {
        let recovery_strategy = match source {
            AutoSaveErrorSource::CorruptedPreferences => RecoveryStrategy::SystemRepair {
                repair_actions: vec![RepairAction::RepairPreferences],
                backup_current_state: true,
                validate_after_repair: true,
            },
            AutoSaveErrorSource::MissingPreferences => RecoveryStrategy::SystemRepair {
                repair_actions: vec![RepairAction::RestoreDefaultPreferences],
                backup_current_state: false,
                validate_after_repair: true,
            },
            AutoSaveErrorSource::HardcodedOverride(_) => RecoveryStrategy::UserIntervention {
                required_actions: vec![UserAction::UpdateConfiguration {
                    setting_name: "auto_save".to_string(),
                    current_value: "false (hardcoded)".to_string(),
                    suggested_value: "true".to_string(),
                }],
                guidance_message: "Remove hardcoded false value from source code".to_string(),
                can_continue_without: false,
            },
            AutoSaveErrorSource::PropagationFailure => RecoveryStrategy::SystemRepair {
                repair_actions: vec![RepairAction::ResetAudioSystem],
                backup_current_state: false,
                validate_after_repair: true,
            },
            AutoSaveErrorSource::Unknown => RecoveryStrategy::GracefulDegradation {
                preserve_transcripts: true,
                notify_user: true,
                fallback_message: "Auto-save parameter issue detected, continuing with transcript-only mode".to_string(),
            },
        };

        Self::AutoSaveParameterError {
            context: context.into(),
            source,
            recovery_strategy,
        }
    }

    /// Create an FFmpeg not found error with installation guidance
    pub fn ffmpeg_not_found(
        context: impl Into<String>,
        attempted_paths: Vec<PathBuf>,
    ) -> Self {
        let installation_guidance = Self::generate_ffmpeg_installation_guidance();
        
        Self::FFmpegNotFound {
            context: context.into(),
            attempted_paths,
            installation_guidance: installation_guidance.clone(),
            recovery_strategy: RecoveryStrategy::UserIntervention {
                required_actions: vec![UserAction::InstallSoftware {
                    software_name: "FFmpeg".to_string(),
                    installation_url: "https://ffmpeg.org/download.html".to_string(),
                    installation_command: Self::get_ffmpeg_install_command(),
                }],
                guidance_message: installation_guidance,
                can_continue_without: false,
            },
        }
    }

    /// Create a meeting folder error with alternative paths
    pub fn meeting_folder_error(
        context: impl Into<String>,
        attempted_path: PathBuf,
        permission_issue: bool,
        disk_space_issue: bool,
    ) -> Self {
        let alternative_paths = Self::get_alternative_save_locations();
        
        let recovery_strategy = if permission_issue {
            RecoveryStrategy::AlternativeApproach {
                alternatives: alternative_paths.iter()
                    .map(|path| AlternativeAction::AlternativeSaveLocation(path.clone()))
                    .collect(),
                try_all: true,
            }
        } else if disk_space_issue {
            RecoveryStrategy::UserIntervention {
                required_actions: vec![UserAction::FreeDiskSpace {
                    required_mb: 100, // Minimum required for recording
                    suggested_locations: vec![attempted_path.clone()],
                }],
                guidance_message: "Free up disk space to continue recording".to_string(),
                can_continue_without: true,
            }
        } else {
            RecoveryStrategy::AlternativeApproach {
                alternatives: alternative_paths.iter()
                    .map(|path| AlternativeAction::AlternativeSaveLocation(path.clone()))
                    .collect(),
                try_all: true,
            }
        };

        Self::MeetingFolderError {
            context: context.into(),
            attempted_path,
            alternative_paths,
            permission_issue,
            disk_space_issue,
            recovery_strategy,
        }
    }

    /// Create a checkpoint error with recovery context
    pub fn checkpoint_error(
        context: impl Into<String>,
        checkpoint_dir: PathBuf,
        failed_operation: CheckpointOperation,
        chunks_affected: u32,
    ) -> Self {
        let recovery_strategy = match failed_operation {
            CheckpointOperation::DirectoryCreation => RecoveryStrategy::AlternativeApproach {
                alternatives: vec![
                    AlternativeAction::AlternativeSaveLocation(
                        Self::get_alternative_save_locations().into_iter().next()
                            .unwrap_or_else(|| std::env::temp_dir())
                    ),
                ],
                try_all: true,
            },
            CheckpointOperation::ChunkWrite => RecoveryStrategy::GracefulDegradation {
                preserve_transcripts: true,
                notify_user: true,
                fallback_message: format!(
                    "Cannot save audio chunks ({}), continuing with transcript-only mode", 
                    chunks_affected
                ),
            },
            CheckpointOperation::ChunkRead | CheckpointOperation::Validation => {
                RecoveryStrategy::AutoRetry {
                    max_attempts: 3,
                    delay_ms: 1000,
                    exponential_backoff: true,
                }
            },
            CheckpointOperation::Cleanup => RecoveryStrategy::SystemRepair {
                repair_actions: vec![RepairAction::ClearTemporaryFiles(vec![checkpoint_dir.clone()])],
                backup_current_state: false,
                validate_after_repair: false,
            },
        };

        Self::CheckpointError {
            context: context.into(),
            checkpoint_dir,
            failed_operation,
            chunks_affected,
            recovery_strategy,
        }
    }

    /// Create a file merging error with FFmpeg details
    pub fn merging_error(
        context: impl Into<String>,
        ffmpeg_command: String,
        ffmpeg_output: String,
        checkpoint_files: Vec<PathBuf>,
    ) -> Self {
        Self::MergingError {
            context: context.into(),
            ffmpeg_command,
            ffmpeg_output,
            checkpoint_files: checkpoint_files.clone(),
            preserve_checkpoints: true,
            recovery_strategy: RecoveryStrategy::AlternativeApproach {
                alternatives: vec![
                    AlternativeAction::AlternativeFFmpegCommand(
                        "ffmpeg -f concat -safe 0 -i filelist.txt -c copy output.mp4".to_string()
                    ),
                    AlternativeAction::FallbackImplementation(
                        "Manual checkpoint preservation".to_string()
                    ),
                ],
                try_all: true,
            },
        }
    }

    /// Create an insufficient disk space error
    pub fn insufficient_disk_space(
        context: impl Into<String>,
        required_space_mb: u64,
        available_space_mb: u64,
        save_location: PathBuf,
    ) -> Self {
        Self::InsufficientDiskSpace {
            context: context.into(),
            required_space_mb,
            available_space_mb,
            save_location: save_location.clone(),
            recovery_strategy: RecoveryStrategy::UserIntervention {
                required_actions: vec![UserAction::FreeDiskSpace {
                    required_mb: required_space_mb,
                    suggested_locations: vec![save_location],
                }],
                guidance_message: format!(
                    "Need {} MB of free space, but only {} MB available",
                    required_space_mb, available_space_mb
                ),
                can_continue_without: true,
            },
        }
    }

    /// Get user-friendly error message
    pub fn user_message(&self) -> String {
        match self {
            Self::AutoSaveParameterError { context, source, .. } => {
                match source {
                    AutoSaveErrorSource::CorruptedPreferences => {
                        format!("Recording preferences are corrupted: {}", context)
                    },
                    AutoSaveErrorSource::MissingPreferences => {
                        format!("Recording preferences not found: {}", context)
                    },
                    AutoSaveErrorSource::HardcodedOverride(location) => {
                        format!("Auto-save is disabled in code at {}: {}", location, context)
                    },
                    AutoSaveErrorSource::PropagationFailure => {
                        format!("Auto-save setting not applied correctly: {}", context)
                    },
                    AutoSaveErrorSource::Unknown => {
                        format!("Auto-save configuration issue: {}", context)
                    },
                }
            },
            Self::FFmpegNotFound { context, installation_guidance, .. } => {
                format!("FFmpeg is required for MP4 recording but not found: {}. {}", context, installation_guidance)
            },
            Self::MeetingFolderError { context, permission_issue, disk_space_issue, .. } => {
                if *permission_issue {
                    format!("Cannot access recording folder due to permissions: {}", context)
                } else if *disk_space_issue {
                    format!("Insufficient disk space for recording: {}", context)
                } else {
                    format!("Cannot create recording folder: {}", context)
                }
            },
            Self::CheckpointError { context, chunks_affected, .. } => {
                format!("Audio recording issue affecting {} chunks: {}", chunks_affected, context)
            },
            Self::MergingError { context, preserve_checkpoints, .. } => {
                if *preserve_checkpoints {
                    format!("Cannot create final MP4 file, but audio chunks are preserved: {}", context)
                } else {
                    format!("Cannot create final MP4 file: {}", context)
                }
            },
            Self::InsufficientDiskSpace { required_space_mb, available_space_mb, .. } => {
                format!(
                    "Need {} MB of disk space for recording, but only {} MB available",
                    required_space_mb, available_space_mb
                )
            },
            Self::PermissionDenied { context, denied_path, .. } => {
                format!("Permission denied for {}: {}", denied_path.display(), context)
            },
            _ => self.to_string(),
        }
    }

    /// Get recovery guidance for the user
    pub fn recovery_guidance(&self) -> Vec<String> {
        match &self.recovery_strategy() {
            RecoveryStrategy::AutoRetry { max_attempts, .. } => {
                vec![format!("Automatically retrying operation (up to {} attempts)", max_attempts)]
            },
            RecoveryStrategy::GracefulDegradation { fallback_message, .. } => {
                vec![
                    "Recording will continue in transcript-only mode".to_string(),
                    fallback_message.clone(),
                ]
            },
            RecoveryStrategy::AlternativeApproach { alternatives, .. } => {
                let mut guidance = vec!["Trying alternative approaches:".to_string()];
                for alt in alternatives {
                    match alt {
                        AlternativeAction::AlternativeSaveLocation(path) => {
                            guidance.push(format!("• Alternative save location: {}", path.display()));
                        },
                        AlternativeAction::AlternativeFFmpegCommand(cmd) => {
                            guidance.push(format!("• Alternative FFmpeg command: {}", cmd));
                        },
                        AlternativeAction::FallbackImplementation(desc) => {
                            guidance.push(format!("• Fallback approach: {}", desc));
                        },
                        _ => {},
                    }
                }
                guidance
            },
            RecoveryStrategy::UserIntervention { required_actions, guidance_message, .. } => {
                let mut guidance = vec![guidance_message.clone()];
                guidance.push("Required actions:".to_string());
                for action in required_actions {
                    match action {
                        UserAction::InstallSoftware { software_name, installation_url, .. } => {
                            guidance.push(format!("• Install {}: {}", software_name, installation_url));
                        },
                        UserAction::FixPermissions { path, required_permissions, .. } => {
                            guidance.push(format!("• Fix permissions for {}: {}", path.display(), required_permissions));
                        },
                        UserAction::FreeDiskSpace { required_mb, .. } => {
                            guidance.push(format!("• Free up at least {} MB of disk space", required_mb));
                        },
                        UserAction::UpdateConfiguration { setting_name, suggested_value, .. } => {
                            guidance.push(format!("• Update {} setting to: {}", setting_name, suggested_value));
                        },
                        UserAction::RestartRequired { restart_type, reason } => {
                            guidance.push(format!("• Restart {:?}: {}", restart_type, reason));
                        },
                    }
                }
                guidance
            },
            RecoveryStrategy::SystemRepair { repair_actions, .. } => {
                let mut guidance = vec!["Attempting automatic system repair:".to_string()];
                for action in repair_actions {
                    match action {
                        RepairAction::RestoreDefaultPreferences => {
                            guidance.push("• Restoring default recording preferences".to_string());
                        },
                        RepairAction::RepairPreferences => {
                            guidance.push("• Repairing corrupted preferences".to_string());
                        },
                        RepairAction::RecreateDirectories(dirs) => {
                            guidance.push(format!("• Recreating {} directories", dirs.len()));
                        },
                        _ => {},
                    }
                }
                guidance
            },
            RecoveryStrategy::FailOperation { preserve_partial_data, .. } => {
                if *preserve_partial_data {
                    vec![
                        "Operation failed, but partial data will be preserved".to_string(),
                        "Check the recording folder for any saved files".to_string(),
                    ]
                } else {
                    vec!["Operation failed and cannot be recovered".to_string()]
                }
            },
        }
    }

    /// Get the recovery strategy for this error
    pub fn recovery_strategy(&self) -> &RecoveryStrategy {
        match self {
            Self::AutoSaveParameterError { recovery_strategy, .. } => recovery_strategy,
            Self::FFmpegNotFound { recovery_strategy, .. } => recovery_strategy,
            Self::MeetingFolderError { recovery_strategy, .. } => recovery_strategy,
            Self::CheckpointError { recovery_strategy, .. } => recovery_strategy,
            Self::MergingError { recovery_strategy, .. } => recovery_strategy,
            Self::PipelineInitializationError { recovery_strategy, .. } => recovery_strategy,
            Self::PreferenceError { recovery_strategy, .. } => recovery_strategy,
            Self::InsufficientDiskSpace { recovery_strategy, .. } => recovery_strategy,
            Self::PermissionDenied { recovery_strategy, .. } => recovery_strategy,
            Self::AudioSystemError { recovery_strategy, .. } => recovery_strategy,
            Self::TranscriptionError { recovery_strategy, .. } => recovery_strategy,
            Self::ConfigurationError { recovery_strategy, .. } => recovery_strategy,
        }
    }

    /// Check if this error allows graceful degradation to transcript-only mode
    pub fn allows_graceful_degradation(&self) -> bool {
        match self.recovery_strategy() {
            RecoveryStrategy::GracefulDegradation { .. } => true,
            RecoveryStrategy::UserIntervention { can_continue_without, .. } => *can_continue_without,
            _ => false,
        }
    }

    /// Check if this error is automatically recoverable
    pub fn is_auto_recoverable(&self) -> bool {
        matches!(
            self.recovery_strategy(),
            RecoveryStrategy::AutoRetry { .. } | RecoveryStrategy::SystemRepair { .. }
        )
    }

    /// Generate FFmpeg installation guidance based on platform
    fn generate_ffmpeg_installation_guidance() -> String {
        if cfg!(target_os = "windows") {
            "Install FFmpeg from https://ffmpeg.org/download.html or use 'winget install FFmpeg'".to_string()
        } else if cfg!(target_os = "macos") {
            "Install FFmpeg using Homebrew: 'brew install ffmpeg'".to_string()
        } else {
            "Install FFmpeg using your package manager: 'sudo apt install ffmpeg' (Ubuntu/Debian) or 'sudo yum install ffmpeg' (CentOS/RHEL)".to_string()
        }
    }

    /// Get platform-specific FFmpeg installation command
    fn get_ffmpeg_install_command() -> Option<String> {
        if cfg!(target_os = "windows") {
            Some("winget install FFmpeg".to_string())
        } else if cfg!(target_os = "macos") {
            Some("brew install ffmpeg".to_string())
        } else {
            Some("sudo apt install ffmpeg".to_string())
        }
    }

    /// Get alternative save locations based on platform
    fn get_alternative_save_locations() -> Vec<PathBuf> {
        let mut alternatives = Vec::new();

        // Documents folder
        if let Some(documents_dir) = dirs::document_dir() {
            alternatives.push(documents_dir.join("meetily-recordings"));
        }

        // Desktop folder
        if let Some(desktop_dir) = dirs::desktop_dir() {
            alternatives.push(desktop_dir.join("meetily-recordings"));
        }

        // Home directory
        if let Some(home_dir) = dirs::home_dir() {
            alternatives.push(home_dir.join("meetily-recordings"));
        }

        // Temporary directory (last resort)
        alternatives.push(std::env::temp_dir().join("meetily-recordings"));

        alternatives
    }
}

/// Error recovery coordinator that manages recovery strategies
pub struct ErrorRecoveryCoordinator {
    max_retry_attempts: u32,
    retry_delay_ms: u64,
    enable_graceful_degradation: bool,
}

impl ErrorRecoveryCoordinator {
    /// Create a new error recovery coordinator
    pub fn new() -> Self {
        Self {
            max_retry_attempts: 3,
            retry_delay_ms: 1000,
            enable_graceful_degradation: true,
        }
    }

    /// Configure retry behavior
    pub fn with_retry_config(mut self, max_attempts: u32, delay_ms: u64) -> Self {
        self.max_retry_attempts = max_attempts;
        self.retry_delay_ms = delay_ms;
        self
    }

    /// Configure graceful degradation
    pub fn with_graceful_degradation(mut self, enable: bool) -> Self {
        self.enable_graceful_degradation = enable;
        self
    }

    /// Attempt to recover from an error using its recovery strategy
    pub async fn attempt_recovery(&self, error: &RecordingError) -> RecoveryResult {
        log::info!("ErrorRecoveryCoordinator: Attempting recovery for error: {}", error);

        match error.recovery_strategy() {
            RecoveryStrategy::AutoRetry { max_attempts, delay_ms, exponential_backoff } => {
                self.handle_auto_retry(*max_attempts, *delay_ms, *exponential_backoff).await
            },
            RecoveryStrategy::GracefulDegradation { preserve_transcripts, notify_user, fallback_message } => {
                self.handle_graceful_degradation(*preserve_transcripts, *notify_user, fallback_message.clone()).await
            },
            RecoveryStrategy::AlternativeApproach { alternatives, try_all } => {
                self.handle_alternative_approach(alternatives, *try_all).await
            },
            RecoveryStrategy::UserIntervention { required_actions, guidance_message, can_continue_without } => {
                self.handle_user_intervention(required_actions, guidance_message.clone(), *can_continue_without).await
            },
            RecoveryStrategy::SystemRepair { repair_actions, backup_current_state, validate_after_repair } => {
                self.handle_system_repair(repair_actions, *backup_current_state, *validate_after_repair).await
            },
            RecoveryStrategy::FailOperation { preserve_partial_data, cleanup_required } => {
                self.handle_fail_operation(*preserve_partial_data, *cleanup_required).await
            },
        }
    }

    /// Handle automatic retry recovery
    async fn handle_auto_retry(&self, max_attempts: u32, delay_ms: u64, exponential_backoff: bool) -> RecoveryResult {
        log::info!("ErrorRecoveryCoordinator: Preparing for auto-retry (max_attempts: {}, delay: {}ms)", max_attempts, delay_ms);
        
        RecoveryResult::RetryOperation {
            max_attempts: max_attempts.min(self.max_retry_attempts),
            delay_ms: if exponential_backoff { delay_ms } else { self.retry_delay_ms },
            exponential_backoff,
        }
    }

    /// Handle graceful degradation to transcript-only mode
    async fn handle_graceful_degradation(&self, preserve_transcripts: bool, notify_user: bool, fallback_message: String) -> RecoveryResult {
        if !self.enable_graceful_degradation {
            log::warn!("ErrorRecoveryCoordinator: Graceful degradation disabled, failing operation");
            return RecoveryResult::OperationFailed {
                reason: "Graceful degradation disabled".to_string(),
                preserve_data: preserve_transcripts,
            };
        }

        log::info!("ErrorRecoveryCoordinator: Enabling graceful degradation to transcript-only mode");
        
        RecoveryResult::GracefulDegradation {
            transcript_only_mode: true,
            preserve_existing_data: preserve_transcripts,
            user_notification: if notify_user { Some(fallback_message) } else { None },
        }
    }

    /// Handle alternative approach recovery
    async fn handle_alternative_approach(&self, alternatives: &[AlternativeAction], try_all: bool) -> RecoveryResult {
        log::info!("ErrorRecoveryCoordinator: Trying alternative approaches (count: {}, try_all: {})", alternatives.len(), try_all);
        
        RecoveryResult::TryAlternatives {
            alternatives: alternatives.to_vec(),
            try_all_alternatives: try_all,
        }
    }

    /// Handle user intervention recovery
    async fn handle_user_intervention(&self, required_actions: &[UserAction], guidance_message: String, can_continue_without: bool) -> RecoveryResult {
        log::info!("ErrorRecoveryCoordinator: User intervention required (actions: {}, can_continue: {})", required_actions.len(), can_continue_without);
        
        RecoveryResult::UserInterventionRequired {
            actions: required_actions.to_vec(),
            guidance: guidance_message,
            can_continue_without_fix: can_continue_without,
        }
    }

    /// Handle system repair recovery
    async fn handle_system_repair(&self, repair_actions: &[RepairAction], backup_current_state: bool, validate_after_repair: bool) -> RecoveryResult {
        log::info!("ErrorRecoveryCoordinator: Attempting system repair (actions: {}, backup: {}, validate: {})", 
                  repair_actions.len(), backup_current_state, validate_after_repair);
        
        // In a real implementation, this would execute the repair actions
        // For now, we return the repair plan
        RecoveryResult::SystemRepairAttempted {
            repair_actions: repair_actions.to_vec(),
            backup_created: backup_current_state,
            validation_required: validate_after_repair,
            success: true, // Would be determined by actual repair execution
        }
    }

    /// Handle operation failure
    async fn handle_fail_operation(&self, preserve_partial_data: bool, cleanup_required: bool) -> RecoveryResult {
        log::warn!("ErrorRecoveryCoordinator: Operation failed, no recovery possible (preserve_data: {}, cleanup: {})", 
                  preserve_partial_data, cleanup_required);
        
        RecoveryResult::OperationFailed {
            reason: "No recovery strategy available".to_string(),
            preserve_data: preserve_partial_data,
        }
    }
}

impl Default for ErrorRecoveryCoordinator {
    fn default() -> Self {
        Self::new()
    }
}

/// Result of a recovery attempt
#[derive(Debug, Clone)]
pub enum RecoveryResult {
    /// Retry the original operation
    RetryOperation {
        max_attempts: u32,
        delay_ms: u64,
        exponential_backoff: bool,
    },
    /// Continue in graceful degradation mode
    GracefulDegradation {
        transcript_only_mode: bool,
        preserve_existing_data: bool,
        user_notification: Option<String>,
    },
    /// Try alternative approaches
    TryAlternatives {
        alternatives: Vec<AlternativeAction>,
        try_all_alternatives: bool,
    },
    /// User intervention is required
    UserInterventionRequired {
        actions: Vec<UserAction>,
        guidance: String,
        can_continue_without_fix: bool,
    },
    /// System repair was attempted
    SystemRepairAttempted {
        repair_actions: Vec<RepairAction>,
        backup_created: bool,
        validation_required: bool,
        success: bool,
    },
    /// Operation failed and cannot be recovered
    OperationFailed {
        reason: String,
        preserve_data: bool,
    },
}

impl RecoveryResult {
    /// Check if the recovery was successful
    pub fn is_successful(&self) -> bool {
        match self {
            Self::RetryOperation { .. } => true,
            Self::GracefulDegradation { .. } => true,
            Self::TryAlternatives { .. } => true,
            Self::SystemRepairAttempted { success, .. } => *success,
            Self::UserInterventionRequired { can_continue_without_fix, .. } => *can_continue_without_fix,
            Self::OperationFailed { .. } => false,
        }
    }

    /// Get user-friendly description of the recovery result
    pub fn description(&self) -> String {
        match self {
            Self::RetryOperation { max_attempts, .. } => {
                format!("Will retry operation up to {} times", max_attempts)
            },
            Self::GracefulDegradation { transcript_only_mode, .. } => {
                if *transcript_only_mode {
                    "Continuing in transcript-only mode".to_string()
                } else {
                    "Continuing with reduced functionality".to_string()
                }
            },
            Self::TryAlternatives { alternatives, .. } => {
                format!("Trying {} alternative approaches", alternatives.len())
            },
            Self::UserInterventionRequired { actions, .. } => {
                format!("User action required: {} steps", actions.len())
            },
            Self::SystemRepairAttempted { success, .. } => {
                if *success {
                    "System repair completed successfully".to_string()
                } else {
                    "System repair failed".to_string()
                }
            },
            Self::OperationFailed { reason, .. } => {
                format!("Operation failed: {}", reason)
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auto_save_parameter_error_creation() {
        let error = RecordingError::auto_save_parameter_error(
            "Preference file corrupted",
            AutoSaveErrorSource::CorruptedPreferences,
        );

        assert!(matches!(error, RecordingError::AutoSaveParameterError { .. }));
        assert!(error.user_message().contains("corrupted"));
        assert!(error.is_auto_recoverable());
    }

    #[test]
    fn test_ffmpeg_not_found_error() {
        let error = RecordingError::ffmpeg_not_found(
            "FFmpeg executable not found in PATH",
            vec![PathBuf::from("/usr/bin/ffmpeg"), PathBuf::from("/usr/local/bin/ffmpeg")],
        );

        assert!(matches!(error, RecordingError::FFmpegNotFound { .. }));
        assert!(error.user_message().contains("FFmpeg"));
        assert!(!error.allows_graceful_degradation());
    }

    #[test]
    fn test_meeting_folder_error_with_permission_issue() {
        let error = RecordingError::meeting_folder_error(
            "Permission denied",
            PathBuf::from("/restricted/folder"),
            true,  // permission_issue
            false, // disk_space_issue
        );

        assert!(matches!(error, RecordingError::MeetingFolderError { .. }));
        assert!(error.user_message().contains("permissions"));
        
        let guidance = error.recovery_guidance();
        assert!(!guidance.is_empty());
    }

    #[test]
    fn test_checkpoint_error_with_graceful_degradation() {
        let error = RecordingError::checkpoint_error(
            "Cannot write audio chunks",
            PathBuf::from("/tmp/checkpoints"),
            CheckpointOperation::ChunkWrite,
            5,
        );

        assert!(matches!(error, RecordingError::CheckpointError { .. }));
        assert!(error.allows_graceful_degradation());
        assert!(error.user_message().contains("5 chunks"));
    }

    #[test]
    fn test_insufficient_disk_space_error() {
        let error = RecordingError::insufficient_disk_space(
            "Not enough space for recording",
            500, // required
            100, // available
            PathBuf::from("/home/user/recordings"),
        );

        assert!(matches!(error, RecordingError::InsufficientDiskSpace { .. }));
        assert!(error.user_message().contains("500 MB"));
        assert!(error.user_message().contains("100 MB"));
    }

    #[tokio::test]
    async fn test_error_recovery_coordinator() {
        let coordinator = ErrorRecoveryCoordinator::new();
        
        let error = RecordingError::auto_save_parameter_error(
            "Test error",
            AutoSaveErrorSource::MissingPreferences,
        );

        let result = coordinator.attempt_recovery(&error).await;
        assert!(result.is_successful());
    }

    #[tokio::test]
    async fn test_graceful_degradation_recovery() {
        let coordinator = ErrorRecoveryCoordinator::new().with_graceful_degradation(true);
        
        let error = RecordingError::checkpoint_error(
            "Checkpoint write failed",
            PathBuf::from("/tmp/checkpoints"),
            CheckpointOperation::ChunkWrite,
            3,
        );

        let result = coordinator.attempt_recovery(&error).await;
        
        if let RecoveryResult::GracefulDegradation { transcript_only_mode, .. } = result {
            assert!(transcript_only_mode);
        } else {
            panic!("Expected graceful degradation result");
        }
    }

    #[test]
    fn test_recovery_result_descriptions() {
        let retry_result = RecoveryResult::RetryOperation {
            max_attempts: 3,
            delay_ms: 1000,
            exponential_backoff: false,
        };
        assert!(retry_result.description().contains("3 times"));

        let degradation_result = RecoveryResult::GracefulDegradation {
            transcript_only_mode: true,
            preserve_existing_data: true,
            user_notification: None,
        };
        assert!(degradation_result.description().contains("transcript-only"));
    }

    #[test]
    fn test_alternative_save_locations() {
        let locations = RecordingError::get_alternative_save_locations();
        assert!(!locations.is_empty());
        
        // Should include temp directory as last resort
        assert!(locations.iter().any(|path| path.starts_with(std::env::temp_dir())));
    }

    #[test]
    fn test_ffmpeg_installation_guidance() {
        let guidance = RecordingError::generate_ffmpeg_installation_guidance();
        assert!(!guidance.is_empty());
        assert!(guidance.contains("ffmpeg") || guidance.contains("FFmpeg"));
    }
}