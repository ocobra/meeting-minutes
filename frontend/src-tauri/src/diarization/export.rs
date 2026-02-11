//! Export Module - Export transcripts with speaker labels
//!
//! This module provides functionality to export enhanced transcripts
//! in multiple formats with speaker information and statistics.
//!
//! # Supported Formats
//!
//! - **Text**: Plain text format with timestamps and speaker labels
//! - **Markdown**: Formatted markdown with tables for statistics
//! - **JSON**: Structured JSON for programmatic access
//!
//! # Format Features
//!
//! All formats include:
//! - Speaker names with timestamps
//! - Confidence indicators (?) for uncertain identifications
//! - Overlapping speech markers
//! - Speaker statistics (speaking time, turns, percentages)
//!
//! # Example
//!
//! ```no_run
//! use crate::diarization::export::{export_transcript, ExportFormat};
//!
//! // Export as markdown
//! let markdown = export_transcript(&enhanced_transcript, ExportFormat::Markdown)?;
//! std::fs::write("transcript.md", markdown)?;
//!
//! // Export as JSON
//! let json = export_transcript(&enhanced_transcript, ExportFormat::Json)?;
//! std::fs::write("transcript.json", json)?;
//! ```

use crate::diarization::{types::EnhancedTranscript, DiarizationError};
use log::info;
use serde_json;

/// Export format options
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportFormat {
    Text,
    Markdown,
    Json,
}

/// Export enhanced transcript to specified format
pub fn export_transcript(
    transcript: &EnhancedTranscript,
    format: ExportFormat,
) -> Result<String, DiarizationError> {
    info!("Exporting transcript in {:?} format", format);

    match format {
        ExportFormat::Text => export_as_text(transcript),
        ExportFormat::Markdown => export_as_markdown(transcript),
        ExportFormat::Json => export_as_json(transcript),
    }
}

/// Export as plain text format
fn export_as_text(transcript: &EnhancedTranscript) -> Result<String, DiarizationError> {
    let mut output = String::new();

    // Add header
    output.push_str("=== Meeting Transcript ===\n\n");

    // Add segments
    for segment in &transcript.segments {
        let timestamp = format_timestamp(segment.start_time);
        let confidence_marker = if segment.confidence < 0.7 { " (?)" } else { "" };
        let overlap_marker = if segment.is_overlapping { " [overlapping]" } else { "" };

        output.push_str(&format!(
            "[{}] {}{}{}: {}\n",
            timestamp, segment.speaker_name, confidence_marker, overlap_marker, segment.text
        ));
    }

    // Add statistics
    output.push_str("\n=== Speaker Statistics ===\n\n");
    output.push_str(&format!(
        "Total Duration: {:.2} seconds\n\n",
        transcript.statistics.total_duration
    ));

    for speaker in &transcript.statistics.speakers {
        output.push_str(&format!(
            "{}: {:.2}s ({:.1}%), {} turns, avg {:.2}s/turn\n",
            speaker.name,
            speaker.total_speaking_time,
            speaker.percentage,
            speaker.turn_count,
            speaker.average_turn_duration
        ));
    }

    Ok(output)
}

/// Export as markdown format
fn export_as_markdown(transcript: &EnhancedTranscript) -> Result<String, DiarizationError> {
    let mut output = String::new();

    // Add header
    output.push_str("# Meeting Transcript\n\n");

    // Add segments
    output.push_str("## Transcript\n\n");
    for segment in &transcript.segments {
        let timestamp = format_timestamp(segment.start_time);
        let confidence_marker = if segment.confidence < 0.7 { " *(?)*" } else { "" };
        let overlap_marker = if segment.is_overlapping { " *[overlapping]*" } else { "" };

        output.push_str(&format!(
            "**[{}] {}{}{}:** {}\n\n",
            timestamp, segment.speaker_name, confidence_marker, overlap_marker, segment.text
        ));
    }

    // Add statistics
    output.push_str("## Speaker Statistics\n\n");
    output.push_str(&format!(
        "**Total Duration:** {:.2} seconds\n\n",
        transcript.statistics.total_duration
    ));

    output.push_str("| Speaker | Speaking Time | Percentage | Turns | Avg Turn Duration |\n");
    output.push_str("|---------|---------------|------------|-------|-------------------|\n");

    for speaker in &transcript.statistics.speakers {
        output.push_str(&format!(
            "| {} | {:.2}s | {:.1}% | {} | {:.2}s |\n",
            speaker.name,
            speaker.total_speaking_time,
            speaker.percentage,
            speaker.turn_count,
            speaker.average_turn_duration
        ));
    }

    Ok(output)
}

/// Export as JSON format
fn export_as_json(transcript: &EnhancedTranscript) -> Result<String, DiarizationError> {
    serde_json::to_string_pretty(transcript).map_err(|e| {
        DiarizationError::ExportError(format!("Failed to serialize to JSON: {}", e))
    })
}

/// Format timestamp as MM:SS
fn format_timestamp(seconds: f64) -> String {
    let minutes = (seconds / 60.0).floor() as u32;
    let secs = (seconds % 60.0).floor() as u32;
    format!("{:02}:{:02}", minutes, secs)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diarization::types::{EnhancedSegment, SpeakerStatistics, SpeakerStats};

    fn create_test_transcript() -> EnhancedTranscript {
        EnhancedTranscript {
            segments: vec![
                EnhancedSegment {
                    speaker_name: "Alice".to_string(),
                    text: "Hello everyone".to_string(),
                    start_time: 0.0,
                    end_time: 2.0,
                    confidence: 0.9,
                    is_overlapping: false,
                },
                EnhancedSegment {
                    speaker_name: "Bob".to_string(),
                    text: "Hi there".to_string(),
                    start_time: 2.0,
                    end_time: 4.0,
                    confidence: 0.85,
                    is_overlapping: false,
                },
            ],
            statistics: SpeakerStatistics {
                speakers: vec![
                    SpeakerStats {
                        name: "Alice".to_string(),
                        total_speaking_time: 2.0,
                        percentage: 50.0,
                        turn_count: 1,
                        average_turn_duration: 2.0,
                    },
                    SpeakerStats {
                        name: "Bob".to_string(),
                        total_speaking_time: 2.0,
                        percentage: 50.0,
                        turn_count: 1,
                        average_turn_duration: 2.0,
                    },
                ],
                total_duration: 4.0,
            },
        }
    }

    #[test]
    fn test_export_as_text() {
        let transcript = create_test_transcript();
        let result = export_as_text(&transcript).unwrap();

        assert!(result.contains("=== Meeting Transcript ==="));
        assert!(result.contains("[00:00] Alice: Hello everyone"));
        assert!(result.contains("[00:02] Bob: Hi there"));
        assert!(result.contains("=== Speaker Statistics ==="));
        assert!(result.contains("Total Duration: 4.00 seconds"));
        assert!(result.contains("Alice: 2.00s (50.0%), 1 turns"));
        assert!(result.contains("Bob: 2.00s (50.0%), 1 turns"));
    }

    #[test]
    fn test_export_as_markdown() {
        let transcript = create_test_transcript();
        let result = export_as_markdown(&transcript).unwrap();

        assert!(result.contains("# Meeting Transcript"));
        assert!(result.contains("## Transcript"));
        assert!(result.contains("**[00:00] Alice:** Hello everyone"));
        assert!(result.contains("**[00:02] Bob:** Hi there"));
        assert!(result.contains("## Speaker Statistics"));
        assert!(result.contains("| Speaker | Speaking Time"));
        assert!(result.contains("| Alice | 2.00s | 50.0% | 1 | 2.00s |"));
    }

    #[test]
    fn test_export_as_json() {
        let transcript = create_test_transcript();
        let result = export_as_json(&transcript).unwrap();

        // Verify it's valid JSON
        let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert!(parsed["segments"].is_array());
        assert!(parsed["statistics"].is_object());
        assert_eq!(parsed["segments"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn test_format_timestamp() {
        assert_eq!(format_timestamp(0.0), "00:00");
        assert_eq!(format_timestamp(65.0), "01:05");
        assert_eq!(format_timestamp(125.5), "02:05");
        assert_eq!(format_timestamp(3661.0), "61:01");
    }

    #[test]
    fn test_export_with_low_confidence() {
        let transcript = EnhancedTranscript {
            segments: vec![EnhancedSegment {
                speaker_name: "Alice".to_string(),
                text: "Maybe this".to_string(),
                start_time: 0.0,
                end_time: 2.0,
                confidence: 0.5,
                is_overlapping: false,
            }],
            statistics: SpeakerStatistics {
                speakers: vec![],
                total_duration: 2.0,
            },
        };

        let text_result = export_as_text(&transcript).unwrap();
        assert!(text_result.contains("Alice (?)"));

        let md_result = export_as_markdown(&transcript).unwrap();
        assert!(md_result.contains("Alice *(?)*"));
    }

    #[test]
    fn test_export_with_overlapping() {
        let transcript = EnhancedTranscript {
            segments: vec![EnhancedSegment {
                speaker_name: "Alice".to_string(),
                text: "Talking together".to_string(),
                start_time: 0.0,
                end_time: 2.0,
                confidence: 0.75,
                is_overlapping: true,
            }],
            statistics: SpeakerStatistics {
                speakers: vec![],
                total_duration: 2.0,
            },
        };

        let text_result = export_as_text(&transcript).unwrap();
        assert!(text_result.contains("[overlapping]"));

        let md_result = export_as_markdown(&transcript).unwrap();
        assert!(md_result.contains("*[overlapping]*"));
    }

    #[test]
    fn test_export_all_formats() {
        let transcript = create_test_transcript();

        let text = export_transcript(&transcript, ExportFormat::Text).unwrap();
        assert!(!text.is_empty());

        let markdown = export_transcript(&transcript, ExportFormat::Markdown).unwrap();
        assert!(!markdown.is_empty());

        let json = export_transcript(&transcript, ExportFormat::Json).unwrap();
        assert!(!json.is_empty());
    }
}
