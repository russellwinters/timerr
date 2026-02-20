/// Format seconds into a human-readable duration string (Xh Ym Zs)
pub fn format_duration(seconds: i64) -> String {
    if seconds < 0 {
        return "0s".to_string();
    }

    let hours = seconds / 3600;
    let minutes = (seconds % 3600) / 60;
    let secs = seconds % 60;

    let mut parts = Vec::new();

    if hours > 0 {
        parts.push(format!("{}h", hours));
        // Always show minutes when hours are present
        parts.push(format!("{}m", minutes));
        // Only show seconds if non-zero
        if secs > 0 {
            parts.push(format!("{}s", secs));
        }
    } else if minutes > 0 {
        parts.push(format!("{}m", minutes));
        // Only show seconds if non-zero
        if secs > 0 {
            parts.push(format!("{}s", secs));
        }
    } else {
        // Just seconds
        parts.push(format!("{}s", secs));
    }

    parts.join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_duration() {
        // Zero and small durations
        assert_eq!(format_duration(0), "0s");
        assert_eq!(format_duration(45), "45s");

        // Minute boundaries
        assert_eq!(format_duration(60), "1m");
        assert_eq!(format_duration(90), "1m 30s");
        assert_eq!(format_duration(120), "2m");

        // Hour boundaries
        assert_eq!(format_duration(3600), "1h 0m");
        assert_eq!(format_duration(3605), "1h 0m 5s");
        assert_eq!(format_duration(3660), "1h 1m");
        assert_eq!(format_duration(3661), "1h 1m 1s");
        assert_eq!(format_duration(7200), "2h 0m");
        assert_eq!(format_duration(7265), "2h 1m 5s");

        // Negative handling
        assert_eq!(format_duration(-10), "0s");
    }
}
