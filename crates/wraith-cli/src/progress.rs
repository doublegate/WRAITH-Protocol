//! Transfer progress display with progress bars.

use indicatif::{ProgressBar, ProgressStyle};
use std::time::Duration;

/// Transfer progress tracker
pub struct TransferProgress {
    bar: ProgressBar,
}

impl TransferProgress {
    /// Create a new progress tracker
    ///
    /// # Panics
    ///
    /// Panics if the progress bar template is invalid (this should never happen
    /// as the template is hardcoded and verified at compile-time).
    #[must_use]
    pub fn new(total_bytes: u64, filename: &str) -> Self {
        let bar = ProgressBar::new(total_bytes);

        bar.set_style(
            ProgressStyle::default_bar()
                .template("{msg}\n{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({bytes_per_sec}, {eta})")
                .expect("Invalid progress bar template")
                .progress_chars("#>-")
        );

        bar.set_message(format!("Transferring: {filename}"));

        Self { bar }
    }

    /// Update progress
    #[allow(dead_code)]
    pub fn update(&self, transferred_bytes: u64) {
        self.bar.set_position(transferred_bytes);
    }

    /// Set custom message
    #[allow(dead_code)]
    pub fn set_message(&self, msg: String) {
        self.bar.set_message(msg);
    }

    /// Finish with success message
    #[allow(dead_code)]
    pub fn finish(&self) {
        self.bar.finish_with_message("Transfer complete!");
    }

    /// Finish with custom message
    pub fn finish_with_message(&self, msg: String) {
        self.bar.finish_with_message(msg);
    }

    /// Abandon the progress bar (for errors)
    #[allow(dead_code)]
    pub fn abandon(&self) {
        self.bar.abandon();
    }
}

/// Format bytes in human-readable format
///
/// # Example
///
/// ```
/// use wraith_cli::progress::format_bytes;
///
/// assert_eq!(format_bytes(1024), "1.00 KB");
/// assert_eq!(format_bytes(1024 * 1024), "1.00 MB");
/// assert_eq!(format_bytes(1024 * 1024 * 1024), "1.00 GB");
/// ```
#[must_use]
pub fn format_bytes(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    let mut size = bytes as f64;
    let mut unit_idx = 0;

    while size >= 1024.0 && unit_idx < UNITS.len() - 1 {
        size /= 1024.0;
        unit_idx += 1;
    }

    format!("{size:.2} {}", UNITS[unit_idx])
}

/// Format speed in human-readable format (bytes/sec)
///
/// # Example
///
/// ```
/// use wraith_cli::progress::format_speed;
///
/// assert_eq!(format_speed(1024.0), "1.00 KB/s");
/// assert_eq!(format_speed(1024.0 * 1024.0), "1.00 MB/s");
/// ```
#[must_use]
#[allow(dead_code)]
pub fn format_speed(bytes_per_sec: f64) -> String {
    format!("{}/s", format_bytes(bytes_per_sec as u64))
}

/// Format duration in human-readable format
///
/// # Example
///
/// ```
/// use wraith_cli::progress::format_duration;
/// use std::time::Duration;
///
/// assert_eq!(format_duration(Duration::from_secs(30)), "30s");
/// assert_eq!(format_duration(Duration::from_secs(90)), "1m 30s");
/// assert_eq!(format_duration(Duration::from_secs(3661)), "1h 1m");
/// ```
#[must_use]
#[allow(dead_code)]
pub fn format_duration(duration: Duration) -> String {
    let secs = duration.as_secs();

    if secs < 60 {
        format!("{secs}s")
    } else if secs < 3600 {
        format!("{}m {}s", secs / 60, secs % 60)
    } else {
        format!("{}h {}m", secs / 3600, (secs % 3600) / 60)
    }
}

/// Format ETA from seconds
///
/// # Example
///
/// ```
/// use wraith_cli::progress::format_eta;
///
/// assert_eq!(format_eta(30.0), "30s");
/// assert_eq!(format_eta(90.5), "1m 30s");
/// ```
#[must_use]
#[allow(dead_code)]
pub fn format_eta(seconds: f64) -> String {
    format_duration(Duration::from_secs_f64(seconds))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_bytes() {
        assert_eq!(format_bytes(0), "0.00 B");
        assert_eq!(format_bytes(512), "512.00 B");
        assert_eq!(format_bytes(1024), "1.00 KB");
        assert_eq!(format_bytes(1536), "1.50 KB");
        assert_eq!(format_bytes(1024 * 1024), "1.00 MB");
        assert_eq!(format_bytes(1024 * 1024 * 1024), "1.00 GB");
        assert_eq!(format_bytes(1024_u64.pow(4)), "1.00 TB");
    }

    #[test]
    fn test_format_bytes_edge_cases() {
        // Very small values
        assert_eq!(format_bytes(1), "1.00 B");
        assert_eq!(format_bytes(100), "100.00 B");
        assert_eq!(format_bytes(1023), "1023.00 B");

        // Boundary values
        assert_eq!(format_bytes(1024), "1.00 KB");
        assert_eq!(format_bytes(1024 * 1024 - 1), "1024.00 KB");
        assert_eq!(format_bytes(1024 * 1024), "1.00 MB");

        // Large values
        assert_eq!(format_bytes(2_500_000_000), "2.33 GB");
        assert_eq!(format_bytes(5_000_000_000_000), "4.55 TB");

        // Maximum u64
        let max_formatted = format_bytes(u64::MAX);
        assert!(max_formatted.contains("TB"));
    }

    #[test]
    fn test_format_speed() {
        assert_eq!(format_speed(0.0), "0.00 B/s");
        assert_eq!(format_speed(512.0), "512.00 B/s");
        assert_eq!(format_speed(1024.0), "1.00 KB/s");
        assert_eq!(format_speed(1_048_576.0), "1.00 MB/s");
        assert_eq!(format_speed(1_073_741_824.0), "1.00 GB/s");
    }

    #[test]
    fn test_format_speed_fractional() {
        assert_eq!(format_speed(1536.0), "1.50 KB/s");
        assert_eq!(format_speed(2_560_000.0), "2.44 MB/s");
    }

    #[test]
    fn test_format_duration() {
        assert_eq!(format_duration(Duration::from_secs(0)), "0s");
        assert_eq!(format_duration(Duration::from_secs(30)), "30s");
        assert_eq!(format_duration(Duration::from_secs(60)), "1m 0s");
        assert_eq!(format_duration(Duration::from_secs(90)), "1m 30s");
        assert_eq!(format_duration(Duration::from_secs(3600)), "1h 0m");
        assert_eq!(format_duration(Duration::from_secs(3661)), "1h 1m");
    }

    #[test]
    fn test_format_duration_edge_cases() {
        // Seconds only
        assert_eq!(format_duration(Duration::from_secs(1)), "1s");
        assert_eq!(format_duration(Duration::from_secs(59)), "59s");

        // Minutes boundary
        assert_eq!(format_duration(Duration::from_secs(60)), "1m 0s");
        assert_eq!(format_duration(Duration::from_secs(61)), "1m 1s");
        assert_eq!(format_duration(Duration::from_secs(3599)), "59m 59s");

        // Hours boundary
        assert_eq!(format_duration(Duration::from_secs(3600)), "1h 0m");
        assert_eq!(format_duration(Duration::from_secs(7200)), "2h 0m");
        assert_eq!(format_duration(Duration::from_secs(7260)), "2h 1m");

        // Large durations
        assert_eq!(format_duration(Duration::from_secs(86400)), "24h 0m"); // 1 day
        assert_eq!(format_duration(Duration::from_secs(90061)), "25h 1m");
    }

    #[test]
    fn test_format_eta() {
        assert_eq!(format_eta(30.0), "30s");
        assert_eq!(format_eta(90.5), "1m 30s");
        assert_eq!(format_eta(3661.9), "1h 1m");
    }

    #[test]
    fn test_format_eta_fractional_seconds() {
        assert_eq!(format_eta(0.0), "0s");
        assert_eq!(format_eta(0.5), "0s");
        assert_eq!(format_eta(1.9), "1s");
        assert_eq!(format_eta(59.9), "59s");
        assert_eq!(format_eta(60.5), "1m 0s");
    }

    #[test]
    fn test_transfer_progress_new() {
        let progress = TransferProgress::new(1024 * 1024, "test.txt");

        // Progress bar should be created (we can't easily test the internal state
        // but we can verify it doesn't panic)
        drop(progress);
    }

    #[test]
    fn test_transfer_progress_update() {
        let progress = TransferProgress::new(1024 * 1024, "test.txt");

        // Update progress - should not panic
        progress.update(512 * 1024);
        progress.update(1024 * 1024);

        drop(progress);
    }

    #[test]
    fn test_transfer_progress_set_message() {
        let progress = TransferProgress::new(1024 * 1024, "test.txt");

        // Set custom message - should not panic
        progress.set_message("Custom message".to_string());

        drop(progress);
    }

    #[test]
    fn test_transfer_progress_finish() {
        let progress = TransferProgress::new(1024 * 1024, "test.txt");

        // Finish with default message - should not panic
        progress.finish();
    }

    #[test]
    fn test_transfer_progress_finish_with_message() {
        let progress = TransferProgress::new(1024 * 1024, "test.txt");

        // Finish with custom message - should not panic
        progress.finish_with_message("Custom completion message".to_string());
    }

    #[test]
    fn test_transfer_progress_abandon() {
        let progress = TransferProgress::new(1024 * 1024, "test.txt");

        // Abandon progress - should not panic
        progress.abandon();
    }

    #[test]
    fn test_transfer_progress_workflow() {
        let progress = TransferProgress::new(1024 * 1024, "test.txt");

        // Simulate a complete transfer workflow
        progress.update(256 * 1024); // 25%
        progress.update(512 * 1024); // 50%
        progress.update(768 * 1024); // 75%
        progress.update(1024 * 1024); // 100%
        progress.finish();
    }

    #[test]
    fn test_transfer_progress_zero_size() {
        let progress = TransferProgress::new(0, "empty.txt");

        // Should handle zero-size files gracefully
        progress.finish();
    }

    #[test]
    fn test_transfer_progress_large_file() {
        let large_size = 100_000_000_000u64; // 100 GB
        let progress = TransferProgress::new(large_size, "large_file.dat");

        // Update with various percentages
        progress.update(large_size / 4);
        progress.update(large_size / 2);
        progress.update(large_size);
        progress.finish();
    }

    #[test]
    fn test_transfer_progress_special_filenames() {
        // Test with various filename patterns
        let filenames = [
            "simple.txt",
            "file with spaces.txt",
            "unicode_文件名.dat",
            "very_long_filename_that_might_overflow_display_buffers_in_some_implementations.txt",
            "",
        ];

        for filename in &filenames {
            let progress = TransferProgress::new(1024, filename);
            progress.finish();
        }
    }

    #[test]
    fn test_format_bytes_consistency() {
        // Verify that format_bytes and format_speed are consistent
        let bytes = 1024 * 1024;
        let bytes_str = format_bytes(bytes);
        let speed_str = format_speed(bytes as f64);

        assert!(speed_str.starts_with(&bytes_str[..bytes_str.len() - 2])); // Remove unit
    }

    #[test]
    fn test_format_duration_from_eta() {
        // Verify that format_eta and format_duration are consistent
        for seconds in [0.0, 30.5, 90.0, 3600.5] {
            let eta_str = format_eta(seconds);
            let duration_str = format_duration(Duration::from_secs_f64(seconds));
            assert_eq!(eta_str, duration_str);
        }
    }
}
