//! Parameter Tracing System Demo
//!
//! This example demonstrates how the parameter tracing system works
//! to track the auto_save parameter through the recording pipeline.

use meetily::recording::diagnostics::{DiagnosticEngine, ParameterTrace, PipelineTracer};

#[tokio::main]
async fn main() {
    // Initialize logging
    env_logger::init();

    println!("🔍 Parameter Tracing System Demo");
    println!("=================================");

    // Create a diagnostic engine
    let engine = DiagnosticEngine::new();
    
    println!("\n1. Tracing auto_save parameter through the pipeline...");
    
    // Trace the auto_save parameter through all components
    let trace: ParameterTrace = engine.trace_auto_save_parameter().await;
    
    println!("   ✅ Parameter trace completed!");
    println!("   📍 Source: {:?}", trace.source);
    println!("   🔢 Initial value: {}", trace.value);
    println!("   🎯 Final value: {}", trace.final_value());
    println!("   🔗 Components traced: {}", trace.propagation_path.len());
    println!("   ⚠️  Override points: {}", trace.override_points.len());
    
    println!("\n2. Component-by-component trace:");
    for (i, component) in trace.propagation_path.iter().enumerate() {
        println!("   {}. {} ({})", 
            i + 1, 
            component.component, 
            component.location
        );
        println!("      Received: {} → Passed: {}", 
            component.received_value, 
            component.passed_value
        );
        
        if component.received_value != component.passed_value {
            println!("      ⚠️  Value changed in this component!");
        }
    }
    
    if !trace.override_points.is_empty() {
        println!("\n3. Override points detected:");
        for (i, override_point) in trace.override_points.iter().enumerate() {
            println!("   {}. {} ({})", 
                i + 1, 
                override_point.reason, 
                override_point.location
            );
            println!("      {} → {}", 
                override_point.original_value, 
                override_point.new_value
            );
        }
    } else {
        println!("\n3. ✅ No override points detected - parameter flows correctly!");
    }
    
    println!("\n4. Pipeline validation...");
    let status = engine.validate_recording_pipeline().await;
    println!("   Pipeline status: {:?}", status);
    
    println!("\n5. Full diagnostic report...");
    let report = engine.run_full_diagnosis().await;
    println!("   Auto-save status: {:?}", report.auto_save_status);
    println!("   Preference status: {:?}", report.preference_status);
    println!("   Pipeline status: {:?}", report.pipeline_status);
    println!("   Dependency status: {:?}", report.dependency_status);
    println!("   Filesystem status: {:?}", report.filesystem_status);
    println!("   Recommendations: {} items", report.recommendations.len());
    
    if !report.recommendations.is_empty() {
        println!("\n6. Recommendations:");
        for (i, rec) in report.recommendations.iter().enumerate() {
            println!("   {}. {}", i + 1, rec.description());
        }
    }
    
    println!("\n7. Hardcoded value detection...");
    let tracer = PipelineTracer::new();
    let overrides = tracer.detect_hardcoded_false_values().await;
    println!("   Found {} potential hardcoded overrides", overrides.len());
    
    for (i, override_point) in overrides.iter().enumerate() {
        println!("   {}. {} ({})", 
            i + 1, 
            override_point.reason, 
            override_point.location
        );
    }
    
    println!("\n✅ Parameter tracing demo completed!");
    println!("   The system can now trace the auto_save parameter from preferences");
    println!("   through all recording pipeline components to identify where");
    println!("   MP4 recording might be failing.");
}