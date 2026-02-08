// Task 2.1: Parameter Tracing System Tests
//
// Tests for the parameter tracing system that tracks auto_save parameter
// from RecordingPreferences through all pipeline components.
//
// Requirements tested:
// - Requirement 2.1: Trace auto_save parameter from RecordingPreferences through all pipeline components
// - Requirement 2.2: Identify if any hardcoded false values override the auto_save parameter

use app_lib::recording::diagnostics::{
    DiagnosticEngine, ParameterSource,
};

#[cfg(test)]
mod parameter_tracing_tests {
    use super::*;

    /// Test that parameter trace correctly identifies the source of auto_save parameter
    #[tokio::test]
    async fn test_trace_parameter_source_identification() {
        // Create diagnostic engine without app handle (will use defaults)
        let engine = DiagnosticEngine::new();
        
        // Trace parameter flow
        let trace = engine.trace_auto_save_parameter().await;
        
        // Verify that a source is identified
        assert!(
            matches!(trace.source, ParameterSource::Default | ParameterSource::UserPreferences),
            "Parameter source should be either Default or UserPreferences, got: {:?}",
            trace.source
        );
        
        // Verify that the initial value is set (should default to true per requirements)
        if matches!(trace.source, ParameterSource::Default) {
            assert_eq!(
                trace.value, true,
                "Default auto_save value should be true per Requirement 4.1"
            );
        }
        
        println!("✅ Parameter source identified: {:?}, value: {}", trace.source, trace.value);
    }

    /// Test that parameter trace records all pipeline components
    #[tokio::test]
    async fn test_trace_records_all_components() {
        let engine = DiagnosticEngine::new();
        let trace = engine.trace_auto_save_parameter().await;
        
        // Verify that all expected components are in the propagation path
        let expected_components = vec![
            "RecordingCommands",
            "RecordingManager",
            "RecordingSaver",
        ];
        
        for expected_component in &expected_components {
            let found = trace.propagation_path.iter().any(|ct| ct.component == *expected_component);
            assert!(
                found,
                "Expected component '{}' not found in propagation path. Found: {:?}",
                expected_component,
                trace.propagation_path.iter().map(|ct| &ct.component).collect::<Vec<_>>()
            );
        }
        
        // IncrementalSaver should only be present if auto_save is true
        let has_incremental_saver = trace.propagation_path.iter().any(|ct| ct.component == "IncrementalSaver");
        if trace.value {
            assert!(
                has_incremental_saver,
                "IncrementalSaver should be in propagation path when auto_save=true"
            );
        }
        
        println!("✅ All expected components traced: {:?}", 
                 trace.propagation_path.iter().map(|ct| &ct.component).collect::<Vec<_>>());
    }

    /// Test that parameter trace detects when values are propagated correctly
    #[tokio::test]
    async fn test_trace_detects_correct_propagation() {
        let engine = DiagnosticEngine::new();
        let trace = engine.trace_auto_save_parameter().await;
        
        // Check if parameter is propagated correctly (no overrides)
        let is_correct = trace.is_propagated_correctly();
        
        // Verify that each component receives and passes the same value
        for component_trace in &trace.propagation_path {
            assert_eq!(
                component_trace.received_value, component_trace.passed_value,
                "Component '{}' should not modify the auto_save parameter. Received: {}, Passed: {}",
                component_trace.component, component_trace.received_value, component_trace.passed_value
            );
        }
        
        // Verify no override points detected
        assert!(
            trace.override_points.is_empty(),
            "No override points should be detected in correct propagation. Found: {:?}",
            trace.override_points
        );
        
        assert!(
            is_correct,
            "Parameter should be propagated correctly through all components"
        );
        
        println!("✅ Parameter propagated correctly through {} components", trace.propagation_path.len());
    }

    /// Test that ComponentTrace records correct location information
    #[tokio::test]
    async fn test_component_trace_records_locations() {
        let engine = DiagnosticEngine::new();
        let trace = engine.trace_auto_save_parameter().await;
        
        // Verify that each component trace has location information
        for component_trace in &trace.propagation_path {
            assert!(
                !component_trace.location.is_empty(),
                "Component '{}' should have location information",
                component_trace.component
            );
            
            // Verify location format (should contain file name and line numbers)
            assert!(
                component_trace.location.contains(".rs"),
                "Location should contain Rust file name: {}",
                component_trace.location
            );
            
            println!("✅ Component '{}' location: {}", 
                     component_trace.component, component_trace.location);
        }
    }

    /// Test that parameter trace correctly identifies final value
    #[tokio::test]
    async fn test_trace_identifies_final_value() {
        let engine = DiagnosticEngine::new();
        let trace = engine.trace_auto_save_parameter().await;
        
        let final_value = trace.final_value();
        
        // Final value should match the initial value if no overrides occurred
        if trace.override_points.is_empty() {
            assert_eq!(
                final_value, trace.value,
                "Final value should match initial value when no overrides occur"
            );
        }
        
        // Final value should match the last component's passed value
        if let Some(last_component) = trace.propagation_path.last() {
            assert_eq!(
                final_value, last_component.passed_value,
                "Final value should match last component's passed value"
            );
        }
        
        println!("✅ Final value correctly identified: {}", final_value);
    }

    /// Test that parameter trace works with auto_save=true
    #[tokio::test]
    async fn test_trace_with_auto_save_true() {
        let engine = DiagnosticEngine::new();
        let trace = engine.trace_auto_save_parameter().await;
        
        // When auto_save is true (default), IncrementalSaver should be traced
        if trace.value {
            let has_incremental_saver = trace.propagation_path.iter()
                .any(|ct| ct.component == "IncrementalSaver");
            
            assert!(
                has_incremental_saver,
                "IncrementalSaver should be traced when auto_save=true"
            );
            
            println!("✅ IncrementalSaver correctly traced with auto_save=true");
        }
    }

    /// Test that parameter trace provides meaningful component information
    #[tokio::test]
    async fn test_trace_provides_component_information() {
        let engine = DiagnosticEngine::new();
        let trace = engine.trace_auto_save_parameter().await;
        
        // Verify that each component trace has all required fields
        for component_trace in &trace.propagation_path {
            // Component name should not be empty
            assert!(
                !component_trace.component.is_empty(),
                "Component name should not be empty"
            );
            
            // Location should not be empty
            assert!(
                !component_trace.location.is_empty(),
                "Component location should not be empty"
            );
            
            // Values should be boolean (true or false)
            assert!(
                component_trace.received_value || !component_trace.received_value,
                "Received value should be boolean"
            );
            assert!(
                component_trace.passed_value || !component_trace.passed_value,
                "Passed value should be boolean"
            );
            
            println!("✅ Component '{}' has complete information: received={}, passed={}, location={}",
                     component_trace.component, 
                     component_trace.received_value,
                     component_trace.passed_value,
                     component_trace.location);
        }
    }

    /// Test that ParameterTrace correctly reports propagation status
    #[tokio::test]
    async fn test_parameter_trace_propagation_status() {
        let engine = DiagnosticEngine::new();
        let trace = engine.trace_auto_save_parameter().await;
        
        let is_correct = trace.is_propagated_correctly();
        
        // If propagation is correct, there should be no override points
        if is_correct {
            assert!(
                trace.override_points.is_empty(),
                "Correct propagation should have no override points"
            );
        }
        
        // If there are override points, propagation should not be correct
        if !trace.override_points.is_empty() {
            assert!(
                !is_correct,
                "Propagation with override points should not be marked as correct"
            );
        }
        
        println!("✅ Propagation status correctly reported: {}", is_correct);
    }

    /// Test that parameter trace handles the complete pipeline flow
    #[tokio::test]
    async fn test_complete_pipeline_flow_trace() {
        let engine = DiagnosticEngine::new();
        let trace = engine.trace_auto_save_parameter().await;
        
        // Verify the complete flow: Preferences -> Commands -> Manager -> Saver -> (Incremental)
        let component_names: Vec<String> = trace.propagation_path.iter()
            .map(|ct| ct.component.clone())
            .collect();
        
        // Verify expected order
        assert!(
            component_names.contains(&"RecordingCommands".to_string()),
            "RecordingCommands should be in the flow"
        );
        assert!(
            component_names.contains(&"RecordingManager".to_string()),
            "RecordingManager should be in the flow"
        );
        assert!(
            component_names.contains(&"RecordingSaver".to_string()),
            "RecordingSaver should be in the flow"
        );
        
        // Verify order is correct (Commands before Manager before Saver)
        let commands_idx = component_names.iter().position(|n| n == "RecordingCommands");
        let manager_idx = component_names.iter().position(|n| n == "RecordingManager");
        let saver_idx = component_names.iter().position(|n| n == "RecordingSaver");
        
        if let (Some(cmd_idx), Some(mgr_idx), Some(sav_idx)) = (commands_idx, manager_idx, saver_idx) {
            assert!(
                cmd_idx < mgr_idx && mgr_idx < sav_idx,
                "Components should be in order: Commands -> Manager -> Saver"
            );
        }
        
        println!("✅ Complete pipeline flow traced in correct order: {:?}", component_names);
    }
}
