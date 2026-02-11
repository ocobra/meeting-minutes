//! Identification Service - Extracts speaker names from transcripts using LLM
//!
//! This module analyzes transcript text for introduction patterns and extracts
//! speaker names using Large Language Models (LLMs).
//!
//! # Architecture
//!
//! The identification service uses LLMs to analyze meeting transcripts and identify
//! speakers based on natural language patterns like:
//! - "I'm \[name\]" or "I am \[name\]"
//! - "This is \[name\]"
//! - "My name is \[name\]"
//! - "\[name\] here" or "\[name\] speaking"
//!
//! # Supported LLM Providers
//!
//! - **OpenAI**: GPT-3.5, GPT-4 (requires OPENAI_API_KEY)
//! - **Anthropic**: Claude models (requires ANTHROPIC_API_KEY)
//! - **Hugging Face**: Inference API (requires HUGGINGFACE_API_KEY)
//! - **Ollama**: Local LLM server (default: llama3.2:latest)
//!
//! # Workflow
//!
//! 1. Build combined transcript with speaker labels
//! 2. Generate LLM prompt with identification instructions
//! 3. Call LLM API to analyze transcript
//! 4. Parse JSON response with identified names and confidence scores
//! 5. Return identification results for mapping
//!
//! # Example
//!
//! ```no_run
//! use crate::diarization::identification::{IdentificationService, IdentificationConfig, IdentificationRequest};
//! use crate::summary::llm_client::LLMProvider;
//!
//! let config = IdentificationConfig {
//!     provider: LLMProvider::Ollama,
//!     model_name: "llama3.2:latest".to_string(),
//!     ..Default::default()
//! };
//! let service = IdentificationService::new(config);
//!
//! let request = IdentificationRequest {
//!     transcript_segments: vec![/* ... */],
//!     speaker_segments: vec![/* ... */],
//! };
//!
//! let identifications = service.identify_speakers(request).await?;
//! ```

use crate::diarization::{
    router::ModelRouter,
    types::{IdentificationResult, SpeakerSegment, TranscriptSegment},
    DiarizationError,
};
use crate::summary::llm_client::{generate_summary, LLMProvider};
use log::{debug, info, warn};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

/// Request for speaker identification
#[derive(Debug, Clone)]
pub struct IdentificationRequest {
    pub transcript_segments: Vec<TranscriptSegment>,
    pub speaker_segments: Vec<SpeakerSegment>,
}

/// JSON response structure from LLM
#[derive(Debug, Deserialize, Serialize)]
struct IdentificationResponse {
    identifications: Vec<IdentificationEntry>,
}

#[derive(Debug, Deserialize, Serialize)]
struct IdentificationEntry {
    speaker_label: String,
    name: Option<String>,
    confidence: f32,
    source_text: String,
}

/// Configuration for identification service
#[derive(Debug, Clone)]
pub struct IdentificationConfig {
    /// LLM provider to use
    pub provider: LLMProvider,
    /// Model name (e.g., "gpt-4", "llama3.2:latest")
    pub model_name: String,
    /// API key (if needed)
    pub api_key: Option<String>,
    /// Ollama endpoint (if using Ollama)
    pub ollama_endpoint: Option<String>,
}

impl Default for IdentificationConfig {
    fn default() -> Self {
        Self {
            provider: LLMProvider::Ollama,
            model_name: "llama3.2:latest".to_string(),
            api_key: None,
            ollama_endpoint: Some("http://localhost:11434".to_string()),
        }
    }
}

/// Service for identifying speakers from transcript content
pub struct IdentificationService {
    config: IdentificationConfig,
    http_client: Client,
    model_router: Option<Arc<ModelRouter>>,
}

impl IdentificationService {
    /// Create a new identification service with configuration
    pub fn new(config: IdentificationConfig) -> Self {
        info!("Initializing IdentificationService with provider: {:?}", config.provider);
        Self {
            config,
            http_client: Client::new(),
            model_router: None,
        }
    }

    /// Create with model router for automatic model selection
    pub fn with_router(config: IdentificationConfig, router: Arc<ModelRouter>) -> Self {
        info!("Initializing IdentificationService with ModelRouter");
        Self {
            config,
            http_client: Client::new(),
            model_router: Some(router),
        }
    }

    /// Identify speakers from transcript content
    pub async fn identify_speakers(
        &self,
        request: IdentificationRequest,
    ) -> Result<Vec<IdentificationResult>, DiarizationError> {
        info!(
            "Identifying speakers from {} transcript segments and {} speaker segments",
            request.transcript_segments.len(),
            request.speaker_segments.len()
        );

        // Build combined transcript with speaker labels
        let transcript = self.build_transcript(&request);
        
        if transcript.trim().is_empty() {
            warn!("Empty transcript provided for identification");
            return Ok(Vec::new());
        }

        // Build LLM prompt
        let prompt = self.build_identification_prompt(&transcript);
        debug!("Built identification prompt ({} chars)", prompt.len());

        // Call LLM for analysis
        let llm_response = self.call_llm(&prompt).await?;
        debug!("Received LLM response ({} chars)", llm_response.len());

        // Parse JSON response
        let identifications = self.parse_llm_response(&llm_response, &request)?;
        
        info!("Identified {} speakers", identifications.len());
        Ok(identifications)
    }

    /// Build combined transcript from segments
    fn build_transcript(&self, request: &IdentificationRequest) -> String {
        let mut transcript = String::new();
        
        for (idx, segment) in request.transcript_segments.iter().enumerate() {
            // Find corresponding speaker segment
            let speaker_label = self.find_speaker_for_segment(segment, &request.speaker_segments);
            
            transcript.push_str(&format!(
                "{}: {}\n",
                speaker_label.unwrap_or_else(|| "Unknown".to_string()),
                segment.text
            ));
        }
        
        transcript
    }

    /// Find speaker label for a transcript segment based on timing
    fn find_speaker_for_segment(
        &self,
        segment: &TranscriptSegment,
        speaker_segments: &[SpeakerSegment],
    ) -> Option<String> {
        // Find speaker segment that overlaps with this transcript segment
        for speaker in speaker_segments {
            if speaker.start_time <= segment.end_time && speaker.end_time >= segment.start_time {
                return Some(speaker.speaker_label.clone());
            }
        }
        None
    }

    /// Build LLM prompt for speaker identification
    fn build_identification_prompt(&self, transcript: &str) -> String {
        format!(
            r#"Analyze the following meeting transcript and identify speaker names from introductions.

Look for patterns like:
- "I'm \[name\]" or "I am \[name\]"
- "This is \[name\]"
- "My name is \[name\]"
- "\[name\] here" or "\[name\] speaking"
- "Hello, \[name\] speaking"

Transcript:
{}

For each speaker label (Speaker 1, Speaker 2, etc.), provide:
1. The identified name (if found, otherwise null)
2. Confidence score (0-100, where 100 is certain)
3. The sentence where the name was mentioned

Return ONLY valid JSON in this exact format (no markdown, no code blocks):
{{
  "identifications": [
    {{
      "speaker_label": "Speaker 1",
      "name": "John Smith",
      "confidence": 95,
      "source_text": "Hi everyone, I'm John Smith from engineering"
    }},
    {{
      "speaker_label": "Speaker 2",
      "name": null,
      "confidence": 0,
      "source_text": ""
    }}
  ]
}}

IMPORTANT: Return ONLY the JSON object, no other text."#,
            transcript
        )
    }

    /// Call LLM for speaker identification
    async fn call_llm(&self, prompt: &str) -> Result<String, DiarizationError> {
        let system_prompt = "You are a helpful assistant that identifies speaker names from meeting transcripts. You always respond with valid JSON only, no markdown formatting.";
        
        let api_key = self.config.api_key.as_deref().unwrap_or("");
        let ollama_endpoint = self.config.ollama_endpoint.as_deref();
        
        let response = generate_summary(
            &self.http_client,
            &self.config.provider,
            &self.config.model_name,
            api_key,
            system_prompt,
            prompt,
            ollama_endpoint,
            None, // custom_openai_endpoint
            Some(2000), // max_tokens
            Some(0.3), // temperature (lower for more consistent JSON)
            None, // top_p
            None, // app_data_dir
            None, // cancellation_token
        )
        .await
        .map_err(|e| DiarizationError::IdentificationError(format!("LLM call failed: {}", e)))?;
        
        Ok(response)
    }

    /// Parse LLM JSON response into identification results
    fn parse_llm_response(
        &self,
        response: &str,
        request: &IdentificationRequest,
    ) -> Result<Vec<IdentificationResult>, DiarizationError> {
        // Clean up response - remove markdown code blocks if present
        let cleaned = response
            .trim()
            .trim_start_matches("```json")
            .trim_start_matches("```")
            .trim_end_matches("```")
            .trim();
        
        debug!("Parsing LLM response: {}", cleaned);
        
        // Parse JSON
        let parsed: IdentificationResponse = serde_json::from_str(cleaned)
            .map_err(|e| {
                warn!("Failed to parse LLM response as JSON: {}", e);
                DiarizationError::IdentificationError(format!("Invalid JSON response: {}", e))
            })?;
        
        // Convert to IdentificationResult with segment indices
        let mut results = Vec::new();
        let mut segment_map: HashMap<String, usize> = HashMap::new();
        
        // Build map of speaker labels to first segment index
        for (idx, segment) in request.speaker_segments.iter().enumerate() {
            segment_map.entry(segment.speaker_label.clone()).or_insert(idx);
        }
        
        for entry in parsed.identifications {
            // Normalize confidence to 0.0-1.0 range
            let confidence = (entry.confidence / 100.0).clamp(0.0, 1.0);
            
            // Get segment index for this speaker
            let source_segment = segment_map.get(&entry.speaker_label).copied().unwrap_or(0);
            
            results.push(IdentificationResult {
                speaker_label: entry.speaker_label,
                identified_name: entry.name,
                confidence,
                source_segment,
            });
        }
        
        Ok(results)
    }
}

impl Default for IdentificationService {
    fn default() -> Self {
        Self::new(IdentificationConfig::default())
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::diarization::types::WordTiming;

    #[test]
    fn test_build_transcript() {
        let config = IdentificationConfig::default();
        let service = IdentificationService::new(config);
        
        let transcript_segments = vec![
            TranscriptSegment {
                text: "Hello everyone".to_string(),
                start_time: 0.0,
                end_time: 1.9,
                words: vec![],
            },
            TranscriptSegment {
                text: "I'm Alice from engineering".to_string(),
                start_time: 2.1,
                end_time: 5.0,
                words: vec![],
            },
        ];
        
        let speaker_segments = vec![
            SpeakerSegment {
                speaker_label: "Speaker 1".to_string(),
                start_time: 0.0,
                end_time: 2.0,
                confidence: 0.9,
                embedding: vec![0.1, 0.2, 0.3],
            },
            SpeakerSegment {
                speaker_label: "Speaker 2".to_string(),
                start_time: 2.0,
                end_time: 5.0,
                confidence: 0.85,
                embedding: vec![0.4, 0.5, 0.6],
            },
        ];
        
        let request = IdentificationRequest {
            transcript_segments,
            speaker_segments,
        };
        
        let transcript = service.build_transcript(&request);
        
        assert!(transcript.contains("Speaker 1: Hello everyone"));
        assert!(transcript.contains("Speaker 2: I'm Alice from engineering"));
    }

    #[test]
    fn test_find_speaker_for_segment() {
        let config = IdentificationConfig::default();
        let service = IdentificationService::new(config);
        
        let transcript_segment = TranscriptSegment {
            text: "Test".to_string(),
            start_time: 1.0,
            end_time: 3.0,
            words: vec![],
        };
        
        let speaker_segments = vec![
            SpeakerSegment {
                speaker_label: "Speaker 1".to_string(),
                start_time: 0.0,
                end_time: 2.0,
                confidence: 0.9,
                embedding: vec![],
            },
            SpeakerSegment {
                speaker_label: "Speaker 2".to_string(),
                start_time: 2.5,
                end_time: 5.0,
                confidence: 0.85,
                embedding: vec![],
            },
        ];
        
        let speaker = service.find_speaker_for_segment(&transcript_segment, &speaker_segments);
        
        // Should match Speaker 1 (overlaps with 1.0-2.0)
        assert_eq!(speaker, Some("Speaker 1".to_string()));
    }

    #[test]
    fn test_build_identification_prompt() {
        let config = IdentificationConfig::default();
        let service = IdentificationService::new(config);
        
        let transcript = "Speaker 1: Hello, I'm John\nSpeaker 2: Hi, this is Sarah";
        let prompt = service.build_identification_prompt(transcript);
        
        assert!(prompt.contains("Speaker 1: Hello, I'm John"));
        assert!(prompt.contains("Speaker 2: Hi, this is Sarah"));
        assert!(prompt.contains("I'm [name]"));
        assert!(prompt.contains("JSON"));
    }

    #[test]
    fn test_parse_llm_response() {
        let config = IdentificationConfig::default();
        let service = IdentificationService::new(config);
        
        let json_response = r#"{
            "identifications": [
                {
                    "speaker_label": "Speaker 1",
                    "name": "John Smith",
                    "confidence": 95,
                    "source_text": "I'm John Smith"
                },
                {
                    "speaker_label": "Speaker 2",
                    "name": null,
                    "confidence": 0,
                    "source_text": ""
                }
            ]
        }"#;
        
        let request = IdentificationRequest {
            transcript_segments: vec![],
            speaker_segments: vec![
                SpeakerSegment {
                    speaker_label: "Speaker 1".to_string(),
                    start_time: 0.0,
                    end_time: 5.0,
                    confidence: 0.9,
                    embedding: vec![],
                },
                SpeakerSegment {
                    speaker_label: "Speaker 2".to_string(),
                    start_time: 5.0,
                    end_time: 10.0,
                    confidence: 0.85,
                    embedding: vec![],
                },
            ],
        };
        
        let results = service.parse_llm_response(json_response, &request).unwrap();
        
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].speaker_label, "Speaker 1");
        assert_eq!(results[0].identified_name, Some("John Smith".to_string()));
        assert!((results[0].confidence - 0.95).abs() < 0.01);
        
        assert_eq!(results[1].speaker_label, "Speaker 2");
        assert_eq!(results[1].identified_name, None);
        assert_eq!(results[1].confidence, 0.0);
    }

    #[test]
    fn test_parse_llm_response_with_markdown() {
        let config = IdentificationConfig::default();
        let service = IdentificationService::new(config);
        
        // Test with markdown code blocks (common LLM response format)
        let json_response = r#"```json
{
    "identifications": [
        {
            "speaker_label": "Speaker 1",
            "name": "Alice",
            "confidence": 90,
            "source_text": "I'm Alice"
        }
    ]
}
```"#;
        
        let request = IdentificationRequest {
            transcript_segments: vec![],
            speaker_segments: vec![
                SpeakerSegment {
                    speaker_label: "Speaker 1".to_string(),
                    start_time: 0.0,
                    end_time: 5.0,
                    confidence: 0.9,
                    embedding: vec![],
                },
            ],
        };
        
        let results = service.parse_llm_response(json_response, &request).unwrap();
        
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].identified_name, Some("Alice".to_string()));
    }

    #[test]
    fn test_empty_transcript() {
        let config = IdentificationConfig::default();
        let service = IdentificationService::new(config);
        
        let request = IdentificationRequest {
            transcript_segments: vec![],
            speaker_segments: vec![],
        };
        
        let transcript = service.build_transcript(&request);
        assert!(transcript.trim().is_empty());
    }
}
