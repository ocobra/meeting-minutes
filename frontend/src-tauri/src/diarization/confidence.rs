//! Confidence Scoring - Calculates and applies confidence thresholds
//!
//! This module handles confidence scoring for speaker identifications with
//! intelligent threshold-based assignment logic.
//!
//! # Architecture
//!
//! The confidence scorer evaluates identification quality based on multiple factors:
//! 1. **Pattern Match Strength**: How clear the introduction pattern was
//! 2. **Context Quality**: Quality of surrounding text (penalizes filler words)
//! 3. **Name Quality**: Validity of extracted name (penalizes short/invalid names)
//!
//! # Confidence Thresholds
//!
//! - **>= 0.7**: Auto-assign name (default threshold)
//! - **0.5-0.7**: Keep speaker label, show low confidence indicator (?)
//! - **< 0.5**: Keep speaker label, no name assignment
//!
//! # Scoring Factors
//!
//! **Pattern Match** (base confidence from LLM):
//! - Clear introduction: 0.9-1.0
//! - Implicit mention: 0.7-0.9
//! - Uncertain: 0.5-0.7
//!
//! **Context Quality** (adjusts confidence):
//! - Penalizes: filler words, short context, unclear speech
//! - Boosts: clear introductions, formal language
//!
//! **Name Quality** (adjusts confidence):
//! - Penalizes: single character, numbers, special characters
//! - Boosts: full names (first + last), proper capitalization
//!
//! # Example
//!
//! ```no_run
//! use crate::diarization::confidence::{ConfidenceScorer, ConfidenceConfig};
//!
//! let config = ConfidenceConfig {
//!     name_assignment_threshold: 0.7,
//!     low_confidence_threshold: 0.5,
//!     show_confidence_indicators: true,
//! };
//! let scorer = ConfidenceScorer::new(config);
//!
//! // Calculate confidence
//! let confidence = scorer.calculate_confidence(
//!     &identification,
//!     "Hi everyone, I'm Alice Smith from engineering"
//! );
//!
//! // Apply threshold
//! if scorer.should_assign_name(confidence) {
//!     // Use identified name
//! } else {
//!     // Keep speaker label
//! }
//! ```

use crate::diarization::{
    types::{IdentificationResult, SpeakerMapping, EnhancedSegment},
    DiarizationError,
};
use log::{debug, info};

/// Configuration for confidence scoring
#[derive(Debug, Clone)]
pub struct ConfidenceConfig {
    /// Minimum confidence threshold for name assignment (0.0-1.0)
    pub name_assignment_threshold: f32,
    /// Threshold for marking as low confidence (0.0-1.0)
    pub low_confidence_threshold: f32,
    /// Whether to show confidence indicators in output
    pub show_confidence_indicators: bool,
}

impl Default for ConfidenceConfig {
    fn default() -> Self {
        Self {
            name_assignment_threshold: 0.7,
            low_confidence_threshold: 0.5,
            show_confidence_indicators: true,
        }
    }
}

/// Confidence scorer for speaker identifications
pub struct ConfidenceScorer {
    config: ConfidenceConfig,
}

impl ConfidenceScorer {
    /// Create a new confidence scorer
    pub fn new(config: ConfidenceConfig) -> Self {
        Self { config }
    }

    /// Calculate confidence score for an identification
    /// 
    /// This combines multiple factors:
    /// - Pattern match strength (how clear the introduction was)
    /// - Context quality (surrounding text clarity)
    /// - Name uniqueness (how distinct the name is)
    pub fn calculate_confidence(
        &self,
        identification: &IdentificationResult,
        context: &str,
    ) -> f32 {
        let mut confidence = identification.confidence;

        // Adjust based on context quality
        let context_quality = self.assess_context_quality(context);
        confidence *= context_quality;

        // Adjust based on name characteristics
        let name_quality = if let Some(name) = &identification.identified_name {
            self.assess_name_quality(name)
        } else {
            0.0
        };
        confidence *= name_quality;

        // Clamp to valid range
        confidence.max(0.0).min(1.0)
    }

    /// Assess the quality of the context where the name was found
    fn assess_context_quality(&self, context: &str) -> f32 {
        let mut quality: f32 = 1.0;

        // Penalize very short context
        if context.len() < 20 {
            quality *= 0.7;
        }

        // Penalize noisy context (lots of filler words)
        let filler_words = ["um", "uh", "like", "you know", "I mean"];
        let filler_count = filler_words.iter()
            .filter(|&word| context.to_lowercase().contains(word))
            .count();
        
        if filler_count > 2 {
            quality *= 0.8;
        }

        // Boost for clear introduction patterns
        let clear_patterns = ["my name is", "i'm", "this is", "i am"];
        let has_clear_pattern = clear_patterns.iter()
            .any(|&pattern| context.to_lowercase().contains(pattern));
        
        if has_clear_pattern {
            quality *= 1.2;
        }

        quality.max(0.0).min(1.0)
    }

    /// Assess the quality of the extracted name
    fn assess_name_quality(&self, name: &str) -> f32 {
        let mut quality: f32 = 1.0;

        // Penalize very short names (likely incomplete)
        if name.len() < 2 {
            quality *= 0.3;
        } else if name.len() < 4 {
            quality *= 0.7;
        }

        // Penalize names with numbers (likely errors)
        if name.chars().any(|c| c.is_numeric()) {
            quality *= 0.5;
        }

        // Penalize names with special characters (except spaces, hyphens, apostrophes)
        let special_chars = name.chars()
            .filter(|c| !c.is_alphanumeric() && *c != ' ' && *c != '-' && *c != '\'')
            .count();
        
        if special_chars > 0 {
            quality *= 0.6;
        }

        // Boost for full names (first + last)
        let word_count = name.split_whitespace().count();
        if word_count >= 2 {
            quality *= 1.1;
        }

        quality.max(0.0).min(1.0)
    }

    /// Determine if a name should be assigned based on confidence threshold
    pub fn should_assign_name(&self, confidence: f32) -> bool {
        confidence >= self.config.name_assignment_threshold
    }

    /// Determine if confidence is low enough to show indicator
    pub fn is_low_confidence(&self, confidence: f32) -> bool {
        confidence < self.config.low_confidence_threshold
    }

    /// Apply confidence-based name assignment to a mapping
    pub fn apply_confidence_threshold(
        &self,
        mapping: &mut SpeakerMapping,
    ) {
        if !self.should_assign_name(mapping.confidence) {
            debug!(
                "Confidence {} below threshold {} for speaker {}, using label instead",
                mapping.confidence,
                self.config.name_assignment_threshold,
                mapping.speaker_label
            );
            mapping.speaker_name = None;
        }
    }

    /// Add confidence indicator to segment text if needed
    pub fn add_confidence_indicator(
        &self,
        segment: &mut EnhancedSegment,
    ) {
        if !self.config.show_confidence_indicators {
            return;
        }

        if self.is_low_confidence(segment.confidence) {
            // Add (?) indicator for low confidence
            segment.speaker_name = format!("{} (?)", segment.speaker_name);
            debug!(
                "Added low confidence indicator for speaker: {}",
                segment.speaker_name
            );
        }
    }

    /// Get confidence level as a string
    pub fn confidence_level(&self, confidence: f32) -> &'static str {
        if confidence >= 0.9 {
            "very high"
        } else if confidence >= 0.7 {
            "high"
        } else if confidence >= 0.5 {
            "medium"
        } else if confidence >= 0.3 {
            "low"
        } else {
            "very low"
        }
    }

    /// Validate confidence score is in valid range
    pub fn validate_confidence(&self, confidence: f32) -> Result<(), DiarizationError> {
        if confidence < 0.0 || confidence > 1.0 {
            return Err(DiarizationError::InvalidConfiguration(
                format!("Confidence score {} is out of range [0.0, 1.0]", confidence)
            ));
        }
        Ok(())
    }

    /// Update configuration
    pub fn update_config(&mut self, config: ConfidenceConfig) {
        info!(
            "Updating confidence config: threshold={}, low_threshold={}",
            config.name_assignment_threshold,
            config.low_confidence_threshold
        );
        self.config = config;
    }

    /// Get current configuration
    pub fn config(&self) -> &ConfidenceConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_confidence() {
        let config = ConfidenceConfig::default();
        let scorer = ConfidenceScorer::new(config);

        let identification = IdentificationResult {
            speaker_label: "Speaker 1".to_string(),
            identified_name: Some("John Smith".to_string()),
            confidence: 0.9,
            source_segment: 0,
        };

        let context = "Hi everyone, my name is John Smith from engineering";
        let confidence = scorer.calculate_confidence(&identification, context);

        // Should be high confidence due to clear pattern and good name
        assert!(confidence > 0.8);
    }

    #[test]
    fn test_calculate_confidence_low_quality() {
        let config = ConfidenceConfig::default();
        let scorer = ConfidenceScorer::new(config);

        let identification = IdentificationResult {
            speaker_label: "Speaker 1".to_string(),
            identified_name: Some("J".to_string()), // Very short name
            confidence: 0.9,
            source_segment: 0,
        };

        let context = "um uh like"; // Poor context
        let confidence = scorer.calculate_confidence(&identification, context);

        // Should be lower confidence due to poor quality
        assert!(confidence < 0.5);
    }

    #[test]
    fn test_should_assign_name() {
        let config = ConfidenceConfig {
            name_assignment_threshold: 0.7,
            ..Default::default()
        };
        let scorer = ConfidenceScorer::new(config);

        assert!(scorer.should_assign_name(0.8));
        assert!(scorer.should_assign_name(0.7));
        assert!(!scorer.should_assign_name(0.6));
    }

    #[test]
    fn test_is_low_confidence() {
        let config = ConfidenceConfig {
            low_confidence_threshold: 0.5,
            ..Default::default()
        };
        let scorer = ConfidenceScorer::new(config);

        assert!(scorer.is_low_confidence(0.4));
        assert!(!scorer.is_low_confidence(0.5));
        assert!(!scorer.is_low_confidence(0.6));
    }

    #[test]
    fn test_apply_confidence_threshold() {
        let config = ConfidenceConfig {
            name_assignment_threshold: 0.7,
            ..Default::default()
        };
        let scorer = ConfidenceScorer::new(config);

        let mut mapping = SpeakerMapping {
            meeting_id: "meeting1".to_string(),
            speaker_label: "Speaker 1".to_string(),
            speaker_name: Some("John Doe".to_string()),
            voice_profile_id: None,
            confidence: 0.6, // Below threshold
            is_manual: false,
        };

        scorer.apply_confidence_threshold(&mut mapping);

        // Name should be removed due to low confidence
        assert!(mapping.speaker_name.is_none());
    }

    #[test]
    fn test_apply_confidence_threshold_high() {
        let config = ConfidenceConfig {
            name_assignment_threshold: 0.7,
            ..Default::default()
        };
        let scorer = ConfidenceScorer::new(config);

        let mut mapping = SpeakerMapping {
            meeting_id: "meeting1".to_string(),
            speaker_label: "Speaker 1".to_string(),
            speaker_name: Some("Jane Smith".to_string()),
            voice_profile_id: None,
            confidence: 0.8, // Above threshold
            is_manual: false,
        };

        scorer.apply_confidence_threshold(&mut mapping);

        // Name should be kept
        assert_eq!(mapping.speaker_name, Some("Jane Smith".to_string()));
    }

    #[test]
    fn test_add_confidence_indicator() {
        let config = ConfidenceConfig {
            low_confidence_threshold: 0.5,
            show_confidence_indicators: true,
            ..Default::default()
        };
        let scorer = ConfidenceScorer::new(config);

        let mut segment = EnhancedSegment {
            speaker_name: "Bob Jones".to_string(),
            text: "Hello everyone".to_string(),
            start_time: 0.0,
            end_time: 2.0,
            confidence: 0.4, // Low confidence
            is_overlapping: false,
        };

        scorer.add_confidence_indicator(&mut segment);

        // Should have (?) indicator
        assert!(segment.speaker_name.contains("(?)"));
    }

    #[test]
    fn test_add_confidence_indicator_disabled() {
        let config = ConfidenceConfig {
            low_confidence_threshold: 0.5,
            show_confidence_indicators: false, // Disabled
            ..Default::default()
        };
        let scorer = ConfidenceScorer::new(config);

        let mut segment = EnhancedSegment {
            speaker_name: "Alice Brown".to_string(),
            text: "Hello everyone".to_string(),
            start_time: 0.0,
            end_time: 2.0,
            confidence: 0.4, // Low confidence
            is_overlapping: false,
        };

        scorer.add_confidence_indicator(&mut segment);

        // Should NOT have (?) indicator
        assert!(!segment.speaker_name.contains("(?)"));
    }

    #[test]
    fn test_confidence_level() {
        let config = ConfidenceConfig::default();
        let scorer = ConfidenceScorer::new(config);

        assert_eq!(scorer.confidence_level(0.95), "very high");
        assert_eq!(scorer.confidence_level(0.8), "high");
        assert_eq!(scorer.confidence_level(0.6), "medium");
        assert_eq!(scorer.confidence_level(0.4), "low");
        assert_eq!(scorer.confidence_level(0.2), "very low");
    }

    #[test]
    fn test_validate_confidence() {
        let config = ConfidenceConfig::default();
        let scorer = ConfidenceScorer::new(config);

        assert!(scorer.validate_confidence(0.0).is_ok());
        assert!(scorer.validate_confidence(0.5).is_ok());
        assert!(scorer.validate_confidence(1.0).is_ok());
        assert!(scorer.validate_confidence(-0.1).is_err());
        assert!(scorer.validate_confidence(1.1).is_err());
    }

    #[test]
    fn test_assess_context_quality() {
        let config = ConfidenceConfig::default();
        let scorer = ConfidenceScorer::new(config);

        // Good context
        let good_context = "Hi everyone, my name is John Smith from the engineering team";
        let quality = scorer.assess_context_quality(good_context);
        assert!(quality > 0.9);

        // Poor context (short and noisy)
        let poor_context = "um uh like";
        let quality = scorer.assess_context_quality(poor_context);
        assert!(quality < 0.7);
    }

    #[test]
    fn test_assess_name_quality() {
        let config = ConfidenceConfig::default();
        let scorer = ConfidenceScorer::new(config);

        // Good name
        assert!(scorer.assess_name_quality("John Smith") > 0.9);

        // Short name
        assert!(scorer.assess_name_quality("J") < 0.5);

        // Name with numbers
        assert!(scorer.assess_name_quality("John123") < 0.7);

        // Name with special characters
        assert!(scorer.assess_name_quality("John@Smith") < 0.7);

        // Name with valid characters
        assert!(scorer.assess_name_quality("Mary-Jane O'Connor") > 0.9);
    }

    #[test]
    fn test_update_config() {
        let config = ConfidenceConfig::default();
        let mut scorer = ConfidenceScorer::new(config);

        let new_config = ConfidenceConfig {
            name_assignment_threshold: 0.8,
            low_confidence_threshold: 0.6,
            show_confidence_indicators: false,
        };

        scorer.update_config(new_config.clone());

        assert_eq!(scorer.config().name_assignment_threshold, 0.8);
        assert_eq!(scorer.config().low_confidence_threshold, 0.6);
        assert_eq!(scorer.config().show_confidence_indicators, false);
    }
}
