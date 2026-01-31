//! Recording System Module
//!
//! This module contains the recording system components including diagnostics,
//! preference management, pipeline coordination, comprehensive error handling,
//! and graceful degradation for MP4 audio recording.

pub mod diagnostics;
pub mod diagnostic_commands;
pub mod error_handling;
pub mod graceful_degradation_commands;

// Re-export diagnostic components for easy access
pub use diagnostics::{
    DiagnosticEngine, DiagnosticReport, ParameterTrace,
    AutoSaveStatus, PreferenceStatus, PipelineStatus, DependencyStatus, FilesystemStatus,
    ParameterSource, ComponentTrace, OverridePoint, FixRecommendation,
    PreferenceValidator, PipelineTracer, DependencyChecker, FilesystemValidator,
    DiagnosticError, DiagnosticResult,
};

// Re-export error handling components for easy access
pub use error_handling::{
    RecordingError, AutoSaveErrorSource, CheckpointOperation, RecoveryStrategy,
    AlternativeAction, UserAction, RepairAction, RestartType,
    ErrorRecoveryCoordinator, RecoveryResult,
};

// Re-export graceful degradation components for easy access
pub use graceful_degradation_commands::{
    GracefulDegradationStatus, Mp4RestorationResult, RecordingModeDetails,
    RecordingPipelineState,
};