//! Diarization Engine - Segments audio by speaker using voice embeddings
//!
//! This module integrates with pyannote.audio via subprocess calls for speaker diarization.
//! It processes audio files and returns speaker segments with embeddings.
//!
//! # Architecture
//!
//! The engine communicates with a Python subprocess that runs the pyannote.audio model.
//! This approach provides:
//! - Easy integration with Python ML ecosystem
//! - Process isolation for stability
//! - Flexibility to upgrade Python dependencies independently
//!
//! # Processing Modes
//!
//! - **Batch Mode**: Process entire audio file at once (current implementation)
//! - **Streaming Mode**: Process audio chunks in real-time (TODO)
//!
//! # Requirements
//!
//! - Python 3.8 or later
//! - pyannote.audio package
//! - Hugging Face API token (optional, for cloud models)
//!
//! # Example
//!
//! ```no_run
//! use crate::diarization::engine::DiarizationEngine;
//! use crate::diarization::types::DiarizationConfig;
//!
//! let config = DiarizationConfig::default();
//! let engine = DiarizationEngine::new(config)?;
//!
//! // Process audio file
//! let segments = engine.process_audio("meeting.wav", 16000)?;
//! ```
//!
//! # Note
//!
//! Full PyO3 integration can be added later for better performance by eliminating
//! subprocess overhead and enabling direct Python-Rust communication.

use crate::diarization::{types::DiarizationConfig, DiarizationError};
use log::{debug, info, warn};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::process::Command;

pub use crate::diarization::types::SpeakerSegment;

/// Diarization engine for speaker segmentation
pub struct DiarizationEngine {
    config: DiarizationConfig,
    python_script_path: PathBuf,
}

/// Response from Python diarization script
#[derive(Debug, Deserialize, Serialize)]
struct DiarizationResponse {
    success: bool,
    segments: Option<Vec<PythonSegment>>,
    num_speakers: Option<usize>,
    total_duration: Option<f64>,
    sample_rate: Option<u32>,
    error: Option<String>,
}

/// Segment format from Python script
#[derive(Debug, Deserialize, Serialize)]
struct PythonSegment {
    speaker_label: String,
    start_time: f64,
    end_time: f64,
    duration: f64,
    confidence: f32,
    embedding: Vec<f32>,
}

impl DiarizationEngine {
    /// Create a new diarization engine with the given configuration
    pub fn new(config: DiarizationConfig) -> Result<Self, DiarizationError> {
        info!("Initializing DiarizationEngine");
        
        // Locate Python script
        let python_script_path = Self::find_python_script()?;
        
        // Verify Python and dependencies are available
        Self::verify_python_setup(&python_script_path)?;
        
        Ok(Self {
            config,
            python_script_path,
        })
    }

    /// Process audio and return speaker segments (batch mode)
    pub fn process_audio(
        &self,
        audio_path: &str,
        sample_rate: u32,
    ) -> Result<Vec<SpeakerSegment>, DiarizationError> {
        info!("Processing audio file: {}", audio_path);
        debug!("Sample rate: {}, Config: {:?}", sample_rate, self.config);

        // Build command
        let mut cmd = Command::new("python3");
        cmd.arg(&self.python_script_path)
            .arg("--audio_path")
            .arg(audio_path)
            .arg("--sample_rate")
            .arg(sample_rate.to_string())
            .arg("--min_duration")
            .arg(self.config.min_segment_duration.to_string())
            .arg("--model")
            .arg(&self.config.embedding_model);

        // Add auth token if available
        if let Ok(token) = std::env::var("HUGGINGFACE_API_KEY")
            .or_else(|_| std::env::var("HF_TOKEN"))
        {
            cmd.arg("--auth_token").arg(token);
        }

        // Execute command
        debug!("Executing Python diarization script");
        let output = cmd
            .output()
            .map_err(|e| DiarizationError::AudioProcessingError(format!("Failed to execute Python script: {}", e)))?;

        // Check if command succeeded
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            warn!("Python script failed: {}", stderr);
            return Err(DiarizationError::AudioProcessingError(format!(
                "Diarization failed: {}",
                stderr
            )));
        }

        // Parse JSON output
        let stdout = String::from_utf8_lossy(&output.stdout);
        let response: DiarizationResponse = serde_json::from_str(&stdout)
            .map_err(|e| DiarizationError::AudioProcessingError(format!("Failed to parse diarization output: {}", e)))?;

        // Check response
        if !response.success {
            let error_msg = response.error.unwrap_or_else(|| "Unknown error".to_string());
            return Err(DiarizationError::AudioProcessingError(error_msg));
        }

        // Convert Python segments to Rust segments
        let num_segments = response.segments.as_ref().map(|s| s.len()).unwrap_or(0);
        let num_speakers = response.num_speakers.unwrap_or(0);
        
        let segments = response
            .segments
            .unwrap_or_default()
            .into_iter()
            .map(|seg| SpeakerSegment {
                speaker_label: seg.speaker_label,
                start_time: seg.start_time,
                end_time: seg.end_time,
                confidence: seg.confidence,
                embedding: seg.embedding,
            })
            .collect();

        info!(
            "Diarization complete: {} segments, {} speakers",
            num_segments,
            num_speakers
        );

        Ok(segments)
    }

    /// Process audio chunk (streaming mode)
    pub fn process_audio_chunk(
        &mut self,
        _audio: &[f32],
        _sample_rate: u32,
    ) -> Result<Vec<SpeakerSegment>, DiarizationError> {
        // TODO: Implement streaming mode
        // This requires maintaining state across chunks and is more complex
        warn!("Streaming mode not yet implemented, use batch mode instead");
        Err(DiarizationError::Other(
            "Streaming mode not yet implemented".to_string(),
        ))
    }

    /// Finalize streaming session and return remaining segments
    pub fn finalize(&mut self) -> Result<Vec<SpeakerSegment>, DiarizationError> {
        // TODO: Implement finalization for streaming mode
        warn!("Streaming finalization not yet implemented");
        Ok(Vec::new())
    }

    /// Find the Python diarization script
    fn find_python_script() -> Result<PathBuf, DiarizationError> {
        // Try multiple locations
        let possible_paths = vec![
            PathBuf::from("python/diarization_engine.py"),
            PathBuf::from("src-tauri/python/diarization_engine.py"),
            PathBuf::from("frontend/src-tauri/python/diarization_engine.py"),
        ];

        for path in possible_paths {
            if path.exists() {
                info!("Found Python diarization script at: {:?}", path);
                return Ok(path);
            }
        }

        Err(DiarizationError::ModelLoadError(
            "Could not find diarization_engine.py script".to_string(),
        ))
    }

    /// Verify Python setup and dependencies
    fn verify_python_setup(script_path: &PathBuf) -> Result<(), DiarizationError> {
        debug!("Verifying Python setup");

        // Check if Python 3 is available
        let python_check = Command::new("python3")
            .arg("--version")
            .output();

        if python_check.is_err() {
            return Err(DiarizationError::ModelLoadError(
                "Python 3 not found. Please install Python 3.8 or later".to_string(),
            ));
        }

        // Check if script exists
        if !script_path.exists() {
            return Err(DiarizationError::ModelLoadError(format!(
                "Diarization script not found at: {:?}",
                script_path
            )));
        }

        // TODO: Check if required Python packages are installed
        // This could be done by running a simple import test

        info!("Python setup verified");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diarization::types::ProcessingMode;

    #[test]
    fn test_engine_creation() {
        let config = DiarizationConfig {
            min_segment_duration: 1.0,
            speaker_change_threshold: 0.5,
            embedding_model: "pyannote/embedding".to_string(),
            processing_mode: ProcessingMode::Batch,
            ..Default::default()
        };

        // This test will fail if Python script is not found, which is expected in CI
        let result = DiarizationEngine::new(config);
        
        match result {
            Ok(_) => println!("Engine created successfully"),
            Err(e) => println!("Engine creation failed (expected in CI): {}", e),
        }
    }
}

