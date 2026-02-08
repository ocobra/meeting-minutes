use chrono::{DateTime, Local, Utc};

/// Formats a DateTime for display in meeting titles (concise format)
/// 
/// Example: "Jan 15, 2024 2:30 PM EST"
/// 
/// # Arguments
/// * `dt` - A DateTime in local timezone
/// 
/// # Returns
/// A formatted string suitable for appending to meeting titles
pub fn format_timestamp_for_title(dt: &DateTime<Local>) -> String {
    // Format: "Jan 15, 2024 2:30 PM EST"
    dt.format("%b %-d, %Y %-I:%M %p %Z").to_string()
}

/// Formats a DateTime for display in summaries (detailed format)
/// 
/// Example: "January 15, 2024 at 2:30 PM EST"
/// 
/// # Arguments
/// * `dt` - A DateTime in local timezone
/// 
/// # Returns
/// A formatted string suitable for inclusion in meeting summaries
pub fn format_timestamp_for_summary(dt: &DateTime<Local>) -> String {
    // Format: "January 15, 2024 at 2:30 PM EST"
    dt.format("%B %-d, %Y at %-I:%M %p %Z").to_string()
}

/// Converts a UTC DateTime to local timezone
/// 
/// # Arguments
/// * `utc_dt` - A DateTime in UTC timezone
/// 
/// # Returns
/// The same instant in time, but represented in the local timezone
pub fn utc_to_local(utc_dt: &DateTime<Utc>) -> DateTime<Local> {
    utc_dt.with_timezone(&Local)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn test_format_timestamp_for_title() {
        // Create a known UTC time: 2024-01-15 14:30:00 UTC
        let utc_time = Utc.with_ymd_and_hms(2024, 1, 15, 14, 30, 0).unwrap();
        let local_time = utc_to_local(&utc_time);
        
        let formatted = format_timestamp_for_title(&local_time);
        
        // Should contain the date components (exact format depends on timezone)
        assert!(formatted.contains("2024"));
        assert!(formatted.contains("Jan") || formatted.contains("15"));
    }

    #[test]
    fn test_format_timestamp_for_summary() {
        // Create a known UTC time: 2024-01-15 14:30:00 UTC
        let utc_time = Utc.with_ymd_and_hms(2024, 1, 15, 14, 30, 0).unwrap();
        let local_time = utc_to_local(&utc_time);
        
        let formatted = format_timestamp_for_summary(&local_time);
        
        // Should contain "at" and the date components
        assert!(formatted.contains("at"));
        assert!(formatted.contains("2024"));
        assert!(formatted.contains("January") || formatted.contains("15"));
    }

    #[test]
    fn test_utc_to_local() {
        let utc_time = Utc.with_ymd_and_hms(2024, 1, 15, 14, 30, 0).unwrap();
        let local_time = utc_to_local(&utc_time);
        
        // The timestamp should represent the same instant
        assert_eq!(utc_time.timestamp(), local_time.timestamp());
    }

    #[test]
    fn test_midnight() {
        let utc_time = Utc.with_ymd_and_hms(2024, 1, 15, 0, 0, 0).unwrap();
        let local_time = utc_to_local(&utc_time);
        
        let formatted = format_timestamp_for_title(&local_time);
        assert!(formatted.contains("2024"));
    }

    #[test]
    fn test_noon() {
        let utc_time = Utc.with_ymd_and_hms(2024, 1, 15, 12, 0, 0).unwrap();
        let local_time = utc_to_local(&utc_time);
        
        let formatted = format_timestamp_for_title(&local_time);
        assert!(formatted.contains("2024"));
    }

    #[test]
    fn test_year_boundary() {
        let utc_time = Utc.with_ymd_and_hms(2023, 12, 31, 23, 59, 59).unwrap();
        let local_time = utc_to_local(&utc_time);
        
        let formatted = format_timestamp_for_title(&local_time);
        // Should contain 2023 or 2024 depending on timezone
        assert!(formatted.contains("2023") || formatted.contains("2024"));
    }
}
