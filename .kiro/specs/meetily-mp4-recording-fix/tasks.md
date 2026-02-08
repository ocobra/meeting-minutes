# Implementation Plan: Meetily MP4 Recording Fix

## Overview

This implementation plan systematically diagnoses and fixes the critical MP4 recording bug in Meetily where audio recordings are no longer being saved. The approach follows a diagnostic-first strategy to identify the root cause, followed by targeted fixes to restore the recording pipeline, and comprehensive testing to prevent future regressions.

## Tasks

- [ ] 1. Create diagnostic engine infrastructure
  - [x] 1.1 Create diagnostic engine module with core structures
    - Create `src/recording/diagnostics/mod.rs` with DiagnosticEngine, DiagnosticReport, ParameterTrace, and ComponentTrace structs
    - Define AutoSaveStatus, PreferenceStatus, PipelineStatus enums for comprehensive reporting
    - Implement basic diagnostic report generation
    - _Requirements: 1.1, 1.2, 2.1_
  
  - [ ]* 1.2 Write property test for diagnostic engine
    - **Property 1: Diagnostic system validates recording pipeline state**
    - **Validates: Requirements 1.1, 1.2, 1.3, 2.1, 2.2**
  
  - [x] 1.3 Implement preference validator component
    - Create PreferenceValidator that checks auto_save parameter loading from RecordingPreferences
    - Add validation for preference file integrity and corruption detection
    - Implement default value handling (default to true when missing/corrupted)
    - _Requirements: 1.2, 4.1, 4.2_

- [ ] 2. Implement pipeline tracing and parameter flow analysis
  - [x] 2.1 Create parameter tracing system
    - Implement ParameterTrace to track auto_save parameter from RecordingPreferences through all pipeline components
    - Add ComponentTrace to record parameter value at each component (recording_commands → recording_saver → incremental_saver)
    - Detect where parameter values change or are overridden
    - _Requirements: 2.1, 2.2_
  
  - [x] 2.2 Add hardcoded value detection
    - Scan codebase for hardcoded false values that might override auto_save parameter
    - Create detection logic for common override patterns in initialization code
    - Report locations where auto_save might be incorrectly set to false
    - _Requirements: 2.2_
  
  - [ ]* 2.3 Write property test for parameter tracing
    - **Property 8: End-to-end pipeline validation**
    - **Validates: Requirements 6.1, 6.4, 6.5**

- [ ] 3. Implement dependency and filesystem validation
  - [x] 3.1 Create dependency checker for FFmpeg
    - Implement FFmpeg detection in system PATH and common installation locations
    - Add version validation to ensure FFmpeg supports required features
    - Check executable permissions and accessibility
    - _Requirements: 1.4, 5.1_
  
  - [x] 3.2 Implement filesystem validator
    - Create Meeting_Folder validation and creation logic
    - Add permission checking for write access to save locations
    - Implement alternative location fallback when primary location fails
    - _Requirements: 1.5, 5.2_
  
  - [ ]* 3.3 Write property test for external dependency validation
    - **Property 2: External dependency validation**
    - **Validates: Requirements 1.4, 1.5**

- [x] 4. Checkpoint - Run initial diagnostics
  - Run diagnostic engine on current system to identify root cause of MP4 recording failure
  - Review diagnostic report to determine which components need fixes
  - Ensure all tests pass, ask the user if questions arise.

- [ ] 5. Enhance preference management system
  - [x] 5.1 Implement robust preference loading with validation
    - Enhance RecordingPreferences::load() with validation and corruption detection
    - Add repair_corrupted_preferences() method to restore defaults when needed
    - Implement ensure_defaults() to provide safe fallback values (auto_save=true)
    - _Requirements: 4.1, 4.2, 4.5_
  
  - [x] 5.2 Add preference persistence validation
    - Implement reliable preference saving with integrity checks
    - Add validation that auto_save parameter is correctly persisted to storage
    - Implement rollback capability for failed preference updates
    - _Requirements: 4.3_
  
  - [ ]* 5.3 Write property test for preference management
    - **Property 5: Preference management robustness**
    - **Validates: Requirements 4.1, 4.2, 4.3, 4.4, 4.5**

- [ ] 6. Fix recording pipeline initialization and checkpoint creation
  - [x] 6.1 Enhance recording pipeline with diagnostics integration
    - Modify RecordingPipeline to use DiagnosticEngine during initialization
    - Validate that auto_save parameter flows correctly from RecordingPreferences to IncrementalSaver
    - Ensure IncrementalSaver is properly initialized when auto_save=true
    - _Requirements: 1.3, 2.3, 4.4_
  
  - [x] 6.2 Fix checkpoint directory and file creation
    - Ensure .checkpoints/ directory is created when auto_save=true
    - Implement 30-second audio chunk saving as Checkpoint_Files
    - Add proper error handling for directory and file creation failures
    - _Requirements: 3.1, 3.2, 5.3_
  
  - [~] 6.3 Implement FFmpeg merging and final MP4 creation
    - Ensure FFmpeg correctly merges all Checkpoint_Files into single audio.mp4
    - Verify final MP4 file is saved to designated Meeting_Folder
    - Add cleanup of checkpoint files after successful merge
    - _Requirements: 2.4, 2.5, 3.3, 3.4_
  
  - [ ]* 6.4 Write property test for recording pipeline with auto_save enabled
    - **Property 3: Recording pipeline behavior with auto_save enabled**
    - **Validates: Requirements 3.1, 3.2, 3.3, 3.4, 2.3, 2.4, 2.5**

- [ ] 7. Implement transcript-only mode validation
  - [x] 7.1 Ensure correct behavior when auto_save=false
    - Verify that audio chunks are properly discarded when auto_save is disabled
    - Ensure only transcripts are saved (no MP4 files created)
    - Validate that transcript functionality works independently of audio recording
    - _Requirements: 3.5_
  
  - [ ]* 7.2 Write property test for auto_save disabled behavior
    - **Property 4: Recording pipeline behavior with auto_save disabled**
    - **Validates: Requirements 3.5**

- [ ] 8. Implement enhanced error handling and recovery
  - [x] 8.1 Create comprehensive error handling system
    - Implement RecordingError enum with detailed error contexts (FFmpegNotFound, MeetingFolderError, CheckpointError, MergingError)
    - Add recovery strategies for each error type
    - Implement RecoveryStrategy with primary fix and fallback actions
    - _Requirements: 5.1, 5.2, 5.3, 5.4_
  
  - [x] 8.2 Implement graceful degradation to transcript-only mode
    - Add fallback logic when MP4 recording fails (FFmpeg missing, disk full, permission errors)
    - Ensure transcript functionality continues during audio recording failures
    - Provide clear error messages and guidance for resolution
    - _Requirements: 5.1, 5.2, 5.3, 5.5_
  
  - [x] 8.3 Implement checkpoint preservation on merge failure
    - When FFmpeg merging fails, preserve individual Checkpoint_Files for manual recovery
    - Log locations of preserved checkpoint files
    - Provide guidance for manual recovery process
    - _Requirements: 5.4_
  
  - [ ]* 8.4 Write property test for error handling and graceful degradation
    - **Property 6: Error handling and graceful degradation**
    - **Validates: Requirements 5.1, 5.2, 5.3, 5.4, 5.5**

- [ ] 9. Add comprehensive logging throughout pipeline
  - [x] 9.1 Implement structured logging for recording operations
    - Add logging for auto_save parameter value and source at recording start
    - Log checkpoint file creation with paths and sizes
    - Add timestamps and context to all log entries
    - _Requirements: 7.1, 7.2_
  
  - [x] 9.2 Add FFmpeg operation and error logging
    - Log FFmpeg commands executed with full arguments
    - Log FFmpeg output (stdout and stderr)
    - Add detailed error logging with component context for all pipeline errors
    - _Requirements: 7.3, 7.4_
  
  - [x] 9.3 Add completion and final file logging
    - Log final MP4 file location and size when recording completes successfully
    - Add summary logging for both successful and failed recordings
    - Include diagnostic information in error logs
    - _Requirements: 7.5_
  
  - [ ]* 9.4 Write property test for comprehensive logging
    - **Property 7: Comprehensive logging throughout pipeline**
    - **Validates: Requirements 7.1, 7.2, 7.3, 7.4, 7.5**

- [ ] 10. Create comprehensive test suite
  - [x] 10.1 Set up property-based testing framework
    - Add proptest dependency to Cargo.toml
    - Create custom strategies for preference files (valid, corrupted, missing)
    - Create strategies for recording states and error conditions
    - Configure minimum 100 iterations per property test
    - _Requirements: 6.1, 6.2, 6.3_
  
  - [ ]* 10.2 Write property test for error condition coverage
    - **Property 9: Error condition testing coverage**
    - **Validates: Requirements 6.2, 6.3**
  
  - [ ]* 10.3 Write unit tests for specific regression scenarios
    - Test known failure scenarios that caused the original bug
    - Add tests for edge cases (empty recordings, interrupted recordings)
    - Test integration points between components
    - _Requirements: 6.1, 6.2, 6.5_

- [ ] 11. Integration and final validation
  - [x] 11.1 Wire all components together in main recording system
    - Integrate DiagnosticEngine into existing recording_commands.rs
    - Connect enhanced RecordingPreferences with RecordingPipeline
    - Ensure all enhanced components work together seamlessly
    - _Requirements: All requirements_
  
  - [x] 11.2 Add diagnostic command for troubleshooting
    - Create CLI command to run full diagnostic report
    - Add user-friendly output for common issues and recommended fixes
    - Include auto_save parameter trace in diagnostic output
    - _Requirements: 1.1, 1.2, 1.3, 1.4, 1.5_
  
  - [ ]* 11.3 Write integration tests for end-to-end recording flow
    - Test complete recording workflow from start to final MP4 creation
    - Test error scenarios and recovery paths
    - Validate that all components interact correctly
    - _Requirements: 6.1, 6.4, 6.5_

- [x] 12. Final checkpoint - Comprehensive testing and validation
  - Run full test suite including all property tests and unit tests
  - Verify that MP4 recording works correctly with auto_save=true (creates checkpoints, merges to audio.mp4, saves to Meeting_Folder)
  - Verify that transcript-only mode works correctly with auto_save=false (no MP4 files created)
  - Run diagnostic command to confirm all systems are healthy
  - Ensure all tests pass, ask the user if questions arise.

## Notes

- Tasks marked with `*` are optional and can be skipped for faster MVP
- Each task references specific requirements for traceability
- The diagnostic-first approach (tasks 1-4) ensures we identify the root cause before implementing fixes
- Property tests validate universal correctness properties across all inputs (minimum 100 iterations each)
- Unit tests validate specific examples, edge cases, and regression scenarios
- Comprehensive logging provides visibility for future debugging and maintenance
- Error handling focuses on graceful degradation to maintain transcript functionality even when MP4 recording fails
