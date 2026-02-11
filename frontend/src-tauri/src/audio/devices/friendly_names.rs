/// Friendly name generator module
/// 
/// This module provides the FriendlyNameGenerator that combines PulseAudio queries
/// with pattern-based parsing to generate human-readable device names.

use std::collections::HashMap;
use crate::audio::devices::configuration::DeviceType;
use super::patterns::{
    parse_hw_card, parse_hw_simple, parse_alsa_plugin, parse_monitor,
    extract_usb_info, cleanup_name, format_profile
};
use super::pulseaudio::query_device_descriptions;

/// Generator for friendly device names
/// 
/// Uses a hybrid approach:
/// 1. Try PulseAudio cache first (if available)
/// 2. Fall back to pattern-based parsing
/// 3. Apply generic cleanup for unknown patterns
pub struct FriendlyNameGenerator {
    /// Cached PulseAudio device descriptions (Linux only)
    pulse_cache: Option<HashMap<String, String>>,
}

impl FriendlyNameGenerator {
    /// Create a new generator and optionally query PulseAudio
    /// 
    /// On Linux, attempts to query PulseAudio for device descriptions.
    /// On other platforms or if PulseAudio is unavailable, uses pattern-based parsing only.
    pub fn new() -> Self {
        let pulse_cache = if cfg!(target_os = "linux") {
            // Try to query PulseAudio, but don't fail if unavailable
            match query_device_descriptions() {
                Ok(cache) if !cache.is_empty() => {
                    log::info!("PulseAudio query successful, cached {} device descriptions", cache.len());
                    log::debug!("PulseAudio cache contents:");
                    for (key, value) in cache.iter() {
                        log::debug!("  '{}' → '{}'", key, value);
                    }
                    Some(cache)
                }
                Ok(_) => {
                    log::info!("PulseAudio query returned no devices, using pattern-based parsing");
                    None
                }
                Err(e) => {
                    log::warn!("PulseAudio query failed: {}, using pattern-based parsing", e);
                    None
                }
            }
        } else {
            None
        };

        FriendlyNameGenerator { pulse_cache }
    }

    /// Generate a friendly name for a device
    /// 
    /// # Arguments
    /// * `raw_name` - The raw device name from CPAL
    /// * `device_type` - The device type (Input or Output)
    /// 
    /// # Returns
    /// A human-readable friendly name
    /// 
    /// # Examples
    /// ```no_run
    /// use crate::audio::devices::friendly_names::FriendlyNameGenerator;
    /// use crate::audio::devices::configuration::DeviceType;
    /// 
    /// let generator = FriendlyNameGenerator::new();
    /// let friendly = generator.generate("hw:CARD=PCH,DEV=0", DeviceType::Input);
    /// assert_eq!(friendly, "Hardware: PCH (Device 0)");
    /// ```
    pub fn generate(&self, raw_name: &str, device_type: DeviceType) -> String {
        log::debug!("Generating friendly name for device: {}", raw_name);
        
        // Handle empty input
        if raw_name.trim().is_empty() {
            return match device_type {
                DeviceType::Input => "Unknown Input Device".to_string(),
                DeviceType::Output => "Unknown Output Device".to_string(),
            };
        }

        // 1. Try PulseAudio cache first with exact match
        if let Some(ref cache) = self.pulse_cache {
            if let Some(description) = cache.get(raw_name) {
                log::info!("Using PulseAudio/PipeWire description for '{}': '{}'", raw_name, description);
                return description.clone();
            }
        }

        // 2. Try card-based matching for ALSA devices
        if let Some(friendly) = self.find_pulseaudio_match_by_card(raw_name, &device_type) {
            return friendly;
        }

        // 3. Try name variants
        if let Some(ref cache) = self.pulse_cache {
            let normalized_variants = self.generate_name_variants(raw_name, &device_type);
            for variant in &normalized_variants {
                if let Some(description) = cache.get(variant) {
                    log::info!("Using PulseAudio/PipeWire description for '{}' (matched variant '{}'): '{}'", 
                              raw_name, variant, description);
                    return description.clone();
                }
            }
            
            log::debug!("Device '{}' not found in PulseAudio/PipeWire cache (tried {} variants, cache has {} entries)", 
                       raw_name, normalized_variants.len() + 1, cache.len());
        }

        // 2. Try pattern-based parsing
        if let Some(friendly) = self.parse_patterns(raw_name, &device_type) {
            log::debug!("Generated friendly name from pattern for {}: {}", raw_name, friendly);
            return friendly;
        }

        // 3. Fall back to generic cleanup
        let cleaned = cleanup_name(raw_name);
        log::debug!("Using generic cleanup for {}: {}", raw_name, cleaned);
        
        // Ensure we never return empty string
        if cleaned.is_empty() {
            match device_type {
                DeviceType::Input => "Unknown Input Device".to_string(),
                DeviceType::Output => "Unknown Output Device".to_string(),
            }
        } else {
            cleaned
        }
    }

    /// Generate name variants for PulseAudio cache lookup
    /// 
    /// CPAL and PulseAudio may use different naming conventions.
    /// This generates common variants to try matching against the cache.
    fn generate_name_variants(&self, raw_name: &str, device_type: &DeviceType) -> Vec<String> {
        let mut variants = Vec::new();
        
        // If name doesn't start with alsa_input/alsa_output, try adding the prefix
        if !raw_name.starts_with("alsa_input.") && !raw_name.starts_with("alsa_output.") {
            let prefix = match device_type {
                DeviceType::Input => "alsa_input.",
                DeviceType::Output => "alsa_output.",
            };
            variants.push(format!("{}{}", prefix, raw_name));
        }
        
        // If name starts with alsa_input/alsa_output, try removing the prefix
        if let Some(stripped) = raw_name.strip_prefix("alsa_input.") {
            variants.push(stripped.to_string());
        }
        if let Some(stripped) = raw_name.strip_prefix("alsa_output.") {
            variants.push(stripped.to_string());
        }
        
        // Try opposite device type prefix (sometimes monitor sources are listed as inputs)
        if raw_name.starts_with("alsa_input.") {
            if let Some(stripped) = raw_name.strip_prefix("alsa_input.") {
                variants.push(format!("alsa_output.{}", stripped));
            }
        } else if raw_name.starts_with("alsa_output.") {
            if let Some(stripped) = raw_name.strip_prefix("alsa_output.") {
                variants.push(format!("alsa_input.{}", stripped));
            }
        }
        
        variants
    }

    /// Try to find a matching PulseAudio device by card name
    /// 
    /// CPAL on Linux uses ALSA backend which returns names like "hw:CARD=sofhdadsp,DEV=0"
    /// PulseAudio uses names like "alsa_input.pci-0000_00_1f.3-platform-skl_hda_dsp_generic.HiFi__hw_sofhdadsp__source"
    /// This function extracts the card name from ALSA and tries to find a matching PulseAudio device.
    fn find_pulseaudio_match_by_card(&self, raw_name: &str, device_type: &DeviceType) -> Option<String> {
        let cache = self.pulse_cache.as_ref()?;
        
        // Special handling for monitor sources
        // If the device type is Output and the name contains "monitor", we need to find the corresponding sink
        if matches!(device_type, DeviceType::Output) && raw_name.contains("monitor") {
            log::debug!("Handling monitor source: '{}'", raw_name);
            
            // Try to extract card name from the monitor source name
            // The monitor might be named like "hw:CARD=sofhdadsp,DEV=0" or similar
            let card_name = if let Some(card_start) = raw_name.find("CARD=") {
                let after_card = &raw_name[card_start + 5..];
                if let Some(comma_pos) = after_card.find(',') {
                    Some(&after_card[..comma_pos])
                } else {
                    Some(after_card.trim_end_matches(".monitor"))
                }
            } else {
                None
            };
            
            if let Some(card) = card_name {
                log::debug!("Extracted card name '{}' from monitor source", card);
                
                // Look for a sink (output device) with this card name
                for (pa_name, pa_desc) in cache.iter() {
                    if pa_name.starts_with("alsa_output.") && 
                       pa_name.to_lowercase().contains(&card.to_lowercase()) &&
                       pa_name.ends_with(".monitor") {
                        log::info!("Found PulseAudio monitor match: '{}' -> '{}'", pa_name, pa_desc);
                        return Some(pa_desc.clone());
                    }
                }
                
                // If no direct monitor match, look for the sink and construct the monitor description
                for (pa_name, pa_desc) in cache.iter() {
                    if pa_name.starts_with("alsa_output.") && 
                       pa_name.to_lowercase().contains(&card.to_lowercase()) &&
                       !pa_name.ends_with(".monitor") {
                        let monitor_desc = format!("Monitor of {}", pa_desc);
                        log::info!("Constructed monitor description from sink '{}': '{}'", pa_name, monitor_desc);
                        return Some(monitor_desc);
                    }
                }
            }
            
            log::debug!("No PulseAudio match found for monitor source");
            return None;
        }
        
        // Regular device handling (non-monitor)
        let card_name = if let Some(card_start) = raw_name.find("CARD=") {
            let after_card = &raw_name[card_start + 5..];
            if let Some(comma_pos) = after_card.find(',') {
                Some(&after_card[..comma_pos])
            } else {
                Some(after_card)
            }
        } else {
            None
        };

        if let Some(card) = card_name {
            log::debug!("Extracted card name '{}' from ALSA device '{}'", card, raw_name);
            
            // Try to find a PulseAudio device that contains this card name
            let prefix = match device_type {
                DeviceType::Input => "alsa_input.",
                DeviceType::Output => "alsa_output.",
            };
            
            // Look for devices that match the card name
            // Prefer exact matches, but fall back to partial matches
            let mut best_match: Option<(String, String)> = None;
            
            for (pa_name, pa_desc) in cache.iter() {
                if pa_name.starts_with(prefix) && pa_name.to_lowercase().contains(&card.to_lowercase()) {
                    // Check if this is a better match than what we have
                    if best_match.is_none() {
                        best_match = Some((pa_name.clone(), pa_desc.clone()));
                    }
                }
            }
            
            if let Some((pa_name, pa_desc)) = best_match {
                log::info!("Found PulseAudio match for card '{}': '{}' -> '{}'", card, pa_name, pa_desc);
                return Some(pa_desc);
            }
            
            log::debug!("No PulseAudio match found for card '{}'", card);
        }
        
        None
    }

    /// Try to parse device name using known patterns
    fn parse_patterns(&self, raw_name: &str, device_type: &DeviceType) -> Option<String> {
        // Try monitor source pattern first (most specific)
        if let Some(friendly) = parse_monitor(raw_name) {
            return Some(friendly);
        }

        // Try USB device pattern
        if let Some((vendor, model, profile)) = extract_usb_info(raw_name) {
            if vendor.is_empty() {
                return Some(format!("{} ({})", model, profile));
            } else {
                return Some(format!("{} {} ({})", vendor, model, profile));
            }
        }

        // Try ALSA plugin pattern (built-in devices)
        if let Some((vendor, dev_type, profile)) = parse_alsa_plugin(raw_name) {
            return Some(format!("{} {} ({})", vendor, dev_type, profile));
        }

        // Try ALSA hardware patterns
        if let Some(friendly) = parse_hw_card(raw_name) {
            return Some(friendly);
        }

        if let Some(friendly) = parse_hw_simple(raw_name) {
            return Some(friendly);
        }

        None
    }
}

impl Default for FriendlyNameGenerator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_alsa_hardware() {
        let generator = FriendlyNameGenerator { pulse_cache: None };
        
        let result = generator.generate("hw:CARD=PCH,DEV=0", DeviceType::Input);
        assert_eq!(result, "Hardware: PCH (Device 0)");
        
        let result = generator.generate("hw:1", DeviceType::Input);
        assert_eq!(result, "Hardware Device 1");
    }

    #[test]
    fn test_generate_alsa_plugin() {
        let generator = FriendlyNameGenerator { pulse_cache: None };
        
        let result = generator.generate(
            "alsa_input.pci-0000_00_1f.3.analog-stereo",
            DeviceType::Input
        );
        assert_eq!(result, "Built-in Microphone (Analog Stereo)");
        
        let result = generator.generate(
            "alsa_output.pci-0000_00_1f.3.analog-stereo",
            DeviceType::Output
        );
        assert_eq!(result, "Built-in Audio (Analog Stereo)");
    }

    #[test]
    fn test_generate_usb_device() {
        let generator = FriendlyNameGenerator { pulse_cache: None };
        
        let result = generator.generate(
            "alsa_input.usb-Blue_Microphones_Yeti_Stereo_Microphone-00.analog-stereo",
            DeviceType::Input
        );
        assert_eq!(result, "Blue Microphones Yeti Stereo Microphone (Analog Stereo)");
    }

    #[test]
    fn test_generate_monitor_source() {
        let generator = FriendlyNameGenerator { pulse_cache: None };
        
        let result = generator.generate(
            "alsa_output.pci-0000_00_1f.3.analog-stereo.monitor",
            DeviceType::Output
        );
        assert_eq!(result, "Built-in Audio Analog Stereo (System Audio)");
    }

    #[test]
    fn test_generate_unknown_pattern() {
        let generator = FriendlyNameGenerator { pulse_cache: None };
        
        let result = generator.generate("some_unknown_device", DeviceType::Input);
        assert_eq!(result, "Some Unknown Device");
    }

    #[test]
    fn test_generate_with_pulseaudio_cache() {
        let mut cache = HashMap::new();
        cache.insert(
            "alsa_input.pci-0000_00_1f.3.analog-stereo".to_string(),
            "Built-in Audio Analog Stereo".to_string()
        );
        
        let generator = FriendlyNameGenerator {
            pulse_cache: Some(cache)
        };
        
        // Should use PulseAudio description
        let result = generator.generate(
            "alsa_input.pci-0000_00_1f.3.analog-stereo",
            DeviceType::Input
        );
        assert_eq!(result, "Built-in Audio Analog Stereo");
        
        // Should fall back to pattern parsing for uncached device
        let result = generator.generate("hw:CARD=PCH,DEV=0", DeviceType::Input);
        assert_eq!(result, "Hardware: PCH (Device 0)");
    }

    #[test]
    fn test_generate_completeness() {
        let generator = FriendlyNameGenerator { pulse_cache: None };
        
        // Test that generator never returns empty string
        let test_cases = vec![
            "",
            "hw:CARD=test,DEV=0",
            "alsa_input.pci-test",
            "alsa_input.usb-test",
            "device.monitor",
            "random_device_name",
            "!!!invalid!!!",
        ];
        
        for test_case in test_cases {
            let result = generator.generate(test_case, DeviceType::Input);
            assert!(!result.is_empty(), "Generated empty string for: {}", test_case);
        }
    }
}
