/// Pattern-based device name parser
/// 
/// This module provides functions to parse various audio device name patterns
/// and extract meaningful information for generating friendly names.

use regex::Regex;
use once_cell::sync::Lazy;

/// Parse ALSA hardware device name (hw:CARD=<cardname>,DEV=<num>)
/// 
/// Examples:
/// - `hw:CARD=PCH,DEV=0` → Some("Hardware: PCH (Device 0)")
/// - `hw:CARD=softdadsp,DEV=3` → Some("Hardware: softdadsp (Device 3)")
/// - `hw:0` → None (handled by parse_hw_simple)
pub fn parse_hw_card(raw_name: &str) -> Option<String> {
    static HW_CARD_REGEX: Lazy<Regex> = Lazy::new(|| {
        Regex::new(r"^hw:CARD=([^,]+),DEV=(\d+)$").unwrap()
    });
    
    HW_CARD_REGEX.captures(raw_name).map(|caps| {
        let card_name = caps.get(1).unwrap().as_str();
        let device_num = caps.get(2).unwrap().as_str();
        format!("Hardware: {} (Device {})", card_name, device_num)
    })
}

/// Parse simple ALSA hardware device name (hw:<num>)
/// 
/// Examples:
/// - `hw:0` → Some("Hardware Device 0")
/// - `hw:1` → Some("Hardware Device 1")
pub fn parse_hw_simple(raw_name: &str) -> Option<String> {
    static HW_SIMPLE_REGEX: Lazy<Regex> = Lazy::new(|| {
        Regex::new(r"^hw:(\d+)$").unwrap()
    });
    
    HW_SIMPLE_REGEX.captures(raw_name).map(|caps| {
        let device_num = caps.get(1).unwrap().as_str();
        format!("Hardware Device {}", device_num)
    })
}

/// Parse ALSA plugin device name (alsa_input/output.*)
/// 
/// Examples:
/// - `alsa_input.pci-0000_00_1f.3.analog-stereo` → Some(("Built-in", "Microphone", "Analog Stereo"))
/// - `alsa_output.pci-0000_00_1f.3.analog-stereo` → Some(("Built-in", "Audio", "Analog Stereo"))
/// - `alsa_input.usb-Blue_Microphones_Yeti-00.analog-stereo` → None (handled by extract_usb_info)
pub fn parse_alsa_plugin(raw_name: &str) -> Option<(String, String, String)> {
    // Check if it's a PCI (built-in) device
    if raw_name.contains(".pci-") {
        let device_type = if raw_name.starts_with("alsa_input") {
            "Microphone"
        } else {
            "Audio"
        };
        
        // Extract profile (e.g., analog-stereo, hdmi-stereo)
        let profile = raw_name
            .split('.')
            .last()
            .map(format_profile)
            .unwrap_or_else(|| "Unknown".to_string());
        
        return Some(("Built-in".to_string(), device_type.to_string(), profile));
    }
    
    None
}

/// Parse monitor source name (*.monitor)
/// 
/// Examples:
/// - `alsa_output.pci-0000_00_1f.3.analog-stereo.monitor` → Some("Built-in Audio (System Audio)")
/// - `some_device.monitor` → Some("some device (System Audio)")
pub fn parse_monitor(raw_name: &str) -> Option<String> {
    if !raw_name.ends_with(".monitor") {
        return None;
    }
    
    // Remove .monitor suffix
    let base_name = raw_name.trim_end_matches(".monitor");
    
    // Try to parse the base name for better formatting
    if let Some((vendor, device_type, profile)) = parse_alsa_plugin(base_name) {
        Some(format!("{} {} {} (System Audio)", vendor, device_type, profile))
    } else {
        // Generic cleanup for unknown base names
        let cleaned = cleanup_name(base_name);
        Some(format!("{} (System Audio)", cleaned))
    }
}

/// Extract USB vendor and model information
/// 
/// Examples:
/// - `alsa_input.usb-Blue_Microphones_Yeti_Stereo_Microphone-00.analog-stereo` 
///   → Some(("Blue Microphones", "Yeti Stereo Microphone", "Analog Stereo"))
/// - `alsa_output.usb-Logitech_USB_Headset-00.analog-stereo`
///   → Some(("Logitech", "USB Headset", "Analog Stereo"))
pub fn extract_usb_info(raw_name: &str) -> Option<(String, String, String)> {
    static USB_REGEX: Lazy<Regex> = Lazy::new(|| {
        Regex::new(r"alsa_(?:input|output)\.usb-([^-]+)-\d+\.(.+)$").unwrap()
    });
    
    USB_REGEX.captures(raw_name).map(|caps| {
        let device_info = caps.get(1).unwrap().as_str();
        let profile = caps.get(2).unwrap().as_str();
        
        // Split device info by underscores and try to identify vendor/model
        let parts: Vec<&str> = device_info.split('_').collect();
        
        let (vendor, model) = if parts.len() >= 2 {
            // Assume first part is vendor, rest is model
            let vendor = parts[0].to_string();
            let model = parts[1..].join(" ");
            (vendor, model)
        } else {
            // Single part, use as model with empty vendor
            ("".to_string(), device_info.replace('_', " "))
        };
        
        let formatted_profile = format_profile(profile);
        
        (vendor, model, formatted_profile)
    })
}

/// Clean up device name (remove underscores, capitalize, etc.)
/// 
/// Examples:
/// - `alsa_input` → "Alsa Input"
/// - `some_device_name` → "Some Device Name"
/// - `pulse` → "Pulse"
pub fn cleanup_name(raw_name: &str) -> String {
    raw_name
        .replace('_', " ")
        .replace('-', " ")
        .split('.')
        .next()
        .unwrap_or(raw_name)
        .split_whitespace()
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => first.to_uppercase().chain(chars).collect(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Format profile name (analog-stereo → Analog Stereo)
/// 
/// Examples:
/// - `analog-stereo` → "Analog Stereo"
/// - `hdmi-stereo` → "HDMI Stereo"
/// - `iec958-stereo` → "IEC958 Stereo"
pub fn format_profile(profile: &str) -> String {
    profile
        .replace('-', " ")
        .replace('_', " ")
        .split_whitespace()
        .map(|word| {
            // Special case for common acronyms
            match word.to_lowercase().as_str() {
                "hdmi" => "HDMI".to_string(),
                "usb" => "USB".to_string(),
                "iec958" => "IEC958".to_string(),
                "spdif" => "S/PDIF".to_string(),
                _ => {
                    let mut chars = word.chars();
                    match chars.next() {
                        None => String::new(),
                        Some(first) => first.to_uppercase().chain(chars).collect(),
                    }
                }
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_hw_card() {
        assert_eq!(
            parse_hw_card("hw:CARD=PCH,DEV=0"),
            Some("Hardware: PCH (Device 0)".to_string())
        );
        assert_eq!(
            parse_hw_card("hw:CARD=softdadsp,DEV=3"),
            Some("Hardware: softdadsp (Device 3)".to_string())
        );
        assert_eq!(parse_hw_card("hw:0"), None);
        assert_eq!(parse_hw_card("invalid"), None);
    }

    #[test]
    fn test_parse_hw_simple() {
        assert_eq!(
            parse_hw_simple("hw:0"),
            Some("Hardware Device 0".to_string())
        );
        assert_eq!(
            parse_hw_simple("hw:1"),
            Some("Hardware Device 1".to_string())
        );
        assert_eq!(parse_hw_simple("hw:CARD=PCH,DEV=0"), None);
    }

    #[test]
    fn test_parse_alsa_plugin() {
        let result = parse_alsa_plugin("alsa_input.pci-0000_00_1f.3.analog-stereo");
        assert_eq!(
            result,
            Some(("Built-in".to_string(), "Microphone".to_string(), "Analog Stereo".to_string()))
        );

        let result = parse_alsa_plugin("alsa_output.pci-0000_00_1f.3.analog-stereo");
        assert_eq!(
            result,
            Some(("Built-in".to_string(), "Audio".to_string(), "Analog Stereo".to_string()))
        );

        // USB devices should return None (handled by extract_usb_info)
        assert_eq!(parse_alsa_plugin("alsa_input.usb-Blue_Microphones_Yeti-00.analog-stereo"), None);
    }

    #[test]
    fn test_parse_monitor() {
        let result = parse_monitor("alsa_output.pci-0000_00_1f.3.analog-stereo.monitor");
        assert_eq!(
            result,
            Some("Built-in Audio Analog Stereo (System Audio)".to_string())
        );

        let result = parse_monitor("some_device.monitor");
        assert_eq!(
            result,
            Some("Some Device (System Audio)".to_string())
        );

        assert_eq!(parse_monitor("not_a_monitor"), None);
    }

    #[test]
    fn test_extract_usb_info() {
        let result = extract_usb_info("alsa_input.usb-Blue_Microphones_Yeti_Stereo_Microphone-00.analog-stereo");
        assert_eq!(
            result,
            Some(("Blue".to_string(), "Microphones Yeti Stereo Microphone".to_string(), "Analog Stereo".to_string()))
        );

        let result = extract_usb_info("alsa_output.usb-Logitech_USB_Headset-00.analog-stereo");
        assert_eq!(
            result,
            Some(("Logitech".to_string(), "USB Headset".to_string(), "Analog Stereo".to_string()))
        );

        assert_eq!(extract_usb_info("alsa_input.pci-0000_00_1f.3.analog-stereo"), None);
    }

    #[test]
    fn test_cleanup_name() {
        assert_eq!(cleanup_name("alsa_input"), "Alsa Input");
        assert_eq!(cleanup_name("some_device_name"), "Some Device Name");
        assert_eq!(cleanup_name("pulse"), "Pulse");
        assert_eq!(cleanup_name("alsa_output.pci-0000"), "Alsa Output");
    }

    #[test]
    fn test_format_profile() {
        assert_eq!(format_profile("analog-stereo"), "Analog Stereo");
        assert_eq!(format_profile("hdmi-stereo"), "HDMI Stereo");
        assert_eq!(format_profile("iec958-stereo"), "IEC958 Stereo");
        assert_eq!(format_profile("usb-audio"), "USB Audio");
    }
}
