/// Task 2.2: Hardcoded Value Detection Test
/// 
/// This test validates the hardcoded false value detection system that scans
/// the recording pipeline codebase for potential auto_save parameter overrides.
/// 
/// Requirements tested:
/// - Requirement 2.2: Identify if any hardcoded false values override the Auto_Save_Parameter
/// 
/// Test approach:
/// 1. Create a diagnostic engine with pipeline tracer
/// 2. Run hardcoded value detection scan
/// 3. Verify that the scan completes successfully
/// 4. Check that the detection report is properly formatted
/// 5. Validate that any detected issues are properly categorized

use app_lib::recording::diagnostics::{
    DiagnosticEngine, PipelineTracer, HardcodedDetectionReport,
};

#[tokio::test]
async fn test_hardcoded_value_detection_scan() {
    // Initialize logging for test visibility
    let _ = env_logger::builder()
        .is_test(true)
        .filter_level(log::LevelFilter::Info)
        .try_init();

    println!("\n╔════════════════════════════════════════════════════════════════╗");
    println!("║  Task 2.2: Hardcoded Value Detection Test                     ║");
    println!("╚════════════════════════════════════════════════════════════════╝\n");

    // Step 1: Create diagnostic engine
    println!("Step 1: Creating diagnostic engine...");
    let engine = DiagnosticEngine::new();
    
    // Step 2: Run hardcoded value detection
    println!("Step 2: Running hardcoded value detection scan...");
    let override_points = engine.detect_hardcoded_false_values().await;
    
    println!("   Found {} potential override points", override_points.len());
    
    // Step 3: Create comprehensive detection report
    println!("Step 3: Creating comprehensive detection report...");
    let report = engine.create_hardcoded_detection_report().await;
    
    // Step 4: Validate report structure
    println!("Step 4: Validating detection report...");
    assert_eq!(report.total_issues_found, override_points.len(), 
               "Report total should match detected override points");
    
    println!("   Total issues found: {}", report.total_issues_found);
    println!("   Critical issues: {}", report.critical_issues.len());
    println!("   Warning issues: {}", report.warning_issues.len());
    println!("   Files scanned: {}", report.files_scanned.len());
    
    // Step 5: Display report summary
    println!("\nStep 5: Detection Report Summary:");
    println!("   {}", report.get_summary());
    
    // Step 6: Display files scanned
    println!("\nStep 6: Files Scanned:");
    for file in &report.files_scanned {
        println!("   - {}", file);
    }
    
    // Step 7: Display critical issues if any
    if !report.critical_issues.is_empty() {
        println!("\n⚠️  CRITICAL ISSUES DETECTED:");
        for (idx, issue) in report.critical_issues.iter().enumerate() {
            println!("   {}. Location: {}", idx + 1, issue.location);
            println!("      Reason: {}", issue.reason);
            println!("      Override: {} -> {}", issue.original_value, issue.new_value);
        }
    } else {
        println!("\n✅ No critical hardcoded false values detected");
    }
    
    // Step 8: Display warning issues if any
    if !report.warning_issues.is_empty() {
        println!("\nℹ️  WARNING ISSUES:");
        for (idx, issue) in report.warning_issues.iter().enumerate() {
            println!("   {}. Location: {}", idx + 1, issue.location);
            println!("      Reason: {}", issue.reason);
        }
    }
    
    // Step 9: Display recommendations
    if !report.recommendations.is_empty() {
        println!("\n📋 RECOMMENDATIONS:");
        for (idx, recommendation) in report.recommendations.iter().enumerate() {
            println!("   {}. {}", idx + 1, recommendation);
        }
    }
    
    // Step 10: Validate that scan timestamp is recent
    println!("\nStep 10: Validating scan metadata...");
    let now = chrono::Utc::now();
    let scan_age = now.signed_duration_since(report.scan_timestamp);
    assert!(scan_age.num_seconds() < 60, 
            "Scan timestamp should be within the last minute");
    println!("   Scan timestamp: {}", report.scan_timestamp);
    println!("   Scan age: {} seconds", scan_age.num_seconds());
    
    println!("\n╔════════════════════════════════════════════════════════════════╗");
    println!("║  ✅ Task 2.2 Test Completed Successfully                       ║");
    println!("╚════════════════════════════════════════════════════════════════╝\n");
}

#[tokio::test]
async fn test_pipeline_tracer_hardcoded_detection() {
    // Initialize logging
    let _ = env_logger::builder()
        .is_test(true)
        .filter_level(log::LevelFilter::Debug)
        .try_init();

    println!("\n╔════════════════════════════════════════════════════════════════╗");
    println!("║  Task 2.2: Pipeline Tracer Hardcoded Detection Test           ║");
    println!("╚════════════════════════════════════════════════════════════════╝\n");

    // Create pipeline tracer
    println!("Creating pipeline tracer...");
    let tracer = PipelineTracer::new();
    
    // Run detection
    println!("Running hardcoded value detection...");
    let override_points = tracer.detect_hardcoded_false_values().await;
    
    println!("Detection completed: {} override points found", override_points.len());
    
    // Validate that detection ran without errors
    // Note: We don't assert that no issues are found, because the codebase
    // might legitimately have some patterns that look like hardcoded values
    // (e.g., in error handling or fallback logic)
    
    // Display any detected issues for manual review
    if !override_points.is_empty() {
        println!("\nDetected override points:");
        for (idx, point) in override_points.iter().enumerate() {
            println!("   {}. {}", idx + 1, point.location);
            println!("      Reason: {}", point.reason);
            println!("      Override: {} -> {}", point.original_value, point.new_value);
        }
    }
    
    println!("\n✅ Pipeline tracer hardcoded detection test completed");
}

#[tokio::test]
async fn test_hardcoded_detection_report_structure() {
    // Initialize logging
    let _ = env_logger::builder()
        .is_test(true)
        .filter_level(log::LevelFilter::Info)
        .try_init();

    println!("\n╔════════════════════════════════════════════════════════════════╗");
    println!("║  Task 2.2: Detection Report Structure Test                    ║");
    println!("╚════════════════════════════════════════════════════════════════╝\n");

    // Create diagnostic engine
    let engine = DiagnosticEngine::new();
    
    // Create detection report
    println!("Creating hardcoded detection report...");
    let report = engine.create_hardcoded_detection_report().await;
    
    // Validate report structure
    println!("Validating report structure...");
    
    // Check that files_scanned is not empty
    assert!(!report.files_scanned.is_empty(), 
            "Report should list scanned files");
    println!("   ✓ Files scanned list is populated ({} files)", report.files_scanned.len());
    
    // Check that expected files are in the scan list
    let expected_files = vec![
        "recording_commands.rs",
        "recording_manager.rs",
        "recording_saver.rs",
        "incremental_saver.rs",
    ];
    
    for expected_file in expected_files {
        assert!(report.files_scanned.contains(&expected_file.to_string()),
                "Report should include {} in scanned files", expected_file);
        println!("   ✓ {} is in scan list", expected_file);
    }
    
    // Check that total_issues_found matches the sum of critical and warning issues
    let calculated_total = report.critical_issues.len() + report.warning_issues.len();
    assert_eq!(report.total_issues_found, calculated_total,
               "Total issues should equal critical + warning issues");
    println!("   ✓ Total issues count is consistent");
    
    // Check that recommendations are provided
    assert!(!report.recommendations.is_empty(),
            "Report should provide recommendations");
    println!("   ✓ Recommendations are provided ({} recommendations)", report.recommendations.len());
    
    // Check that scan timestamp is set
    let now = chrono::Utc::now();
    assert!(report.scan_timestamp <= now,
            "Scan timestamp should not be in the future");
    println!("   ✓ Scan timestamp is valid");
    
    // Test report methods
    println!("\nTesting report methods...");
    
    // Test has_critical_issues
    let has_critical = report.has_critical_issues();
    println!("   has_critical_issues(): {}", has_critical);
    assert_eq!(has_critical, !report.critical_issues.is_empty());
    
    // Test get_summary
    let summary = report.get_summary();
    println!("   get_summary(): {}", summary);
    assert!(!summary.is_empty());
    
    // Test get_most_critical_issue
    if has_critical {
        let most_critical = report.get_most_critical_issue();
        assert!(most_critical.is_some());
        println!("   get_most_critical_issue(): {:?}", most_critical.unwrap().location);
    } else {
        println!("   get_most_critical_issue(): None (no critical issues)");
    }
    
    println!("\n✅ Detection report structure test completed");
}

#[tokio::test]
async fn test_diagnostic_engine_scan_for_hardcoded_issues() {
    // Initialize logging
    let _ = env_logger::builder()
        .is_test(true)
        .filter_level(log::LevelFilter::Info)
        .try_init();

    println!("\n╔════════════════════════════════════════════════════════════════╗");
    println!("║  Task 2.2: Diagnostic Engine Focused Scan Test                ║");
    println!("╚════════════════════════════════════════════════════════════════╝\n");

    // Create diagnostic engine
    let engine = DiagnosticEngine::new();
    
    // Run focused hardcoded issues scan
    println!("Running focused hardcoded issues scan...");
    let result = engine.scan_for_hardcoded_issues().await;
    
    // Validate that scan completed successfully
    assert!(result.is_ok(), "Scan should complete without errors");
    
    let report = result.unwrap();
    
    println!("Scan completed successfully");
    println!("   Total issues: {}", report.total_issues_found);
    println!("   Critical issues: {}", report.critical_issues.len());
    println!("   Warning issues: {}", report.warning_issues.len());
    
    // Display summary
    println!("\nScan Summary:");
    println!("   {}", report.get_summary());
    
    // If critical issues found, display them
    if report.has_critical_issues() {
        println!("\n⚠️  CRITICAL ISSUES REQUIRE ATTENTION:");
        for issue in &report.critical_issues {
            println!("   - {} at {}", issue.reason, issue.location);
        }
    } else {
        println!("\n✅ No critical hardcoded issues detected");
    }
    
    println!("\n✅ Focused scan test completed");
}
