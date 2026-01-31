use log::{info, warn, error};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tauri::{AppHandle, Runtime};
use tauri_plugin_store::StoreExt;

use anyhow::Result;
#[cfg(target_os = "macos")]
use log::error;

#[cfg(target_os = "macos")]
use crate::audio::capture::AudioCaptureBackend;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RecordingPreferences {
    pub save_folder: PathBuf,
    pub auto_save: bool,
    pub file_format: String,
    #[serde(default)]
    pub preferred_mic_device: Option<String>,
    #[serde(default)]
    pub preferred_system_device: Option<String>,
    #[cfg(target_os = "macos")]
    #[serde(default)]
    pub system_audio_backend: Option<String>,
}

impl Default for RecordingPreferences {
    fn default() -> Self {
        Self {
            save_folder: get_default_recordings_folder(),
            // Requirement 4.1 & 4.2: Default auto_save to true
            auto_save: true,
            file_format: "mp4".to_string(),
            preferred_mic_device: None,
            preferred_system_device: None,
            #[cfg(target_os = "macos")]
            system_audio_backend: Some("coreaudio".to_string()),
        }
    }
}

/// Get the default recordings folder based on platform
pub fn get_default_recordings_folder() -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        // Windows: %USERPROFILE%\Music\meetily-recordings
        if let Some(music_dir) = dirs::audio_dir() {
            music_dir.join("meetily-recordings")
        } else {
            // Fallback to Documents if Music folder is not available
            dirs::document_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join("meetily-recordings")
        }
    }

    #[cfg(target_os = "macos")]
    {
        // macOS: ~/Movies/meetily-recordings
        if let Some(movies_dir) = dirs::video_dir() {
            movies_dir.join("meetily-recordings")
        } else {
            // Fallback to Documents if Movies folder is not available
            dirs::document_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join("meetily-recordings")
        }
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        // Linux/Others: ~/Documents/meetily-recordings
        dirs::document_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("meetily-recordings")
    }
}

/// Ensure the recordings directory exists
pub fn ensure_recordings_directory(path: &PathBuf) -> Result<()> {
    if !path.exists() {
        std::fs::create_dir_all(path)?;
        info!("Created recordings directory: {:?}", path);
    }
    Ok(())
}

/// Generate a unique filename for a recording
pub fn generate_recording_filename(format: &str) -> String {
    let now = chrono::Utc::now();
    let timestamp = now.format("%Y%m%d_%H%M%S");
    format!("recording_{}.{}", timestamp, format)
}

/// Load recording preferences from store
pub async fn load_recording_preferences<R: Runtime>(
    app: &AppHandle<R>,
) -> Result<RecordingPreferences> {
    // Try to load from Tauri store
    let store = match app.store("recording_preferences.json") {
        Ok(store) => store,
        Err(e) => {
            warn!("Failed to access store: {}, using defaults", e);
            return Ok(RecordingPreferences::default());
        }
    };

    // Try to get the preferences from store
    let prefs = if let Some(value) = store.get("preferences") {
        match serde_json::from_value::<RecordingPreferences>(value.clone()) {
            Ok(mut p) => {
                info!("Loaded recording preferences from store");
                // Update macOS backend to current value if needed
                #[cfg(target_os = "macos")]
                {
                    let backend = crate::audio::capture::get_current_backend();
                    p.system_audio_backend = Some(backend.to_string());
                }
                p
            }
            Err(e) => {
                warn!("Failed to deserialize preferences: {}, using defaults", e);
                RecordingPreferences::default()
            }
        }
    } else {
        info!("No stored preferences found, using defaults");
        RecordingPreferences::default()
    };

    info!("Loaded recording preferences: save_folder={:?}, auto_save={}, format={}, mic={:?}, system={:?}",
          prefs.save_folder, prefs.auto_save, prefs.file_format,
          prefs.preferred_mic_device, prefs.preferred_system_device);
    Ok(prefs)
}

/// Enhanced preference loading with validation and repair capabilities
/// Implements Requirements 4.1, 4.2, 4.5
pub async fn load_recording_preferences_with_validation<R: Runtime>(
    app: &AppHandle<R>,
) -> Result<RecordingPreferences> {
    info!("Loading recording preferences with validation and repair capabilities");
    
    // Try to load preferences normally first
    match load_recording_preferences_internal(app).await {
        Ok(prefs) => {
            // Validate the loaded preferences
            match validate_preference_integrity(&prefs) {
                Ok(()) => {
                    info!("Preferences loaded and validated successfully");
                    Ok(prefs)
                }
                Err(validation_error) => {
                    error!("Preference validation failed: {}", validation_error);
                    // Requirement 4.2: Restore to default when corrupted/invalid
                    repair_corrupted_preferences(app).await
                }
            }
        }
        Err(e) => {
            // Requirement 4.5: Log error and use safe default values
            error!("Failed to load preferences: {}", e);
            info!("Using safe default values as per Requirement 4.5");
            Ok(RecordingPreferences::default())
        }
    }
}

/// Internal preference loading with detailed error handling
async fn load_recording_preferences_internal<R: Runtime>(
    app: &AppHandle<R>,
) -> Result<RecordingPreferences> {
    // Try to load from Tauri store
    let store = app.store("recording_preferences.json")
        .map_err(|e| anyhow::anyhow!("Failed to access preference store: {}", e))?;

    // Try to get the preferences from store
    if let Some(value) = store.get("preferences") {
        let mut prefs = serde_json::from_value::<RecordingPreferences>(value.clone())
            .map_err(|e| anyhow::anyhow!("Failed to deserialize preferences: {}", e))?;
        
        info!("Successfully loaded recording preferences from store");
        
        // Update macOS backend to current value if needed
        #[cfg(target_os = "macos")]
        {
            let backend = crate::audio::capture::get_current_backend();
            prefs.system_audio_backend = Some(backend.to_string());
        }
        
        Ok(prefs)
    } else {
        // Requirement 4.1: Default to true if no preference exists
        info!("No stored preferences found, defaulting auto_save to true (Requirement 4.1)");
        Ok(RecordingPreferences::default())
    }
}

/// Validate the integrity of loaded preferences
fn validate_preference_integrity(preferences: &RecordingPreferences) -> Result<()> {
    // Check that save_folder is not empty
    if preferences.save_folder.as_os_str().is_empty() {
        return Err(anyhow::anyhow!("Save folder path is empty"));
    }

    // Check that file_format is valid
    if preferences.file_format.is_empty() {
        return Err(anyhow::anyhow!("File format is empty"));
    }

    // Validate file format is supported
    match preferences.file_format.as_str() {
        "mp4" | "wav" | "m4a" => {
            // Valid formats
        }
        _ => {
            return Err(anyhow::anyhow!("Unsupported file format: {}", preferences.file_format));
        }
    }

    // Check that save_folder path is reasonable (not root, not system directories)
    let save_path_str = preferences.save_folder.to_string_lossy();
    
    // Check for exact matches to dangerous root paths
    if save_path_str == "/" || save_path_str == "C:\\" || save_path_str == "\\" {
        return Err(anyhow::anyhow!("Save folder path appears to be a system directory (root): {}", save_path_str));
    }
    
    // Check for dangerous system directories (exact matches or starting with these paths)
    let dangerous_system_paths = [
        "/usr/bin", "/bin", "/sbin", "/etc", "/var", "/sys", "/proc",
        "/System32", "C:\\System32", "C:\\Windows\\System32",
        "/Windows", "C:\\Windows", 
        "/Program Files", "C:\\Program Files",
        "/System", "C:\\System"
    ];
    
    for dangerous_path in &dangerous_system_paths {
        if save_path_str == *dangerous_path || save_path_str.starts_with(&format!("{}/", dangerous_path)) || save_path_str.starts_with(&format!("{}\\", dangerous_path)) {
            return Err(anyhow::anyhow!("Save folder path appears to be a system directory: {}", save_path_str));
        }
    }

    // Validate that auto_save is properly set (should be boolean, which it is by type)
    info!("Preference validation passed: auto_save={}, save_folder={:?}, format={}", 
          preferences.auto_save, preferences.save_folder, preferences.file_format);
    
    Ok(())
}

/// Repair corrupted preferences by restoring defaults
/// Implements Requirement 4.2
pub async fn repair_corrupted_preferences<R: Runtime>(
    app: &AppHandle<R>,
) -> Result<RecordingPreferences> {
    error!("Attempting to repair corrupted preferences (Requirement 4.2)");
    
    // Create default preferences with auto_save=true
    let default_prefs = RecordingPreferences::default();
    
    // Ensure auto_save is true as per requirements
    if !default_prefs.auto_save {
        error!("CRITICAL: Default preferences have auto_save=false, this violates requirements!");
        // Force auto_save to true
        let mut repaired_prefs = default_prefs;
        repaired_prefs.auto_save = true;
        
        // Try to save the repaired preferences
        match save_recording_preferences(app, &repaired_prefs).await {
            Ok(()) => {
                info!("Successfully repaired preferences with auto_save=true");
                Ok(repaired_prefs)
            }
            Err(e) => {
                error!("Failed to save repaired preferences: {}", e);
                // Return the repaired preferences even if we can't save them
                // This ensures the current session works correctly
                Ok(repaired_prefs)
            }
        }
    } else {
        // Default preferences are correct, try to save them
        match save_recording_preferences(app, &default_prefs).await {
            Ok(()) => {
                info!("Successfully repaired preferences with defaults");
                Ok(default_prefs)
            }
            Err(e) => {
                error!("Failed to save repaired preferences: {}", e);
                // Return the default preferences even if we can't save them
                // This ensures the current session works correctly
                Ok(default_prefs)
            }
        }
    }
}

/// Ensure default values are properly configured
/// Validates Requirements 4.1 and 4.2
pub fn ensure_default_values() -> RecordingPreferences {
    let mut prefs = RecordingPreferences::default();
    
    // Requirement 4.1 & 4.2: Ensure auto_save defaults to true
    if !prefs.auto_save {
        warn!("Default auto_save was false, correcting to true per requirements");
        prefs.auto_save = true;
    }
    
    // Ensure save folder is set
    if prefs.save_folder.as_os_str().is_empty() {
        warn!("Default save folder was empty, setting to default location");
        prefs.save_folder = get_default_recordings_folder();
    }
    
    // Ensure file format is set
    if prefs.file_format.is_empty() {
        warn!("Default file format was empty, setting to mp4");
        prefs.file_format = "mp4".to_string();
    }
    
    info!("Ensured default values: auto_save={}, save_folder={:?}, format={}", 
          prefs.auto_save, prefs.save_folder, prefs.file_format);
    
    prefs
}

/// Save recording preferences to store with enhanced validation and rollback capability
/// Implements Requirement 4.3: Reliable preference saving with integrity checks and rollback
pub async fn save_recording_preferences<R: Runtime>(
    app: &AppHandle<R>,
    preferences: &RecordingPreferences,
) -> Result<()> {
    info!("Saving recording preferences with integrity checks and rollback capability");
    info!("Preferences: save_folder={:?}, auto_save={}, format={}, mic={:?}, system={:?}",
          preferences.save_folder, preferences.auto_save, preferences.file_format,
          preferences.preferred_mic_device, preferences.preferred_system_device);

    // Validate preferences before saving (Requirement 4.3)
    validate_preference_integrity(preferences)
        .map_err(|e| anyhow::anyhow!("Cannot save invalid preferences: {}", e))?;

    // Create backup of current preferences for rollback capability
    let backup_preferences = match load_recording_preferences_internal(app).await {
        Ok(current_prefs) => {
            info!("Created backup of current preferences for rollback capability");
            Some(current_prefs)
        }
        Err(e) => {
            warn!("Could not create backup of current preferences: {}", e);
            None
        }
    };

    // Get or create store
    let store = app
        .store("recording_preferences.json")
        .map_err(|e| anyhow::anyhow!("Failed to access store: {}", e))?;

    // Serialize preferences to JSON value
    let prefs_value = serde_json::to_value(preferences)
        .map_err(|e| anyhow::anyhow!("Failed to serialize preferences: {}", e))?;

    // Save to store
    store.set("preferences", prefs_value.clone());

    // Persist to disk with error handling
    match store.save() {
        Ok(()) => {
            info!("Successfully persisted recording preferences to disk");
        }
        Err(e) => {
            error!("Failed to save store to disk: {}", e);
            return Err(anyhow::anyhow!("Failed to save store to disk: {}", e));
        }
    }

    // Enhanced integrity validation with comprehensive checks
    match validate_preference_persistence(app, preferences).await {
        Ok(()) => {
            info!("Preference persistence validation passed - all integrity checks successful");
        }
        Err(validation_error) => {
            error!("Preference persistence validation failed: {}", validation_error);
            
            // Attempt rollback if we have backup preferences
            if let Some(backup_prefs) = backup_preferences {
                warn!("Attempting rollback to previous preferences due to validation failure");
                match rollback_preferences(app, &backup_prefs).await {
                    Ok(()) => {
                        error!("Successfully rolled back to previous preferences after validation failure");
                        return Err(anyhow::anyhow!("Preference save failed validation and was rolled back: {}", validation_error));
                    }
                    Err(rollback_error) => {
                        error!("CRITICAL: Rollback failed after validation failure! Rollback error: {}, Original error: {}", 
                               rollback_error, validation_error);
                        return Err(anyhow::anyhow!("Preference save failed validation and rollback failed: {} (rollback error: {})", 
                                                 validation_error, rollback_error));
                    }
                }
            } else {
                error!("No backup available for rollback after validation failure");
                return Err(anyhow::anyhow!("Preference save failed validation and no backup available for rollback: {}", validation_error));
            }
        }
    }

    // Save backend preference to global config
    #[cfg(target_os = "macos")]
    if let Some(backend_str) = &preferences.system_audio_backend {
        if let Some(backend) = AudioCaptureBackend::from_string(backend_str) {
            info!("Setting audio capture backend to: {:?}", backend);
            crate::audio::capture::set_current_backend(backend);
        }
    }

    // Ensure the directory exists
    ensure_recordings_directory(&preferences.save_folder)?;

    info!("Preference save operation completed successfully with all validations passed");
    Ok(())
}

/// Enhanced preference persistence validation with comprehensive integrity checks
/// Implements Requirement 4.3: Reliable preference saving with integrity checks
async fn validate_preference_persistence<R: Runtime>(
    app: &AppHandle<R>,
    expected_preferences: &RecordingPreferences,
) -> Result<()> {
    info!("Running comprehensive preference persistence validation");

    // Attempt to reload preferences from disk
    let reloaded_prefs = load_recording_preferences_internal(app).await
        .map_err(|e| anyhow::anyhow!("Failed to reload preferences for validation: {}", e))?;

    // Comprehensive field-by-field validation
    let mut validation_errors = Vec::new();

    // Critical field: auto_save parameter
    if reloaded_prefs.auto_save != expected_preferences.auto_save {
        validation_errors.push(format!(
            "auto_save parameter not persisted correctly! Expected: {}, Got: {}",
            expected_preferences.auto_save, reloaded_prefs.auto_save
        ));
    }

    // Critical field: save_folder
    if reloaded_prefs.save_folder != expected_preferences.save_folder {
        validation_errors.push(format!(
            "save_folder not persisted correctly! Expected: {:?}, Got: {:?}",
            expected_preferences.save_folder, reloaded_prefs.save_folder
        ));
    }

    // Critical field: file_format
    if reloaded_prefs.file_format != expected_preferences.file_format {
        validation_errors.push(format!(
            "file_format not persisted correctly! Expected: {}, Got: {}",
            expected_preferences.file_format, reloaded_prefs.file_format
        ));
    }

    // Optional fields: preferred devices
    if reloaded_prefs.preferred_mic_device != expected_preferences.preferred_mic_device {
        validation_errors.push(format!(
            "preferred_mic_device not persisted correctly! Expected: {:?}, Got: {:?}",
            expected_preferences.preferred_mic_device, reloaded_prefs.preferred_mic_device
        ));
    }

    if reloaded_prefs.preferred_system_device != expected_preferences.preferred_system_device {
        validation_errors.push(format!(
            "preferred_system_device not persisted correctly! Expected: {:?}, Got: {:?}",
            expected_preferences.preferred_system_device, reloaded_prefs.preferred_system_device
        ));
    }

    // Platform-specific field: system_audio_backend
    #[cfg(target_os = "macos")]
    if reloaded_prefs.system_audio_backend != expected_preferences.system_audio_backend {
        validation_errors.push(format!(
            "system_audio_backend not persisted correctly! Expected: {:?}, Got: {:?}",
            expected_preferences.system_audio_backend, reloaded_prefs.system_audio_backend
        ));
    }

    // Check if any validation errors occurred
    if !validation_errors.is_empty() {
        let error_summary = validation_errors.join("; ");
        error!("Preference persistence validation failed with {} errors: {}", 
               validation_errors.len(), error_summary);
        return Err(anyhow::anyhow!("Preference persistence validation failed: {}", error_summary));
    }

    // Additional integrity check: validate the reloaded preferences
    validate_preference_integrity(&reloaded_prefs)
        .map_err(|e| anyhow::anyhow!("Reloaded preferences failed integrity check: {}", e))?;

    info!("All preference persistence validation checks passed successfully");
    Ok(())
}

/// Rollback preferences to a previous state
/// Implements Requirement 4.3: Rollback capability for failed preference updates
async fn rollback_preferences<R: Runtime>(
    app: &AppHandle<R>,
    backup_preferences: &RecordingPreferences,
) -> Result<()> {
    info!("Rolling back preferences to previous state");
    info!("Rollback preferences: save_folder={:?}, auto_save={}, format={}",
          backup_preferences.save_folder, backup_preferences.auto_save, backup_preferences.file_format);

    // Validate backup preferences before rollback
    validate_preference_integrity(backup_preferences)
        .map_err(|e| anyhow::anyhow!("Cannot rollback to invalid backup preferences: {}", e))?;

    // Get store
    let store = app
        .store("recording_preferences.json")
        .map_err(|e| anyhow::anyhow!("Failed to access store for rollback: {}", e))?;

    // Serialize backup preferences to JSON value
    let backup_value = serde_json::to_value(backup_preferences)
        .map_err(|e| anyhow::anyhow!("Failed to serialize backup preferences: {}", e))?;

    // Restore backup to store
    store.set("preferences", backup_value);

    // Persist rollback to disk
    store
        .save()
        .map_err(|e| anyhow::anyhow!("Failed to persist rollback to disk: {}", e))?;

    // Validate rollback was successful
    let restored_prefs = load_recording_preferences_internal(app).await
        .map_err(|e| anyhow::anyhow!("Failed to validate rollback: {}", e))?;

    // Verify rollback worked correctly
    if restored_prefs.auto_save != backup_preferences.auto_save {
        return Err(anyhow::anyhow!("Rollback validation failed: auto_save mismatch"));
    }

    info!("Successfully rolled back preferences and validated restoration");
    Ok(())
}

/// Tauri commands for recording preferences
#[tauri::command]
pub async fn get_recording_preferences<R: Runtime>(
    app: AppHandle<R>,
) -> Result<RecordingPreferences, String> {
    // Use the enhanced preference loading with validation and repair
    load_recording_preferences_with_validation(&app)
        .await
        .map_err(|e| format!("Failed to load recording preferences: {}", e))
}

#[tauri::command]
pub async fn set_recording_preferences<R: Runtime>(
    app: AppHandle<R>,
    preferences: RecordingPreferences,
) -> Result<(), String> {
    save_recording_preferences(&app, &preferences)
        .await
        .map_err(|e| format!("Failed to save recording preferences: {}", e))
}

#[tauri::command]
pub async fn set_recording_preferences_with_rollback<R: Runtime>(
    app: AppHandle<R>,
    preferences: RecordingPreferences,
) -> Result<(), String> {
    info!("Setting recording preferences with enhanced validation and rollback capability");
    save_recording_preferences(&app, &preferences)
        .await
        .map_err(|e| {
            error!("Failed to save recording preferences with rollback: {}", e);
            format!("Failed to save recording preferences: {}", e)
        })
}

#[tauri::command]
pub async fn repair_recording_preferences<R: Runtime>(
    app: AppHandle<R>,
) -> Result<RecordingPreferences, String> {
    info!("Repair recording preferences command called");
    repair_corrupted_preferences(&app)
        .await
        .map_err(|e| format!("Failed to repair recording preferences: {}", e))
}

#[tauri::command]
pub async fn validate_recording_preferences<R: Runtime>(
    app: AppHandle<R>,
) -> Result<bool, String> {
    info!("Validate recording preferences command called");
    match load_recording_preferences_with_validation(&app).await {
        Ok(_) => {
            info!("Recording preferences validation passed");
            Ok(true)
        }
        Err(e) => {
            error!("Recording preferences validation failed: {}", e);
            Ok(false)
        }
    }
}

#[tauri::command]
pub async fn get_default_recordings_folder_path() -> Result<String, String> {
    let path = get_default_recordings_folder();
    Ok(path.to_string_lossy().to_string())
}

#[tauri::command]
pub async fn open_recordings_folder<R: Runtime>(app: AppHandle<R>) -> Result<(), String> {
    let preferences = load_recording_preferences(&app)
        .await
        .map_err(|e| format!("Failed to load preferences: {}", e))?;

    // Ensure directory exists before trying to open it
    ensure_recordings_directory(&preferences.save_folder)
        .map_err(|e| format!("Failed to create directory: {}", e))?;

    let folder_path = preferences.save_folder.to_string_lossy().to_string();

    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .arg(&folder_path)
            .spawn()
            .map_err(|e| format!("Failed to open folder: {}", e))?;
    }

    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(&folder_path)
            .spawn()
            .map_err(|e| format!("Failed to open folder: {}", e))?;
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        std::process::Command::new("xdg-open")
            .arg(&folder_path)
            .spawn()
            .map_err(|e| format!("Failed to open folder: {}", e))?;
    }

    info!("Opened recordings folder: {}", folder_path);
    Ok(())
}

#[tauri::command]
pub async fn select_recording_folder<R: Runtime>(
    _app: AppHandle<R>,
) -> Result<Option<String>, String> {
    // Use Tauri's dialog to select folder
    // For now, return None - this would need to be implemented with tauri-plugin-dialog
    // when it's available in the Cargo.toml
    warn!("Folder selection not yet implemented - using dialog plugin");
    Ok(None)
}

// Backend selection commands

/// Get available audio capture backends for the current platform
#[tauri::command]
pub async fn get_available_audio_backends() -> Result<Vec<String>, String> {
    #[cfg(target_os = "macos")]
    {
        let backends = crate::audio::capture::get_available_backends();
        Ok(backends.iter().map(|b| b.to_string()).collect())
    }

    #[cfg(not(target_os = "macos"))]
    {
        // Only ScreenCaptureKit available on non-macOS
        Ok(vec!["screencapturekit".to_string()])
    }
}

/// Get current audio capture backend
#[tauri::command]
pub async fn get_current_audio_backend() -> Result<String, String> {
    #[cfg(target_os = "macos")]
    {
        let backend = crate::audio::capture::get_current_backend();
        Ok(backend.to_string())
    }

    #[cfg(not(target_os = "macos"))]
    {
        Ok("screencapturekit".to_string())
    }
}

/// Set audio capture backend
#[tauri::command]
pub async fn set_audio_backend(backend: String) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        use crate::audio::capture::AudioCaptureBackend;
        use crate::audio::permissions::{
            check_screen_recording_permission, request_screen_recording_permission,
        };

        let backend_enum = AudioCaptureBackend::from_string(&backend)
            .ok_or_else(|| format!("Invalid backend: {}", backend))?;

        // If switching to Core Audio, log information about Audio Capture permission
        if backend_enum == AudioCaptureBackend::CoreAudio {
            info!("🔐 Core Audio backend requires Audio Capture permission (macOS 14.4+)");
            info!("📍 Permission dialog will appear automatically when recording starts");

            // Check if permission is already granted (this is informational only)
            if !check_screen_recording_permission() {
                warn!("⚠️  Audio Capture permission may not be granted");

                // Attempt to open System Settings (opens System Settings)
                if let Err(e) = request_screen_recording_permission() {
                    error!("Failed to open System Settings: {}", e);
                }

                return Err(
                    "Core Audio requires Audio Capture permission. \
                    The permission dialog will appear when you start recording. \
                    If already denied, enable it in System Settings → Privacy & Security → Audio Capture, \
                    then restart the app.".to_string()
                );
            }

            info!(
                "✅ Core Audio backend selected - permission check will occur at recording start"
            );
        }

        info!("Setting audio backend to: {:?}", backend_enum);
        crate::audio::capture::set_current_backend(backend_enum);
        Ok(())
    }

    #[cfg(not(target_os = "macos"))]
    {
        if backend != "screencapturekit" {
            return Err(format!(
                "Backend {} not available on this platform",
                backend
            ));
        }
        Ok(())
    }
}

/// Get backend information (name and description)
#[derive(Serialize)]
pub struct BackendInfo {
    pub id: String,
    pub name: String,
    pub description: String,
}

#[tauri::command]
pub async fn get_audio_backend_info() -> Result<Vec<BackendInfo>, String> {
    #[cfg(target_os = "macos")]
    {
        use crate::audio::capture::AudioCaptureBackend;

        let backends = vec![
            BackendInfo {
                id: AudioCaptureBackend::ScreenCaptureKit.to_string(),
                name: AudioCaptureBackend::ScreenCaptureKit.name().to_string(),
                description: AudioCaptureBackend::ScreenCaptureKit
                    .description()
                    .to_string(),
            },
            BackendInfo {
                id: AudioCaptureBackend::CoreAudio.to_string(),
                name: AudioCaptureBackend::CoreAudio.name().to_string(),
                description: AudioCaptureBackend::CoreAudio.description().to_string(),
            },
        ];
        Ok(backends)
    }

    #[cfg(not(target_os = "macos"))]
    {
        Ok(vec![BackendInfo {
            id: "screencapturekit".to_string(),
            name: "ScreenCaptureKit".to_string(),
            description: "Default system audio capture".to_string(),
        }])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_default_preferences_have_auto_save_true() {
        // Requirement 4.1 & 4.2: Default auto_save should be true
        let prefs = RecordingPreferences::default();
        assert!(prefs.auto_save, "Default auto_save should be true per Requirements 4.1 & 4.2");
        assert!(!prefs.save_folder.as_os_str().is_empty(), "Default save folder should not be empty");
        assert_eq!(prefs.file_format, "mp4", "Default file format should be mp4");
    }

    #[test]
    fn test_ensure_default_values() {
        // Test the ensure_default_values function
        let prefs = ensure_default_values();
        assert!(prefs.auto_save, "Ensured default auto_save should be true");
        assert!(!prefs.save_folder.as_os_str().is_empty(), "Ensured default save folder should not be empty");
        assert!(!prefs.file_format.is_empty(), "Ensured default file format should not be empty");
    }

    #[test]
    fn test_validate_preference_integrity_valid() {
        let valid_prefs = RecordingPreferences {
            save_folder: PathBuf::from("/home/user/recordings"),
            auto_save: true,
            file_format: "mp4".to_string(),
            preferred_mic_device: None,
            preferred_system_device: None,
            #[cfg(target_os = "macos")]
            system_audio_backend: Some("coreaudio".to_string()),
        };

        let result = validate_preference_integrity(&valid_prefs);
        assert!(result.is_ok(), "Valid preferences should pass validation");
    }

    #[test]
    fn test_validate_preference_integrity_empty_save_folder() {
        let invalid_prefs = RecordingPreferences {
            save_folder: PathBuf::new(), // Empty path
            auto_save: true,
            file_format: "mp4".to_string(),
            preferred_mic_device: None,
            preferred_system_device: None,
            #[cfg(target_os = "macos")]
            system_audio_backend: Some("coreaudio".to_string()),
        };

        let result = validate_preference_integrity(&invalid_prefs);
        assert!(result.is_err(), "Empty save folder should fail validation");
        assert!(result.unwrap_err().to_string().contains("Save folder path is empty"));
    }

    #[test]
    fn test_validate_preference_integrity_invalid_format() {
        let invalid_prefs = RecordingPreferences {
            save_folder: PathBuf::from("/home/user/recordings"),
            auto_save: true,
            file_format: "invalid_format".to_string(),
            preferred_mic_device: None,
            preferred_system_device: None,
            #[cfg(target_os = "macos")]
            system_audio_backend: Some("coreaudio".to_string()),
        };

        let result = validate_preference_integrity(&invalid_prefs);
        assert!(result.is_err(), "Invalid file format should fail validation");
        assert!(result.unwrap_err().to_string().contains("Unsupported file format"));
    }

    #[test]
    fn test_validate_preference_integrity_system_directory() {
        let invalid_prefs = RecordingPreferences {
            save_folder: PathBuf::from("/"), // Root directory
            auto_save: true,
            file_format: "mp4".to_string(),
            preferred_mic_device: None,
            preferred_system_device: None,
            #[cfg(target_os = "macos")]
            system_audio_backend: Some("coreaudio".to_string()),
        };

        let result = validate_preference_integrity(&invalid_prefs);
        assert!(result.is_err(), "System directory should fail validation");
        assert!(result.unwrap_err().to_string().contains("system directory"));
    }

    #[test]
    fn test_get_default_recordings_folder_not_empty() {
        let folder = get_default_recordings_folder();
        assert!(!folder.as_os_str().is_empty(), "Default recordings folder should not be empty");
        assert!(folder.to_string_lossy().contains("meetily-recordings"), "Default folder should contain 'meetily-recordings'");
    }

    // Tests for enhanced preference persistence validation (Task 5.2)
    
    #[test]
    fn test_validate_preference_integrity_comprehensive() {
        // Test all validation scenarios for comprehensive coverage
        
        // Valid preferences should pass
        let valid_prefs = RecordingPreferences {
            save_folder: PathBuf::from("/home/user/recordings"),
            auto_save: true,
            file_format: "mp4".to_string(),
            preferred_mic_device: Some("Test Mic".to_string()),
            preferred_system_device: Some("Test System".to_string()),
            #[cfg(target_os = "macos")]
            system_audio_backend: Some("coreaudio".to_string()),
        };
        assert!(validate_preference_integrity(&valid_prefs).is_ok());

        // Test with different valid formats
        let mut wav_prefs = valid_prefs.clone();
        wav_prefs.file_format = "wav".to_string();
        assert!(validate_preference_integrity(&wav_prefs).is_ok());

        let mut m4a_prefs = valid_prefs.clone();
        m4a_prefs.file_format = "m4a".to_string();
        assert!(validate_preference_integrity(&m4a_prefs).is_ok());
    }

    #[test]
    fn test_validate_preference_integrity_edge_cases() {
        let base_prefs = RecordingPreferences {
            save_folder: PathBuf::from("/home/user/recordings"),
            auto_save: true,
            file_format: "mp4".to_string(),
            preferred_mic_device: None,
            preferred_system_device: None,
            #[cfg(target_os = "macos")]
            system_audio_backend: Some("coreaudio".to_string()),
        };

        // Test empty file format
        let mut empty_format_prefs = base_prefs.clone();
        empty_format_prefs.file_format = "".to_string();
        let result = validate_preference_integrity(&empty_format_prefs);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("File format is empty"));

        // Test dangerous system paths
        let dangerous_paths = vec![
            "/",
            "C:\\",
            "/System32",
            "/Windows/System32",
            "/usr/bin",
            "/etc",
        ];

        for dangerous_path in dangerous_paths {
            let mut dangerous_prefs = base_prefs.clone();
            dangerous_prefs.save_folder = PathBuf::from(dangerous_path);
            let result = validate_preference_integrity(&dangerous_prefs);
            assert!(result.is_err(), "Path {} should be rejected", dangerous_path);
        }
    }

    #[test]
    fn test_preference_integrity_auto_save_validation() {
        // Test that auto_save parameter is properly validated
        let prefs_true = RecordingPreferences {
            save_folder: PathBuf::from("/home/user/recordings"),
            auto_save: true,
            file_format: "mp4".to_string(),
            preferred_mic_device: None,
            preferred_system_device: None,
            #[cfg(target_os = "macos")]
            system_audio_backend: Some("coreaudio".to_string()),
        };

        let prefs_false = RecordingPreferences {
            save_folder: PathBuf::from("/home/user/recordings"),
            auto_save: false,
            file_format: "mp4".to_string(),
            preferred_mic_device: None,
            preferred_system_device: None,
            #[cfg(target_os = "macos")]
            system_audio_backend: Some("coreaudio".to_string()),
        };

        // Both true and false should be valid (validation is about structure, not value)
        assert!(validate_preference_integrity(&prefs_true).is_ok());
        assert!(validate_preference_integrity(&prefs_false).is_ok());
    }

    #[test]
    fn test_ensure_default_values_comprehensive() {
        // Test that ensure_default_values handles all edge cases
        let prefs = ensure_default_values();
        
        // Verify all required fields are properly set
        assert!(prefs.auto_save, "Default auto_save must be true per requirements");
        assert!(!prefs.save_folder.as_os_str().is_empty(), "Default save folder must not be empty");
        assert!(!prefs.file_format.is_empty(), "Default file format must not be empty");
        assert_eq!(prefs.file_format, "mp4", "Default file format should be mp4");
        
        // Verify the save folder contains the expected directory name
        assert!(prefs.save_folder.to_string_lossy().contains("meetily-recordings"));
    }

    // Mock tests for async functions would require a more complex test setup
    // These tests focus on the synchronous validation logic that can be tested directly
}