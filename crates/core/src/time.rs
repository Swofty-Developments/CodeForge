//! Timestamp wrappers, relative formatting, and time range utilities.
//!
//! Provides a [`Timestamp`] type that wraps `chrono::DateTime<Utc>` with
//! human-friendly relative formatting ("2 minutes ago") and a [`TimeRange`]
//! struct for filtering operations by date.

use chrono::{DateTime, Duration, TimeZone, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;

/// A UTC timestamp with helper methods for display and comparison.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Timestamp(DateTime<Utc>);

impl Timestamp {
    /// Create a timestamp for the current instant.
    pub fn now() -> Self {
        Self(Utc::now())
    }

    /// Create a timestamp from a `chrono::DateTime<Utc>`.
    pub fn from_datetime(dt: DateTime<Utc>) -> Self {
        Self(dt)
    }

    /// Create a timestamp from a Unix epoch in seconds.
    pub fn from_epoch_secs(secs: i64) -> Self {
        Self(Utc.timestamp_opt(secs, 0).single().unwrap_or_default())
    }

    /// Create a timestamp from a Unix epoch in milliseconds.
    pub fn from_epoch_millis(millis: i64) -> Self {
        let secs = millis / 1000;
        let nanos = ((millis % 1000) * 1_000_000) as u32;
        Self(Utc.timestamp_opt(secs, nanos).single().unwrap_or_default())
    }

    /// Return the inner `DateTime<Utc>`.
    pub fn as_datetime(&self) -> &DateTime<Utc> {
        &self.0
    }

    /// Return the Unix epoch in seconds.
    pub fn epoch_secs(&self) -> i64 {
        self.0.timestamp()
    }

    /// Return the Unix epoch in milliseconds.
    pub fn epoch_millis(&self) -> i64 {
        self.0.timestamp_millis()
    }

    /// Return the elapsed duration since this timestamp (clamped to zero).
    pub fn elapsed(&self) -> Duration {
        let diff = Utc::now() - self.0;
        if diff < Duration::zero() {
            Duration::zero()
        } else {
            diff
        }
    }

    /// Format the timestamp as an ISO 8601 string.
    pub fn to_iso8601(&self) -> String {
        self.0.to_rfc3339()
    }

    /// Return a human-readable relative string like "2 minutes ago".
    pub fn relative(&self) -> String {
        format_relative(self.elapsed())
    }

    /// Check if the timestamp falls within the given range.
    pub fn is_within(&self, range: &TimeRange) -> bool {
        range.contains(self)
    }
}

impl Default for Timestamp {
    fn default() -> Self {
        Self::now()
    }
}

impl From<DateTime<Utc>> for Timestamp {
    fn from(dt: DateTime<Utc>) -> Self {
        Self(dt)
    }
}

impl From<Timestamp> for DateTime<Utc> {
    fn from(ts: Timestamp) -> Self {
        ts.0
    }
}

impl fmt::Display for Timestamp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0.format("%Y-%m-%d %H:%M:%S UTC"))
    }
}

/// Format a duration as a human-readable relative string.
pub fn format_relative(dur: Duration) -> String {
    let secs = dur.num_seconds();
    if secs < 5 {
        return "just now".to_string();
    }
    if secs < 60 {
        return format!("{secs} seconds ago");
    }
    let mins = dur.num_minutes();
    if mins == 1 {
        return "1 minute ago".to_string();
    }
    if mins < 60 {
        return format!("{mins} minutes ago");
    }
    let hours = dur.num_hours();
    if hours == 1 {
        return "1 hour ago".to_string();
    }
    if hours < 24 {
        return format!("{hours} hours ago");
    }
    let days = dur.num_days();
    if days == 1 {
        return "yesterday".to_string();
    }
    if days < 7 {
        return format!("{days} days ago");
    }
    let weeks = days / 7;
    if weeks == 1 {
        return "1 week ago".to_string();
    }
    if weeks < 5 {
        return format!("{weeks} weeks ago");
    }
    let months = days / 30;
    if months == 1 {
        return "1 month ago".to_string();
    }
    if months < 12 {
        return format!("{months} months ago");
    }
    let years = days / 365;
    if years == 1 {
        return "1 year ago".to_string();
    }
    format!("{years} years ago")
}

/// Format a duration into a compact human string like "2h 15m".
pub fn format_duration_compact(dur: Duration) -> String {
    let total_secs = dur.num_seconds().unsigned_abs();
    if total_secs < 60 {
        return format!("{total_secs}s");
    }
    let mins = total_secs / 60;
    let secs = total_secs % 60;
    if mins < 60 {
        if secs == 0 {
            return format!("{mins}m");
        }
        return format!("{mins}m {secs}s");
    }
    let hours = mins / 60;
    let remaining_mins = mins % 60;
    if hours < 24 {
        if remaining_mins == 0 {
            return format!("{hours}h");
        }
        return format!("{hours}h {remaining_mins}m");
    }
    let days = hours / 24;
    let remaining_hours = hours % 24;
    if remaining_hours == 0 {
        return format!("{days}d");
    }
    format!("{days}d {remaining_hours}h")
}

/// Format a duration into a verbose human string like "2 hours, 15 minutes".
pub fn format_duration_verbose(dur: Duration) -> String {
    let total_secs = dur.num_seconds().unsigned_abs();
    if total_secs == 0 {
        return "0 seconds".to_string();
    }

    let mut parts = Vec::new();
    let days = total_secs / 86400;
    let hours = (total_secs % 86400) / 3600;
    let mins = (total_secs % 3600) / 60;
    let secs = total_secs % 60;

    if days > 0 {
        parts.push(if days == 1 {
            "1 day".to_string()
        } else {
            format!("{days} days")
        });
    }
    if hours > 0 {
        parts.push(if hours == 1 {
            "1 hour".to_string()
        } else {
            format!("{hours} hours")
        });
    }
    if mins > 0 {
        parts.push(if mins == 1 {
            "1 minute".to_string()
        } else {
            format!("{mins} minutes")
        });
    }
    if secs > 0 && days == 0 {
        parts.push(if secs == 1 {
            "1 second".to_string()
        } else {
            format!("{secs} seconds")
        });
    }

    parts.join(", ")
}

/// A time range defined by optional start and end bounds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimeRange {
    /// Inclusive lower bound (if set).
    pub start: Option<Timestamp>,
    /// Exclusive upper bound (if set).
    pub end: Option<Timestamp>,
}

impl TimeRange {
    /// Create an unbounded time range that matches everything.
    pub fn unbounded() -> Self {
        Self {
            start: None,
            end: None,
        }
    }

    /// Create a range starting from the given timestamp.
    pub fn since(start: Timestamp) -> Self {
        Self {
            start: Some(start),
            end: None,
        }
    }

    /// Create a range ending before the given timestamp.
    pub fn until(end: Timestamp) -> Self {
        Self {
            start: None,
            end: Some(end),
        }
    }

    /// Create a range between two timestamps.
    pub fn between(start: Timestamp, end: Timestamp) -> Self {
        Self {
            start: Some(start),
            end: Some(end),
        }
    }

    /// Create a range covering the last N days from now.
    pub fn last_n_days(n: i64) -> Self {
        let end = Timestamp::now();
        let start = Timestamp::from_datetime(Utc::now() - Duration::days(n));
        Self::between(start, end)
    }

    /// Create a range covering the last N hours from now.
    pub fn last_n_hours(n: i64) -> Self {
        let end = Timestamp::now();
        let start = Timestamp::from_datetime(Utc::now() - Duration::hours(n));
        Self::between(start, end)
    }

    /// Check whether a timestamp falls within this range.
    pub fn contains(&self, ts: &Timestamp) -> bool {
        if let Some(ref start) = self.start {
            if ts < start {
                return false;
            }
        }
        if let Some(ref end) = self.end {
            if ts >= end {
                return false;
            }
        }
        true
    }

    /// Return the duration of the range, if both bounds are set.
    pub fn duration(&self) -> Option<Duration> {
        match (self.start, self.end) {
            (Some(s), Some(e)) => {
                let diff = *e.as_datetime() - *s.as_datetime();
                Some(diff)
            }
            _ => None,
        }
    }
}

impl Default for TimeRange {
    fn default() -> Self {
        Self::unbounded()
    }
}

impl fmt::Display for TimeRange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match (self.start, self.end) {
            (Some(s), Some(e)) => write!(f, "{} .. {}", s, e),
            (Some(s), None) => write!(f, "{} ..", s),
            (None, Some(e)) => write!(f, ".. {}", e),
            (None, None) => write!(f, "(unbounded)"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn relative_formatting() {
        assert_eq!(format_relative(Duration::seconds(2)), "just now");
        assert_eq!(format_relative(Duration::seconds(45)), "45 seconds ago");
        assert_eq!(format_relative(Duration::minutes(3)), "3 minutes ago");
        assert_eq!(format_relative(Duration::hours(1)), "1 hour ago");
        assert_eq!(format_relative(Duration::days(1)), "yesterday");
        assert_eq!(format_relative(Duration::days(10)), "1 week ago");
    }

    #[test]
    fn compact_duration() {
        assert_eq!(format_duration_compact(Duration::seconds(45)), "45s");
        assert_eq!(format_duration_compact(Duration::minutes(90)), "1h 30m");
    }

    #[test]
    fn time_range_contains() {
        let range = TimeRange::last_n_hours(1);
        assert!(range.contains(&Timestamp::now()));
    }
}
