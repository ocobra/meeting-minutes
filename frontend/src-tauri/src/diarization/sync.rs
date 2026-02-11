//! Synchronization Layer - Aligns diarization with transcription timestamps
//!
//! This module synchronizes speaker segments with transcript segments,
//! handling timing discrepancies and overlapping speech.
//!
//! # Architecture
//!
//! The synchronization layer solves the challenge of aligning two independent
//! timing sources:
//! 1. **Transcript Timestamps**: From speech recognition (word-level accuracy)
//! 2. **Speaker Timestamps**: From diarization (segment-level accuracy)
//!
//! # Timing Strategy
//!
//! - **Ground Truth**: Transcript timestamps are considered authoritative
//! - **Tolerance**: 500ms tolerance for alignment adjustments
//! - **Overlap Detection**: Identifies multiple speakers in same time window
//! - **Word-Level Alignment**: Uses word timings when available for precision
//!
//! # Overlap Handling
//!
//! When multiple speakers overlap:
//! - Selects primary speaker (longest overlap duration)
//! - Reduces confidence score (multiply by 0.8)
//! - Marks segment as overlapping for UI indication
//!
//! # Example
//!
//! ```no_run
//! use crate::diarization::sync::SynchronizationLayer;
//!
//! let synchronized = SynchronizationLayer::synchronize(
//!     &transcript_segments,
//!     &speaker_segments
//! )?;
//!
//! // Access synchronized segments
//! for segment in &synchronized {
//!     println!("{}: {} (confidence: {:.2})",
//!         segment.speaker_label,
//!         segment.text,
//!         segment.confidence
//!     );
//! }
//! ```

use crate::diarization::{
    types::{SpeakerSegment, SynchronizedSegment, TranscriptSegment, WordTiming},
    DiarizationError,
};
use log::{debug, info, warn};

/// Tolerance for timing alignment (500ms)
const TIMING_TOLERANCE_MS: f64 = 0.5;

/// Synchronization layer for aligning transcript and speaker segments
pub struct SynchronizationLayer;

impl SynchronizationLayer {
    /// Synchronize transcript and speaker segments
    pub fn synchronize(
        transcript_segments: &[TranscriptSegment],
        speaker_segments: &[SpeakerSegment],
    ) -> Result<Vec<SynchronizedSegment>, DiarizationError> {
        info!(
            "Synchronizing {} transcript segments with {} speaker segments",
            transcript_segments.len(),
            speaker_segments.len()
        );

        let mut synchronized = Vec::new();

        for transcript in transcript_segments {
            // Find all speaker segments that overlap with this transcript
            let overlapping = Self::find_overlapping_segments(transcript, speaker_segments);

            if overlapping.is_empty() {
                // No speaker found - mark as Unknown
                debug!(
                    "No speaker found for transcript segment at {:.2}s-{:.2}s",
                    transcript.start_time, transcript.end_time
                );
                synchronized.push(SynchronizedSegment {
                    speaker_label: "Unknown".to_string(),
                    speaker_name: None,
                    text: transcript.text.clone(),
                    start_time: transcript.start_time,
                    end_time: transcript.end_time,
                    words: transcript.words.clone(),
                    confidence: 0.0,
                });
            } else if overlapping.len() == 1 {
                // Single speaker - straightforward assignment
                let speaker = overlapping[0];
                let (start, end) = Self::resolve_timing_conflicts(
                    (transcript.start_time, transcript.end_time),
                    (speaker.start_time, speaker.end_time),
                );

                synchronized.push(SynchronizedSegment {
                    speaker_label: speaker.speaker_label.clone(),
                    speaker_name: None, // Will be filled by mapper
                    text: transcript.text.clone(),
                    start_time: start,
                    end_time: end,
                    words: transcript.words.clone(),
                    confidence: speaker.confidence,
                });
            } else {
                // Multiple speakers - find primary speaker (longest overlap)
                let primary = Self::find_primary_speaker(transcript, &overlapping);
                let (start, end) = Self::resolve_timing_conflicts(
                    (transcript.start_time, transcript.end_time),
                    (primary.start_time, primary.end_time),
                );

                debug!(
                    "Multiple speakers detected at {:.2}s-{:.2}s, using primary: {}",
                    transcript.start_time, transcript.end_time, primary.speaker_label
                );

                synchronized.push(SynchronizedSegment {
                    speaker_label: primary.speaker_label.clone(),
                    speaker_name: None,
                    text: transcript.text.clone(),
                    start_time: start,
                    end_time: end,
                    words: transcript.words.clone(),
                    confidence: primary.confidence * 0.8, // Reduce confidence for overlapping
                });
            }
        }

        info!("Synchronized {} segments", synchronized.len());
        Ok(synchronized)
    }

    /// Find speaker segments that overlap with a transcript segment
    fn find_overlapping_segments<'a>(
        transcript: &TranscriptSegment,
        speakers: &'a [SpeakerSegment],
    ) -> Vec<&'a SpeakerSegment> {
        speakers
            .iter()
            .filter(|s| {
                // Check if speaker segment overlaps with transcript segment
                s.start_time < transcript.end_time && s.end_time > transcript.start_time
            })
            .collect()
    }

    /// Find the primary speaker when multiple speakers overlap
    /// Returns the speaker with the longest overlap duration
    fn find_primary_speaker<'a>(
        transcript: &TranscriptSegment,
        speakers: &[&'a SpeakerSegment],
    ) -> &'a SpeakerSegment {
        speakers
            .iter()
            .max_by(|a, b| {
                let overlap_a = Self::calculate_overlap(transcript, a);
                let overlap_b = Self::calculate_overlap(transcript, b);
                overlap_a
                    .partial_cmp(&overlap_b)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .copied()
            .unwrap_or(speakers[0])
    }

    /// Calculate overlap duration between transcript and speaker segment
    fn calculate_overlap(transcript: &TranscriptSegment, speaker: &SpeakerSegment) -> f64 {
        let start = transcript.start_time.max(speaker.start_time);
        let end = transcript.end_time.min(speaker.end_time);
        (end - start).max(0.0)
    }

    /// Resolve timing conflicts between transcript and speaker segments
    /// Uses transcript timestamps as ground truth
    fn resolve_timing_conflicts(
        transcript_time: (f64, f64),
        speaker_time: (f64, f64),
    ) -> (f64, f64) {
        // Use transcript timestamps as ground truth
        // But allow small adjustments if speaker boundaries are very close
        let start = if (transcript_time.0 - speaker_time.0).abs() < TIMING_TOLERANCE_MS {
            speaker_time.0
        } else {
            transcript_time.0
        };

        let end = if (transcript_time.1 - speaker_time.1).abs() < TIMING_TOLERANCE_MS {
            speaker_time.1
        } else {
            transcript_time.1
        };

        (start, end)
    }

    /// Align speaker segments with word-level timestamps
    /// This provides more precise alignment when word timings are available
    pub fn align_with_words(
        segment: &SynchronizedSegment,
    ) -> Result<SynchronizedSegment, DiarizationError> {
        if segment.words.is_empty() {
            return Ok(segment.clone());
        }

        // Use first and last word timestamps for precise boundaries
        let first_word = &segment.words[0];
        let last_word = &segment.words[segment.words.len() - 1];

        Ok(SynchronizedSegment {
            speaker_label: segment.speaker_label.clone(),
            speaker_name: segment.speaker_name.clone(),
            text: segment.text.clone(),
            start_time: first_word.start,
            end_time: last_word.end,
            words: segment.words.clone(),
            confidence: segment.confidence,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_synchronize_single_speaker() {
        let transcript_segments = vec![TranscriptSegment {
            text: "Hello world".to_string(),
            start_time: 0.0,
            end_time: 2.0,
            words: vec![],
        }];

        let speaker_segments = vec![SpeakerSegment {
            speaker_label: "Speaker 1".to_string(),
            start_time: 0.0,
            end_time: 2.0,
            confidence: 0.9,
            embedding: vec![],
        }];

        let result = SynchronizationLayer::synchronize(&transcript_segments, &speaker_segments)
            .unwrap();

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].speaker_label, "Speaker 1");
        assert_eq!(result[0].text, "Hello world");
        assert_eq!(result[0].confidence, 0.9);
    }

    #[test]
    fn test_synchronize_no_speaker() {
        let transcript_segments = vec![TranscriptSegment {
            text: "Hello world".to_string(),
            start_time: 0.0,
            end_time: 2.0,
            words: vec![],
        }];

        let speaker_segments = vec![];

        let result = SynchronizationLayer::synchronize(&transcript_segments, &speaker_segments)
            .unwrap();

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].speaker_label, "Unknown");
        assert_eq!(result[0].confidence, 0.0);
    }

    #[test]
    fn test_synchronize_multiple_speakers() {
        let transcript_segments = vec![TranscriptSegment {
            text: "Hello world".to_string(),
            start_time: 0.0,
            end_time: 3.0,
            words: vec![],
        }];

        let speaker_segments = vec![
            SpeakerSegment {
                speaker_label: "Speaker 1".to_string(),
                start_time: 0.0,
                end_time: 1.5,
                confidence: 0.9,
                embedding: vec![],
            },
            SpeakerSegment {
                speaker_label: "Speaker 2".to_string(),
                start_time: 1.0,
                end_time: 3.0,
                confidence: 0.85,
                embedding: vec![],
            },
        ];

        let result = SynchronizationLayer::synchronize(&transcript_segments, &speaker_segments)
            .unwrap();

        assert_eq!(result.len(), 1);
        // Should pick Speaker 2 as primary (longer overlap: 2.0s vs 1.5s)
        assert_eq!(result[0].speaker_label, "Speaker 2");
        // Confidence should be reduced for overlapping speech
        assert!(result[0].confidence < 0.85);
    }

    #[test]
    fn test_find_overlapping_segments() {
        let transcript = TranscriptSegment {
            text: "Test".to_string(),
            start_time: 1.0,
            end_time: 3.0,
            words: vec![],
        };

        let speakers = vec![
            SpeakerSegment {
                speaker_label: "Speaker 1".to_string(),
                start_time: 0.0,
                end_time: 1.5,
                confidence: 0.9,
                embedding: vec![],
            },
            SpeakerSegment {
                speaker_label: "Speaker 2".to_string(),
                start_time: 2.5,
                end_time: 4.0,
                confidence: 0.85,
                embedding: vec![],
            },
            SpeakerSegment {
                speaker_label: "Speaker 3".to_string(),
                start_time: 5.0,
                end_time: 6.0,
                confidence: 0.8,
                embedding: vec![],
            },
        ];

        let overlapping = SynchronizationLayer::find_overlapping_segments(&transcript, &speakers);

        assert_eq!(overlapping.len(), 2);
        assert_eq!(overlapping[0].speaker_label, "Speaker 1");
        assert_eq!(overlapping[1].speaker_label, "Speaker 2");
    }

    #[test]
    fn test_calculate_overlap() {
        let transcript = TranscriptSegment {
            text: "Test".to_string(),
            start_time: 1.0,
            end_time: 3.0,
            words: vec![],
        };

        let speaker = SpeakerSegment {
            speaker_label: "Speaker 1".to_string(),
            start_time: 0.0,
            end_time: 2.0,
            confidence: 0.9,
            embedding: vec![],
        };

        let overlap = SynchronizationLayer::calculate_overlap(&transcript, &speaker);
        assert_eq!(overlap, 1.0); // Overlap from 1.0 to 2.0
    }

    #[test]
    fn test_resolve_timing_conflicts() {
        // Test exact match
        let result = SynchronizationLayer::resolve_timing_conflicts((1.0, 3.0), (1.0, 3.0));
        assert_eq!(result, (1.0, 3.0));

        // Test within tolerance
        let result = SynchronizationLayer::resolve_timing_conflicts((1.0, 3.0), (1.1, 2.9));
        assert_eq!(result, (1.1, 2.9));

        // Test outside tolerance
        let result = SynchronizationLayer::resolve_timing_conflicts((1.0, 3.0), (0.0, 4.0));
        assert_eq!(result, (1.0, 3.0));
    }

    #[test]
    fn test_align_with_words() {
        let segment = SynchronizedSegment {
            speaker_label: "Speaker 1".to_string(),
            speaker_name: None,
            text: "Hello world".to_string(),
            start_time: 0.0,
            end_time: 2.0,
            words: vec![
                WordTiming {
                    word: "Hello".to_string(),
                    start: 0.1,
                    end: 0.5,
                    confidence: 0.9,
                },
                WordTiming {
                    word: "world".to_string(),
                    start: 0.6,
                    end: 1.8,
                    confidence: 0.95,
                },
            ],
            confidence: 0.9,
        };

        let aligned = SynchronizationLayer::align_with_words(&segment).unwrap();

        assert_eq!(aligned.start_time, 0.1);
        assert_eq!(aligned.end_time, 1.8);
    }

    #[test]
    fn test_align_with_words_empty() {
        let segment = SynchronizedSegment {
            speaker_label: "Speaker 1".to_string(),
            speaker_name: None,
            text: "Hello world".to_string(),
            start_time: 0.0,
            end_time: 2.0,
            words: vec![],
            confidence: 0.9,
        };

        let aligned = SynchronizationLayer::align_with_words(&segment).unwrap();

        // Should remain unchanged
        assert_eq!(aligned.start_time, 0.0);
        assert_eq!(aligned.end_time, 2.0);
    }
}
