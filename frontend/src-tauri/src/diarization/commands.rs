//! Tauri commands for speaker diarization and identification
//!
//! This module provides the Tauri command interface for the diarization system,
//! allowing the frontend to interact with speaker detection and identification features.

use log::{debug as log_debug, error as log_error, info as log_info};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Runtime, State};

use crate::state::AppState;
use super::{
    DiarizationConfig, ProcessingMode, PrivacyMode, SpeakerSegment,
    SpeakerStatistics, VoiceProfile,
};

// ===== REQUEST/RESPONSE TYPES =====

#[derive(Debug, Serialize, Deserialize)]
pub struct StartDiarizationRequest {
    pub meeting_id: String,
    pub audio_path: String,
    pub config: Option<DiarizationConfigDto>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DiarizationConfigDto {
    pub processing_mode: String, // "Batch" | "RealTime" | "Adaptive"
    pub privacy_mode: String,    // "LocalOnly" | "PreferExternal" | "ExternalOnly"
    pub confidence_threshold: Option<f32>,
    pub enable_identification: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SpeakerSegmentDto {
    pub speaker_label: String,
    pub speaker_name: Option<String>,
    pub start_time: f64,
    pub end_time: f64,
    pub confidence: f32,
    pub embedding_hash: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateSpeakerNameRequest {
    pub meeting_id: String,
    pub speaker_label: String,
    pub new_name: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MergeSpeakersRequest {
    pub meeting_id: String,
    pub source_label: String,
    pub target_label: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SpeakerStatisticsDto {
    pub speaker_label: String,
    pub speaker_name: Option<String>,
    pub speaking_time_seconds: f64,
    pub speaking_percentage: f32,
    pub turn_count: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EnrollSpeakerRequest {
    pub name: String,
    pub audio_samples: Vec<String>, // Paths to audio samples
    pub consent_given: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct VoiceProfileDto {
    pub id: String,
    pub name: String,
    pub embedding_hash: String,
    pub created_at: String,
    pub last_seen: String,
    pub meeting_count: u32,
}

// ===== CONVERSION HELPERS =====

impl From<&SpeakerSegment> for SpeakerSegmentDto {
    fn from(segment: &SpeakerSegment) -> Self {
        Self {
            speaker_label: segment.speaker_label.clone(),
            speaker_name: None, // Not stored in SpeakerSegment, comes from mapping
            start_time: segment.start_time,
            end_time: segment.end_time,
            confidence: segment.confidence,
            embedding_hash: None, // Computed from embedding if needed
        }
    }
}

impl From<&VoiceProfile> for VoiceProfileDto {
    fn from(profile: &VoiceProfile) -> Self {
        Self {
            id: profile.id.clone(),
            name: profile.name.clone(),
            embedding_hash: profile.embedding_hash.clone(),
            created_at: profile.created_at.to_rfc3339(),
            last_seen: profile.last_seen.to_rfc3339(),
            meeting_count: profile.meeting_count,
        }
    }
}

fn parse_processing_mode(mode: &str) -> ProcessingMode {
    match mode.to_lowercase().as_str() {
        "batch" => ProcessingMode::Batch,
        "realtime" => ProcessingMode::RealTime { chunk_size_ms: 5000 },
        _ => ProcessingMode::Batch, // Default
    }
}

fn parse_privacy_mode(mode: &str) -> PrivacyMode {
    match mode.to_lowercase().as_str() {
        "localonly" => PrivacyMode::LocalOnly,
        "preferexternal" => PrivacyMode::PreferExternal,
        "externalonly" => PrivacyMode::ExternalOnly,
        _ => PrivacyMode::PreferExternal, // Default
    }
}

// ===== TAURI COMMANDS =====

/// Start diarization for a meeting
///
/// This command initiates speaker diarization for a recorded meeting.
/// It processes the audio file and generates speaker segments.
#[tauri::command]
pub async fn start_diarization<R: Runtime>(
    _app: AppHandle<R>,
    state: State<'_, AppState>,
    meeting_id: String,
    audio_path: String,
    config: Option<DiarizationConfigDto>,
) -> Result<serde_json::Value, String> {
    log_info!("start_diarization called for meeting: {}", meeting_id);
    
    // Parse configuration
    let diarization_config = if let Some(cfg) = config {
        DiarizationConfig {
            processing_mode: parse_processing_mode(&cfg.processing_mode),
            privacy_mode: parse_privacy_mode(&cfg.privacy_mode),
            confidence_threshold: cfg.confidence_threshold.unwrap_or(0.7),
            ..Default::default()
        }
    } else {
        DiarizationConfig::default()
    };

    log_debug!("Diarization config: {:?}", diarization_config);

    // TODO: Implement actual diarization processing
    // This will involve:
    // 1. Loading audio file
    // 2. Running diarization engine
    // 3. Running identification service
    // 4. Storing results in database
    // 5. Returning speaker segments
    
    // For now, return a placeholder response
    log_info!("Diarization started for meeting: {}", meeting_id);
    
    Ok(serde_json::json!({
        "status": "success",
        "message": "Diarization started",
        "meeting_id": meeting_id
    }))
}

/// Get speaker segments for a meeting
///
/// Retrieves all speaker segments that have been identified for a meeting.
#[tauri::command]
pub async fn get_speaker_segments<R: Runtime>(
    _app: AppHandle<R>,
    state: State<'_, AppState>,
    meeting_id: String,
) -> Result<Vec<SpeakerSegmentDto>, String> {
    log_info!("get_speaker_segments called for meeting: {}", meeting_id);
    
    let pool = state.db_manager.pool();
    
    // Query speaker segments from database
    let segments: Vec<(String, f64, f64, f32)> = sqlx::query_as(
        "SELECT speaker_label, start_time, end_time, confidence 
         FROM speaker_segments 
         WHERE meeting_id = ? 
         ORDER BY start_time ASC"
    )
    .bind(&meeting_id)
    .fetch_all(pool)
    .await
    .map_err(|e| {
        log_error!("Failed to fetch speaker segments: {}", e);
        format!("Database error: {}", e)
    })?;
    
    // Query speaker names from mappings
    let mappings: Vec<(String, Option<String>)> = sqlx::query_as(
        "SELECT speaker_label, speaker_name 
         FROM speaker_mappings 
         WHERE meeting_id = ?"
    )
    .bind(&meeting_id)
    .fetch_all(pool)
    .await
    .unwrap_or_default();
    
    let mapping_map: std::collections::HashMap<String, Option<String>> = 
        mappings.into_iter().collect();
    
    let result: Vec<SpeakerSegmentDto> = segments
        .into_iter()
        .map(|(label, start, end, conf)| SpeakerSegmentDto {
            speaker_label: label.clone(),
            speaker_name: mapping_map.get(&label).and_then(|n| n.clone()),
            start_time: start,
            end_time: end,
            confidence: conf,
            embedding_hash: None,
        })
        .collect();
    
    log_info!("Retrieved {} speaker segments", result.len());
    Ok(result)
}

/// Update speaker name (manual correction)
///
/// Allows users to manually correct or assign a name to a speaker label.
#[tauri::command]
pub async fn update_speaker_name<R: Runtime>(
    _app: AppHandle<R>,
    state: State<'_, AppState>,
    meeting_id: String,
    speaker_label: String,
    new_name: String,
) -> Result<serde_json::Value, String> {
    log_info!(
        "update_speaker_name called: meeting={}, label={}, name={}",
        meeting_id, speaker_label, new_name
    );
    
    let pool = state.db_manager.pool();
    
    // Update speaker name in speaker_mappings table
    let _result = sqlx::query(
        "INSERT INTO speaker_mappings (meeting_id, speaker_label, speaker_name, is_manual, confidence)
         VALUES (?, ?, ?, 1, 1.0)
         ON CONFLICT(meeting_id, speaker_label) 
         DO UPDATE SET speaker_name = ?, is_manual = 1, confidence = 1.0, updated_at = CURRENT_TIMESTAMP"
    )
    .bind(&meeting_id)
    .bind(&speaker_label)
    .bind(&new_name)
    .bind(&new_name)
    .execute(pool)
    .await
    .map_err(|e| {
        log_error!("Failed to update speaker name: {}", e);
        format!("Database error: {}", e)
    })?;
    
    // Update all segments with this label
    sqlx::query(
        "UPDATE speaker_segments 
         SET speaker_name = ? 
         WHERE meeting_id = ? AND speaker_label = ?"
    )
    .bind(&new_name)
    .bind(&meeting_id)
    .bind(&speaker_label)
    .execute(pool)
    .await
    .map_err(|e| {
        log_error!("Failed to update speaker segments: {}", e);
        format!("Database error: {}", e)
    })?;
    
    log_info!("Successfully updated speaker name");
    Ok(serde_json::json!({
        "status": "success",
        "message": "Speaker name updated successfully"
    }))
}

/// Merge two speaker labels
///
/// Consolidates two speaker labels into one, useful when the system
/// incorrectly identifies one speaker as multiple people.
#[tauri::command]
pub async fn merge_speakers<R: Runtime>(
    _app: AppHandle<R>,
    state: State<'_, AppState>,
    meeting_id: String,
    source_label: String,
    target_label: String,
) -> Result<serde_json::Value, String> {
    log_info!(
        "merge_speakers called: meeting={}, source={}, target={}",
        meeting_id, source_label, target_label
    );
    
    let pool = state.db_manager.pool();
    
    // Update all segments with source label to use target label
    let result = sqlx::query(
        "UPDATE speaker_segments 
         SET speaker_label = ? 
         WHERE meeting_id = ? AND speaker_label = ?"
    )
    .bind(&target_label)
    .bind(&meeting_id)
    .bind(&source_label)
    .execute(pool)
    .await
    .map_err(|e| {
        log_error!("Failed to merge speakers: {}", e);
        format!("Database error: {}", e)
    })?;
    
    let rows_affected = result.rows_affected();
    
    // Delete the source mapping
    sqlx::query(
        "DELETE FROM speaker_mappings 
         WHERE meeting_id = ? AND speaker_label = ?"
    )
    .bind(&meeting_id)
    .bind(&source_label)
    .execute(pool)
    .await
    .map_err(|e| {
        log_error!("Failed to delete source mapping: {}", e);
        format!("Database error: {}", e)
    })?;
    
    log_info!("Successfully merged {} segments", rows_affected);
    Ok(serde_json::json!({
        "status": "success",
        "message": format!("Merged {} segments", rows_affected),
        "segments_merged": rows_affected
    }))
}

/// Get speaker statistics for a meeting
///
/// Returns speaking time, percentage, and turn count for each speaker.
#[tauri::command]
pub async fn get_speaker_statistics<R: Runtime>(
    _app: AppHandle<R>,
    state: State<'_, AppState>,
    meeting_id: String,
) -> Result<Vec<SpeakerStatisticsDto>, String> {
    log_info!("get_speaker_statistics called for meeting: {}", meeting_id);
    
    let pool = state.db_manager.pool();
    
    // Calculate statistics from speaker segments
    let stats: Vec<(String, Option<String>, f64, i64)> = sqlx::query_as(
        "SELECT 
            speaker_label,
            speaker_name,
            SUM(end_time - start_time) as speaking_time,
            COUNT(*) as turn_count
         FROM speaker_segments 
         WHERE meeting_id = ? 
         GROUP BY speaker_label, speaker_name"
    )
    .bind(&meeting_id)
    .fetch_all(pool)
    .await
    .map_err(|e| {
        log_error!("Failed to fetch speaker statistics: {}", e);
        format!("Database error: {}", e)
    })?;
    
    // Calculate total speaking time for percentages
    let total_time: f64 = stats.iter().map(|(_, _, time, _)| time).sum();
    
    let result: Vec<SpeakerStatisticsDto> = stats
        .into_iter()
        .map(|(label, name, time, turns)| SpeakerStatisticsDto {
            speaker_label: label,
            speaker_name: name,
            speaking_time_seconds: time,
            speaking_percentage: if total_time > 0.0 {
                ((time / total_time) * 100.0) as f32
            } else {
                0.0
            },
            turn_count: turns as usize,
        })
        .collect();
    
    log_info!("Retrieved statistics for {} speakers", result.len());
    Ok(result)
}

/// Configure diarization settings
///
/// Updates the global diarization configuration.
/// Note: Configuration is currently stored in localStorage on the frontend.
/// This command validates the configuration and returns success.
#[tauri::command]
pub async fn configure_diarization<R: Runtime>(
    _app: AppHandle<R>,
    _state: State<'_, AppState>,
    config: DiarizationConfigDto,
) -> Result<serde_json::Value, String> {
    log_info!("configure_diarization called");
    
    // Validate configuration
    let valid_modes = ["Batch", "RealTime"];
    if !valid_modes.contains(&config.processing_mode.as_str()) {
        return Err(format!("Invalid processing mode: {}", config.processing_mode));
    }
    
    let valid_privacy = ["LocalOnly", "PreferExternal", "ExternalOnly"];
    if !valid_privacy.contains(&config.privacy_mode.as_str()) {
        return Err(format!("Invalid privacy mode: {}", config.privacy_mode));
    }
    
    if let Some(threshold) = config.confidence_threshold {
        if threshold < 0.0 || threshold > 1.0 {
            return Err(format!("Confidence threshold must be between 0.0 and 1.0, got: {}", threshold));
        }
    }
    
    // Configuration is valid and will be stored in localStorage by frontend
    log_info!("Diarization configuration validated successfully");
    Ok(serde_json::json!({
        "status": "success",
        "message": "Diarization configuration saved"
    }))
}

/// Enroll a speaker (create voice profile)
///
/// Creates a voice profile for a speaker using audio samples.
/// Requires explicit user consent.
#[tauri::command]
pub async fn enroll_speaker<R: Runtime>(
    _app: AppHandle<R>,
    state: State<'_, AppState>,
    name: String,
    audio_samples: Vec<String>,
    consent_given: bool,
) -> Result<VoiceProfileDto, String> {
    log_info!("enroll_speaker called for: {}", name);
    
    if !consent_given {
        return Err("User consent is required for voice profile enrollment".to_string());
    }
    
    if audio_samples.is_empty() {
        return Err("At least one audio sample is required".to_string());
    }
    
    // TODO: Implement actual enrollment
    // This will involve:
    // 1. Processing audio samples
    // 2. Extracting voice embeddings
    // 3. Creating voice profile
    // 4. Storing in database
    
    // For now, return a placeholder
    log_info!("Voice profile enrollment started for: {}", name);
    
    Err("Voice profile enrollment not yet implemented".to_string())
}

/// Delete a voice profile
///
/// Removes a voice profile and all associated data.
#[tauri::command]
pub async fn delete_voice_profile<R: Runtime>(
    _app: AppHandle<R>,
    state: State<'_, AppState>,
    profile_id: String,
) -> Result<serde_json::Value, String> {
    log_info!("delete_voice_profile called for: {}", profile_id);
    
    let pool = state.db_manager.pool();
    
    // Delete voice profile
    let result = sqlx::query(
        "DELETE FROM voice_profiles WHERE id = ?"
    )
    .bind(&profile_id)
    .execute(pool)
    .await
    .map_err(|e| {
        log_error!("Failed to delete voice profile: {}", e);
        format!("Database error: {}", e)
    })?;
    
    if result.rows_affected() == 0 {
        return Err("Voice profile not found".to_string());
    }
    
    log_info!("Voice profile deleted successfully");
    Ok(serde_json::json!({
        "status": "success",
        "message": "Voice profile deleted successfully"
    }))
}

/// List all voice profiles
///
/// Returns all stored voice profiles.
#[tauri::command]
pub async fn list_voice_profiles<R: Runtime>(
    _app: AppHandle<R>,
    state: State<'_, AppState>,
) -> Result<Vec<VoiceProfileDto>, String> {
    log_info!("list_voice_profiles called");
    
    let pool = state.db_manager.pool();
    
    // Query voice profiles
    let profiles: Vec<(String, String, String, String, String, i32)> = sqlx::query_as(
        "SELECT id, name, embedding_hash, created_at, last_seen, meeting_count 
         FROM voice_profiles 
         ORDER BY name ASC"
    )
    .fetch_all(pool)
    .await
    .map_err(|e| {
        log_error!("Failed to fetch voice profiles: {}", e);
        format!("Database error: {}", e)
    })?;
    
    let result: Vec<VoiceProfileDto> = profiles
        .into_iter()
        .map(|(id, name, hash, created, last_seen, count)| VoiceProfileDto {
            id,
            name,
            embedding_hash: hash,
            created_at: created,
            last_seen,
            meeting_count: count as u32,
        })
        .collect();
    
    log_info!("Retrieved {} voice profiles", result.len());
    Ok(result)
}
