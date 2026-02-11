/// PulseAudio device query module (Linux only)
/// 
/// This module provides functions to query PulseAudio for device descriptions
/// using the pactl command-line tool. It gracefully handles PulseAudio unavailability.

use std::collections::HashMap;
use std::process::Command;
use anyhow::{Result, anyhow};

/// Query PulseAudio for device descriptions
/// 
/// Executes `pactl list sources` and `pactl list sinks` to get device information.
/// Returns a HashMap mapping raw device names to their PulseAudio descriptions.
/// 
/// # Returns
/// - Ok(HashMap) with device mappings on success
/// - Ok(empty HashMap) if PulseAudio is unavailable or query fails (graceful fallback)
/// 
/// # Examples
/// ```no_run
/// use crate::audio::devices::pulseaudio::query_device_descriptions;
/// 
/// let descriptions = query_device_descriptions().unwrap_or_default();
/// if let Some(desc) = descriptions.get("alsa_input.pci-0000_00_1f.3.analog-stereo") {
///     println!("Device description: {}", desc);
/// }
/// ```
pub fn query_device_descriptions() -> Result<HashMap<String, String>> {
    let mut device_map = HashMap::new();
    
    // Query input sources
    match query_pactl_sources() {
        Ok(sources) => device_map.extend(sources),
        Err(e) => {
            log::warn!("Failed to query PulseAudio sources: {}, using pattern-based parsing", e);
        }
    }
    
    // Query output sinks
    match query_pactl_sinks() {
        Ok(sinks) => device_map.extend(sinks),
        Err(e) => {
            log::warn!("Failed to query PulseAudio sinks: {}, using pattern-based parsing", e);
        }
    }
    
    // For each sink, also add its monitor source with "Monitor of" prefix
    // This helps us provide friendly names for system audio capture
    let sinks: Vec<(String, String)> = device_map.iter()
        .filter(|(name, _)| name.starts_with("alsa_output."))
        .map(|(name, desc)| (name.clone(), desc.clone()))
        .collect();
    
    for (sink_name, sink_desc) in sinks {
        let monitor_name = format!("{}.monitor", sink_name);
        let monitor_desc = format!("Monitor of {}", sink_desc);
        log::info!("Adding monitor mapping: '{}' -> '{}'", monitor_name, monitor_desc);
        device_map.insert(monitor_name, monitor_desc);
    }
    
    Ok(device_map)
}

/// Query PulseAudio sources (input devices)
fn query_pactl_sources() -> Result<HashMap<String, String>> {
    let output = Command::new("pactl")
        .args(&["list", "sources"])
        .output()
        .map_err(|e| anyhow!("Failed to execute pactl: {}", e))?;
    
    if !output.status.success() {
        return Err(anyhow!("pactl command failed with status: {}", output.status));
    }
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    parse_pactl_list_output(&stdout)
}

/// Query PulseAudio sinks (output devices)
fn query_pactl_sinks() -> Result<HashMap<String, String>> {
    let output = Command::new("pactl")
        .args(&["list", "sinks"])
        .output()
        .map_err(|e| anyhow!("Failed to execute pactl: {}", e))?;
    
    if !output.status.success() {
        return Err(anyhow!("pactl command failed with status: {}", output.status));
    }
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    parse_pactl_list_output(&stdout)
}

/// Parse pactl list output into device map
/// 
/// Parses the output of `pactl list sources` or `pactl list sinks` and extracts
/// device names and descriptions.
/// 
/// Expected format:
/// ```text
/// Source #0
///     Name: alsa_input.pci-0000_00_1f.3.analog-stereo
///     Description: Built-in Audio Analog Stereo
///     ...
/// ```
fn parse_pactl_list_output(output: &str) -> Result<HashMap<String, String>> {
    let mut device_map = HashMap::new();
    let mut current_name: Option<String> = None;
    
    for line in output.lines() {
        let trimmed = line.trim();
        
        // Look for Name: field
        if let Some(name_value) = trimmed.strip_prefix("Name: ") {
            current_name = Some(name_value.to_string());
            log::debug!("Found PulseAudio/PipeWire device name: {}", name_value);
        }
        
        // Look for Description: field
        if let Some(desc_value) = trimmed.strip_prefix("Description: ") {
            if let Some(name) = current_name.take() {
                log::info!("Mapping device '{}' to description '{}'", name, desc_value);
                device_map.insert(name, desc_value.to_string());
            }
        }
    }
    
    log::info!("PulseAudio/PipeWire query found {} devices", device_map.len());
    Ok(device_map)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_pactl_list_output() {
        let sample_output = r#"
Source #0
    State: SUSPENDED
    Name: alsa_input.pci-0000_00_1f.3.analog-stereo
    Description: Built-in Audio Analog Stereo
    Driver: PipeWire
    Sample Specification: s32le 2ch 48000Hz

Source #1
    State: RUNNING
    Name: alsa_output.pci-0000_00_1f.3.analog-stereo.monitor
    Description: Monitor of Built-in Audio Analog Stereo
    Driver: PipeWire
    Sample Specification: s32le 2ch 48000Hz
"#;

        let result = parse_pactl_list_output(sample_output).unwrap();
        
        assert_eq!(result.len(), 2);
        assert_eq!(
            result.get("alsa_input.pci-0000_00_1f.3.analog-stereo"),
            Some(&"Built-in Audio Analog Stereo".to_string())
        );
        assert_eq!(
            result.get("alsa_output.pci-0000_00_1f.3.analog-stereo.monitor"),
            Some(&"Monitor of Built-in Audio Analog Stereo".to_string())
        );
    }

    #[test]
    fn test_parse_pactl_list_output_empty() {
        let result = parse_pactl_list_output("").unwrap();
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_parse_pactl_list_output_malformed() {
        let malformed_output = r#"
Some random text
Name: device1
More random text
Description: Device 1 Description
Name: device2
"#;

        let result = parse_pactl_list_output(malformed_output).unwrap();
        
        // Should still parse what it can
        assert_eq!(result.len(), 1);
        assert_eq!(
            result.get("device1"),
            Some(&"Device 1 Description".to_string())
        );
    }

    #[test]
    fn test_parse_pactl_list_output_no_description() {
        let output = r#"
Source #0
    Name: device_without_description
    Driver: PipeWire
"#;

        let result = parse_pactl_list_output(output).unwrap();
        assert_eq!(result.len(), 0);
    }
}
