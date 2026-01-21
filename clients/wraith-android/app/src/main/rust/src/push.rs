// WRAITH Android Push Notification Module
//
// Provides push notification handling for Android via FCM (Firebase Cloud Messaging).
// This module handles the Rust-side of push notifications:
// - Token management (registration, storage, refresh)
// - Payload decryption
// - Background sync triggering
// - Notification action determination
//
// # Privacy Architecture
// Following the Minimal Cloud Relay approach:
// 1. Push server sends only opaque "wake up" signals
// 2. No message content on push infrastructure
// 3. Actual messages fetched via WRAITH protocol after wake-up
// 4. Device token never linked to user identity server-side
//
// # Usage Flow
// 1. Android app receives FCM token -> registerToken()
// 2. FCM delivers push -> handlePush() -> returns action JSON
// 3. App performs action (sync, show notification, etc.)

use crate::error::Error;
use jni::JNIEnv;
use jni::objects::{JByteArray, JClass, JString};
use jni::sys::{JNI_FALSE, JNI_TRUE, jboolean, jstring};
use std::sync::RwLock;
use std::time::{SystemTime, UNIX_EPOCH};

// =============================================================================
// Constants
// =============================================================================

/// Keychain/keystore key for push token storage (reserved for persistent storage)
#[allow(dead_code)]
const PUSH_TOKEN_KEY: &str = "wraith_push_token";

/// Keychain/keystore key for push settings storage (reserved for persistent storage)
#[allow(dead_code)]
const PUSH_SETTINGS_KEY: &str = "wraith_push_settings";

/// Maximum age of push token before refresh is recommended (7 days)
const TOKEN_REFRESH_AGE_SECS: u64 = 7 * 24 * 60 * 60;

// =============================================================================
// Types
// =============================================================================

/// Push notification platform
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PushPlatform {
    Android,
    #[allow(dead_code)]
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
#[derive(Debug, Clone)]
pub struct PushToken {
    /// Platform (Android/iOS)
    pub platform: PushPlatform,
    /// FCM/APNs token string
    pub token: String,
    /// Creation timestamp (Unix seconds)
    pub created_at: u64,
    /// Expiration timestamp (Unix seconds), if known
    pub expires_at: Option<u64>,
}

/// Incoming push payload (encrypted/opaque from server)
#[derive(Debug, Clone)]
pub struct PushPayload {
    /// Unique notification identifier
    pub notification_id: String,
    /// Sender ID (encrypted or hashed for privacy)
    pub sender_id: Option<String>,
    /// Timestamp of the notification
    pub timestamp: u64,
    /// Small encrypted hint (e.g., message type indicator)
    pub encrypted_hint: Vec<u8>,
}

/// Push notification settings
#[derive(Debug, Clone)]
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
}

impl Default for PushSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            show_previews: false,    // Privacy-first default
            show_sender_name: false, // Privacy-first default
            sound_enabled: true,
            badge_enabled: true,
        }
    }
}

/// Action to take after processing a push notification
#[derive(Debug, Clone)]
pub enum PushAction {
    /// Trigger background sync to fetch new messages
    TriggerSync { peer_id: Option<String> },
    /// Show a notification to the user
    ShowNotification {
        title: String,
        body: String,
        channel_id: String,
    },
    /// Silent update (badge count, etc.)
    SilentUpdate { badge_count: Option<u32> },
    /// No action needed (duplicate, expired, etc.)
    NoAction { reason: String },
}

/// Push notification error types
#[derive(Debug, Clone, thiserror::Error)]
pub enum PushError {
    #[error("Push notifications not enabled")]
    NotEnabled,
    #[error("Token not registered")]
    TokenNotRegistered,
    #[error("Invalid token format: {0}")]
    InvalidToken(String),
    #[error("Payload decryption failed: {0}")]
    DecryptionFailed(String),
    #[error("Storage error: {0}")]
    StorageError(String),
    #[error("Invalid settings: {0}")]
    InvalidSettings(String),
}

impl From<PushError> for Error {
    fn from(err: PushError) -> Self {
        Error::Other(err.to_string())
    }
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

/// Helper to convert JString to Rust String safely
fn jstring_to_string(env: &mut JNIEnv, s: &JString) -> Result<String, Error> {
    env.get_string(s).map(|s| s.into()).map_err(Error::Jni)
}

/// Parse push settings from JSON
fn parse_settings_json(json: &str) -> Result<PushSettings, PushError> {
    let parsed: serde_json::Value =
        serde_json::from_str(json).map_err(|e| PushError::InvalidSettings(e.to_string()))?;

    Ok(PushSettings {
        enabled: parsed
            .get("enabled")
            .and_then(|v| v.as_bool())
            .unwrap_or(true),
        show_previews: parsed
            .get("showPreviews")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
        show_sender_name: parsed
            .get("showSenderName")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
        sound_enabled: parsed
            .get("soundEnabled")
            .and_then(|v| v.as_bool())
            .unwrap_or(true),
        badge_enabled: parsed
            .get("badgeEnabled")
            .and_then(|v| v.as_bool())
            .unwrap_or(true),
    })
}

/// Serialize push settings to JSON
fn settings_to_json(settings: &PushSettings) -> String {
    serde_json::json!({
        "enabled": settings.enabled,
        "showPreviews": settings.show_previews,
        "showSenderName": settings.show_sender_name,
        "soundEnabled": settings.sound_enabled,
        "badgeEnabled": settings.badge_enabled
    })
    .to_string()
}

/// Serialize push token to JSON
fn token_to_json(token: &PushToken) -> String {
    serde_json::json!({
        "platform": token.platform.to_string(),
        "token": token.token,
        "createdAt": token.created_at,
        "expiresAt": token.expires_at,
        "needsRefresh": current_timestamp().saturating_sub(token.created_at) > TOKEN_REFRESH_AGE_SECS
    })
    .to_string()
}

/// Parse push payload from encrypted bytes
///
/// The payload format is:
/// - 16 bytes: notification ID (UUID)
/// - 8 bytes: timestamp (big-endian u64)
/// - 32 bytes: sender ID hash (optional, zeros if not present)
/// - Remaining: encrypted hint
fn parse_push_payload(data: &[u8]) -> Result<PushPayload, PushError> {
    if data.len() < 56 {
        return Err(PushError::DecryptionFailed("Payload too short".to_string()));
    }

    // Extract notification ID (first 16 bytes as hex)
    let notification_id = hex::encode(&data[0..16]);

    // Extract timestamp (bytes 16-24)
    let timestamp_bytes: [u8; 8] = data[16..24]
        .try_into()
        .map_err(|_| PushError::DecryptionFailed("Invalid timestamp".to_string()))?;
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
                .map(|s| format!("Message from {}", &s[..8]))
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
            channel_id: "wraith_messages".to_string(),
        }
    } else {
        // For normal priority, just trigger background sync
        PushAction::TriggerSync {
            peer_id: payload.sender_id.clone(),
        }
    }
}

/// Serialize PushAction to JSON for JNI return
fn action_to_json(action: &PushAction) -> String {
    match action {
        PushAction::TriggerSync { peer_id } => serde_json::json!({
            "action": "triggerSync",
            "peerId": peer_id
        })
        .to_string(),
        PushAction::ShowNotification {
            title,
            body,
            channel_id,
        } => serde_json::json!({
            "action": "showNotification",
            "title": title,
            "body": body,
            "channelId": channel_id
        })
        .to_string(),
        PushAction::SilentUpdate { badge_count } => serde_json::json!({
            "action": "silentUpdate",
            "badgeCount": badge_count
        })
        .to_string(),
        PushAction::NoAction { reason } => serde_json::json!({
            "action": "noAction",
            "reason": reason
        })
        .to_string(),
    }
}

// =============================================================================
// Public API (Rust)
// =============================================================================

/// Register a push token
pub fn register_token(token: String) -> Result<(), PushError> {
    if token.is_empty() {
        return Err(PushError::InvalidToken("Token cannot be empty".to_string()));
    }

    let push_token = PushToken {
        platform: PushPlatform::Android,
        token,
        created_at: current_timestamp(),
        expires_at: None,
    };

    // Cache the token
    if let Ok(mut cache) = CACHED_TOKEN.write() {
        *cache = Some(push_token);
    } else {
        return Err(PushError::StorageError("Failed to cache token".to_string()));
    }

    log::info!("Push token registered successfully");
    Ok(())
}

/// Unregister the current push token
pub fn unregister_token() -> Result<(), PushError> {
    if let Ok(mut cache) = CACHED_TOKEN.write() {
        *cache = None;
    }
    log::info!("Push token unregistered");
    Ok(())
}

/// Get the stored push token
pub fn get_stored_token() -> Option<PushToken> {
    CACHED_TOKEN.read().ok().and_then(|cache| cache.clone())
}

/// Handle an incoming push notification
pub fn handle_push(payload_data: &[u8]) -> Result<PushAction, PushError> {
    init_processed_notifications();

    // Parse the payload
    let payload = parse_push_payload(payload_data)?;

    // Get current settings (use defaults if not set)
    let settings = CACHED_SETTINGS
        .read()
        .ok()
        .and_then(|s| s.clone())
        .unwrap_or_default();

    // Determine the appropriate action
    let action = determine_action(&payload, &settings);

    log::debug!(
        "Push notification processed: id={}, action={:?}",
        payload.notification_id,
        action
    );

    Ok(action)
}

/// Process a silent push (data-only, no notification)
pub fn process_silent_push(data: &[u8]) -> Result<bool, PushError> {
    // Silent pushes are used to trigger background sync
    if data.is_empty() {
        return Ok(false);
    }

    // Just validate the payload format
    let _payload = parse_push_payload(data)?;

    log::debug!("Silent push processed, triggering background sync");
    Ok(true)
}

/// Update push notification settings
pub fn update_settings(settings: PushSettings) -> Result<(), PushError> {
    if let Ok(mut cache) = CACHED_SETTINGS.write() {
        *cache = Some(settings);
    } else {
        return Err(PushError::StorageError(
            "Failed to cache settings".to_string(),
        ));
    }
    log::info!("Push settings updated");
    Ok(())
}

/// Get current push notification settings
pub fn get_settings() -> PushSettings {
    CACHED_SETTINGS
        .read()
        .ok()
        .and_then(|s| s.clone())
        .unwrap_or_default()
}

// =============================================================================
// JNI Functions - Called from Java/Kotlin
// =============================================================================

/// Register a push token from FCM
///
/// # Safety
/// This function is called from Java via JNI.
#[unsafe(no_mangle)]
pub extern "C" fn Java_com_wraith_android_WraithPush_registerToken(
    mut env: JNIEnv,
    _class: JClass,
    token: JString,
) -> jboolean {
    let token_str = match jstring_to_string(&mut env, &token) {
        Ok(s) => s,
        Err(e) => {
            log::error!("Failed to parse token string: {}", e);
            return JNI_FALSE;
        }
    };

    match register_token(token_str) {
        Ok(()) => JNI_TRUE,
        Err(e) => {
            log::error!("Failed to register token: {}", e);
            JNI_FALSE
        }
    }
}

/// Unregister the current push token
///
/// # Safety
/// This function is called from Java via JNI.
#[unsafe(no_mangle)]
pub extern "C" fn Java_com_wraith_android_WraithPush_unregisterToken(
    _env: JNIEnv,
    _class: JClass,
) -> jboolean {
    match unregister_token() {
        Ok(()) => JNI_TRUE,
        Err(e) => {
            log::error!("Failed to unregister token: {}", e);
            JNI_FALSE
        }
    }
}

/// Get the stored push token as JSON
///
/// # Safety
/// This function is called from Java via JNI.
#[unsafe(no_mangle)]
pub extern "C" fn Java_com_wraith_android_WraithPush_getStoredToken(
    env: JNIEnv,
    _class: JClass,
) -> jstring {
    let token = match get_stored_token() {
        Some(t) => t,
        None => return std::ptr::null_mut(),
    };

    let json = token_to_json(&token);

    #[allow(unused_mut)]
    let mut env = env;
    match env.new_string(json) {
        Ok(s) => s.into_raw(),
        Err(e) => {
            log::error!("Failed to create Java string: {}", e);
            std::ptr::null_mut()
        }
    }
}

/// Handle an incoming push notification
///
/// # Safety
/// This function is called from Java via JNI.
///
/// Parameters:
/// - `payload`: byte[] - The raw push payload
///
/// Returns: JSON string describing the action to take
#[unsafe(no_mangle)]
pub extern "C" fn Java_com_wraith_android_WraithPush_handlePush(
    env: JNIEnv,
    _class: JClass,
    payload: JByteArray,
) -> jstring {
    // JNIEnv uses interior mutability for byte array operations
    #[allow(unused_mut)]
    let mut env = env;
    let payload_bytes = match env.convert_byte_array(&payload) {
        Ok(bytes) => bytes,
        Err(e) => {
            log::error!("Failed to convert payload array: {}", e);
            let error_json = serde_json::json!({
                "action": "noAction",
                "reason": format!("Invalid payload: {}", e)
            })
            .to_string();
            return match env.new_string(error_json) {
                Ok(s) => s.into_raw(),
                Err(_) => std::ptr::null_mut(),
            };
        }
    };

    let action = match handle_push(&payload_bytes) {
        Ok(a) => a,
        Err(e) => {
            log::error!("Failed to handle push: {}", e);
            PushAction::NoAction {
                reason: e.to_string(),
            }
        }
    };

    let json = action_to_json(&action);
    match env.new_string(json) {
        Ok(s) => s.into_raw(),
        Err(e) => {
            log::error!("Failed to create Java string: {}", e);
            std::ptr::null_mut()
        }
    }
}

/// Process a silent push (data-only)
///
/// # Safety
/// This function is called from Java via JNI.
#[unsafe(no_mangle)]
pub extern "C" fn Java_com_wraith_android_WraithPush_processSilentPush(
    #[allow(unused_mut)] mut env: JNIEnv,
    _class: JClass,
    data: JByteArray,
) -> jboolean {
    let data_bytes = match env.convert_byte_array(&data) {
        Ok(bytes) => bytes,
        Err(e) => {
            log::error!("Failed to convert data array: {}", e);
            return JNI_FALSE;
        }
    };

    match process_silent_push(&data_bytes) {
        Ok(true) => JNI_TRUE,
        Ok(false) | Err(_) => JNI_FALSE,
    }
}

/// Update push notification settings
///
/// # Safety
/// This function is called from Java via JNI.
///
/// Parameters:
/// - `settingsJson`: String - JSON representation of PushSettings
#[unsafe(no_mangle)]
pub extern "C" fn Java_com_wraith_android_WraithPush_updateSettings(
    #[allow(unused_mut)] mut env: JNIEnv,
    _class: JClass,
    settings_json: JString,
) -> jboolean {
    let json_str = match jstring_to_string(&mut env, &settings_json) {
        Ok(s) => s,
        Err(e) => {
            log::error!("Failed to parse settings JSON: {}", e);
            return JNI_FALSE;
        }
    };

    let settings = match parse_settings_json(&json_str) {
        Ok(s) => s,
        Err(e) => {
            log::error!("Failed to parse settings: {}", e);
            return JNI_FALSE;
        }
    };

    match update_settings(settings) {
        Ok(()) => JNI_TRUE,
        Err(e) => {
            log::error!("Failed to update settings: {}", e);
            JNI_FALSE
        }
    }
}

/// Get current push notification settings as JSON
///
/// # Safety
/// This function is called from Java via JNI.
#[unsafe(no_mangle)]
pub extern "C" fn Java_com_wraith_android_WraithPush_getSettings(
    env: JNIEnv,
    _class: JClass,
) -> jstring {
    let settings = get_settings();
    let json = settings_to_json(&settings);

    #[allow(unused_mut)]
    let mut env = env;
    match env.new_string(json) {
        Ok(s) => s.into_raw(),
        Err(e) => {
            log::error!("Failed to create Java string: {}", e);
            std::ptr::null_mut()
        }
    }
}

/// Check if push notifications are currently enabled
///
/// # Safety
/// This function is called from Java via JNI.
#[unsafe(no_mangle)]
pub extern "C" fn Java_com_wraith_android_WraithPush_isEnabled(
    _env: JNIEnv,
    _class: JClass,
) -> jboolean {
    let settings = get_settings();
    if settings.enabled {
        JNI_TRUE
    } else {
        JNI_FALSE
    }
}

/// Check if a push token is registered
///
/// # Safety
/// This function is called from Java via JNI.
#[unsafe(no_mangle)]
pub extern "C" fn Java_com_wraith_android_WraithPush_hasToken(
    _env: JNIEnv,
    _class: JClass,
) -> jboolean {
    if get_stored_token().is_some() {
        JNI_TRUE
    } else {
        JNI_FALSE
    }
}

/// Check if the stored token needs refresh
///
/// # Safety
/// This function is called from Java via JNI.
#[unsafe(no_mangle)]
pub extern "C" fn Java_com_wraith_android_WraithPush_tokenNeedsRefresh(
    _env: JNIEnv,
    _class: JClass,
) -> jboolean {
    let token = match get_stored_token() {
        Some(t) => t,
        None => return JNI_TRUE, // No token = needs refresh
    };

    let age = current_timestamp().saturating_sub(token.created_at);
    if age > TOKEN_REFRESH_AGE_SECS {
        JNI_TRUE
    } else {
        JNI_FALSE
    }
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
    }

    #[test]
    fn test_settings_serialization() {
        let settings = PushSettings {
            enabled: true,
            show_previews: true,
            show_sender_name: false,
            sound_enabled: true,
            badge_enabled: false,
        };

        let json = settings_to_json(&settings);
        assert!(json.contains("\"enabled\":true"));
        assert!(json.contains("\"showPreviews\":true"));
        assert!(json.contains("\"showSenderName\":false"));
    }

    #[test]
    fn test_settings_parsing() {
        let json = r#"{"enabled":true,"showPreviews":false,"showSenderName":true,"soundEnabled":false,"badgeEnabled":true}"#;
        let settings = parse_settings_json(json).unwrap();

        assert!(settings.enabled);
        assert!(!settings.show_previews);
        assert!(settings.show_sender_name);
        assert!(!settings.sound_enabled);
        assert!(settings.badge_enabled);
    }

    #[test]
    fn test_settings_parsing_partial() {
        // Test parsing with missing fields (should use defaults)
        let json = r#"{"enabled":false}"#;
        let settings = parse_settings_json(json).unwrap();

        assert!(!settings.enabled);
        assert!(!settings.show_previews); // Default
        assert!(settings.sound_enabled); // Default
    }

    #[test]
    fn test_token_registration() {
        // Clear any existing token
        let _ = unregister_token();

        // Register a new token
        let result = register_token("test_token_12345".to_string());
        assert!(result.is_ok());

        // Verify token is stored
        let stored = get_stored_token();
        assert!(stored.is_some());

        let token = stored.unwrap();
        assert_eq!(token.token, "test_token_12345");
        assert_eq!(token.platform, PushPlatform::Android);
        assert!(token.created_at > 0);

        // Cleanup
        let _ = unregister_token();
    }

    #[test]
    fn test_token_unregistration() {
        // Register then unregister
        let _ = register_token("temp_token".to_string());
        let _ = unregister_token();

        let stored = get_stored_token();
        assert!(stored.is_none());
    }

    #[test]
    fn test_empty_token_rejected() {
        let result = register_token(String::new());
        assert!(result.is_err());
    }

    #[test]
    fn test_payload_parsing() {
        // Create a valid payload
        // 16 bytes notification ID + 8 bytes timestamp + 32 bytes sender hash + hint
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
    fn test_payload_parsing_no_sender() {
        let mut payload = Vec::new();

        // Notification ID (16 bytes)
        payload.extend_from_slice(&[0xff; 16]);

        // Timestamp (8 bytes)
        payload.extend_from_slice(&1700000000u64.to_be_bytes());

        // Sender hash (32 bytes, all zeros = no sender)
        payload.extend_from_slice(&[0x00; 32]);

        let parsed = parse_push_payload(&payload).unwrap();
        assert!(parsed.sender_id.is_none());
    }

    #[test]
    fn test_payload_too_short() {
        let payload = vec![0u8; 10]; // Too short
        let result = parse_push_payload(&payload);
        assert!(result.is_err());
    }

    #[test]
    fn test_action_json_serialization() {
        let action = PushAction::TriggerSync {
            peer_id: Some("abc123".to_string()),
        };
        let json = action_to_json(&action);
        assert!(json.contains("\"action\":\"triggerSync\""));
        assert!(json.contains("\"peerId\":\"abc123\""));

        let action = PushAction::ShowNotification {
            title: "Test Title".to_string(),
            body: "Test Body".to_string(),
            channel_id: "test_channel".to_string(),
        };
        let json = action_to_json(&action);
        assert!(json.contains("\"action\":\"showNotification\""));
        assert!(json.contains("\"title\":\"Test Title\""));

        let action = PushAction::NoAction {
            reason: "test reason".to_string(),
        };
        let json = action_to_json(&action);
        assert!(json.contains("\"action\":\"noAction\""));
        assert!(json.contains("\"reason\":\"test reason\""));
    }

    #[test]
    fn test_token_json_serialization() {
        let token = PushToken {
            platform: PushPlatform::Android,
            token: "test_token".to_string(),
            created_at: 1700000000,
            expires_at: Some(1700100000),
        };

        let json = token_to_json(&token);
        assert!(json.contains("\"platform\":\"android\""));
        assert!(json.contains("\"token\":\"test_token\""));
        assert!(json.contains("\"createdAt\":1700000000"));
        assert!(json.contains("\"expiresAt\":1700100000"));
    }

    #[test]
    fn test_push_error_display() {
        let err = PushError::NotEnabled;
        assert!(err.to_string().contains("not enabled"));

        let err = PushError::TokenNotRegistered;
        assert!(err.to_string().contains("not registered"));

        let err = PushError::InvalidToken("bad token".to_string());
        assert!(err.to_string().contains("bad token"));

        let err = PushError::DecryptionFailed("failed".to_string());
        assert!(err.to_string().contains("decryption"));
    }

    #[test]
    fn test_deduplication() {
        init_processed_notifications();

        let notification_id = "test_notification_id_unique_12345";

        // First check should return false (not processed) and mark it
        let already_processed = check_and_mark_processed(notification_id);
        assert!(!already_processed);

        // Second check should return true (already processed)
        let already_processed = check_and_mark_processed(notification_id);
        assert!(already_processed);
    }

    #[test]
    fn test_determine_action_disabled() {
        let payload = PushPayload {
            notification_id: "test_disabled".to_string(),
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
            notification_id: "test_expired_unique".to_string(),
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
            notification_id: format!("test_sync_{}", current_timestamp()),
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
    fn test_settings_update_and_retrieve() {
        let new_settings = PushSettings {
            enabled: true,
            show_previews: true,
            show_sender_name: true,
            sound_enabled: false,
            badge_enabled: false,
        };

        update_settings(new_settings.clone()).unwrap();

        let retrieved = get_settings();
        assert_eq!(retrieved.enabled, new_settings.enabled);
        assert_eq!(retrieved.show_previews, new_settings.show_previews);
        assert_eq!(retrieved.show_sender_name, new_settings.show_sender_name);
        assert_eq!(retrieved.sound_enabled, new_settings.sound_enabled);
        assert_eq!(retrieved.badge_enabled, new_settings.badge_enabled);
    }

    #[test]
    fn test_current_timestamp() {
        let ts = current_timestamp();
        // Should be after 2020-01-01
        assert!(ts > 1577836800);
        // Should be before 2100-01-01
        assert!(ts < 4102444800);
    }

    #[test]
    fn test_silent_push_empty() {
        let result = process_silent_push(&[]);
        assert!(result.is_ok());
        assert!(!result.unwrap());
    }

    #[test]
    fn test_silent_push_valid() {
        // Create a valid payload
        let mut payload = vec![0u8; 56];
        // Fill with some data
        payload[..16].copy_from_slice(&[1; 16]); // notification ID
        payload[16..24].copy_from_slice(&current_timestamp().to_be_bytes()); // timestamp

        let result = process_silent_push(&payload);
        assert!(result.is_ok());
        assert!(result.unwrap());
    }
}
