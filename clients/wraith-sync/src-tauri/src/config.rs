//! Sync Configuration Module
//!
//! Manages application settings and configuration.

use crate::database::Database;
use crate::error::SyncResult;
use crate::sync_engine::ConflictStrategy;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Application settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    /// Bandwidth limit for uploads (bytes/sec, 0 = unlimited)
    pub upload_limit: u64,
    /// Bandwidth limit for downloads (bytes/sec, 0 = unlimited)
    pub download_limit: u64,
    /// Conflict resolution strategy
    pub conflict_strategy: String,
    /// Maximum file versions to keep
    pub max_versions: i64,
    /// Version retention days
    pub version_retention_days: i64,
    /// Enable delta sync
    pub enable_delta_sync: bool,
    /// Debounce interval (milliseconds)
    pub debounce_ms: u64,
    /// Start sync on application launch
    pub auto_start: bool,
    /// Show notifications
    pub notifications_enabled: bool,
    /// Theme (light, dark, system)
    pub theme: String,
    /// Device name
    pub device_name: String,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            upload_limit: 0,
            download_limit: 0,
            conflict_strategy: "last_writer_wins".to_string(),
            max_versions: 10,
            version_retention_days: 30,
            enable_delta_sync: true,
            debounce_ms: 100,
            auto_start: true,
            notifications_enabled: true,
            theme: "system".to_string(),
            device_name: hostname::get()
                .map(|h| h.to_string_lossy().to_string())
                .unwrap_or_else(|_| "Unknown Device".to_string()),
        }
    }
}

impl AppSettings {
    /// Parse conflict strategy from string
    pub fn parse_conflict_strategy(&self) -> ConflictStrategy {
        match self.conflict_strategy.as_str() {
            "keep_both" => ConflictStrategy::KeepBoth,
            "manual" => ConflictStrategy::Manual,
            _ => ConflictStrategy::LastWriterWins,
        }
    }
}

/// Configuration manager
pub struct ConfigManager {
    db: Arc<Database>,
}

impl ConfigManager {
    /// Create a new configuration manager
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }

    /// Load settings from database
    pub fn load_settings(&self) -> SyncResult<AppSettings> {
        let mut settings = AppSettings::default();

        if let Some(value) = self.db.get_setting("upload_limit")? {
            settings.upload_limit = value.parse().unwrap_or(0);
        }
        if let Some(value) = self.db.get_setting("download_limit")? {
            settings.download_limit = value.parse().unwrap_or(0);
        }
        if let Some(value) = self.db.get_setting("conflict_strategy")? {
            settings.conflict_strategy = value;
        }
        if let Some(value) = self.db.get_setting("max_versions")? {
            settings.max_versions = value.parse().unwrap_or(10);
        }
        if let Some(value) = self.db.get_setting("version_retention_days")? {
            settings.version_retention_days = value.parse().unwrap_or(30);
        }
        if let Some(value) = self.db.get_setting("enable_delta_sync")? {
            settings.enable_delta_sync = value == "true";
        }
        if let Some(value) = self.db.get_setting("debounce_ms")? {
            settings.debounce_ms = value.parse().unwrap_or(100);
        }
        if let Some(value) = self.db.get_setting("auto_start")? {
            settings.auto_start = value == "true";
        }
        if let Some(value) = self.db.get_setting("notifications_enabled")? {
            settings.notifications_enabled = value == "true";
        }
        if let Some(value) = self.db.get_setting("theme")? {
            settings.theme = value;
        }
        if let Some(value) = self.db.get_setting("device_name")? {
            settings.device_name = value;
        }

        Ok(settings)
    }

    /// Save settings to database
    pub fn save_settings(&self, settings: &AppSettings) -> SyncResult<()> {
        self.db
            .set_setting("upload_limit", &settings.upload_limit.to_string())?;
        self.db
            .set_setting("download_limit", &settings.download_limit.to_string())?;
        self.db
            .set_setting("conflict_strategy", &settings.conflict_strategy)?;
        self.db
            .set_setting("max_versions", &settings.max_versions.to_string())?;
        self.db.set_setting(
            "version_retention_days",
            &settings.version_retention_days.to_string(),
        )?;
        self.db
            .set_setting("enable_delta_sync", &settings.enable_delta_sync.to_string())?;
        self.db
            .set_setting("debounce_ms", &settings.debounce_ms.to_string())?;
        self.db
            .set_setting("auto_start", &settings.auto_start.to_string())?;
        self.db.set_setting(
            "notifications_enabled",
            &settings.notifications_enabled.to_string(),
        )?;
        self.db.set_setting("theme", &settings.theme)?;
        self.db.set_setting("device_name", &settings.device_name)?;

        Ok(())
    }

    /// Update a single setting
    pub fn update_setting(&self, key: &str, value: &str) -> SyncResult<()> {
        Ok(self.db.set_setting(key, value)?)
    }

    /// Get a single setting
    pub fn get_setting(&self, key: &str) -> SyncResult<Option<String>> {
        Ok(self.db.get_setting(key)?)
    }
}
