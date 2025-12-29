//! Time bucketing utilities for stats aggregation
//!
//! Provides functions to compute time buckets for efficient database queries.
//! - Day buckets: "YYYY-MM-DD" for daily aggregates
//! - Interval buckets: "YYYY-MM-DD-HH-MM" for 15-minute intervals

use chrono::{DateTime, Datelike, Timelike, Utc};

/// Compute the day bucket string from a Unix timestamp in milliseconds.
///
/// Returns a string in format "YYYY-MM-DD".
///
/// # Example
/// ```
/// let bucket = day_bucket(1703721600000); // 2023-12-28
/// assert_eq!(bucket, "2023-12-28");
/// ```
pub fn day_bucket(timestamp_ms: i64) -> String {
    let dt = DateTime::from_timestamp_millis(timestamp_ms).unwrap_or_else(Utc::now);
    format!("{:04}-{:02}-{:02}", dt.year(), dt.month(), dt.day())
}

/// Compute the 15-minute interval bucket string from a Unix timestamp in milliseconds.
///
/// Minutes are aligned to 0, 15, 30, or 45.
/// Returns a string in format "YYYY-MM-DD-HH-MM".
///
/// # Example
/// ```
/// let bucket = interval_bucket(1703721600000); // 2023-12-28 00:00:00
/// assert_eq!(bucket, "2023-12-28-00-00");
///
/// let bucket = interval_bucket(1703722500000); // 2023-12-28 00:15:00
/// assert_eq!(bucket, "2023-12-28-00-15");
/// ```
pub fn interval_bucket(timestamp_ms: i64) -> String {
    let dt = DateTime::from_timestamp_millis(timestamp_ms).unwrap_or_else(Utc::now);
    let aligned_minute = (dt.minute() / 15) * 15;
    format!(
        "{:04}-{:02}-{:02}-{:02}-{:02}",
        dt.year(),
        dt.month(),
        dt.day(),
        dt.hour(),
        aligned_minute
    )
}

/// Get the current day bucket.
pub fn current_day_bucket() -> String {
    day_bucket(Utc::now().timestamp_millis())
}

/// Get the current 15-minute interval bucket.
pub fn current_interval_bucket() -> String {
    interval_bucket(Utc::now().timestamp_millis())
}

/// Parse a day bucket string back to a timestamp (start of day, UTC).
pub fn parse_day_bucket(bucket: &str) -> Option<i64> {
    // Parse "YYYY-MM-DD"
    let parts: Vec<&str> = bucket.split('-').collect();
    if parts.len() != 3 {
        return None;
    }
    let year: i32 = parts[0].parse().ok()?;
    let month: u32 = parts[1].parse().ok()?;
    let day: u32 = parts[2].parse().ok()?;

    chrono::NaiveDate::from_ymd_opt(year, month, day)
        .and_then(|d| d.and_hms_opt(0, 0, 0))
        .map(|dt| dt.and_utc().timestamp_millis())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_day_bucket() {
        // 2023-12-28 12:34:56 UTC
        let ts = 1703766896000i64;
        assert_eq!(day_bucket(ts), "2023-12-28");
    }

    #[test]
    fn test_interval_bucket_alignment() {
        // Test minute alignment to 15-minute intervals
        // 12:00 -> 12:00
        let ts_00 = chrono::NaiveDate::from_ymd_opt(2023, 12, 28)
            .unwrap()
            .and_hms_opt(12, 0, 0)
            .unwrap()
            .and_utc()
            .timestamp_millis();
        assert_eq!(interval_bucket(ts_00), "2023-12-28-12-00");

        // 12:07 -> 12:00
        let ts_07 = chrono::NaiveDate::from_ymd_opt(2023, 12, 28)
            .unwrap()
            .and_hms_opt(12, 7, 0)
            .unwrap()
            .and_utc()
            .timestamp_millis();
        assert_eq!(interval_bucket(ts_07), "2023-12-28-12-00");

        // 12:15 -> 12:15
        let ts_15 = chrono::NaiveDate::from_ymd_opt(2023, 12, 28)
            .unwrap()
            .and_hms_opt(12, 15, 0)
            .unwrap()
            .and_utc()
            .timestamp_millis();
        assert_eq!(interval_bucket(ts_15), "2023-12-28-12-15");

        // 12:29 -> 12:15
        let ts_29 = chrono::NaiveDate::from_ymd_opt(2023, 12, 28)
            .unwrap()
            .and_hms_opt(12, 29, 0)
            .unwrap()
            .and_utc()
            .timestamp_millis();
        assert_eq!(interval_bucket(ts_29), "2023-12-28-12-15");

        // 12:45 -> 12:45
        let ts_45 = chrono::NaiveDate::from_ymd_opt(2023, 12, 28)
            .unwrap()
            .and_hms_opt(12, 45, 0)
            .unwrap()
            .and_utc()
            .timestamp_millis();
        assert_eq!(interval_bucket(ts_45), "2023-12-28-12-45");
    }

    #[test]
    fn test_parse_day_bucket() {
        let bucket = "2023-12-28";
        let ts = parse_day_bucket(bucket).unwrap();
        // Should be start of day UTC
        let dt = DateTime::from_timestamp_millis(ts).unwrap();
        assert_eq!(dt.year(), 2023);
        assert_eq!(dt.month(), 12);
        assert_eq!(dt.day(), 28);
        assert_eq!(dt.hour(), 0);
        assert_eq!(dt.minute(), 0);
    }
}
