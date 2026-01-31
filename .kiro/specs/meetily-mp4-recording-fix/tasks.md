# Implementation Plan: Meetily MP4 Recording Fix

## Overview

This implementation plan systematically diagnoses and fixes the critical MP4 recording bug in Meetily. The approach follows a diagnostic-first strategy to identify the root cause, followed by targeted fixes and comprehensive testing to prevent regression.

## Tasks

- [ ] 1. Create diagnostic engine infrastructure
  - [x] 1.1 Create diagnostic engine module with core structures
    - Create `src/recording/diagnostics/mod.rs` with DiagnosticEngine, DiagnosticReport, and ParameterTrace structs
    - Define error types and status enums for comprehensive reporting
    - _Requirements: 1.1, 1.2, 2.1_
  
  - [ ]* 1.2 Write property test for diagnostic engine initialization
    - **Property 1: Diagnostic system validates recording pipeline state**
    - **Validates: Requirements 1.1, 1.2, 1.3, 2.1, 2.2**
  
  - [x] 1.3 Implement preference validator component
    - Create PreferenceValidator that checks auto_save parameter loading and corruption
    - Add validation for preference file integrity and default value handling
    - _Requirements: 1.2, 4.1, 4.2_

- [ ] 2. Implement pipeline tracing and parameter flow analysis
  - [x] 2.1 Create parameter tracing system
    - Implement ParameterTrace to track auto_save parameter through all components
    - Add ComponentTrace to identify where parameter values change
    - _Requirements: 2.1, 2.2_
  
  - [x] 2.2 Add hardcoded value detection
    - Scan code for hardcoded false values that override auto_save parameter
    - Create detection logic for common override patterns
    - _Requirements: 2.2_
  
  - [ ]* 2.3 Write property test for parameter tracing
    - **Property 8: End-to-end pipeline validation**
    - **Validates: Requirements 6.1, 6.4, 6.5**

- [ ] 3. Implement dependency and filesystem validation
  - [x] 3.1 Create dependency checker for FFmpeg
    - Implement FFmpeg detection and version validation
    - Add path resolution and executable permission checking
    - _Requirements: 1.4, 5.1_
  
  - [x] 3.2 Implement filesystem validator
    - Create meeting folder validation and creation logic
    - Add permission checking and alternative location fallback
    - _Requirements: 1.5, 5.2_
  
  - [ ]* 3.3 Write property test for external dependency validation
    - **Property 2: External dependency validation**
    - **Validates: Requirements 1.4, 1.5**

- [x] 4. Checkpoint - Run initial diagnostics
  - Run diagnostic engine on current system to identify root cause
  - Ensure all tests pass, ask the user if questions arise.

- [ ] 5. Enhance preference management system
  - [x] 5.1 Implement robust preference loading
    - Enhance RecordingPreferences with validation and repair capabilities
    - Add default value restoration for corrupted preferences
    - _Requirements: 4.1, 4.2, 4.5_
  
  - [x] 5.2 Add preference persistence validation
    - Implement reliable preference saving with integrity checks
    - Add rollback capability for failed preference updates
    - _Requirements: 4.3_
  
  - [ ]* 5.3 Write property test for preference management
    - **Property 5: Preference management robustness**
    - **Validates: Requirements 4.1, 4.2, 4.3, 4.4, 4.5**

- [ ] 6. Fix recording pipeline initialization
  - [x] 6.1 Enhance recording pipeline with diagnostics integration
    - Modify RecordingPipeline to use DiagnosticEngine during initialization
    - Add validation that auto_save parameter flows correctly to IncrementalSaver
    - _Requirements: 1.3, 2.3, 4.4_
  
  - [x] 6.2 Fix checkpoint directory creation
    - Ensure .checkpoints/ directory is created when auto_save=true
    - Add proper error handling for directory creation failures
    - _Requirements: 3.1_
  
  - [ ]* 6.3 Write property test for recording pipeline with auto_save enabled
    - **Property 3: Recording pipeline behavior with auto_save enabled**
    - **Validates: Requirements 3.1, 3.2, 3.3, 3.4, 2.3, 2.4, 2.5**

- [ ] 7. Implement enhanced error handling and recovery
  - [x] 7.1 Create comprehensive error handling system
    - Implement RecordingError enum with detailed error contexts
    - Add recovery strategies for each error type
    - _Requirements: 5.1, 5.2, 5.3, 5.4, 5.5_
  
  - [x] 7.2 Implement graceful degradation to transcript-only mode
    - Add fallback logic when MP4 recording fails
    - Ensure transcript functionality continues during audio recording failures
    - _Requirements: 5.3, 5.5_
  
  - [ ]* 7.3 Write property test for error handling and graceful degradation
    - **Property 6: Error handling and graceful degradation**
    - **Validates: Requirements 5.1, 5.2, 5.3, 5.4, 5.5**

- [ ] 8. Add comprehensive logging throughout pipeline
  - [ ] 8.1 Implement structured logging for recording operations
    - Add logging for auto_save parameter source and value at recording start
    - Log checkpoint file creation with paths and sizes
    - _Requirements: 7.1, 7.2_
  
  - [~] 8.2 Add FFmpeg operation logging
    - Log FFmpeg commands executed and their output
    - Add detailed error logging with component context
    - _Requirements: 7.3, 7.4_
  
  - [~] 8.3 Add completion and final file logging
    - Log final MP4 file location and size when recording completes
    - Add summary logging for successful and failed recordings
    - _Requirements: 7.5_
  
  - [ ]* 8.4 Write property test for comprehensive logging
    - **Property 7: Comprehensive logging throughout pipeline**
    - **Validates: Requirements 7.1, 7.2, 7.3, 7.4, 7.5**

- [ ] 9. Implement transcript-only mode validation
  - [~] 9.1 Ensure transcript-only behavior when auto_save=false
    - Verify that audio chunks are properly discarded when auto_save is disabled
    - Ensure transcript functionality works independently of audio recording
    - _Requirements: 3.5_
  
  - [ ]* 9.2 Write property test for auto_save disabled behavior
    - **Property 4: Recording pipeline behavior with auto_save disabled**
    - **Validates: Requirements 3.5**

- [ ] 10. Create comprehensive test suite
  - [~] 10.1 Set up property-based testing framework
    - Add proptest dependency and configure test generators
    - Create custom strategies for preference files, recording states, and error conditions
    - _Requirements: 6.1, 6.2, 6.3_
  
  - [ ]* 10.2 Write property test for error condition coverage
    - **Property 9: Error condition testing coverage**
    - **Validates: Requirements 6.2, 6.3**
  
  - [ ]* 10.3 Write unit tests for specific regression scenarios
    - Test known failure scenarios that caused the original bug
    - Add tests for edge cases and integration points
    - _Requirements: 6.1, 6.2, 6.5_

- [ ] 11. Integration and final validation
  - [~] 11.1 Wire all components together in main recording system
    - Integrate DiagnosticEngine into existing recording_commands.rs
    - Ensure all enhanced components work together seamlessly
    - _Requirements: All requirements_
  
  - [~] 11.2 Add diagnostic command for troubleshooting
    - Create CLI command to run full diagnostic report
    - Add user-friendly output for common issues and fixes
    - _Requirements: 1.1, 1.2, 1.3, 1.4, 1.5_

- [~] 12. Final checkpoint - Comprehensive testing and validation
  - Run full test suite including all property tests and unit tests
  - Verify that MP4 recording works correctly with auto_save=true
  - Verify that transcript-only mode works correctly with auto_save=false
  - Ensure all tests pass, ask the user if questions arise.

## Notes

- Tasks marked with `*` are optional and can be skipped for faster MVP
- Each task references specific requirements for traceability
- Property tests validate universal correctness properties across all inputs
- Unit tests validate specific examples, edge cases, and regression scenarios
- The diagnostic-first approach ensures we identify the root cause before implementing fixes
- Comprehensive logging provides visibility for future debugging and maintenance