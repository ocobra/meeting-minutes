//! Transcript Enhancer - Updates transcripts with speaker information
//!
//! This module formats transcripts with speaker names and calculates
//! speaker statistics for meeting analysis.
//!
//! # Architecture
//!
//! The enhancer combines three data sources:
//! 1. **Transcript Segments**: Text content with timestamps
//! 2. **Speaker Segments**: Speaker labels with embeddings
//! 3. **Speaker Mappings**: Label-to-name mappings
//!
//! It produces enhanced transcripts with:
//! - Speaker names instead of labels
//! - Confidence indicators for uncertain identifications
//! - Overlapping speech detection
//! - Comprehensive speaker statistics
//!
//! # Statistics Calculated
//!
//! - Total speaking time per speaker
//! - Speaking time percentage
//! - Number of speaking turns
//! - Average turn duration
//! - Total meeting duration
//!
//! # Formatting
//!
//! The enhancer provides intelligent formatting:
//! - Normal: "Alice: Hello everyone"
//! - Low confidence: "Alice (?): Maybe this"
//! - Overlapping: "[Alice (overlapping)]: Talking together"
//!
//! # Example
//!
//! ```no_run
//! use crate::diarization::enhancer::TranscriptEnhancer;
//!
//! let enhancer = TranscriptEnhancer::new();
//!
//! let enhanced = enhancer.enhance_transcript(
//!     &transcript_segments,
//!     &speaker_segments,
//!     &mappings
//! )?;
//!
//! // Access enhanced segments
//! for segment in &enhanced.segments {
//!     println!("{}: {}", segment.speaker_name, segment.text);
//! }
//!
//! // Access statistics
//! for speaker in &enhanced.statistics.speakers {
//!     println!("{}: {:.1}% speaking time", speaker.name, speaker.percentage);
//! }
//! ```

use crate::diarization::{
    sync::SynchronizationLayer,
    types::{
        EnhancedSegment, EnhancedTranscript, SpeakerMapping, SpeakerSegment, SpeakerStatistics,
        SpeakerStats, TranscriptSegment,
    },
    DiarizationError,
};
use log::{debug, info};
use std::collections::HashMap;

/// Transcript enhancer for adding speaker information to transcripts
pub struct TranscriptEnhancer;

impl TranscriptEnhancer {
    /// Create a new transcript enhancer
    pub fn new() -> Self {
        Self
    }

    /// Enhance transcript with speaker information
    pub fn enhance_transcript(
        &self,
        transcript_segments: &[TranscriptSegment],
        speaker_segments: &[SpeakerSegment],
        mappings: &[SpeakerMapping],
    ) -> Result<EnhancedTranscript, DiarizationError> {
        info!(
            "Enhancing transcript: {} segments, {} speakers, {} mappings",
            transcript_segments.len(),
            speaker_segments.len(),
            mappings.len()
        );

        // Synchronize transcript and speaker segments
        let synchronized = SynchronizationLayer::synchronize(transcript_segments, speaker_segments)?;

        // Build mapping lookup
        let mapping_lookup: HashMap<String, &SpeakerMapping> = mappings
            .iter()
            .map(|m| (m.speaker_label.clone(), m))
            .collect();

        // Create enhanced segments
        let mut enhanced_segments = Vec::new();
        for sync_seg in synchronized {
            // Get speaker name from mapping, or use label
            let speaker_name = mapping_lookup
                .get(&sync_seg.speaker_label)
                .and_then(|m| m.speaker_name.clone())
                .unwrap_or_else(|| sync_seg.speaker_label.clone());

            // Detect overlapping speech (confidence < 0.8 often indicates overlap)
            let is_overlapping = sync_seg.confidence < 0.8 && sync_seg.speaker_label != "Unknown";

            enhanced_segments.push(EnhancedSegment {
                speaker_name,
                text: sync_seg.text,
                start_time: sync_seg.start_time,
                end_time: sync_seg.end_time,
                confidence: sync_seg.confidence,
                is_overlapping,
            });
        }

        // Calculate statistics
        let statistics = self.calculate_statistics(&enhanced_segments);

        info!("Enhanced {} segments", enhanced_segments.len());
        Ok(EnhancedTranscript {
            segments: enhanced_segments,
            statistics,
        })
    }

    /// Format a single enhanced segment
    pub fn format_segment(&self, segment: &EnhancedSegment) -> String {
        if segment.is_overlapping {
            format!("[{} (overlapping)]: {}", segment.speaker_name, segment.text)
        } else if segment.confidence < 0.7 {
            format!("{} (?): {}", segment.speaker_name, segment.text)
        } else {
            format!("{}: {}", segment.speaker_name, segment.text)
        }
    }

    /// Calculate speaker statistics from enhanced segments
    pub fn calculate_statistics(&self, segments: &[EnhancedSegment]) -> SpeakerStatistics {
        debug!("Calculating statistics for {} segments", segments.len());

        // Group segments by speaker
        let mut speaker_data: HashMap<String, Vec<&EnhancedSegment>> = HashMap::new();
        for segment in segments {
            speaker_data
                .entry(segment.speaker_name.clone())
                .or_insert_with(Vec::new)
                .push(segment);
        }

        // Calculate total meeting duration
        let total_duration = if segments.is_empty() {
            0.0
        } else {
            let first_start = segments.iter().map(|s| s.start_time).fold(f64::INFINITY, f64::min);
            let last_end = segments.iter().map(|s| s.end_time).fold(f64::NEG_INFINITY, f64::max);
            last_end - first_start
        };

        // Calculate per-speaker statistics
        let mut speaker_stats = Vec::new();
        for (name, segs) in speaker_data {
            let total_speaking_time: f64 = segs
                .iter()
                .map(|s| s.end_time - s.start_time)
                .sum();

            let percentage = if total_duration > 0.0 {
                (total_speaking_time / total_duration * 100.0) as f32
            } else {
                0.0
            };

            let turn_count = segs.len() as u32;
            let average_turn_duration = if turn_count > 0 {
                total_speaking_time / turn_count as f64
            } else {
                0.0
            };

            speaker_stats.push(SpeakerStats {
                name,
                total_speaking_time,
                percentage,
                turn_count,
                average_turn_duration,
            });
        }

        // Sort by speaking time (descending)
        speaker_stats.sort_by(|a, b| {
            b.total_speaking_time
                .partial_cmp(&a.total_speaking_time)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        SpeakerStatistics {
            speakers: speaker_stats,
            total_duration,
        }
    }
}

impl Default for TranscriptEnhancer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_segment_normal() {
        let enhancer = TranscriptEnhancer::new();
        let segment = EnhancedSegment {
            speaker_name: "Alice".to_string(),
            text: "Hello world".to_string(),
            start_time: 0.0,
            end_time: 2.0,
            confidence: 0.9,
            is_overlapping: false,
        };

        let formatted = enhancer.format_segment(&segment);
        assert_eq!(formatted, "Alice: Hello world");
    }

    #[test]
    fn test_format_segment_low_confidence() {
        let enhancer = TranscriptEnhancer::new();
        let segment = EnhancedSegment {
            speaker_name: "Alice".to_string(),
            text: "Hello world".to_string(),
            start_time: 0.0,
            end_time: 2.0,
            confidence: 0.5,
            is_overlapping: false,
        };

        let formatted = enhancer.format_segment(&segment);
        assert_eq!(formatted, "Alice (?): Hello world");
    }

    #[test]
    fn test_format_segment_overlapping() {
        let enhancer = TranscriptEnhancer::new();
        let segment = EnhancedSegment {
            speaker_name: "Alice".to_string(),
            text: "Hello world".to_string(),
            start_time: 0.0,
            end_time: 2.0,
            confidence: 0.9,
            is_overlapping: true,
        };

        let formatted = enhancer.format_segment(&segment);
        assert_eq!(formatted, "[Alice (overlapping)]: Hello world");
    }

    #[test]
    fn test_calculate_statistics() {
        let enhancer = TranscriptEnhancer::new();
        let segments = vec![
            EnhancedSegment {
                speaker_name: "Alice".to_string(),
                text: "Hello".to_string(),
                start_time: 0.0,
                end_time: 2.0,
                confidence: 0.9,
                is_overlapping: false,
            },
            EnhancedSegment {
                speaker_name: "Bob".to_string(),
                text: "Hi".to_string(),
                start_time: 2.0,
                end_time: 5.0,
                confidence: 0.85,
                is_overlapping: false,
            },
            EnhancedSegment {
                speaker_name: "Alice".to_string(),
                text: "How are you?".to_string(),
                start_time: 5.0,
                end_time: 7.0,
                confidence: 0.9,
                is_overlapping: false,
            },
        ];

        let stats = enhancer.calculate_statistics(&segments);

        assert_eq!(stats.total_duration, 7.0);
        assert_eq!(stats.speakers.len(), 2);

        // Alice should be first (4 seconds total)
        assert_eq!(stats.speakers[0].name, "Alice");
        assert_eq!(stats.speakers[0].total_speaking_time, 4.0);
        assert_eq!(stats.speakers[0].turn_count, 2);
        assert_eq!(stats.speakers[0].average_turn_duration, 2.0);
        assert!((stats.speakers[0].percentage - 57.14).abs() < 0.1);

        // Bob should be second (3 seconds total)
        assert_eq!(stats.speakers[1].name, "Bob");
        assert_eq!(stats.speakers[1].total_speaking_time, 3.0);
        assert_eq!(stats.speakers[1].turn_count, 1);
        assert_eq!(stats.speakers[1].average_turn_duration, 3.0);
        assert!((stats.speakers[1].percentage - 42.86).abs() < 0.1);
    }

    #[test]
    fn test_calculate_statistics_empty() {
        let enhancer = TranscriptEnhancer::new();
        let segments = vec![];

        let stats = enhancer.calculate_statistics(&segments);

        assert_eq!(stats.total_duration, 0.0);
        assert_eq!(stats.speakers.len(), 0);
    }

    #[test]
    fn test_enhance_transcript() {
        let enhancer = TranscriptEnhancer::new();

        let transcript_segments = vec![
            TranscriptSegment {
                text: "Hello everyone".to_string(),
                start_time: 0.0,
                end_time: 2.0,
                words: vec![],
            },
            TranscriptSegment {
                text: "Hi there".to_string(),
                start_time: 2.0,
                end_time: 4.0,
                words: vec![],
            },
        ];

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
                start_time: 2.0,
                end_time: 4.0,
                confidence: 0.85,
                embedding: vec![],
            },
        ];

        let mappings = vec![
            SpeakerMapping {
                meeting_id: "test".to_string(),
                speaker_label: "Speaker 1".to_string(),
                speaker_name: Some("Alice".to_string()),
                voice_profile_id: None,
                confidence: 0.95,
                is_manual: false,
            },
            SpeakerMapping {
                meeting_id: "test".to_string(),
                speaker_label: "Speaker 2".to_string(),
                speaker_name: Some("Bob".to_string()),
                voice_profile_id: None,
                confidence: 0.9,
                is_manual: false,
            },
        ];

        let result = enhancer
            .enhance_transcript(&transcript_segments, &speaker_segments, &mappings)
            .unwrap();

        assert_eq!(result.segments.len(), 2);
        assert_eq!(result.segments[0].speaker_name, "Alice");
        assert_eq!(result.segments[0].text, "Hello everyone");
        assert_eq!(result.segments[1].speaker_name, "Bob");
        assert_eq!(result.segments[1].text, "Hi there");

        assert_eq!(result.statistics.speakers.len(), 2);
        assert_eq!(result.statistics.total_duration, 4.0);
    }
}
