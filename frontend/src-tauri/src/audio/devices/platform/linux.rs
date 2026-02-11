use anyhow::Result;
use cpal::traits::{DeviceTrait, HostTrait};
use std::collections::HashSet;
use std::process::Command;

use crate::audio::devices::configuration::{AudioDevice, DeviceType};
use crate::audio::devices::friendly_names::FriendlyNameGenerator;

/// Configure Linux audio devices using PulseAudio/PipeWire
pub fn configure_linux_audio(_host: &cpal::Host) -> Result<Vec<AudioDevice>> {
    let mut devices = Vec::new();
    let mut seen_friendly_names = HashSet::new();
    
    // Create friendly name generator (queries PulseAudio once)
    let generator = FriendlyNameGenerator::new();

    // On Linux, the default host is ALSA which gives us raw ALSA device names.
    let host_to_use = _host;

    // Add input devices (deduplicate by friendly name)
    for device in host_to_use.input_devices()? {
        if let Ok(name) = device.name() {
            // Skip monitor sources in the input pass (ALSA doesn't enumerate them anyway)
            if name.contains("monitor") {
                continue;
            }
            
            log::info!("CPAL input device raw name: '{}'", name);
            let friendly_name = generator.generate(&name, DeviceType::Input);
            log::info!("  → Generated friendly name: '{}'", friendly_name);
            
            // Only add if we haven't seen this friendly name before
            if seen_friendly_names.insert(friendly_name.clone()) {
                devices.push(AudioDevice::with_friendly_name(name, friendly_name, DeviceType::Input));
            } else {
                log::debug!("  → Skipping duplicate device with friendly name: '{}'", friendly_name);
            }
        }
    }

    // Reset seen names for output devices
    seen_friendly_names.clear();

    // ALSA backend doesn't enumerate PulseAudio/PipeWire monitor sources
    // We need to query them directly using pactl
    log::info!("Querying PulseAudio/PipeWire for monitor sources...");
    if let Ok(monitors) = query_pulseaudio_monitors() {
        for (monitor_name, monitor_desc) in monitors {
            log::info!("PulseAudio monitor device: '{}' -> '{}'", monitor_name, monitor_desc);
            
            // Only add if we haven't seen this friendly name before
            if seen_friendly_names.insert(monitor_desc.clone()) {
                devices.push(AudioDevice::with_friendly_name(
                    monitor_name,
                    monitor_desc,
                    DeviceType::Output
                ));
            } else {
                log::debug!("  → Skipping duplicate monitor with friendly name: '{}'", monitor_desc);
            }
        }
    } else {
        log::warn!("Failed to query PulseAudio monitors, system audio capture may not be available");
    }

    Ok(devices)
}

/// Query PulseAudio/PipeWire for monitor sources directly
/// 
/// Returns a list of (device_name, friendly_description) tuples
fn query_pulseaudio_monitors() -> Result<Vec<(String, String)>> {
    let output = Command::new("pactl")
        .args(&["list", "sources", "short"])
        .output()?;
    
    if !output.status.success() {
        return Err(anyhow::anyhow!("pactl command failed"));
    }
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut monitors = Vec::new();
    
    // Parse pactl list sources short output
    // Format: INDEX NAME DRIVER SAMPLE_SPEC STATE
    for line in stdout.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 2 {
            let name = parts[1];
            if name.contains(".monitor") {
                // Get the friendly description from pactl list sources (long format)
                if let Ok(desc) = get_source_description(name) {
                    monitors.push((name.to_string(), desc));
                } else {
                    // Fallback: use the name itself
                    monitors.push((name.to_string(), name.to_string()));
                }
            }
        }
    }
    
    Ok(monitors)
}

/// Get the description for a specific PulseAudio source
fn get_source_description(source_name: &str) -> Result<String> {
    let output = Command::new("pactl")
        .args(&["list", "sources"])
        .output()?;
    
    if !output.status.success() {
        return Err(anyhow::anyhow!("pactl command failed"));
    }
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut current_name: Option<String> = None;
    
    for line in stdout.lines() {
        let trimmed = line.trim();
        
        // Look for Name: field
        if let Some(name_value) = trimmed.strip_prefix("Name: ") {
            current_name = Some(name_value.to_string());
        }
        
        // Look for Description: field
        if let Some(desc_value) = trimmed.strip_prefix("Description: ") {
            if let Some(ref name) = current_name {
                if name == source_name {
                    return Ok(desc_value.to_string());
                }
            }
        }
    }
    
    Err(anyhow::anyhow!("Description not found for source: {}", source_name))
}