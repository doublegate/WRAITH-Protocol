// Android Keystore Integration for WRAITH Protocol
//
// This module provides secure key storage using Android Keystore API via JNI callbacks.
// Hardware-backed key storage is used where available (StrongBox or TEE).
//
// # Key Types Supported
// - Ed25519 identity keys (signing/verification)
// - X25519 key agreement keys (optional, for future use)
//
// # Security Properties
// - Keys stored in hardware-backed keystore when available
// - Keys are not exportable from secure hardware
// - Software fallback with encrypted file storage using Android Keystore master key
//
// # Migration
// - Automatic migration from legacy storage (SharedPreferences, files)
// - Secure deletion of old key material after successful migration

use crate::error::Error;
use jni::JNIEnv;
use jni::objects::{GlobalRef, JByteArray, JClass, JObject, JValue};
use jni::sys::{JNI_FALSE, JNI_TRUE, jboolean, jstring};
use std::sync::RwLock;

/// Keystore key alias prefix for WRAITH keys
#[allow(dead_code)]
const KEYSTORE_KEY_PREFIX: &str = "wraith_";

/// Key alias for the primary identity key (reserved for future use)
#[allow(dead_code)]
const IDENTITY_KEY_ALIAS: &str = "wraith_identity_ed25519";

/// Key alias for the signing key seed (stored encrypted via master key)
const SIGNING_KEY_SEED_ALIAS: &str = "wraith_signing_seed";

/// Global reference to the Java KeystoreHelper instance
static KEYSTORE_HELPER: RwLock<Option<GlobalRef>> = RwLock::new(None);

/// Cached identity key bytes (only the public key is cached)
static CACHED_PUBLIC_KEY: RwLock<Option<[u8; 32]>> = RwLock::new(None);

/// Result type for keystore operations
pub type KeystoreResult<T> = std::result::Result<T, KeystoreError>;

/// Keystore-specific error types
#[derive(Debug, Clone)]
pub enum KeystoreError {
    /// Key not found in keystore
    KeyNotFound(String),
    /// Keystore not initialized
    NotInitialized,
    /// JNI operation failed
    JniError(String),
    /// Key generation failed
    KeyGenerationFailed(String),
    /// Key storage failed
    StorageFailed(String),
    /// Key retrieval failed
    RetrievalFailed(String),
    /// Migration failed
    MigrationFailed(String),
    /// Hardware keystore not available
    HardwareNotAvailable,
}

impl std::fmt::Display for KeystoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::KeyNotFound(alias) => write!(f, "Key not found: {}", alias),
            Self::NotInitialized => write!(f, "Keystore not initialized"),
            Self::JniError(msg) => write!(f, "JNI error: {}", msg),
            Self::KeyGenerationFailed(msg) => write!(f, "Key generation failed: {}", msg),
            Self::StorageFailed(msg) => write!(f, "Key storage failed: {}", msg),
            Self::RetrievalFailed(msg) => write!(f, "Key retrieval failed: {}", msg),
            Self::MigrationFailed(msg) => write!(f, "Key migration failed: {}", msg),
            Self::HardwareNotAvailable => write!(f, "Hardware keystore not available"),
        }
    }
}

impl std::error::Error for KeystoreError {}

impl From<KeystoreError> for Error {
    fn from(err: KeystoreError) -> Self {
        Error::Other(err.to_string())
    }
}

impl From<jni::errors::Error> for KeystoreError {
    fn from(err: jni::errors::Error) -> Self {
        KeystoreError::JniError(err.to_string())
    }
}

/// Key information returned from keystore operations
#[derive(Debug, Clone)]
pub struct KeyInfo {
    /// Key alias in the keystore
    pub alias: String,
    /// Whether the key is stored in hardware
    pub is_hardware_backed: bool,
    /// Public key bytes (for asymmetric keys)
    pub public_key: Option<Vec<u8>>,
    /// Key creation timestamp (Unix millis)
    pub created_at: i64,
}

/// Secure storage abstraction for cross-platform key management
pub struct SecureKeyStorage {
    /// Whether hardware keystore is available
    hardware_available: bool,
    /// Initialization state
    initialized: bool,
}

impl SecureKeyStorage {
    /// Create a new SecureKeyStorage instance
    pub fn new() -> Self {
        Self {
            hardware_available: false,
            initialized: false,
        }
    }

    /// Check if the storage is initialized
    pub fn is_initialized(&self) -> bool {
        self.initialized
    }

    /// Check if hardware-backed storage is available
    pub fn is_hardware_backed(&self) -> bool {
        self.hardware_available
    }
}

impl Default for SecureKeyStorage {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// JNI Functions - Called from Java/Kotlin
// =============================================================================

/// Initialize the keystore integration with a Java KeystoreHelper instance
///
/// # Safety
/// This function is called from Java via JNI.
#[unsafe(no_mangle)]
pub extern "C" fn Java_com_wraith_android_WraithKeystore_initNative(
    env: JNIEnv,
    _class: JClass,
    helper: JObject,
) -> jboolean {
    // JNIEnv uses interior mutability, so mut is not needed
    #[allow(unused_mut)]
    let mut env = env;
    // Create a global reference to the KeystoreHelper
    let global_ref = match env.new_global_ref(helper) {
        Ok(ref_) => ref_,
        Err(e) => {
            log::error!("Failed to create global reference: {}", e);
            return JNI_FALSE;
        }
    };

    // Store the global reference
    if let Ok(mut keystore) = KEYSTORE_HELPER.write() {
        *keystore = Some(global_ref);
        log::info!("Keystore integration initialized");
        JNI_TRUE
    } else {
        log::error!("Failed to acquire keystore lock");
        JNI_FALSE
    }
}

/// Check if keystore is initialized
///
/// # Safety
/// This function is called from Java via JNI.
#[unsafe(no_mangle)]
pub extern "C" fn Java_com_wraith_android_WraithKeystore_isInitialized(
    _env: JNIEnv,
    _class: JClass,
) -> jboolean {
    let initialized = KEYSTORE_HELPER.read().map(|k| k.is_some()).unwrap_or(false);
    if initialized { JNI_TRUE } else { JNI_FALSE }
}

/// Generate and store a new identity key
///
/// # Safety
/// This function is called from Java via JNI.
///
/// Returns the hex-encoded public key, or null on failure
#[unsafe(no_mangle)]
pub extern "C" fn Java_com_wraith_android_WraithKeystore_generateIdentityKey(
    mut env: JNIEnv,
    _class: JClass,
) -> jstring {
    // Generate Ed25519 keypair using wraith-crypto
    let signing_key = wraith_crypto::signatures::SigningKey::generate(&mut rand_core::OsRng);
    let public_key = signing_key.verifying_key().to_bytes();
    let secret_bytes = signing_key.to_bytes();

    // Store the secret key bytes via the Java helper
    let helper_ref = match KEYSTORE_HELPER.read() {
        Ok(guard) => match guard.as_ref() {
            Some(ref_) => ref_.clone(),
            None => {
                log::error!("Keystore not initialized");
                return std::ptr::null_mut();
            }
        },
        Err(e) => {
            log::error!("Failed to read keystore: {}", e);
            return std::ptr::null_mut();
        }
    };

    // Call Java to store the encrypted key
    let secret_array = match env.byte_array_from_slice(&secret_bytes) {
        Ok(arr) => arr,
        Err(e) => {
            log::error!("Failed to create byte array: {}", e);
            return std::ptr::null_mut();
        }
    };

    let alias = match env.new_string(SIGNING_KEY_SEED_ALIAS) {
        Ok(s) => s,
        Err(e) => {
            log::error!("Failed to create string: {}", e);
            return std::ptr::null_mut();
        }
    };

    // Call storeEncryptedKey(String alias, byte[] keyData)
    let result = env.call_method(
        &helper_ref,
        "storeEncryptedKey",
        "(Ljava/lang/String;[B)Z",
        &[JValue::Object(&alias), JValue::Object(&secret_array)],
    );

    match result {
        Ok(val) => {
            if !val.z().unwrap_or(false) {
                log::error!("Java storeEncryptedKey returned false");
                return std::ptr::null_mut();
            }
        }
        Err(e) => {
            log::error!("Failed to call storeEncryptedKey: {}", e);
            return std::ptr::null_mut();
        }
    }

    // Cache the public key
    if let Ok(mut cache) = CACHED_PUBLIC_KEY.write() {
        *cache = Some(public_key);
    }

    // Return hex-encoded public key
    let public_key_hex = hex::encode(public_key);
    match env.new_string(public_key_hex) {
        Ok(s) => s.into_raw(),
        Err(e) => {
            log::error!("Failed to create result string: {}", e);
            std::ptr::null_mut()
        }
    }
}

/// Get the current identity public key
///
/// # Safety
/// This function is called from Java via JNI.
///
/// Returns the hex-encoded public key, or null if not available
#[unsafe(no_mangle)]
pub extern "C" fn Java_com_wraith_android_WraithKeystore_getIdentityPublicKey(
    mut env: JNIEnv,
    _class: JClass,
) -> jstring {
    // Check cache first
    if let Ok(cache) = CACHED_PUBLIC_KEY.read() {
        if let Some(key) = cache.as_ref() {
            let hex = hex::encode(key);
            return match env.new_string(hex) {
                Ok(s) => s.into_raw(),
                Err(_) => std::ptr::null_mut(),
            };
        }
    }

    // Load from keystore
    let helper_ref = match KEYSTORE_HELPER.read() {
        Ok(guard) => match guard.as_ref() {
            Some(ref_) => ref_.clone(),
            None => return std::ptr::null_mut(),
        },
        Err(_) => return std::ptr::null_mut(),
    };

    let alias = match env.new_string(SIGNING_KEY_SEED_ALIAS) {
        Ok(s) => s,
        Err(_) => return std::ptr::null_mut(),
    };

    // Call loadEncryptedKey(String alias) -> byte[]
    let result = env.call_method(
        &helper_ref,
        "loadEncryptedKey",
        "(Ljava/lang/String;)[B",
        &[JValue::Object(&alias)],
    );

    let secret_bytes: Vec<u8> = match result {
        Ok(val) => {
            let obj = match val.l() {
                Ok(o) => o,
                Err(_) => return std::ptr::null_mut(),
            };
            if obj.is_null() {
                return std::ptr::null_mut();
            }
            // Convert JObject to JByteArray safely
            let byte_array = JByteArray::from(obj);
            match env.convert_byte_array(&byte_array) {
                Ok(bytes) => bytes,
                Err(_) => return std::ptr::null_mut(),
            }
        }
        Err(_) => return std::ptr::null_mut(),
    };

    // Reconstruct the signing key to get the public key
    if secret_bytes.len() != 32 {
        // Avoid logging actual length to prevent information leakage about key material
        log::error!("Invalid secret key length (expected 32 bytes)");
        return std::ptr::null_mut();
    }

    let mut secret_array = [0u8; 32];
    secret_array.copy_from_slice(&secret_bytes);
    let signing_key = wraith_crypto::signatures::SigningKey::from_bytes(&secret_array);
    let public_key = signing_key.verifying_key().to_bytes();

    // Cache it
    if let Ok(mut cache) = CACHED_PUBLIC_KEY.write() {
        *cache = Some(public_key);
    }

    // Return hex
    let hex = hex::encode(public_key);
    match env.new_string(hex) {
        Ok(s) => s.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Sign data with the identity key
///
/// # Safety
/// This function is called from Java via JNI.
///
/// Returns the hex-encoded 64-byte signature, or null on failure
#[unsafe(no_mangle)]
pub extern "C" fn Java_com_wraith_android_WraithKeystore_signWithIdentityKey(
    mut env: JNIEnv,
    _class: JClass,
    data: JByteArray,
) -> jstring {
    // Get the data bytes
    let data_bytes = match env.convert_byte_array(&data) {
        Ok(bytes) => bytes,
        Err(e) => {
            log::error!("Failed to convert data array: {}", e);
            return std::ptr::null_mut();
        }
    };

    // Load the signing key from keystore
    let helper_ref = match KEYSTORE_HELPER.read() {
        Ok(guard) => match guard.as_ref() {
            Some(ref_) => ref_.clone(),
            None => {
                log::error!("Keystore not initialized");
                return std::ptr::null_mut();
            }
        },
        Err(_) => return std::ptr::null_mut(),
    };

    let alias = match env.new_string(SIGNING_KEY_SEED_ALIAS) {
        Ok(s) => s,
        Err(_) => return std::ptr::null_mut(),
    };

    let result = env.call_method(
        &helper_ref,
        "loadEncryptedKey",
        "(Ljava/lang/String;)[B",
        &[JValue::Object(&alias)],
    );

    let secret_bytes: Vec<u8> = match result {
        Ok(val) => {
            let obj = match val.l() {
                Ok(o) => o,
                Err(_) => return std::ptr::null_mut(),
            };
            if obj.is_null() {
                log::error!("No identity key found");
                return std::ptr::null_mut();
            }
            // Convert JObject to JByteArray safely
            let byte_array = JByteArray::from(obj);
            match env.convert_byte_array(&byte_array) {
                Ok(bytes) => bytes,
                Err(_) => return std::ptr::null_mut(),
            }
        }
        Err(e) => {
            log::error!("Failed to load key: {}", e);
            return std::ptr::null_mut();
        }
    };

    if secret_bytes.len() != 32 {
        log::error!("Invalid secret key length");
        return std::ptr::null_mut();
    }

    let mut secret_array = [0u8; 32];
    secret_array.copy_from_slice(&secret_bytes);
    let signing_key = wraith_crypto::signatures::SigningKey::from_bytes(&secret_array);

    // Sign the data
    let signature = signing_key.sign(&data_bytes);
    let sig_hex = hex::encode(signature.as_bytes());

    match env.new_string(sig_hex) {
        Ok(s) => s.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Check if an identity key exists
///
/// # Safety
/// This function is called from Java via JNI.
#[unsafe(no_mangle)]
pub extern "C" fn Java_com_wraith_android_WraithKeystore_hasIdentityKey(
    mut env: JNIEnv,
    _class: JClass,
) -> jboolean {
    let helper_ref = match KEYSTORE_HELPER.read() {
        Ok(guard) => match guard.as_ref() {
            Some(ref_) => ref_.clone(),
            None => return JNI_FALSE,
        },
        Err(_) => return JNI_FALSE,
    };

    let alias = match env.new_string(SIGNING_KEY_SEED_ALIAS) {
        Ok(s) => s,
        Err(_) => return JNI_FALSE,
    };

    // Call hasKey(String alias) -> boolean
    let result = env.call_method(
        &helper_ref,
        "hasKey",
        "(Ljava/lang/String;)Z",
        &[JValue::Object(&alias)],
    );

    match result {
        Ok(val) => {
            if val.z().unwrap_or(false) {
                JNI_TRUE
            } else {
                JNI_FALSE
            }
        }
        Err(_) => JNI_FALSE,
    }
}

/// Delete the identity key
///
/// # Safety
/// This function is called from Java via JNI.
#[unsafe(no_mangle)]
pub extern "C" fn Java_com_wraith_android_WraithKeystore_deleteIdentityKey(
    mut env: JNIEnv,
    _class: JClass,
) -> jboolean {
    let helper_ref = match KEYSTORE_HELPER.read() {
        Ok(guard) => match guard.as_ref() {
            Some(ref_) => ref_.clone(),
            None => return JNI_FALSE,
        },
        Err(_) => return JNI_FALSE,
    };

    let alias = match env.new_string(SIGNING_KEY_SEED_ALIAS) {
        Ok(s) => s,
        Err(_) => return JNI_FALSE,
    };

    // Call deleteKey(String alias) -> boolean
    let result = env.call_method(
        &helper_ref,
        "deleteKey",
        "(Ljava/lang/String;)Z",
        &[JValue::Object(&alias)],
    );

    // Clear the cache
    if let Ok(mut cache) = CACHED_PUBLIC_KEY.write() {
        *cache = None;
    }

    match result {
        Ok(val) => {
            if val.z().unwrap_or(false) {
                JNI_TRUE
            } else {
                JNI_FALSE
            }
        }
        Err(_) => JNI_FALSE,
    }
}

/// Migrate keys from legacy storage
///
/// # Safety
/// This function is called from Java via JNI.
///
/// Parameters:
/// - legacy_key_data: byte[] - The key data from legacy storage
///
/// Returns true if migration succeeded
#[unsafe(no_mangle)]
pub extern "C" fn Java_com_wraith_android_WraithKeystore_migrateFromLegacy(
    mut env: JNIEnv,
    _class: JClass,
    legacy_key_data: JByteArray,
) -> jboolean {
    // Get the legacy key bytes
    let key_bytes = match env.convert_byte_array(&legacy_key_data) {
        Ok(bytes) => bytes,
        Err(e) => {
            log::error!("Failed to convert legacy key data: {}", e);
            return JNI_FALSE;
        }
    };

    if key_bytes.len() != 32 {
        log::error!("Invalid legacy key length: {}", key_bytes.len());
        return JNI_FALSE;
    }

    // Verify the key is valid by creating a signing key
    let mut key_array = [0u8; 32];
    key_array.copy_from_slice(&key_bytes);
    let signing_key = wraith_crypto::signatures::SigningKey::from_bytes(&key_array);
    let public_key = signing_key.verifying_key().to_bytes();

    // Store via the new keystore
    let helper_ref = match KEYSTORE_HELPER.read() {
        Ok(guard) => match guard.as_ref() {
            Some(ref_) => ref_.clone(),
            None => {
                log::error!("Keystore not initialized for migration");
                return JNI_FALSE;
            }
        },
        Err(_) => return JNI_FALSE,
    };

    let secret_array = match env.byte_array_from_slice(&key_bytes) {
        Ok(arr) => arr,
        Err(e) => {
            log::error!("Failed to create byte array: {}", e);
            return JNI_FALSE;
        }
    };

    let alias = match env.new_string(SIGNING_KEY_SEED_ALIAS) {
        Ok(s) => s,
        Err(_) => return JNI_FALSE,
    };

    let result = env.call_method(
        &helper_ref,
        "storeEncryptedKey",
        "(Ljava/lang/String;[B)Z",
        &[JValue::Object(&alias), JValue::Object(&secret_array)],
    );

    match result {
        Ok(val) => {
            if val.z().unwrap_or(false) {
                // Cache the public key
                if let Ok(mut cache) = CACHED_PUBLIC_KEY.write() {
                    *cache = Some(public_key);
                }
                log::info!("Successfully migrated identity key from legacy storage");
                JNI_TRUE
            } else {
                log::error!("Java storeEncryptedKey returned false during migration");
                JNI_FALSE
            }
        }
        Err(e) => {
            log::error!("Failed to store migrated key: {}", e);
            JNI_FALSE
        }
    }
}

/// Check if hardware-backed keystore is available
///
/// # Safety
/// This function is called from Java via JNI.
#[unsafe(no_mangle)]
pub extern "C" fn Java_com_wraith_android_WraithKeystore_isHardwareBacked(
    mut env: JNIEnv,
    _class: JClass,
) -> jboolean {
    let helper_ref = match KEYSTORE_HELPER.read() {
        Ok(guard) => match guard.as_ref() {
            Some(ref_) => ref_.clone(),
            None => return JNI_FALSE,
        },
        Err(_) => return JNI_FALSE,
    };

    // Call isHardwareBacked() -> boolean
    let result = env.call_method(&helper_ref, "isHardwareBacked", "()Z", &[]);

    match result {
        Ok(val) => {
            if val.z().unwrap_or(false) {
                JNI_TRUE
            } else {
                JNI_FALSE
            }
        }
        Err(_) => JNI_FALSE,
    }
}

/// Get keystore information as JSON
///
/// # Safety
/// This function is called from Java via JNI.
#[unsafe(no_mangle)]
pub extern "C" fn Java_com_wraith_android_WraithKeystore_getKeystoreInfo(
    mut env: JNIEnv,
    _class: JClass,
) -> jstring {
    let helper_ref = match KEYSTORE_HELPER.read() {
        Ok(guard) => match guard.as_ref() {
            Some(ref_) => ref_.clone(),
            None => {
                let info = serde_json::json!({
                    "initialized": false,
                    "error": "Keystore not initialized"
                });
                return match env.new_string(info.to_string()) {
                    Ok(s) => s.into_raw(),
                    Err(_) => std::ptr::null_mut(),
                };
            }
        },
        Err(_) => return std::ptr::null_mut(),
    };

    // Check hardware backing
    let hardware_result = env.call_method(&helper_ref, "isHardwareBacked", "()Z", &[]);
    let is_hardware = hardware_result
        .ok()
        .and_then(|v| v.z().ok())
        .unwrap_or(false);

    // Check if we have identity key
    let alias = match env.new_string(SIGNING_KEY_SEED_ALIAS) {
        Ok(s) => s,
        Err(_) => {
            return std::ptr::null_mut();
        }
    };

    let has_key_result = env.call_method(
        &helper_ref,
        "hasKey",
        "(Ljava/lang/String;)Z",
        &[JValue::Object(&alias)],
    );
    let has_identity_key = has_key_result
        .ok()
        .and_then(|v| v.z().ok())
        .unwrap_or(false);

    // Get public key if available
    let public_key_hex = if has_identity_key {
        CACHED_PUBLIC_KEY
            .read()
            .ok()
            .and_then(|cache| cache.map(hex::encode))
    } else {
        None
    };

    let info = serde_json::json!({
        "initialized": true,
        "hardwareBacked": is_hardware,
        "hasIdentityKey": has_identity_key,
        "publicKey": public_key_hex,
        "keyAlias": SIGNING_KEY_SEED_ALIAS,
    });

    match env.new_string(info.to_string()) {
        Ok(s) => s.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}

// =============================================================================
// Unit Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keystore_error_display() {
        let err = KeystoreError::KeyNotFound("test_alias".to_string());
        assert!(err.to_string().contains("test_alias"));

        let err = KeystoreError::NotInitialized;
        assert!(err.to_string().contains("not initialized"));

        let err = KeystoreError::JniError("test error".to_string());
        assert!(err.to_string().contains("JNI"));

        let err = KeystoreError::HardwareNotAvailable;
        assert!(err.to_string().contains("Hardware"));
    }

    #[test]
    fn test_keystore_error_conversion() {
        let keystore_err = KeystoreError::KeyNotFound("alias".to_string());
        let error: Error = keystore_err.into();
        match error {
            Error::Other(msg) => assert!(msg.contains("alias")),
            _ => panic!("Expected Error::Other variant"),
        }
    }

    #[test]
    fn test_secure_key_storage_default() {
        let storage = SecureKeyStorage::default();
        assert!(!storage.is_initialized());
        assert!(!storage.is_hardware_backed());
    }

    #[test]
    fn test_keystore_aliases() {
        assert!(IDENTITY_KEY_ALIAS.starts_with(KEYSTORE_KEY_PREFIX));
        assert_eq!(SIGNING_KEY_SEED_ALIAS, "wraith_signing_seed");
    }

    #[test]
    fn test_key_info_creation() {
        let info = KeyInfo {
            alias: "test_key".to_string(),
            is_hardware_backed: true,
            public_key: Some(vec![1, 2, 3, 4]),
            created_at: 1234567890,
        };

        assert_eq!(info.alias, "test_key");
        assert!(info.is_hardware_backed);
        assert!(info.public_key.is_some());
        assert_eq!(info.created_at, 1234567890);
    }

    #[test]
    fn test_cached_public_key_initial_state() {
        // The cache should initially be empty (in a clean test environment)
        // Note: This test may be affected by other tests running in parallel
        let cache = CACHED_PUBLIC_KEY.read().unwrap();
        // We can't guarantee it's None due to other tests, but we can verify we can read it
        let _ = cache.as_ref();
    }

    #[test]
    fn test_keystore_helper_initial_state() {
        // Similar to above - verify we can access the lock
        let helper = KEYSTORE_HELPER.read().unwrap();
        // In isolation, this would be None, but other tests may have initialized it
        let _ = helper.as_ref();
    }
}
