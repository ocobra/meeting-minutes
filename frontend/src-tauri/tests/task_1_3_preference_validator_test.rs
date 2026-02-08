/// Task 1.3: Preference Validator Component Tests
/// 
/// This test file validates that the PreferenceValidator component meets all requirements:
/// 1. Checks auto_save parameter loading from RecordingPreferences
/// 2. Validates preference file integrity and corruption detection
/// 3. Implements default value handling (default to true when missing/corrupted)
/// 
/// Requirements: 1.2, 4.1, 4.2

use app_lib::recording::diagnostics::{
    PreferenceValidator, AutoSaveStatus, PreferenceStatus, DiagnosticError
};
use app_lib::audio::recording_preferences::RecordingPreferences;
use std::path::PathBuf;

#[tokio::test]
async fn test_preference_validator_checks_auto_save_parameter() {
    // Requirement 1.2: Check auto_save parameter loading from RecordingPreferences
    let validator = PreferenceValidator::new();
    
    // Without app handle, should detect that preferences are not found
    let status = validator.check_auto_save_status().await;
    assert_eq!(status, AutoSaveStatus::NotFound, 
        "PreferenceValidator should detect when preferences are not available");
}

#[tokio::test]
async fn test_preference_validator_detects_corruption() {
    // Requirement 4.2: Validation for preference file integrity and corruption detection
    let validator = PreferenceValidator::new();
    
    // Test that validator can detect when preference file is not accessible
    let integrity_result = validator.check_preference_file_integrity().await;
    assert!(integrity_result.is_err(), 
        "PreferenceValidator should detect when preference file is not accessible");
    
    // Verify the error is the correct type
    match integrity_result {
        Err(DiagnosticError::AutoSaveParameterError(_)) => {
            // Expected error type when no app handle is available
        }
        _ => panic!("Expected AutoSaveParameterError when checking file integrity without app handle"),
    }
}

#[tokio::test]
async fn test_preference_validator_default_value_handling() {
    // Requirement 4.1 & 4.2: Default value handling (default to true when missing/corrupted)
    let validator = PreferenceValidator::new();
    
    // Test that default values are properly validated
    let default_validation = validator.validate_default_value_handling().await;
    assert!(default_validation.is_ok(), 
        "PreferenceValidator should validate that default values are correct");
    assert!(default_validation.unwrap(), 
        "Default value handling should return true when validation passes");
}

#[test]
fn test_preference_integrity_validation_valid_preferences() {
    // Test that validator correctly validates valid preferences
    let validator = PreferenceValidator::new();
    
    let valid_prefs = RecordingPreferences {
        save_folder: PathBuf::from("/home/user/recordings"),
        auto_save: true,  // Requirement 4.1: Default to true
        file_format: "mp4".to_string(),
        preferred_mic_device: None,
        preferred_system_device: None,
        #[cfg(target_os = "macos")]
        system_audio_backend: Some("coreaudio".to_string()),
    };
    
    let result = validator.validate_preference_integrity(&valid_prefs);
    assert!(result.is_ok(), 
        "PreferenceValidator should accept valid preferences");
}

#[test]
fn test_preference_integrity_validation_detects_empty_save_folder() {
    // Requirement 4.2: Detect corrupted/invalid preferences
    let validator = PreferenceValidator::new();
    
    let invalid_prefs = RecordingPreferences {
        save_folder: PathBuf::new(),  // Empty path - invalid
        auto_save: true,
        file_format: "mp4".to_string(),
        preferred_mic_device: None,
        preferred_system_device: None,
        #[cfg(target_os = "macos")]
        system_audio_backend: Some("coreaudio".to_string()),
    };
    
    let result = validator.validate_preference_integrity(&invalid_prefs);
    assert!(result.is_err(), 
        "PreferenceValidator should detect empty save folder");
    assert!(result.unwrap_err().contains("Save folder path is empty"),
        "Error message should indicate empty save folder");
}

#[test]
fn test_preference_integrity_validation_detects_invalid_format() {
    // Requirement 4.2: Detect corrupted/invalid preferences
    let validator = PreferenceValidator::new();
    
    let invalid_prefs = RecordingPreferences {
        save_folder: PathBuf::from("/home/user/recordings"),
        auto_save: true,
        file_format: "invalid_format".to_string(),  // Invalid format
        preferred_mic_device: None,
        preferred_system_device: None,
        #[cfg(target_os = "macos")]
        system_audio_backend: Some("coreaudio".to_string()),
    };
    
    let result = validator.validate_preference_integrity(&invalid_prefs);
    assert!(result.is_err(), 
        "PreferenceValidator should detect invalid file format");
    assert!(result.unwrap_err().contains("Unsupported file format"),
        "Error message should indicate unsupported file format");
}

#[test]
fn test_preference_integrity_validation_detects_system_directories() {
    // Requirement 4.2: Detect corrupted/invalid preferences (dangerous paths)
    let validator = PreferenceValidator::new();
    
    // Test root directory
    let root_prefs = RecordingPreferences {
        save_folder: PathBuf::from("/"),
        auto_save: true,
        file_format: "mp4".to_string(),
        preferred_mic_device: None,
        preferred_system_device: None,
        #[cfg(target_os = "macos")]
        system_audio_backend: Some("coreaudio".to_string()),
    };
    
    let result = validator.validate_preference_integrity(&root_prefs);
    assert!(result.is_err(), 
        "PreferenceValidator should detect root directory as invalid");
    assert!(result.unwrap_err().contains("system directory"),
        "Error message should indicate system directory");
}

#[test]
fn test_default_preferences_have_auto_save_true() {
    // Requirement 4.1: Default auto_save to true
    let default_prefs = RecordingPreferences::default();
    
    assert!(default_prefs.auto_save, 
        "Default RecordingPreferences must have auto_save=true per Requirement 4.1");
    assert!(!default_prefs.save_folder.as_os_str().is_empty(),
        "Default RecordingPreferences must have a valid save folder");
    assert_eq!(default_prefs.file_format, "mp4",
        "Default RecordingPreferences should use mp4 format");
}

#[tokio::test]
async fn test_preference_validator_repair_attempts() {
    // Requirement 4.2: Repair corrupted preferences
    let validator = PreferenceValidator::new();
    
    // Without app handle, repair should fail gracefully
    let repair_result = validator.repair_corrupted_preferences().await;
    assert!(repair_result.is_err(), 
        "PreferenceValidator should return error when repair is not possible");
    
    // Verify the error is the correct type
    match repair_result {
        Err(DiagnosticError::CorruptedPreferences(_)) => {
            // Expected error type when no app handle is available
        }
        _ => panic!("Expected CorruptedPreferences error when repairing without app handle"),
    }
}

#[tokio::test]
async fn test_preference_validator_validates_preferences() {
    // Test overall preference validation
    let validator = PreferenceValidator::new();
    
    // Without app handle, should return LoadError
    let pref_status = validator.validate_preferences().await;
    assert!(matches!(pref_status, PreferenceStatus::LoadError(_)),
        "PreferenceValidator should return LoadError when app handle is not available");
}

#[test]
fn test_preference_validator_supports_all_valid_formats() {
    // Test that validator accepts all valid file formats
    let validator = PreferenceValidator::new();
    
    for format in &["mp4", "wav", "m4a"] {
        let prefs = RecordingPreferences {
            save_folder: PathBuf::from("/home/user/recordings"),
            auto_save: true,
            file_format: format.to_string(),
            preferred_mic_device: None,
            preferred_system_device: None,
            #[cfg(target_os = "macos")]
            system_audio_backend: Some("coreaudio".to_string()),
        };
        
        let result = validator.validate_preference_integrity(&prefs);
        assert!(result.is_ok(), 
            "PreferenceValidator should accept {} format", format);
    }
}

#[test]
fn test_auto_save_status_enum_variants() {
    // Test that all AutoSaveStatus variants are properly defined
    let enabled = AutoSaveStatus::Enabled;
    let disabled = AutoSaveStatus::Disabled;
    let corrupted = AutoSaveStatus::Corrupted;
    let not_found = AutoSaveStatus::NotFound;
    let hardcoded = AutoSaveStatus::HardcodedFalse("test.rs:123".to_string());
    
    // Verify they are distinct
    assert_ne!(enabled, disabled);
    assert_ne!(enabled, corrupted);
    assert_ne!(enabled, not_found);
    
    // Verify HardcodedFalse contains location
    if let AutoSaveStatus::HardcodedFalse(location) = hardcoded {
        assert_eq!(location, "test.rs:123");
    } else {
        panic!("Expected HardcodedFalse variant");
    }
}

#[test]
fn test_preference_status_enum_variants() {
    // Test that all PreferenceStatus variants are properly defined
    let valid = PreferenceStatus::Valid;
    let corrupted = PreferenceStatus::Corrupted("test error".to_string());
    let missing = PreferenceStatus::Missing;
    let load_error = PreferenceStatus::LoadError("test error".to_string());
    
    // Verify they can be created and matched
    match valid {
        PreferenceStatus::Valid => {},
        _ => panic!("Expected Valid variant"),
    }
    
    match corrupted {
        PreferenceStatus::Corrupted(msg) => assert_eq!(msg, "test error"),
        _ => panic!("Expected Corrupted variant"),
    }
    
    match missing {
        PreferenceStatus::Missing => {},
        _ => panic!("Expected Missing variant"),
    }
    
    match load_error {
        PreferenceStatus::LoadError(msg) => assert_eq!(msg, "test error"),
        _ => panic!("Expected LoadError variant"),
    }
}
