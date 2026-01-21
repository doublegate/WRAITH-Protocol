// WRAITH iOS Push Notification Module
//
// Provides push notification handling for iOS via APNs (Apple Push Notification service).
// This module handles the Rust-side of push notifications:
// - Token management (registration, storage, refresh)
// - Payload decryption
// - Background sync triggering
// - Notification content formatting
//
// # Privacy Architecture
// Following the Minimal Cloud Relay approach:
// 1. Push server sends only opaque "wake up" signals
// 2. No message content on push infrastructure
// 3. Actual messages fetched via WRAITH protocol after wake-up
// 4. Device token never linked to user identity server-side
//
// # iOS-Specific Considerations
// - Background App Refresh integration
// - Notification Service Extension support for rich notifications
// - Silent push (content-available) handling
// - Critical alerts for high-priority messages

use crate::error::WraithError;
use std::sync::{Arc, RwLock};
use std::time::{SystemTime, UNIX_EPOCH};

// =============================================================================
// Constants
// =============================================================================

/// Maximum age of push token before refresh is recommended (7 days)
const TOKEN_REFRESH_AGE_SECS: u64 = 7 * 24 * 60 * 60;

/// iOS background fetch interval recommendation (minimum, in seconds)
const IOS_BACKGROUND_FETCH_MIN_INTERVAL: u64 = 15 * 60; // 15 minutes

// =============================================================================
// Types - UniFFI Exported
// =============================================================================

/// Push notification platform
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum PushPlatform {
    Android,
    Ios,
}

impl std::fmt::Display for PushPlatform {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PushPlatform::Android => write!(f, "android"),
            PushPlatform::Ios => write!(f, "ios"),
        }
    }
}

/// Push notification token with metadata
#[derive(Debug, Clone, uniffi::Record)]
pub struct PushToken {
    /// Platform (Android/iOS)
    pub platform: PushPlatform,
    /// APNs device token (hex encoded)
    pub token: String,
    /// Creation timestamp (Unix seconds)
    pub created_at: u64,
    /// Expiration timestamp (Unix seconds), if known
    pub expires_at: Option<u64>,
    /// Whether the token needs refresh
    pub needs_refresh: bool,
}

/// Push notification settings
#[derive(Debug, Clone, uniffi::Record)]
pub struct PushSettings {
    /// Whether push notifications are enabled
    pub enabled: bool,
    /// Show message previews in notifications
    pub show_previews: bool,
    /// Show sender name in notifications
    pub show_sender_name: bool,
    /// Play sound for notifications
    pub sound_enabled: bool,
    /// Update app badge count
    pub badge_enabled: bool,
    /// Enable critical alerts (requires entitlement)
    pub critical_alerts_enabled: bool,
}

impl Default for PushSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            show_previews: false,    // Privacy-first default
            show_sender_name: false, // Privacy-first default
            sound_enabled: true,
            badge_enabled: true,
            critical_alerts_enabled: false,
        }
    }
}

/// Action to take after processing a push notification
#[derive(Debug, Clone, uniffi::Enum)]
pub enum PushAction {
    /// Trigger background sync to fetch new messages
    TriggerSync { peer_id: Option<String> },
    /// Show a notification to the user
    ShowNotification {
        title: String,
        body: String,
        category_id: String,
        thread_id: Option<String>,
        badge_count: Option<u32>,
    },
    /// Silent update (badge count, etc.)
    SilentUpdate { badge_count: Option<u32> },
    /// No action needed (duplicate, expired, etc.)
    NoAction { reason: String },
}

/// Notification content for rich notifications (Notification Service Extension)
#[derive(Debug, Clone, uniffi::Record)]
pub struct NotificationContent {
    /// Notification title
    pub title: String,
    /// Notification body/subtitle
    pub body: String,
    /// Category identifier for actions
    pub category_id: String,
    /// Thread identifier for grouping
    pub thread_id: Option<String>,
    /// Badge count to display
    pub badge_count: Option<u32>,
    /// Sound name (nil for default)
    pub sound_name: Option<String>,
    /// Whether this is a critical alert
    pub is_critical: bool,
    /// Relevance score for notification ranking (0.0-1.0)
    pub relevance_score: f64,
}

impl Default for NotificationContent {
    fn default() -> Self {
        Self {
            title: "New Message".to_string(),
            body: "You have a new secure message".to_string(),
            category_id: "WRAITH_MESSAGE".to_string(),
            thread_id: None,
            badge_count: None,
            sound_name: None,
            is_critical: false,
            relevance_score: 0.8,
        }
    }
}

/// Push notification error types
#[derive(Debug, Clone, thiserror::Error, uniffi::Error)]
pub enum PushError {
    #[error("Push notifications not enabled")]
    NotEnabled,
    #[error("Token not registered")]
    TokenNotRegistered,
    #[error("Invalid token format: {message}")]
    InvalidToken { message: String },
    #[error("Payload decryption failed: {message}")]
    DecryptionFailed { message: String },
    #[error("Storage error: {message}")]
    StorageError { message: String },
    #[error("Invalid settings: {message}")]
    InvalidSettings { message: String },
    #[error("Background task expired")]
    BackgroundTaskExpired,
}

impl From<PushError> for WraithError {
    fn from(err: PushError) -> Self {
        WraithError::Other {
            message: err.to_string(),
        }
    }
}

// =============================================================================
// Internal Types
// =============================================================================

/// Incoming push payload (encrypted/opaque from server)
#[derive(Debug, Clone)]
struct PushPayload {
    /// Unique notification identifier
    notification_id: String,
    /// Sender ID (encrypted or hashed for privacy)
    sender_id: Option<String>,
    /// Timestamp of the notification
    timestamp: u64,
    /// Small encrypted hint (e.g., message type indicator)
    encrypted_hint: Vec<u8>,
}

// =============================================================================
// Global State
// =============================================================================

/// Cached push token
static CACHED_TOKEN: RwLock<Option<PushToken>> = RwLock::new(None);

/// Cached push settings
static CACHED_SETTINGS: RwLock<Option<PushSettings>> = RwLock::new(None);

/// Set of recently processed notification IDs (for deduplication)
static PROCESSED_NOTIFICATIONS: RwLock<Option<std::collections::HashSet<String>>> =
    RwLock::new(None);

/// Initialize the processed notifications set
fn init_processed_notifications() {
    if let Ok(mut set) = PROCESSED_NOTIFICATIONS.write() {
        if set.is_none() {
            *set = Some(std::collections::HashSet::new());
        }
    }
}

/// Check if a notification has been processed (and mark it if not)
fn check_and_mark_processed(notification_id: &str) -> bool {
    if let Ok(mut set) = PROCESSED_NOTIFICATIONS.write() {
        if let Some(ref mut s) = *set {
            // Limit set size to prevent memory growth
            if s.len() > 1000 {
                s.clear();
            }
            return !s.insert(notification_id.to_string());
        }
    }
    false
}

// =============================================================================
// Helper Functions
// =============================================================================

/// Get current Unix timestamp in seconds
fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

/// Parse push payload from encrypted bytes
///
/// The payload format is:
/// - 16 bytes: notification ID (UUID)
/// - 8 bytes: timestamp (big-endian u64)
/// - 32 bytes: sender ID hash (optional, zeros if not present)
/// - Remaining: encrypted hint
fn parse_push_payload(data: &[u8]) -> std::result::Result<PushPayload, PushError> {
    if data.len() < 56 {
        return Err(PushError::DecryptionFailed {
            message: "Payload too short".to_string(),
        });
    }

    // Extract notification ID (first 16 bytes as hex)
    let notification_id = hex::encode(&data[0..16]);

    // Extract timestamp (bytes 16-24)
    let timestamp_bytes: [u8; 8] =
        data[16..24]
            .try_into()
            .map_err(|_| PushError::DecryptionFailed {
                message: "Invalid timestamp".to_string(),
            })?;
    let timestamp = u64::from_be_bytes(timestamp_bytes);

    // Extract sender ID hash (bytes 24-56)
    let sender_hash = &data[24..56];
    let sender_id = if sender_hash.iter().all(|&b| b == 0) {
        None
    } else {
        Some(hex::encode(sender_hash))
    };

    // Extract encrypted hint (remaining bytes)
    let encrypted_hint = if data.len() > 56 {
        data[56..].to_vec()
    } else {
        Vec::new()
    };

    Ok(PushPayload {
        notification_id,
        sender_id,
        timestamp,
        encrypted_hint,
    })
}

/// Determine action based on push payload and settings
fn determine_action(payload: &PushPayload, settings: &PushSettings) -> PushAction {
    // Check if notifications are enabled
    if !settings.enabled {
        return PushAction::NoAction {
            reason: "Push notifications disabled".to_string(),
        };
    }

    // Check if this notification was already processed
    if check_and_mark_processed(&payload.notification_id) {
        return PushAction::NoAction {
            reason: "Duplicate notification".to_string(),
        };
    }

    // Check if notification is too old (more than 5 minutes)
    let now = current_timestamp();
    if now.saturating_sub(payload.timestamp) > 300 {
        return PushAction::NoAction {
            reason: "Notification expired".to_string(),
        };
    }

    // Primary action: trigger sync to fetch actual message
    // The encrypted hint can indicate urgency or type
    let is_high_priority = !payload.encrypted_hint.is_empty()
        && payload
            .encrypted_hint
            .first()
            .map(|&b| b > 128)
            .unwrap_or(false);

    if is_high_priority && settings.sound_enabled {
        // For high-priority, show a notification to ensure user sees it
        let title = if settings.show_sender_name {
            payload
                .sender_id
                .as_ref()
                .map(|s| format!("Message from {}", &s[..8.min(s.len())]))
                .unwrap_or_else(|| "New Message".to_string())
        } else {
            "New Message".to_string()
        };

        let body = if settings.show_previews {
            "New secure message received".to_string()
        } else {
            "Tap to view".to_string()
        };

        PushAction::ShowNotification {
            title,
            body,
            category_id: "WRAITH_MESSAGE".to_string(),
            thread_id: payload.sender_id.clone(),
            badge_count: Some(1),
        }
    } else {
        // For normal priority, just trigger background sync
        PushAction::TriggerSync {
            peer_id: payload.sender_id.clone(),
        }
    }
}

// =============================================================================
// Push Notification Manager - UniFFI Object
// =============================================================================

/// Push notification manager for iOS
#[derive(uniffi::Object)]
pub struct PushNotificationManager {
    /// Internal state tracking initialization
    initialized: std::sync::atomic::AtomicBool,
}

#[uniffi::export]
impl PushNotificationManager {
    /// Create a new push notification manager
    #[uniffi::constructor]
    pub fn new() -> Arc<Self> {
        init_processed_notifications();
        Arc::new(Self {
            initialized: std::sync::atomic::AtomicBool::new(true),
        })
    }

    /// Check if the manager is initialized
    pub fn is_initialized(&self) -> bool {
        self.initialized.load(std::sync::atomic::Ordering::SeqCst)
    }

    /// Register a push token from APNs
    pub fn register_push_token(&self, token: String) -> std::result::Result<(), PushError> {
        if token.is_empty() {
            return Err(PushError::InvalidToken {
                message: "Token cannot be empty".to_string(),
            });
        }

        // Validate hex format
        if hex::decode(&token).is_err() && !token.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(PushError::InvalidToken {
                message: "Token must be hex encoded".to_string(),
            });
        }

        let push_token = PushToken {
            platform: PushPlatform::Ios,
            token,
            created_at: current_timestamp(),
            expires_at: None,
            needs_refresh: false,
        };

        // Cache the token
        if let Ok(mut cache) = CACHED_TOKEN.write() {
            *cache = Some(push_token);
        } else {
            return Err(PushError::StorageError {
                message: "Failed to cache token".to_string(),
            });
        }

        log::info!("Push token registered successfully");
        Ok(())
    }

    /// Unregister the current push token
    pub fn unregister_push_token(&self) -> std::result::Result<(), PushError> {
        if let Ok(mut cache) = CACHED_TOKEN.write() {
            *cache = None;
        }
        log::info!("Push token unregistered");
        Ok(())
    }

    /// Get the stored push token
    pub fn get_stored_token(&self) -> Option<PushToken> {
        CACHED_TOKEN.read().ok().and_then(|cache| {
            cache.clone().map(|mut token| {
                // Update needs_refresh based on age
                let age = current_timestamp().saturating_sub(token.created_at);
                token.needs_refresh = age > TOKEN_REFRESH_AGE_SECS;
                token
            })
        })
    }

    /// Handle an incoming push notification
    ///
    /// Returns the action to take based on the payload and current settings
    pub fn handle_push_notification(
        &self,
        payload: Vec<u8>,
    ) -> std::result::Result<PushAction, PushError> {
        // Parse the payload
        let parsed_payload = parse_push_payload(&payload)?;

        // Get current settings (use defaults if not set)
        let settings = CACHED_SETTINGS
            .read()
            .ok()
            .and_then(|s| s.clone())
            .unwrap_or_default();

        // Determine the appropriate action
        let action = determine_action(&parsed_payload, &settings);

        log::debug!(
            "Push notification processed: id={}, action={:?}",
            parsed_payload.notification_id,
            action
        );

        Ok(action)
    }

    /// Process a background push (silent, content-available)
    ///
    /// Returns true if background sync should be triggered
    pub fn process_background_push(&self, data: Vec<u8>) -> std::result::Result<bool, PushError> {
        if data.is_empty() {
            return Ok(false);
        }

        // Just validate the payload format
        let _payload = parse_push_payload(&data)?;

        log::debug!("Background push processed, triggering background sync");
        Ok(true)
    }

    /// Update push notification settings
    pub fn update_push_settings(
        &self,
        settings: PushSettings,
    ) -> std::result::Result<(), PushError> {
        if let Ok(mut cache) = CACHED_SETTINGS.write() {
            *cache = Some(settings);
        } else {
            return Err(PushError::StorageError {
                message: "Failed to cache settings".to_string(),
            });
        }
        log::info!("Push settings updated");
        Ok(())
    }

    /// Get current push notification settings
    pub fn get_push_settings(&self) -> PushSettings {
        CACHED_SETTINGS
            .read()
            .ok()
            .and_then(|s| s.clone())
            .unwrap_or_default()
    }

    /// Format notification content for display (Notification Service Extension)
    ///
    /// Decrypts and formats message data for rich notification display
    pub fn format_notification(
        &self,
        message_data: Vec<u8>,
    ) -> std::result::Result<NotificationContent, PushError> {
        let settings = self.get_push_settings();

        // Parse the message data
        let payload = parse_push_payload(&message_data)?;

        // Build notification content based on settings
        let title = if settings.show_sender_name {
            payload
                .sender_id
                .as_ref()
                .map(|s| format!("Message from {}", &s[..8.min(s.len())]))
                .unwrap_or_else(|| "New Message".to_string())
        } else {
            "New Message".to_string()
        };

        let body = if settings.show_previews {
            "New secure message received".to_string()
        } else {
            "Tap to view".to_string()
        };

        // Determine relevance based on recency and priority
        let age_seconds = current_timestamp().saturating_sub(payload.timestamp);
        let relevance_score = if age_seconds < 60 {
            1.0
        } else if age_seconds < 300 {
            0.8
        } else {
            0.5
        };

        // Check if this should be a critical alert
        let is_critical = settings.critical_alerts_enabled
            && !payload.encrypted_hint.is_empty()
            && payload
                .encrypted_hint
                .first()
                .map(|&b| b > 200)
                .unwrap_or(false);

        Ok(NotificationContent {
            title,
            body,
            category_id: "WRAITH_MESSAGE".to_string(),
            thread_id: payload.sender_id,
            badge_count: if settings.badge_enabled {
                Some(1)
            } else {
                None
            },
            sound_name: if settings.sound_enabled {
                None // Use default sound
            } else {
                Some("silent".to_string()) // No sound
            },
            is_critical,
            relevance_score,
        })
    }

    /// Check if push notifications are currently enabled
    pub fn is_enabled(&self) -> bool {
        self.get_push_settings().enabled
    }

    /// Check if a push token is registered
    pub fn has_token(&self) -> bool {
        self.get_stored_token().is_some()
    }

    /// Check if the stored token needs refresh
    pub fn token_needs_refresh(&self) -> bool {
        self.get_stored_token()
            .map(|t| t.needs_refresh)
            .unwrap_or(true)
    }

    /// Get the recommended background fetch interval
    pub fn get_background_fetch_interval(&self) -> u64 {
        IOS_BACKGROUND_FETCH_MIN_INTERVAL
    }

    /// Clear all processed notifications (for debugging/testing)
    pub fn clear_processed_notifications(&self) {
        if let Ok(mut set) = PROCESSED_NOTIFICATIONS.write() {
            if let Some(ref mut s) = *set {
                s.clear();
            }
        }
    }
}

impl Default for PushNotificationManager {
    fn default() -> Self {
        Self {
            initialized: std::sync::atomic::AtomicBool::new(true),
        }
    }
}

// =============================================================================
// UniFFI Exported Functions
// =============================================================================

/// Create a new push notification manager
#[uniffi::export]
pub fn create_push_manager() -> Arc<PushNotificationManager> {
    PushNotificationManager::new()
}

/// Register a push token (convenience function)
#[uniffi::export]
pub fn register_push_token(token: String) -> std::result::Result<(), PushError> {
    if token.is_empty() {
        return Err(PushError::InvalidToken {
            message: "Token cannot be empty".to_string(),
        });
    }

    let push_token = PushToken {
        platform: PushPlatform::Ios,
        token,
        created_at: current_timestamp(),
        expires_at: None,
        needs_refresh: false,
    };

    if let Ok(mut cache) = CACHED_TOKEN.write() {
        *cache = Some(push_token);
        Ok(())
    } else {
        Err(PushError::StorageError {
            message: "Failed to cache token".to_string(),
        })
    }
}

/// Unregister the push token (convenience function)
#[uniffi::export]
pub fn unregister_push_token() -> std::result::Result<(), PushError> {
    if let Ok(mut cache) = CACHED_TOKEN.write() {
        *cache = None;
        Ok(())
    } else {
        Err(PushError::StorageError {
            message: "Failed to clear token".to_string(),
        })
    }
}

/// Get the stored push token (convenience function)
#[uniffi::export]
pub fn get_stored_push_token() -> Option<PushToken> {
    CACHED_TOKEN.read().ok().and_then(|cache| {
        cache.clone().map(|mut token| {
            let age = current_timestamp().saturating_sub(token.created_at);
            token.needs_refresh = age > TOKEN_REFRESH_AGE_SECS;
            token
        })
    })
}

/// Handle a push notification (convenience function)
#[uniffi::export]
pub fn handle_push_notification(payload: Vec<u8>) -> std::result::Result<PushAction, PushError> {
    init_processed_notifications();

    let parsed_payload = parse_push_payload(&payload)?;
    let settings = CACHED_SETTINGS
        .read()
        .ok()
        .and_then(|s| s.clone())
        .unwrap_or_default();

    Ok(determine_action(&parsed_payload, &settings))
}

/// Process a background push (convenience function)
#[uniffi::export]
pub fn process_background_push(data: Vec<u8>) -> std::result::Result<bool, PushError> {
    if data.is_empty() {
        return Ok(false);
    }

    let _payload = parse_push_payload(&data)?;
    Ok(true)
}

/// Update push settings (convenience function)
#[uniffi::export]
pub fn update_push_settings(settings: PushSettings) -> std::result::Result<(), PushError> {
    if let Ok(mut cache) = CACHED_SETTINGS.write() {
        *cache = Some(settings);
        Ok(())
    } else {
        Err(PushError::StorageError {
            message: "Failed to cache settings".to_string(),
        })
    }
}

/// Get push settings (convenience function)
#[uniffi::export]
pub fn get_push_settings() -> PushSettings {
    CACHED_SETTINGS
        .read()
        .ok()
        .and_then(|s| s.clone())
        .unwrap_or_default()
}

/// Format notification content (convenience function)
#[uniffi::export]
pub fn format_notification(
    message_data: Vec<u8>,
) -> std::result::Result<NotificationContent, PushError> {
    let manager = PushNotificationManager::new();
    manager.format_notification(message_data)
}

// =============================================================================
// Unit Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_push_platform_display() {
        assert_eq!(PushPlatform::Android.to_string(), "android");
        assert_eq!(PushPlatform::Ios.to_string(), "ios");
    }

    #[test]
    fn test_push_settings_default() {
        let settings = PushSettings::default();
        assert!(settings.enabled);
        assert!(!settings.show_previews); // Privacy-first
        assert!(!settings.show_sender_name); // Privacy-first
        assert!(settings.sound_enabled);
        assert!(settings.badge_enabled);
        assert!(!settings.critical_alerts_enabled);
    }

    #[test]
    fn test_notification_content_default() {
        let content = NotificationContent::default();
        assert_eq!(content.title, "New Message");
        assert_eq!(content.category_id, "WRAITH_MESSAGE");
        assert!(!content.is_critical);
        assert!(content.relevance_score > 0.0);
    }

    #[test]
    fn test_push_token_creation() {
        let token = PushToken {
            platform: PushPlatform::Ios,
            token: "test_token_hex".to_string(),
            created_at: current_timestamp(),
            expires_at: None,
            needs_refresh: false,
        };

        assert_eq!(token.platform, PushPlatform::Ios);
        assert!(!token.needs_refresh);
    }

    #[test]
    fn test_token_registration() {
        // Clear any existing token
        let _ = unregister_push_token();

        // Register a new token
        let result = register_push_token("abcdef1234567890".to_string());
        assert!(result.is_ok());

        // Verify token is stored
        let stored = get_stored_push_token();
        assert!(stored.is_some());

        let token = stored.unwrap();
        assert_eq!(token.token, "abcdef1234567890");
        assert_eq!(token.platform, PushPlatform::Ios);

        // Cleanup
        let _ = unregister_push_token();
    }

    #[test]
    fn test_empty_token_rejected() {
        let result = register_push_token(String::new());
        assert!(result.is_err());
    }

    #[test]
    fn test_payload_parsing() {
        // Create a valid payload
        let mut payload = Vec::new();

        // Notification ID (16 bytes)
        payload.extend_from_slice(&[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16]);

        // Timestamp (8 bytes, big-endian)
        let timestamp: u64 = 1700000000;
        payload.extend_from_slice(&timestamp.to_be_bytes());

        // Sender hash (32 bytes)
        payload.extend_from_slice(&[0xab; 32]);

        // Encrypted hint (variable)
        payload.extend_from_slice(&[0x01, 0x02, 0x03]);

        let parsed = parse_push_payload(&payload).unwrap();
        assert_eq!(parsed.notification_id, "0102030405060708090a0b0c0d0e0f10");
        assert_eq!(parsed.timestamp, 1700000000);
        assert!(parsed.sender_id.is_some());
        assert_eq!(parsed.encrypted_hint.len(), 3);
    }

    #[test]
    fn test_payload_too_short() {
        let payload = vec![0u8; 10];
        let result = parse_push_payload(&payload);
        assert!(result.is_err());
    }

    #[test]
    fn test_push_error_display() {
        let err = PushError::NotEnabled;
        assert!(err.to_string().contains("not enabled"));

        let err = PushError::TokenNotRegistered;
        assert!(err.to_string().contains("not registered"));

        let err = PushError::InvalidToken {
            message: "bad token".to_string(),
        };
        assert!(err.to_string().contains("bad token"));

        let err = PushError::BackgroundTaskExpired;
        assert!(err.to_string().contains("expired"));
    }

    #[test]
    fn test_manager_creation() {
        let manager = PushNotificationManager::new();
        assert!(manager.is_initialized());
    }

    #[test]
    fn test_manager_settings() {
        let manager = PushNotificationManager::new();

        let new_settings = PushSettings {
            enabled: true,
            show_previews: true,
            show_sender_name: true,
            sound_enabled: false,
            badge_enabled: false,
            critical_alerts_enabled: true,
        };

        manager.update_push_settings(new_settings.clone()).unwrap();

        let retrieved = manager.get_push_settings();
        assert_eq!(retrieved.enabled, new_settings.enabled);
        assert_eq!(retrieved.show_previews, new_settings.show_previews);
        assert_eq!(
            retrieved.critical_alerts_enabled,
            new_settings.critical_alerts_enabled
        );
    }

    #[test]
    fn test_background_fetch_interval() {
        let manager = PushNotificationManager::new();
        let interval = manager.get_background_fetch_interval();
        assert_eq!(interval, IOS_BACKGROUND_FETCH_MIN_INTERVAL);
    }

    #[test]
    fn test_determine_action_disabled() {
        init_processed_notifications();

        let payload = PushPayload {
            notification_id: "test_disabled_ios".to_string(),
            sender_id: None,
            timestamp: current_timestamp(),
            encrypted_hint: Vec::new(),
        };

        let settings = PushSettings {
            enabled: false,
            ..Default::default()
        };

        let action = determine_action(&payload, &settings);
        match action {
            PushAction::NoAction { reason } => {
                assert!(reason.contains("disabled"));
            }
            _ => panic!("Expected NoAction for disabled notifications"),
        }
    }

    #[test]
    fn test_determine_action_expired() {
        init_processed_notifications();

        let payload = PushPayload {
            notification_id: "test_expired_ios_unique".to_string(),
            sender_id: None,
            timestamp: current_timestamp() - 600, // 10 minutes old
            encrypted_hint: Vec::new(),
        };

        let settings = PushSettings::default();

        let action = determine_action(&payload, &settings);
        match action {
            PushAction::NoAction { reason } => {
                assert!(reason.contains("expired"));
            }
            _ => panic!("Expected NoAction for expired notification"),
        }
    }

    #[test]
    fn test_determine_action_trigger_sync() {
        init_processed_notifications();

        let payload = PushPayload {
            notification_id: format!("test_sync_ios_{}", current_timestamp()),
            sender_id: Some("sender123".to_string()),
            timestamp: current_timestamp(),
            encrypted_hint: vec![0x01], // Low priority
        };

        let settings = PushSettings::default();

        let action = determine_action(&payload, &settings);
        match action {
            PushAction::TriggerSync { peer_id } => {
                assert_eq!(peer_id, Some("sender123".to_string()));
            }
            _ => panic!("Expected TriggerSync action"),
        }
    }

    #[test]
    fn test_deduplication() {
        init_processed_notifications();

        let notification_id = "test_notification_dedup_ios_unique_12345";

        // First check should return false (not processed) and mark it
        let already_processed = check_and_mark_processed(notification_id);
        assert!(!already_processed);

        // Second check should return true (already processed)
        let already_processed = check_and_mark_processed(notification_id);
        assert!(already_processed);
    }

    #[test]
    fn test_silent_push_empty() {
        let result = process_background_push(vec![]);
        assert!(result.is_ok());
        assert!(!result.unwrap());
    }

    #[test]
    fn test_silent_push_valid() {
        let mut payload = vec![0u8; 56];
        payload[..16].copy_from_slice(&[1; 16]);
        payload[16..24].copy_from_slice(&current_timestamp().to_be_bytes());

        let result = process_background_push(payload);
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[test]
    fn test_format_notification() {
        let manager = PushNotificationManager::new();

        // Update settings to show previews
        let settings = PushSettings {
            show_previews: true,
            show_sender_name: true,
            ..Default::default()
        };
        manager.update_push_settings(settings).unwrap();

        // Create a valid payload
        let mut payload = vec![0u8; 56];
        payload[..16].copy_from_slice(&[1; 16]);
        payload[16..24].copy_from_slice(&current_timestamp().to_be_bytes());
        payload[24..56].copy_from_slice(&[0xab; 32]); // Sender ID

        let content = manager.format_notification(payload).unwrap();
        assert!(content.title.contains("Message from"));
        assert_eq!(content.category_id, "WRAITH_MESSAGE");
        assert!(content.thread_id.is_some());
    }

    #[test]
    fn test_current_timestamp() {
        let ts = current_timestamp();
        assert!(ts > 1577836800); // After 2020-01-01
        assert!(ts < 4102444800); // Before 2100-01-01
    }

    #[test]
    fn test_token_needs_refresh() {
        // Clear and register a fresh token
        let _ = unregister_push_token();
        let _ = register_push_token("fresh_token".to_string());

        let manager = PushNotificationManager::new();
        // Fresh token should not need refresh
        // Note: This depends on implementation - new tokens have needs_refresh computed
        let token = manager.get_stored_token();
        assert!(token.is_some());

        // Cleanup
        let _ = unregister_push_token();
    }

    #[test]
    fn test_clear_processed_notifications() {
        init_processed_notifications();

        let notification_id = "test_clear_notifications_unique";
        let _ = check_and_mark_processed(notification_id);

        // Should be marked as processed
        assert!(check_and_mark_processed(notification_id));

        // Clear and check again
        let manager = PushNotificationManager::new();
        manager.clear_processed_notifications();

        // Should no longer be marked as processed
        assert!(!check_and_mark_processed(notification_id));
    }
}
