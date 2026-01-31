//! Simple diagnostic test for MP4 recording issues
//!
//! This test runs the diagnostic components directly to identify the root cause
//! of MP4 recording issues without requiring a full Tauri app.

use app_lib::recording::diagnostics::{DiagnosticEngine, AutoSaveStatus, PreferenceStatus, PipelineStatus, DependencyStatus, FilesystemStatus};

#[tokio::test]
async fn test_simple_mp4_recording_diagnostics() {
    // Initialize logging for the test
    let _ = env_logger::builder().is_test(true).try_init();
    
    println!("\n🔍 Running Simple MP4 Recording Diagnostics Test");
    
    // Create diagnostic engine without app handle
    let engine = DiagnosticEngine::new();
    
    println!("\n=== 1. FFmpeg Dependency Check ===");
    let dependency_status = engine.dependency_checker.check_dependencies().await;
    match &dependency_status {
        DependencyStatus::Available => {
            println!("✅ FFmpeg is available and working");
        }
        DependencyStatus::FFmpegNotFound => {
            println!("❌ FFmpeg not found - this is likely the root cause!");
        }
        DependencyStatus::FFmpegIncompatible(version) => {
            println!("❌ FFmpeg version incompatible: {}", version);
        }
    }
    
    // Get detailed FFmpeg info
    match engine.dependency_checker.get_ffmpeg_info().await {
        Ok(ffmpeg_info) => {
            println!("📊 FFmpeg Details:");
            println!("   Path: {:?}", ffmpeg_info.path);
            println!("   Version: {}", ffmpeg_info.version);
            println!("   Executable: {}", ffmpeg_info.is_executable);
            println!("   Functional: {}", ffmpeg_info.functionality_test_passed);
            println!("   Installation: {}", ffmpeg_info.installation_method);
        }
        Err(e) => {
            println!("❌ Failed to get FFmpeg details: {}", e);
        }
    }
    
    println!("\n=== 2. Pipeline Validation ===");
    let pipeline_status = engine.pipeline_tracer.validate_recording_pipeline().await;
    match &pipeline_status {
        PipelineStatus::Initialized => {
            println!("✅ Recording pipeline is properly initialized");
        }
        PipelineStatus::InitializationFailed(reason) => {
            println!("❌ Pipeline initialization failed: {}", reason);
        }
        PipelineStatus::ParameterNotPropagated => {
            println!("❌ Auto-save parameter not propagated correctly");
        }
        PipelineStatus::IncrementalSaverMissing => {
            println!("❌ IncrementalSaver missing when auto_save is true");
        }
    }
    
    println!("\n=== 3. Hardcoded Value Detection ===");
    let hardcoded_report = engine.create_hardcoded_detection_report().await;
    println!("📊 Hardcoded Value Scan Results:");
    println!("   Total issues: {}", hardcoded_report.total_issues_found);
    println!("   Critical issues: {}", hardcoded_report.critical_issues.len());
    println!("   Warning issues: {}", hardcoded_report.warning_issues.len());
    println!("   Summary: {}", hardcoded_report.get_summary());
    
    if hardcoded_report.has_critical_issues() {
        println!("   ❌ Critical hardcoded issues found:");
        for issue in &hardcoded_report.critical_issues {
            println!("     - {} at {}: {} -> {}", 
                     issue.reason, issue.location,
                     issue.original_value, issue.new_value);
        }
    } else {
        println!("   ✅ No critical hardcoded issues detected");
    }
    
    println!("\n=== 4. Filesystem Validation (Default Location) ===");
    let default_folder = app_lib::audio::recording_preferences::get_default_recordings_folder();
    println!("📊 Default recordings folder: {:?}", default_folder);
    
    match engine.filesystem_validator.validate_meeting_folder(&default_folder).await {
        Ok(()) => {
            println!("✅ Default recordings folder is accessible and writable");
        }
        Err(e) => {
            println!("❌ Default recordings folder has issues: {}", e);
        }
    }
    
    // Test filesystem info gathering
    match engine.filesystem_validator.get_filesystem_info().await {
        Ok(fs_info) => {
            println!("📊 Filesystem Details:");
            println!("   Primary folder: {}", fs_info.primary_save_folder);
            println!("   Exists: {}", fs_info.exists);
            println!("   Writable: {}", fs_info.is_writable);
            println!("   Can create meeting folders: {}", fs_info.can_create_meeting_folders);
            println!("   Can write MP4 files: {}", fs_info.can_write_mp4_files);
            println!("   Available space: {} MB", fs_info.available_space_mb);
        }
        Err(e) => {
            println!("❌ Failed to get filesystem info: {}", e);
        }
    }
    
    println!("\n=== 5. Preference Validation (Without App Handle) ===");
    let preference_validator = app_lib::recording::diagnostics::PreferenceValidator::new();
    
    // Test default value handling
    match preference_validator.validate_default_value_handling().await {
        Ok(valid) => {
            if valid {
                println!("✅ Default preference values are correct (auto_save defaults to true)");
            } else {
                println!("❌ Default preference values are incorrect");
            }
        }
        Err(e) => {
            println!("❌ Default value validation failed: {}", e);
        }
    }
    
    // Check auto_save status without app handle
    let auto_save_status = preference_validator.check_auto_save_status().await;
    match &auto_save_status {
        AutoSaveStatus::Enabled => {
            println!("✅ Auto-save is enabled");
        }
        AutoSaveStatus::Disabled => {
            println!("❌ Auto-save is disabled - this could be the issue!");
        }
        AutoSaveStatus::NotFound => {
            println!("⚠️ Auto-save preference not found (expected without app handle)");
        }
        AutoSaveStatus::Corrupted => {
            println!("❌ Auto-save preference is corrupted");
        }
        AutoSaveStatus::HardcodedFalse(location) => {
            println!("❌ Auto-save is hardcoded to false at: {}", location);
        }
    }
    
    println!("\n=== 6. Root Cause Analysis ===");
    
    // Analyze the results to identify the most likely root cause
    let mut root_causes = Vec::new();
    
    match &dependency_status {
        DependencyStatus::FFmpegNotFound => {
            root_causes.push("🔥 CRITICAL: FFmpeg not found - MP4 files cannot be created without FFmpeg");
        }
        DependencyStatus::FFmpegIncompatible(_) => {
            root_causes.push("🔥 CRITICAL: FFmpeg version incompatible - may cause MP4 creation failures");
        }
        _ => {}
    }
    
    match &pipeline_status {
        PipelineStatus::InitializationFailed(_) => {
            root_causes.push("🔥 CRITICAL: Recording pipeline initialization failed");
        }
        PipelineStatus::ParameterNotPropagated => {
            root_causes.push("🔥 CRITICAL: Auto-save parameter not propagated through pipeline");
        }
        PipelineStatus::IncrementalSaverMissing => {
            root_causes.push("🔥 CRITICAL: IncrementalSaver not initialized when auto_save=true");
        }
        _ => {}
    }
    
    if hardcoded_report.has_critical_issues() {
        root_causes.push("🔥 CRITICAL: Hardcoded false values detected that override auto_save");
    }
    
    match &auto_save_status {
        AutoSaveStatus::Disabled => {
            root_causes.push("🔥 CRITICAL: Auto-save is disabled in preferences");
        }
        AutoSaveStatus::Corrupted => {
            root_causes.push("🔥 CRITICAL: Auto-save preference is corrupted");
        }
        AutoSaveStatus::HardcodedFalse(_) => {
            root_causes.push("🔥 CRITICAL: Auto-save is hardcoded to false in code");
        }
        _ => {}
    }
    
    if root_causes.is_empty() {
        println!("✅ No obvious root causes detected in this simplified test");
        println!("   The issue may require a full app context to diagnose properly");
        println!("   Possible causes:");
        println!("   - Preferences not loading correctly in real app context");
        println!("   - Runtime issues during actual recording");
        println!("   - Component integration issues");
    } else {
        println!("❌ POTENTIAL ROOT CAUSES IDENTIFIED:");
        for (i, cause) in root_causes.iter().enumerate() {
            println!("   {}. {}", i + 1, cause);
        }
    }
    
    println!("\n=== Diagnostic Test Complete ===");
    
    // The test should pass regardless of findings - we're just gathering information
    assert!(true, "Diagnostic test completed successfully");
}