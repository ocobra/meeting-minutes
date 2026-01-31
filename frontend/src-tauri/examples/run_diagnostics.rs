//! Example to run MP4 recording diagnostics
//!
//! This example demonstrates how to use the diagnostic engine to identify
//! issues with MP4 recording functionality.

use crate::recording::diagnostic_commands;
use tauri::Manager;
use log::{info, error};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    env_logger::init();
    
    info!("🔍 Starting MP4 Recording Diagnostics");
    
    // Create a mock Tauri app for testing
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_store::Builder::default().build())
        .build(tauri::generate_context!())
        .expect("Failed to build Tauri app");
    
    let app_handle = app.handle();
    
    println!("\n=== MP4 Recording Diagnostic Report ===\n");
    
    // Run comprehensive diagnostics
    match diagnostic_commands::get_diagnostic_summary(app_handle.clone()).await {
        Ok(summary) => {
            println!("✅ Diagnostic Summary Generated:");
            println!("{}", serde_json::to_string_pretty(&summary)?);
        }
        Err(e) => {
            error!("❌ Failed to generate diagnostic summary: {}", e);
            println!("Error: {}", e);
        }
    }
    
    println!("\n=== Individual Diagnostic Checks ===\n");
    
    // Check auto_save parameter tracing
    println!("1. Auto-save Parameter Tracing:");
    match diagnostic_commands::trace_auto_save_parameter(app_handle.clone()).await {
        Ok(trace) => {
            println!("   Source: {:?}", trace.source);
            println!("   Initial value: {}", trace.value);
            println!("   Final value: {}", trace.final_value());
            println!("   Propagated correctly: {}", trace.is_propagated_correctly());
            if !trace.override_points.is_empty() {
                println!("   Override points found:");
                for point in &trace.override_points {
                    println!("     - {} at {}: {} -> {}", 
                             point.reason, point.location, 
                             point.original_value, point.new_value);
                }
            }
        }
        Err(e) => {
            error!("   Failed to trace auto_save parameter: {}", e);
        }
    }
    
    println!("\n2. FFmpeg Dependency Check:");
    match diagnostic_commands::check_ffmpeg_dependency().await {
        Ok(ffmpeg_info) => {
            println!("   Path: {:?}", ffmpeg_info.path);
            println!("   Version: {}", ffmpeg_info.version);
            println!("   Executable: {}", ffmpeg_info.is_executable);
            println!("   Functional: {}", ffmpeg_info.functionality_test_passed);
            println!("   Installation: {}", ffmpeg_info.installation_method);
        }
        Err(e) => {
            error!("   Failed to check FFmpeg: {}", e);
        }
    }
    
    println!("\n3. Filesystem Status Check:");
    match diagnostic_commands::check_filesystem_status(app_handle.clone()).await {
        Ok(fs_info) => {
            println!("   Save folder: {}", fs_info.primary_save_folder);
            println!("   Exists: {}", fs_info.exists);
            println!("   Writable: {}", fs_info.is_writable);
            println!("   Can create meeting folders: {}", fs_info.can_create_meeting_folders);
            println!("   Can write MP4 files: {}", fs_info.can_write_mp4_files);
            println!("   Available space: {} MB", fs_info.available_space_mb);
        }
        Err(e) => {
            error!("   Failed to check filesystem: {}", e);
        }
    }
    
    println!("\n4. Hardcoded Value Scan:");
    match diagnostic_commands::scan_hardcoded_values(app_handle.clone()).await {
        Ok(scan_report) => {
            println!("   Total issues: {}", scan_report.total_issues_found);
            println!("   Critical issues: {}", scan_report.critical_issues.len());
            println!("   Warning issues: {}", scan_report.warning_issues.len());
            println!("   Summary: {}", scan_report.get_summary());
            
            if scan_report.has_critical_issues() {
                println!("   Critical issues found:");
                for issue in &scan_report.critical_issues {
                    println!("     - {} at {}: {}", 
                             issue.reason, issue.location,
                             if issue.original_value && !issue.new_value { "true -> false" } else { "value changed" });
                }
            }
        }
        Err(e) => {
            error!("   Failed to scan hardcoded values: {}", e);
        }
    }
    
    println!("\n5. Full Diagnostic Report:");
    match diagnostic_commands::run_recording_diagnostics(app_handle).await {
        Ok(report) => {
            println!("   Overall health: {}", report.is_healthy());
            println!("   Auto-save status: {:?}", report.auto_save_status);
            println!("   Preference status: {:?}", report.preference_status);
            println!("   Pipeline status: {:?}", report.pipeline_status);
            println!("   Dependency status: {:?}", report.dependency_status);
            println!("   Filesystem status: {:?}", report.filesystem_status);
            
            if !report.recommendations.is_empty() {
                println!("   Recommendations:");
                for rec in &report.recommendations {
                    println!("     - {}", rec.description());
                }
            }
            
            let critical_issues = report.get_critical_issues();
            if !critical_issues.is_empty() {
                println!("   Critical issues:");
                for issue in &critical_issues {
                    println!("     - {}", issue);
                }
            }
        }
        Err(e) => {
            error!("   Failed to run full diagnostics: {}", e);
        }
    }
    
    println!("\n=== Diagnostic Complete ===");
    
    Ok(())
}