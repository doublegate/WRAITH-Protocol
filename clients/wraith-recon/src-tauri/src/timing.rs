//! Engagement Timing Module
//!
//! This module enforces time-bounded execution windows for engagements.
//! Operations are ONLY allowed within the authorized time window defined
//! in the Rules of Engagement.
//!
//! ## Security Requirements
//! - All operations MUST check timing before execution
//! - Automatic shutdown when window expires
//! - Grace period warnings before window end

use crate::error::{ReconError, Result};
use crate::roe::RulesOfEngagement;
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

/// Window status for UI display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowStatus {
    /// Whether the window is currently active
    pub active: bool,
    /// Start time of the engagement window
    pub start_time: DateTime<Utc>,
    /// End time of the engagement window
    pub end_time: DateTime<Utc>,
    /// Current time
    pub current_time: DateTime<Utc>,
    /// Seconds remaining in window (0 if expired)
    pub seconds_remaining: i64,
    /// Human-readable time remaining
    pub time_remaining_display: String,
    /// Whether we're in the grace period (last 30 minutes)
    pub in_grace_period: bool,
    /// Status message
    pub status_message: String,
}

/// Timing manager for engagement window enforcement
pub struct TimingManager {
    /// Engagement start time
    start_time: Option<DateTime<Utc>>,
    /// Engagement end time
    end_time: Option<DateTime<Utc>>,
    /// Grace period duration (warning before end)
    grace_period: Duration,
    /// Whether timing enforcement is paused (for testing)
    paused: Arc<AtomicBool>,
    /// Manual override (for emergency extension)
    manual_override_until: Option<DateTime<Utc>>,
}

impl TimingManager {
    /// Create a new timing manager
    pub fn new() -> Self {
        Self {
            start_time: None,
            end_time: None,
            grace_period: Duration::minutes(30),
            paused: Arc::new(AtomicBool::new(false)),
            manual_override_until: None,
        }
    }

    /// Initialize from Rules of Engagement
    pub fn from_roe(roe: &RulesOfEngagement) -> Self {
        Self {
            start_time: Some(roe.start_time),
            end_time: Some(roe.end_time),
            grace_period: Duration::minutes(30),
            paused: Arc::new(AtomicBool::new(false)),
            manual_override_until: None,
        }
    }

    /// Set the engagement window
    pub fn set_window(&mut self, start: DateTime<Utc>, end: DateTime<Utc>) -> Result<()> {
        if end <= start {
            return Err(ReconError::ConfigError(
                "End time must be after start time".to_string(),
            ));
        }
        self.start_time = Some(start);
        self.end_time = Some(end);
        Ok(())
    }

    /// Set the grace period duration
    pub fn set_grace_period(&mut self, minutes: i64) {
        self.grace_period = Duration::minutes(minutes);
    }

    /// Check if the engagement window is currently active
    pub fn is_window_active(&self) -> bool {
        if self.paused.load(Ordering::Relaxed) {
            return true;
        }

        // Check manual override
        if let Some(override_until) = self.manual_override_until
            && Utc::now() < override_until
        {
            return true;
        }

        let (start, end) = match (self.start_time, self.end_time) {
            (Some(s), Some(e)) => (s, e),
            _ => return false,
        };

        let now = Utc::now();
        now >= start && now <= end
    }

    /// Check if we're in the grace period (last N minutes before end)
    pub fn in_grace_period(&self) -> bool {
        let end = match self.end_time {
            Some(e) => e,
            None => return false,
        };

        let now = Utc::now();
        let grace_start = end - self.grace_period;
        now >= grace_start && now <= end
    }

    /// Get seconds remaining in the engagement window
    pub fn seconds_remaining(&self) -> i64 {
        let end = match self.end_time {
            Some(e) => e,
            None => return 0,
        };

        let now = Utc::now();
        if now >= end {
            0
        } else {
            (end - now).num_seconds()
        }
    }

    /// Get seconds until the engagement window starts
    pub fn seconds_until_start(&self) -> Option<i64> {
        let start = self.start_time?;

        let now = Utc::now();
        if now >= start {
            Some(0)
        } else {
            Some((start - now).num_seconds())
        }
    }

    /// Format time remaining as human-readable string
    pub fn format_time_remaining(&self) -> String {
        let seconds = self.seconds_remaining();
        if seconds <= 0 {
            return "Expired".to_string();
        }

        let hours = seconds / 3600;
        let minutes = (seconds % 3600) / 60;
        let secs = seconds % 60;

        if hours > 24 {
            let days = hours / 24;
            format!("{}d {}h", days, hours % 24)
        } else if hours > 0 {
            format!("{}h {}m", hours, minutes)
        } else if minutes > 0 {
            format!("{}m {}s", minutes, secs)
        } else {
            format!("{}s", secs)
        }
    }

    /// Get current window status
    pub fn status(&self) -> WindowStatus {
        let now = Utc::now();
        let active = self.is_window_active();
        let seconds_remaining = self.seconds_remaining();
        let in_grace = self.in_grace_period();

        let status_message = if !active {
            if self.start_time.is_none_or(|s| now < s) {
                "Engagement window not yet started".to_string()
            } else {
                "Engagement window has expired".to_string()
            }
        } else if in_grace {
            format!(
                "WARNING: {} remaining in engagement window",
                self.format_time_remaining()
            )
        } else {
            format!(
                "Engagement active: {} remaining",
                self.format_time_remaining()
            )
        };

        WindowStatus {
            active,
            start_time: self.start_time.unwrap_or(now),
            end_time: self.end_time.unwrap_or(now),
            current_time: now,
            seconds_remaining,
            time_remaining_display: self.format_time_remaining(),
            in_grace_period: in_grace,
            status_message,
        }
    }

    /// Validate that an operation can proceed
    /// Returns Ok(()) if within window, Err if outside window
    pub fn validate(&self) -> Result<()> {
        if self.is_window_active() {
            Ok(())
        } else {
            let msg = if self.start_time.is_none_or(|s| Utc::now() < s) {
                "Engagement window has not started"
            } else {
                "Engagement window has expired"
            };
            Err(ReconError::EngagementWindowViolation(msg.to_string()))
        }
    }

    /// Pause timing enforcement (for testing only)
    #[cfg(test)]
    pub fn pause(&self) {
        self.paused.store(true, Ordering::Relaxed);
    }

    /// Resume timing enforcement
    #[cfg(test)]
    pub fn resume(&self) {
        self.paused.store(false, Ordering::Relaxed);
    }

    /// Set a manual override (emergency extension)
    /// This should only be used with proper authorization
    pub fn set_override(&mut self, until: DateTime<Utc>) {
        self.manual_override_until = Some(until);
    }

    /// Clear manual override
    pub fn clear_override(&mut self) {
        self.manual_override_until = None;
    }

    /// Get the start time
    pub fn start_time(&self) -> Option<DateTime<Utc>> {
        self.start_time
    }

    /// Get the end time
    pub fn end_time(&self) -> Option<DateTime<Utc>> {
        self.end_time
    }

    /// Get time remaining as Duration
    pub fn time_remaining(&self) -> Option<Duration> {
        let end = self.end_time?;
        let now = Utc::now();
        if now >= end { None } else { Some(end - now) }
    }

    /// Get timing info
    pub fn info(&self) -> TimingInfo {
        let now = Utc::now();
        TimingInfo {
            start_time: self.start_time.map(|t| t.to_rfc3339()),
            end_time: self.end_time.map(|t| t.to_rfc3339()),
            current_time: now.to_rfc3339(),
            is_active: self.is_window_active(),
            in_grace_period: self.in_grace_period(),
            seconds_remaining: self.seconds_remaining(),
            time_remaining_display: self.format_time_remaining(),
            has_override: self.manual_override_until.is_some(),
        }
    }
}

/// Timing information for serialization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimingInfo {
    pub start_time: Option<String>,
    pub end_time: Option<String>,
    pub current_time: String,
    pub is_active: bool,
    pub in_grace_period: bool,
    pub seconds_remaining: i64,
    pub time_remaining_display: String,
    pub has_override: bool,
}

impl Default for TimingManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Timing guard that validates operations
pub struct TimingGuard<'a> {
    timing: &'a TimingManager,
}

impl<'a> TimingGuard<'a> {
    pub fn new(timing: &'a TimingManager) -> Self {
        Self { timing }
    }

    /// Validate that an operation can proceed
    pub fn check(&self) -> Result<()> {
        self.timing.validate()
    }

    /// Get a warning if in grace period
    pub fn grace_warning(&self) -> Option<String> {
        if self.timing.in_grace_period() {
            Some(format!(
                "WARNING: {} remaining in engagement window",
                self.timing.format_time_remaining()
            ))
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_timing_manager_creation() {
        let manager = TimingManager::new();
        assert!(manager.start_time.is_none());
        assert!(manager.end_time.is_none());
    }

    #[test]
    fn test_set_window() {
        let mut manager = TimingManager::new();
        let start = Utc::now();
        let end = start + Duration::hours(24);
        manager.set_window(start, end).unwrap();

        assert_eq!(manager.start_time, Some(start));
        assert_eq!(manager.end_time, Some(end));
    }

    #[test]
    fn test_invalid_window() {
        let mut manager = TimingManager::new();
        let start = Utc::now();
        let end = start - Duration::hours(1); // End before start
        let result = manager.set_window(start, end);
        assert!(result.is_err());
    }

    #[test]
    fn test_window_active() {
        let mut manager = TimingManager::new();
        let start = Utc::now() - Duration::hours(1);
        let end = Utc::now() + Duration::hours(23);
        manager.set_window(start, end).unwrap();

        assert!(manager.is_window_active());
    }

    #[test]
    fn test_window_not_started() {
        let mut manager = TimingManager::new();
        let start = Utc::now() + Duration::hours(1);
        let end = Utc::now() + Duration::hours(25);
        manager.set_window(start, end).unwrap();

        assert!(!manager.is_window_active());
    }

    #[test]
    fn test_window_expired() {
        let mut manager = TimingManager::new();
        let start = Utc::now() - Duration::hours(25);
        let end = Utc::now() - Duration::hours(1);
        manager.set_window(start, end).unwrap();

        assert!(!manager.is_window_active());
    }

    #[test]
    fn test_seconds_remaining() {
        let mut manager = TimingManager::new();
        let start = Utc::now() - Duration::hours(1);
        let end = Utc::now() + Duration::hours(1);
        manager.set_window(start, end).unwrap();

        let remaining = manager.seconds_remaining();
        assert!(remaining > 3500 && remaining <= 3600);
    }

    #[test]
    fn test_grace_period() {
        let mut manager = TimingManager::new();
        manager.set_grace_period(60); // 60 minutes grace

        let start = Utc::now() - Duration::hours(23);
        let end = Utc::now() + Duration::minutes(30);
        manager.set_window(start, end).unwrap();

        assert!(manager.in_grace_period());
    }

    #[test]
    fn test_format_time_remaining() {
        let mut manager = TimingManager::new();
        let start = Utc::now() - Duration::hours(1);
        let end = Utc::now() + Duration::hours(2) + Duration::minutes(30);
        manager.set_window(start, end).unwrap();

        let formatted = manager.format_time_remaining();
        assert!(formatted.contains("h"));
    }

    #[test]
    fn test_validate_success() {
        let mut manager = TimingManager::new();
        let start = Utc::now() - Duration::hours(1);
        let end = Utc::now() + Duration::hours(23);
        manager.set_window(start, end).unwrap();

        assert!(manager.validate().is_ok());
    }

    #[test]
    fn test_validate_failure() {
        let mut manager = TimingManager::new();
        let start = Utc::now() - Duration::hours(25);
        let end = Utc::now() - Duration::hours(1);
        manager.set_window(start, end).unwrap();

        assert!(manager.validate().is_err());
    }

    #[test]
    fn test_timing_guard() {
        let mut manager = TimingManager::new();
        let start = Utc::now() - Duration::hours(1);
        let end = Utc::now() + Duration::hours(23);
        manager.set_window(start, end).unwrap();

        let guard = TimingGuard::new(&manager);
        assert!(guard.check().is_ok());
    }

    #[test]
    fn test_status() {
        let mut manager = TimingManager::new();
        let start = Utc::now() - Duration::hours(1);
        let end = Utc::now() + Duration::hours(23);
        manager.set_window(start, end).unwrap();

        let status = manager.status();
        assert!(status.active);
        assert!(status.seconds_remaining > 0);
        assert!(!status.in_grace_period);
    }

    #[test]
    fn test_manual_override() {
        let mut manager = TimingManager::new();
        // Expired window
        let start = Utc::now() - Duration::hours(25);
        let end = Utc::now() - Duration::hours(1);
        manager.set_window(start, end).unwrap();

        assert!(!manager.is_window_active());

        // Set override
        manager.set_override(Utc::now() + Duration::hours(1));
        assert!(manager.is_window_active());

        // Clear override
        manager.clear_override();
        assert!(!manager.is_window_active());
    }
}
