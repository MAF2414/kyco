//! Streak tracking system
//!
//! Tracks daily usage streaks and success streaks.

use chrono::{Datelike, Local, NaiveDate, Timelike};

/// Type of streak being tracked
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StreakType {
    /// Consecutive days with at least one job
    Daily,
    /// Consecutive successful jobs (no failures)
    Success,
}

impl StreakType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Daily => "daily",
            Self::Success => "success",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "daily" => Some(Self::Daily),
            "success" => Some(Self::Success),
            _ => None,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::Daily => "Daily Streak",
            Self::Success => "Success Streak",
        }
    }
}

/// Streak data loaded from database
#[derive(Debug, Clone, Default)]
pub struct Streaks {
    pub daily: StreakInfo,
    pub success: StreakInfo,
}

/// Info for a single streak type
#[derive(Debug, Clone, Default)]
pub struct StreakInfo {
    pub current: u32,
    pub best: u32,
    pub last_activity_day: Option<String>,
}

impl StreakInfo {
    /// Check if the streak is still active (activity today or yesterday)
    pub fn is_active(&self) -> bool {
        let Some(last_day) = &self.last_activity_day else {
            return false;
        };

        let Ok(last_date) = NaiveDate::parse_from_str(last_day, "%Y-%m-%d") else {
            return false;
        };

        let today = Local::now().date_naive();
        let days_since = (today - last_date).num_days();

        // Active if activity was today or yesterday
        days_since <= 1
    }

    /// Check if we can extend the streak today
    pub fn can_extend(&self) -> bool {
        let Some(last_day) = &self.last_activity_day else {
            return true; // No activity yet, can start
        };

        let Ok(last_date) = NaiveDate::parse_from_str(last_day, "%Y-%m-%d") else {
            return true;
        };

        let today = Local::now().date_naive();
        // Can extend if last activity was yesterday (not today - already counted)
        last_date < today
    }
}

/// Get today's date as YYYY-MM-DD string
pub fn today_string() -> String {
    Local::now().format("%Y-%m-%d").to_string()
}

/// Check if today is a weekend
pub fn is_weekend() -> bool {
    let weekday = Local::now().weekday();
    weekday == chrono::Weekday::Sat || weekday == chrono::Weekday::Sun
}

/// Get current hour (0-23)
pub fn current_hour() -> u32 {
    Local::now().hour()
}
