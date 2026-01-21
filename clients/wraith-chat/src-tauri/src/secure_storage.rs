// Secure Key Storage for WRAITH-Chat
//
// Uses platform-native secure storage:
// - Linux: Secret Service (libsecret/GNOME Keyring)
// - macOS: Keychain
// - Windows: Credential Manager

use rand::RngCore;
use thiserror::Error;

/// Service name for keyring entries
const SERVICE_NAME: &str = "wraith-chat";

/// Key name for the database encryption key
const DB_KEY_NAME: &str = "database-encryption-key";

/// Secure storage errors
#[derive(Debug, Error)]
pub enum SecureStorageError {
    #[error("Keyring error: {0}")]
    Keyring(String),

    #[error("Key not found")]
    KeyNotFound,

    #[error("Failed to generate key: {0}")]
    KeyGeneration(String),

    #[error("Base64 decode error: {0}")]
    Base64Decode(#[from] base64::DecodeError),

    #[error("Invalid key length: expected {expected}, got {actual}")]
    InvalidKeyLength { expected: usize, actual: usize },
}

/// Secure storage manager for WRAITH-Chat
pub struct SecureStorage {
    service: String,
}

impl SecureStorage {
    /// Create a new SecureStorage instance
    pub fn new() -> Self {
        Self {
            service: SERVICE_NAME.to_string(),
        }
    }

    /// Create a SecureStorage instance with a custom service name (for testing)
    #[allow(dead_code)]
    pub fn with_service(service: impl Into<String>) -> Self {
        Self {
            service: service.into(),
        }
    }

    /// Get or create the database encryption key
    ///
    /// If a key already exists in secure storage, it will be retrieved.
    /// If no key exists, a new 32-byte random key will be generated and stored.
    ///
    /// Returns the key as a base64-encoded string (suitable for SQLCipher).
    pub fn get_or_create_db_key(&self) -> Result<String, SecureStorageError> {
        // Try to get existing key first
        match self.get_key(DB_KEY_NAME) {
            Ok(key) => {
                log::debug!("Retrieved existing database encryption key from secure storage");
                Ok(key)
            }
            Err(SecureStorageError::KeyNotFound) => {
                // Generate and store new key
                log::info!("No existing database key found, generating new key");
                let key = self.generate_and_store_db_key()?;
                Ok(key)
            }
            Err(e) => Err(e),
        }
    }

    /// Get a key from secure storage
    pub fn get_key(&self, key_name: &str) -> Result<String, SecureStorageError> {
        let entry = keyring::Entry::new(&self.service, key_name)
            .map_err(|e| SecureStorageError::Keyring(e.to_string()))?;

        match entry.get_password() {
            Ok(password) => Ok(password),
            Err(keyring::Error::NoEntry) => Err(SecureStorageError::KeyNotFound),
            Err(e) => Err(SecureStorageError::Keyring(e.to_string())),
        }
    }

    /// Store a key in secure storage
    pub fn store_key(&self, key_name: &str, key_value: &str) -> Result<(), SecureStorageError> {
        let entry = keyring::Entry::new(&self.service, key_name)
            .map_err(|e| SecureStorageError::Keyring(e.to_string()))?;

        entry
            .set_password(key_value)
            .map_err(|e| SecureStorageError::Keyring(e.to_string()))?;

        log::debug!("Stored key '{}' in secure storage", key_name);
        Ok(())
    }

    /// Delete a key from secure storage
    #[allow(dead_code)]
    pub fn delete_key(&self, key_name: &str) -> Result<(), SecureStorageError> {
        let entry = keyring::Entry::new(&self.service, key_name)
            .map_err(|e| SecureStorageError::Keyring(e.to_string()))?;

        match entry.delete_credential() {
            Ok(()) => {
                log::debug!("Deleted key '{}' from secure storage", key_name);
                Ok(())
            }
            Err(keyring::Error::NoEntry) => Err(SecureStorageError::KeyNotFound),
            Err(e) => Err(SecureStorageError::Keyring(e.to_string())),
        }
    }

    /// Generate a new database encryption key and store it
    fn generate_and_store_db_key(&self) -> Result<String, SecureStorageError> {
        // Generate 32 random bytes
        let mut key_bytes = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut key_bytes);

        // Encode as base64 for storage
        use base64::Engine;
        let key_b64 = base64::engine::general_purpose::STANDARD.encode(key_bytes);

        // Store in keyring
        self.store_key(DB_KEY_NAME, &key_b64)?;

        // Zeroize key bytes in memory
        key_bytes.fill(0);

        log::info!("Generated and stored new database encryption key");
        Ok(key_b64)
    }

    /// Decode a base64-encoded key to bytes
    #[allow(dead_code)]
    pub fn decode_key(key_b64: &str) -> Result<[u8; 32], SecureStorageError> {
        use base64::Engine;
        let bytes = base64::engine::general_purpose::STANDARD.decode(key_b64)?;

        if bytes.len() != 32 {
            return Err(SecureStorageError::InvalidKeyLength {
                expected: 32,
                actual: bytes.len(),
            });
        }

        let mut key = [0u8; 32];
        key.copy_from_slice(&bytes);
        Ok(key)
    }

    /// Check if secure storage is available on this platform
    ///
    /// Returns true if secure storage operations are likely to succeed.
    pub fn is_available(&self) -> bool {
        // Try to create an entry and check for errors
        match keyring::Entry::new(&self.service, "availability-check") {
            Ok(_) => true,
            Err(_) => {
                log::warn!("Secure storage may not be available on this system");
                false
            }
        }
    }
}

impl Default for SecureStorage {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_key() {
        use base64::Engine;
        // Generate a test key
        let key_bytes = [0x42u8; 32];
        let key_b64 = base64::engine::general_purpose::STANDARD.encode(key_bytes);

        // Decode it
        let decoded = SecureStorage::decode_key(&key_b64).unwrap();
        assert_eq!(decoded, key_bytes);
    }

    #[test]
    fn test_decode_key_invalid_length() {
        use base64::Engine;
        // Too short
        let short_b64 = base64::engine::general_purpose::STANDARD.encode([0x42u8; 16]);
        let result = SecureStorage::decode_key(&short_b64);
        assert!(matches!(
            result,
            Err(SecureStorageError::InvalidKeyLength { .. })
        ));
    }

    #[test]
    fn test_secure_storage_creation() {
        let storage = SecureStorage::new();
        assert_eq!(storage.service, SERVICE_NAME);

        let custom = SecureStorage::with_service("custom-service");
        assert_eq!(custom.service, "custom-service");
    }
}
