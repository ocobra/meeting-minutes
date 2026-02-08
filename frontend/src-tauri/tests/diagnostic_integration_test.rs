//! Integration test for MP4 recording diagnostics
//!
//! This test runs the actual diagnostic commands to identify the root cause
//! of MP4 recording issues.

use std::sync::Arc;
use tokio::sync::RwLock;

#[tokio::test]
#[ignore] // Requires main thread for Tauri event loop
async fn test_mp4_recording_diagnostics() {
    // Initialize logging for the test
    let _ = env_logger::builder().is_test(true).try_init();
    
    println!("\n🔍 Running MP4 Recording Diagnostics Integration Test");
    
    // Create a mock Tauri app for testing
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_store::Builder::default().build())
        .manage(Arc::new(RwLock::new(
            None::<app_lib::notifications::manager::NotificationManager<tauri::Wry>>,
        )) as app_lib::notifications::commands::NotificationManagerState<tauri::Wry>)
        .build(tauri::generate_context!())
        .expect("Failed to build Tauri app");
    
    let app_handle = app.handle();
    
    println!("\n=== 1. Auto-save Parameter Tracing ===");
    match app_lib::recording::diagnostic_commands::trace_auto_save_parameter(app_handle.clone()).await {
        Ok(trace) => {
            println!("✅ Parameter trace completed:");
            println!("   Source: {:?}", trace.source);
            println!("   Initial value: {}", trace.value);
            println!("   Final value: {}", trace.final_value());
            println!("   Propagated correctly: {}", trace.is_propagated_correctly());
            
            if !trace.override_points.is_empty() {
                println!("   ⚠️ Override points found:");
                for point in &trace.override_points {
                    println!("     - {} at {}: {} -> {}", 
                             point.reason, point.location, 
                             point.original_value, point.new_value);
                }
            } else {
                println!("   ✅ No override points detected");
            }
        }
        Err(e) => {
            println!("❌ Failed to trace auto_save parameter: {}", e);
        }
    }
    
    println!("\n=== 2. FFmpeg Dependency Check ===");
    match app_lib::recording::diagnostic_commands::check_ffmpeg_dependency().await {
        Ok(ffmpeg_info) => {
            println!("✅ FFmpeg dependency check completed:");
            println!("   Path: {:?}", ffmpeg_info.path);
            println!("   Version: {}", ffmpeg_info.version);
            println!("   Executable: {}", ffmpeg_info.is_executable);
            println!("   Functional: {}", ffmpeg_info.functionality_test_passed);
            println!("   Installation: {}", ffmpeg_info.installation_method);
            
            if ffmpeg_info.is_executable && ffmpeg_info.functionality_test_passed {
                println!("   ✅ FFmpeg is working correctly");
            } else {
                println!("   ❌ FFmpeg has issues");
            }
        }
        Err(e) => {
            println!("❌ Failed to check FFmpeg: {}", e);
        }
    }
    
    println!("\n=== 3. Filesystem Status Check ===");
    match app_lib::recording::diagnostic_commands::check_filesystem_status(app_handle.clone()).await {
        Ok(fs_info) => {
            println!("✅ Filesystem check completed:");
            println!("   Save folder: {}", fs_info.primary_save_folder);
            println!("   Exists: {}", fs_info.exists);
            println!("   Writable: {}", fs_info.is_writable);
            println!("   Can create meeting folders: {}", fs_info.can_create_meeting_folders);
            println!("   Can write MP4 files: {}", fs_info.can_write_mp4_files);
            println!("   Available space: {} MB", fs_info.available_space_mb);
            
            if fs_info.exists && fs_info.is_writable && 
               fs_info.can_create_meeting_folders && fs_info.can_write_mp4_files {
                println!("   ✅ Filesystem is ready for recording");
            } else {
                println!("   ❌ Filesystem has issues");
            }
        }
        Err(e) => {
            println!("❌ Failed to check filesystem: {}", e);
        }
    }
    
    println!("\n=== 4. Hardcoded Value Scan ===");
    match app_lib::recording::diagnostic_commands::scan_hardcoded_values(app_handle.clone()).await {
        Ok(scan_report) => {
            println!("✅ Hardcoded value scan completed:");
            println!("   Total issues: {}", scan_report.total_issues_found);
            println!("   Critical issues: {}", scan_report.critical_issues.len());
            println!("   Warning issues: {}", scan_report.warning_issues.len());
            println!("   Summary: {}", scan_report.get_summary());
            
            if scan_report.has_critical_issues() {
                println!("   ❌ Critical issues found:");
                for issue in &scan_report.critical_issues {
                    println!("     - {} at {}: {}", 
                             issue.reason, issue.location,
                             if issue.original_value && !issue.new_value { "true -> false" } else { "value changed" });
                }
            } else {
                println!("   ✅ No critical hardcoded issues found");
            }
        }
        Err(e) => {
            println!("❌ Failed to scan hardcoded values: {}", e);
        }
    }
    
    println!("\n=== 5. Full Diagnostic Report ===");
    match app_lib::recording::diagnostic_commands::run_recording_diagnostics(app_handle.clone()).await {
        Ok(report) => {
            println!("✅ Full diagnostic report completed:");
            println!("   Overall health: {}", report.is_healthy());
            println!("   Auto-save status: {:?}", report.auto_save_status);
            println!("   Preference status: {:?}", report.preference_status);
            println!("   Pipeline status: {:?}", report.pipeline_status);
            println!("   Dependency status: {:?}", report.dependency_status);
            println!("   Filesystem status: {:?}", report.filesystem_status);
            
            if !report.recommendations.is_empty() {
                println!("   📋 Recommendations:");
                for rec in &report.recommendations {
                    println!("     - {}", rec.description());
                }
            }
            
            let critical_issues = report.get_critical_issues();
            if !critical_issues.is_empty() {
                println!("   ❌ Critical issues:");
                for issue in &critical_issues {
                    println!("     - {}", issue);
                }
            } else {
                println!("   ✅ No critical issues found");
            }
        }
        Err(e) => {
            println!("❌ Failed to run full diagnostics: {}", e);
        }
    }
    
    println!("\n=== 6. Diagnostic Summary ===");
    match app_lib::recording::diagnostic_commands::get_diagnostic_summary(app_handle.clone()).await {
        Ok(summary) => {
            println!("✅ Diagnostic summary generated:");
            println!("{}", serde_json::to_string_pretty(&summary).unwrap_or_else(|_| "Failed to serialize summary".to_string()));
        }
        Err(e) => {
            println!("❌ Failed to generate diagnostic summary: {}", e);
        }
    }
    
    println!("\n=== Diagnostic Integration Test Complete ===");
}