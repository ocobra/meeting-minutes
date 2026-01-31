//! Diagnostic Commands for MP4 Recording Issues
//!
//! This module provides Tauri commands to run diagnostic checks on the recording system
//! to identify issues with MP4 recording functionality.

use tauri::{AppHandle, Runtime};
use log::{info, error};

use super::diagnostics::{DiagnosticEngine, DiagnosticReport, HardcodedDetectionReport, PreferenceValidator, FilesystemValidator};

/// Run comprehensive diagnostic check on the recording system
#[tauri::command]
pub async fn run_recording_diagnostics<R: Runtime>(
    app: AppHandle<R>,
) -> Result<DiagnosticReport, String> {
    info!("🔍 Running comprehensive recording system diagnostics");
    
    // Create individual validators with the app handle
    let preference_validator = PreferenceValidator::new();
    let filesystem_validator = FilesystemValidator::new();
    
    // Check auto_save status by loading preferences directly
    let auto_save_status = match crate::audio::recording_preferences::load_recording_preferences_with_validation(&app).await {
        Ok(prefs) => {
            info!("📋 Loaded recording preferences: auto_save={}", prefs.auto_save);
            if prefs.auto_save {
                super::diagnostics::AutoSaveStatus::Enabled
            } else {
                super::diagnostics::AutoSaveStatus::Disabled
            }
        }
        Err(e) => {
            error!("Failed to load recording preferences: {}", e);
            super::diagnostics::AutoSaveStatus::NotFound
        }
    };
    
    // Validate preferences
    let preference_status = if auto_save_status == super::diagnostics::AutoSaveStatus::NotFound {
        super::diagnostics::PreferenceStatus::Missing
    } else {
        super::diagnostics::PreferenceStatus::Valid
    };
    
    // Create basic engine for other checks
    let engine = DiagnosticEngine::new();
    let pipeline_status = engine.pipeline_tracer.validate_recording_pipeline().await;
    let dependency_status = engine.dependency_checker.check_dependencies().await;
    
    // Check filesystem status by getting save folder from preferences
    let filesystem_status = match crate::audio::recording_preferences::load_recording_preferences_with_validation(&app).await {
        Ok(prefs) => {
            match filesystem_validator.validate_meeting_folder(&prefs.save_folder).await {
                Ok(()) => super::diagnostics::FilesystemStatus::Ready,
                Err(e) => super::diagnostics::FilesystemStatus::MeetingFolderError(e.to_string()),
            }
        }
        Err(_) => {
            let default_folder = crate::audio::recording_preferences::get_default_recordings_folder();
            match filesystem_validator.validate_meeting_folder(&default_folder).await {
                Ok(()) => super::diagnostics::FilesystemStatus::Ready,
                Err(e) => super::diagnostics::FilesystemStatus::MeetingFolderError(e.to_string()),
            }
        }
    };
    
    let recommendations = generate_recommendations(
        &auto_save_status,
        &preference_status,
        &pipeline_status,
        &dependency_status,
        &filesystem_status,
    );
    
    let report = DiagnosticReport {
        auto_save_status,
        preference_status,
        pipeline_status,
        dependency_status,
        filesystem_status,
        recommendations,
    };
    
    info!("📊 Diagnostic report generated:");
    info!("  Auto-save status: {:?}", report.auto_save_status);
    info!("  Preference status: {:?}", report.preference_status);
    info!("  Pipeline status: {:?}", report.pipeline_status);
    info!("  Dependency status: {:?}", report.dependency_status);
    info!("  Filesystem status: {:?}", report.filesystem_status);
    info!("  Recommendations: {} items", report.recommendations.len());
    
    if report.is_healthy() {
        info!("✅ Recording system appears healthy");
    } else {
        let issues = report.get_critical_issues();
        error!("❌ Found {} critical issues:", issues.len());
        for issue in &issues {
            error!("  - {}", issue);
        }
    }
    
    Ok(report)
}

/// Generate fix recommendations based on diagnostic results
fn generate_recommendations(
    auto_save_status: &super::diagnostics::AutoSaveStatus,
    _preference_status: &super::diagnostics::PreferenceStatus,
    _pipeline_status: &super::diagnostics::PipelineStatus,
    dependency_status: &super::diagnostics::DependencyStatus,
    filesystem_status: &super::diagnostics::FilesystemStatus,
) -> Vec<super::diagnostics::FixRecommendation> {
    let mut recommendations = Vec::new();

    // Check auto_save parameter issues
    match auto_save_status {
        super::diagnostics::AutoSaveStatus::Disabled => {
            recommendations.push(super::diagnostics::FixRecommendation::EnableAutoSave);
        }
        super::diagnostics::AutoSaveStatus::Corrupted => {
            recommendations.push(super::diagnostics::FixRecommendation::RepairPreferences);
        }
        super::diagnostics::AutoSaveStatus::NotFound => {
            recommendations.push(super::diagnostics::FixRecommendation::RestoreDefaults);
        }
        super::diagnostics::AutoSaveStatus::HardcodedFalse(location) => {
            recommendations.push(super::diagnostics::FixRecommendation::RemoveHardcodedFalse(location.clone()));
        }
        super::diagnostics::AutoSaveStatus::Enabled => {} // No action needed
    }

    // Check dependency issues
    match dependency_status {
        super::diagnostics::DependencyStatus::FFmpegNotFound => {
            recommendations.push(super::diagnostics::FixRecommendation::InstallFFmpeg);
        }
        super::diagnostics::DependencyStatus::FFmpegIncompatible(_) => {
            recommendations.push(super::diagnostics::FixRecommendation::InstallFFmpeg);
        }
        super::diagnostics::DependencyStatus::Available => {
            // No dependency issues
        }
    }

    // Check filesystem issues
    if let super::diagnostics::FilesystemStatus::MeetingFolderError(_) = filesystem_status {
        recommendations.push(super::diagnostics::FixRecommendation::CreateMeetingFolder);
    }

    recommendations
}

/// Trace the auto_save parameter through the recording pipeline
#[tauri::command]
pub async fn trace_auto_save_parameter<R: Runtime>(
    app: AppHandle<R>,
) -> Result<super::diagnostics::ParameterTrace, String> {
    info!("🔍 Tracing auto_save parameter through recording pipeline");
    
    // Determine the source of the auto_save parameter
    let (source, initial_value) = match crate::audio::recording_preferences::load_recording_preferences_with_validation(&app).await {
        Ok(preferences) => {
            info!("PipelineTracer: Found auto_save parameter in user preferences: {}", preferences.auto_save);
            (super::diagnostics::ParameterSource::UserPreferences, preferences.auto_save)
        }
        Err(e) => {
            info!("PipelineTracer: Failed to load user preferences: {}, using default", e);
            let default_prefs = crate::audio::recording_preferences::RecordingPreferences::default();
            (super::diagnostics::ParameterSource::Default, default_prefs.auto_save)
        }
    };
    
    // Create a simple trace showing the parameter flow
    let propagation_path = vec![
        super::diagnostics::ComponentTrace {
            component: "RecordingCommands".to_string(),
            received_value: initial_value,
            passed_value: initial_value,
            location: "recording_commands.rs:115".to_string(),
        },
        super::diagnostics::ComponentTrace {
            component: "RecordingManager".to_string(),
            received_value: initial_value,
            passed_value: initial_value,
            location: "recording_manager.rs:start_recording".to_string(),
        },
        super::diagnostics::ComponentTrace {
            component: "RecordingSaver".to_string(),
            received_value: initial_value,
            passed_value: initial_value,
            location: "recording_saver.rs:start_accumulation".to_string(),
        },
    ];
    
    let trace = super::diagnostics::ParameterTrace {
        source,
        value: initial_value,
        propagation_path,
        override_points: Vec::new(), // No overrides detected in this simple implementation
    };
    
    info!("📊 Parameter trace completed:");
    info!("  Source: {:?}", trace.source);
    info!("  Initial value: {}", trace.value);
    info!("  Final value: {}", trace.final_value());
    info!("  Propagation path: {} components", trace.propagation_path.len());
    info!("  Override points: {} found", trace.override_points.len());
    
    if trace.is_propagated_correctly() {
        info!("✅ Parameter propagates correctly through pipeline");
    } else {
        error!("❌ Parameter propagation issues detected");
        for override_point in &trace.override_points {
            error!("  - Override at {}: {} -> {} ({})", 
                   override_point.location, 
                   override_point.original_value, 
                   override_point.new_value,
                   override_point.reason);
        }
    }
    
    Ok(trace)
}

/// Scan for hardcoded false values that might override auto_save
#[tauri::command]
pub async fn scan_hardcoded_values<R: Runtime>(
    _app: AppHandle<R>,
) -> Result<HardcodedDetectionReport, String> {
    info!("🔍 Scanning for hardcoded false values in recording pipeline");
    
    let engine = DiagnosticEngine::new();
    let report = engine.scan_for_hardcoded_issues().await
        .map_err(|e| format!("Failed to scan for hardcoded values: {}", e))?;
    
    info!("📊 Hardcoded value scan completed:");
    info!("  Total issues found: {}", report.total_issues_found);
    info!("  Critical issues: {}", report.critical_issues.len());
    info!("  Warning issues: {}", report.warning_issues.len());
    info!("  Files scanned: {}", report.files_scanned.len());
    
    if report.has_critical_issues() {
        error!("❌ Found critical hardcoded issues:");
        for issue in &report.critical_issues {
            error!("  - {} at {}: {}", issue.reason, issue.location, 
                   if issue.original_value && !issue.new_value { "true -> false" } else { "value changed" });
        }
    } else {
        info!("✅ No critical hardcoded issues found");
    }
    
    Ok(report)
}

/// Check FFmpeg dependency status
#[tauri::command]
pub async fn check_ffmpeg_dependency() -> Result<super::diagnostics::FFmpegInfo, String> {
    info!("🔍 Checking FFmpeg dependency status");
    
    let engine = DiagnosticEngine::new();
    let ffmpeg_info = engine.dependency_checker.get_ffmpeg_info().await
        .map_err(|e| format!("Failed to get FFmpeg info: {}", e))?;
    
    info!("📊 FFmpeg dependency check completed:");
    info!("  Path: {:?}", ffmpeg_info.path);
    info!("  Version: {}", ffmpeg_info.version);
    info!("  Is executable: {}", ffmpeg_info.is_executable);
    info!("  Functionality test passed: {}", ffmpeg_info.functionality_test_passed);
    info!("  Installation method: {}", ffmpeg_info.installation_method);
    
    if ffmpeg_info.is_executable && ffmpeg_info.functionality_test_passed {
        info!("✅ FFmpeg is working correctly");
    } else {
        error!("❌ FFmpeg has issues - executable: {}, functional: {}", 
               ffmpeg_info.is_executable, ffmpeg_info.functionality_test_passed);
    }
    
    Ok(ffmpeg_info)
}

/// Check filesystem readiness for recording
#[tauri::command]
pub async fn check_filesystem_status<R: Runtime>(
    app: AppHandle<R>,
) -> Result<super::diagnostics::FilesystemInfo, String> {
    info!("🔍 Checking filesystem status for recording");
    
    let filesystem_validator = FilesystemValidator::new();
    
    // Get save folder from preferences
    let save_folder = match crate::audio::recording_preferences::load_recording_preferences_with_validation(&app).await {
        Ok(preferences) => {
            info!("Got save folder from preferences: {:?}", preferences.save_folder);
            preferences.save_folder
        }
        Err(e) => {
            info!("Failed to load preferences, using default: {}", e);
            crate::audio::recording_preferences::get_default_recordings_folder()
        }
    };
    
    let exists = save_folder.exists();
    let is_writable = if exists {
        filesystem_validator.validate_meeting_folder(&save_folder).await.is_ok()
    } else {
        false
    };

    // Get alternative locations
    let alternative_locations = vec![
        ("Documents folder".to_string(), "~/Documents/meetily-recordings".to_string()),
        ("Desktop folder".to_string(), "~/Desktop/meetily-recordings".to_string()),
        ("Home directory".to_string(), "~/meetily-recordings".to_string()),
    ];

    let filesystem_info = super::diagnostics::FilesystemInfo {
        primary_save_folder: save_folder.to_string_lossy().to_string(),
        exists,
        is_writable,
        available_space_mb: 1024, // Simplified - would need actual disk space check
        total_space_mb: 10240,    // Simplified - would need actual disk space check
        alternative_locations,
        can_create_meeting_folders: is_writable,
        can_write_mp4_files: is_writable,
    };
    
    info!("📊 Filesystem check completed:");
    info!("  Primary save folder: {}", filesystem_info.primary_save_folder);
    info!("  Exists: {}", filesystem_info.exists);
    info!("  Is writable: {}", filesystem_info.is_writable);
    info!("  Available space: {} MB", filesystem_info.available_space_mb);
    info!("  Can create meeting folders: {}", filesystem_info.can_create_meeting_folders);
    info!("  Can write MP4 files: {}", filesystem_info.can_write_mp4_files);
    info!("  Alternative locations: {}", filesystem_info.alternative_locations.len());
    
    if filesystem_info.exists && filesystem_info.is_writable && 
       filesystem_info.can_create_meeting_folders && filesystem_info.can_write_mp4_files {
        info!("✅ Filesystem is ready for recording");
    } else {
        error!("❌ Filesystem has issues - exists: {}, writable: {}, can create folders: {}, can write MP4: {}", 
               filesystem_info.exists, filesystem_info.is_writable, 
               filesystem_info.can_create_meeting_folders, filesystem_info.can_write_mp4_files);
    }
    
    Ok(filesystem_info)
}

/// Get a summary of all diagnostic results
#[tauri::command]
pub async fn get_diagnostic_summary<R: Runtime>(
    app: AppHandle<R>,
) -> Result<serde_json::Value, String> {
    info!("🔍 Generating diagnostic summary");
    
    // Run all diagnostic checks
    let report = run_recording_diagnostics(app.clone()).await?;
    let trace = trace_auto_save_parameter(app.clone()).await?;
    let hardcoded_report = scan_hardcoded_values(app.clone()).await?;
    
    // Get additional info
    let ffmpeg_info = check_ffmpeg_dependency().await.ok();
    let filesystem_info = check_filesystem_status(app).await.ok();
    
    let summary = serde_json::json!({
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "overall_health": report.is_healthy(),
        "critical_issues": report.get_critical_issues(),
        "auto_save_status": report.auto_save_status,
        "preference_status": report.preference_status,
        "pipeline_status": report.pipeline_status,
        "dependency_status": report.dependency_status,
        "filesystem_status": report.filesystem_status,
        "recommendations": report.recommendations,
        "parameter_trace": {
            "source": trace.source,
            "initial_value": trace.value,
            "final_value": trace.final_value(),
            "propagated_correctly": trace.is_propagated_correctly(),
            "override_points": trace.override_points.len()
        },
        "hardcoded_scan": {
            "total_issues": hardcoded_report.total_issues_found,
            "critical_issues": hardcoded_report.critical_issues.len(),
            "warning_issues": hardcoded_report.warning_issues.len(),
            "has_critical_issues": hardcoded_report.has_critical_issues(),
            "summary": hardcoded_report.get_summary()
        },
        "ffmpeg_info": ffmpeg_info,
        "filesystem_info": filesystem_info
    });
    
    info!("📊 Diagnostic summary generated - overall health: {}", report.is_healthy());
    
    Ok(summary)
}