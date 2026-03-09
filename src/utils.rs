/// Parse a human-readable duration string (e.g. "1h,30m,45s") into total seconds.
/// Supported components: `Xh`, `Ym`, `Zs` (each optional, comma-separated).
/// At least one component must be present and the resulting duration must be positive.
pub fn parse_duration(s: &str) -> anyhow::Result<i64> {
    let s = s.trim();
    if s.is_empty() {
        anyhow::bail!("Duration cannot be empty");
    }

    let mut total_seconds: i64 = 0;
    let mut found_any = false;

    for token in s.split(',').map(str::trim).filter(|t| !t.is_empty()) {
        if let Some(h) = token.strip_suffix('h') {
            let hours: i64 = h
                .parse()
                .map_err(|_| anyhow::anyhow!("Invalid hours value: '{}'", token))?;
            total_seconds += hours * 3600;
            found_any = true;
        } else if let Some(m) = token.strip_suffix('m') {
            let minutes: i64 = m
                .parse()
                .map_err(|_| anyhow::anyhow!("Invalid minutes value: '{}'", token))?;
            total_seconds += minutes * 60;
            found_any = true;
        } else if let Some(sec) = token.strip_suffix('s') {
            let seconds: i64 = sec
                .parse()
                .map_err(|_| anyhow::anyhow!("Invalid seconds value: '{}'", token))?;
            total_seconds += seconds;
            found_any = true;
        } else {
            anyhow::bail!(
                "Unrecognized duration token '{}'. Expected format: 1h,30m,45s",
                token
            );
        }
    }

    if !found_any {
        anyhow::bail!("Duration cannot be empty");
    }

    if total_seconds <= 0 {
        anyhow::bail!("Duration must be greater than zero");
    }

    Ok(total_seconds)
}

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
    fn test_parse_duration() {
        assert_eq!(parse_duration("45s").unwrap(), 45);
        assert_eq!(parse_duration("1m").unwrap(), 60);
        assert_eq!(parse_duration("1m,30s").unwrap(), 90);
        assert_eq!(parse_duration("1h").unwrap(), 3600);
        assert_eq!(parse_duration("1h,0m").unwrap(), 3600);
        assert_eq!(parse_duration("1h,30m").unwrap(), 5400);
        assert_eq!(parse_duration("2h,1m,5s").unwrap(), 7265);
        assert_eq!(parse_duration("  2h , 1m , 5s  ").unwrap(), 7265);
    }

    #[test]
    fn test_parse_duration_invalid() {
        assert!(parse_duration("").is_err());
        assert!(parse_duration("   ").is_err());
        assert!(parse_duration("0s").is_err());
        assert!(parse_duration("0m,0s").is_err());
        assert!(parse_duration("abc").is_err());
        assert!(parse_duration("1x").is_err());
    }

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
