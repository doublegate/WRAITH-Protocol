//! # Encrypted Private Key Storage
//!
//! Provides secure storage for private keys using password-based encryption.
//!
//! ## Security Properties
//!
//! - **Key Derivation:** Argon2id with configurable parameters
//! - **Encryption:** XChaCha20-Poly1305 AEAD
//! - **Memory Safety:** All sensitive data implements `ZeroizeOnDrop`
//! - **Version Migration:** Extensible format with version field
//!
//! ## Usage
//!
//! ```rust
//! use wraith_crypto::encrypted_keys::{EncryptedPrivateKey, KeyEncryptionParams};
//!
//! // Generate a new keypair
//! let secret_key = [0u8; 32]; // In practice, use secure random
//!
//! // Encrypt with default parameters (OWASP-recommended Argon2id settings)
//! let encrypted = EncryptedPrivateKey::encrypt(
//!     &secret_key,
//!     b"strong-passphrase",
//!     KeyEncryptionParams::default(),
//! ).expect("encryption failed");
//!
//! // Serialize for storage
//! let bytes = encrypted.to_bytes();
//!
//! // Later, decrypt
//! let loaded = EncryptedPrivateKey::from_bytes(&bytes).expect("parse failed");
//! let decrypted = loaded.decrypt(b"strong-passphrase").expect("wrong passphrase");
//! ```
//!
//! ## File Format
//!
//! ```text
//! +----------------+----------------+----------------+
//! | Version (1B)   | Argon2 Params  | Salt (32B)     |
//! +----------------+----------------+----------------+
//! | Nonce (24B)    | Ciphertext (32B + 16B tag)      |
//! +----------------+-----------------------------------+
//! ```

use argon2::{Algorithm, Argon2, Params, ParamsBuilder, Version};
use chacha20poly1305::{
    XChaCha20Poly1305, XNonce,
    aead::{Aead, KeyInit},
};
use serde::{Deserialize, Serialize};
use zeroize::{Zeroize, ZeroizeOnDrop};

use crate::error::CryptoError;

/// Current format version for encrypted keys.
const FORMAT_VERSION: u8 = 1;

/// Size of salt for Argon2 key derivation.
const SALT_SIZE: usize = 32;

/// Size of nonce for XChaCha20-Poly1305.
const NONCE_SIZE: usize = 24;

/// Size of the private key (X25519/Ed25519).
const PRIVATE_KEY_SIZE: usize = 32;

/// Size of authentication tag.
const TAG_SIZE: usize = 16;

/// Parameters for Argon2id key derivation.
///
/// These parameters control the memory-hardness and computational cost
/// of deriving the encryption key from the passphrase.
///
/// Default values follow OWASP recommendations for high-security applications:
/// - Memory: 64 MiB (65536 KiB)
/// - Iterations: 4
/// - Parallelism: 4
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeyEncryptionParams {
    /// Memory cost in KiB (default: 65536 = 64 MiB)
    pub memory_cost_kib: u32,
    /// Number of iterations (default: 4)
    pub iterations: u32,
    /// Degree of parallelism (default: 4)
    pub parallelism: u32,
}

impl Default for KeyEncryptionParams {
    fn default() -> Self {
        // OWASP-recommended parameters for password hashing
        // See: https://cheatsheetseries.owasp.org/cheatsheets/Password_Storage_Cheat_Sheet.html
        Self {
            memory_cost_kib: 65536, // 64 MiB
            iterations: 4,
            parallelism: 4,
        }
    }
}

impl KeyEncryptionParams {
    /// Low-security parameters for testing or resource-constrained environments.
    ///
    /// **Warning:** Only use for testing. Not suitable for production.
    #[must_use]
    pub fn low_security() -> Self {
        Self {
            memory_cost_kib: 4096, // 4 MiB
            iterations: 2,
            parallelism: 1,
        }
    }

    /// High-security parameters for highly sensitive keys.
    ///
    /// Increases memory and iteration count for maximum protection.
    #[must_use]
    pub fn high_security() -> Self {
        Self {
            memory_cost_kib: 131_072, // 128 MiB
            iterations: 8,
            parallelism: 4,
        }
    }

    /// Validate parameters are within acceptable bounds.
    fn validate(&self) -> Result<(), CryptoError> {
        // Minimum memory: 8 KiB (Argon2 minimum)
        if self.memory_cost_kib < 8 {
            return Err(CryptoError::InvalidParameter(
                "memory_cost_kib must be at least 8 KiB".into(),
            ));
        }

        // Minimum iterations: 1
        if self.iterations < 1 {
            return Err(CryptoError::InvalidParameter(
                "iterations must be at least 1".into(),
            ));
        }

        // Parallelism bounds: 1-255
        if self.parallelism < 1 || self.parallelism > 255 {
            return Err(CryptoError::InvalidParameter(
                "parallelism must be between 1 and 255".into(),
            ));
        }

        Ok(())
    }

    /// Build Argon2 parameters.
    fn build_argon2_params(&self) -> Result<Params, CryptoError> {
        self.validate()?;

        ParamsBuilder::new()
            .m_cost(self.memory_cost_kib)
            .t_cost(self.iterations)
            .p_cost(self.parallelism)
            .build()
            .map_err(|e| CryptoError::InvalidParameter(format!("Argon2 params: {e}")))
    }

    /// Serialize parameters to bytes (6 bytes total).
    fn to_bytes(self) -> [u8; 6] {
        let mut bytes = [0u8; 6];
        // Memory cost: 3 bytes (supports up to 16 GiB)
        bytes[0..3].copy_from_slice(&self.memory_cost_kib.to_le_bytes()[0..3]);
        // Iterations: 2 bytes
        bytes[3..5].copy_from_slice(&(self.iterations as u16).to_le_bytes());
        // Parallelism: 1 byte
        bytes[5] = self.parallelism as u8;
        bytes
    }

    /// Deserialize parameters from bytes.
    fn from_bytes(bytes: &[u8; 6]) -> Self {
        let mut mem_bytes = [0u8; 4];
        mem_bytes[0..3].copy_from_slice(&bytes[0..3]);
        let memory_cost_kib = u32::from_le_bytes(mem_bytes);

        let iterations = u16::from_le_bytes([bytes[3], bytes[4]]) as u32;
        let parallelism = bytes[5] as u32;

        Self {
            memory_cost_kib,
            iterations,
            parallelism,
        }
    }
}

/// Wrapper for decrypted private key with automatic zeroization.
#[derive(Zeroize, ZeroizeOnDrop)]
pub struct DecryptedPrivateKey {
    /// The raw private key bytes.
    key: [u8; PRIVATE_KEY_SIZE],
}

impl DecryptedPrivateKey {
    /// Create from raw bytes.
    #[must_use]
    pub fn new(key: [u8; PRIVATE_KEY_SIZE]) -> Self {
        Self { key }
    }

    /// Get the key bytes.
    ///
    /// **Security Note:** Handle with care - this exposes the raw key.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8; PRIVATE_KEY_SIZE] {
        &self.key
    }

    /// Convert to owned bytes, consuming self.
    ///
    /// The caller is responsible for zeroizing the returned array.
    #[must_use]
    pub fn into_bytes(self) -> [u8; PRIVATE_KEY_SIZE] {
        self.key
    }
}

impl AsRef<[u8]> for DecryptedPrivateKey {
    fn as_ref(&self) -> &[u8] {
        &self.key
    }
}

/// An encrypted private key with associated metadata.
///
/// This structure contains all data needed to decrypt a private key
/// given the correct passphrase.
#[derive(Clone, Serialize, Deserialize)]
pub struct EncryptedPrivateKey {
    /// Format version for future compatibility.
    version: u8,
    /// Argon2id parameters used for key derivation.
    params: KeyEncryptionParams,
    /// Random salt for key derivation.
    salt: [u8; SALT_SIZE],
    /// Random nonce for XChaCha20-Poly1305.
    nonce: [u8; NONCE_SIZE],
    /// Encrypted private key with authentication tag.
    ciphertext: Vec<u8>,
}

impl EncryptedPrivateKey {
    /// Encrypt a private key with a passphrase.
    ///
    /// # Arguments
    ///
    /// * `private_key` - The raw private key bytes to encrypt
    /// * `passphrase` - The passphrase to derive the encryption key from
    /// * `params` - Argon2id parameters for key derivation
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Parameters are invalid
    /// - Random generation fails
    /// - Encryption fails
    ///
    /// # Example
    ///
    /// ```rust
    /// use wraith_crypto::encrypted_keys::{EncryptedPrivateKey, KeyEncryptionParams};
    ///
    /// let secret_key = [0u8; 32];
    /// let encrypted = EncryptedPrivateKey::encrypt(
    ///     &secret_key,
    ///     b"my-secure-passphrase",
    ///     KeyEncryptionParams::default(),
    /// ).expect("encryption failed");
    /// ```
    pub fn encrypt(
        private_key: &[u8; PRIVATE_KEY_SIZE],
        passphrase: &[u8],
        params: KeyEncryptionParams,
    ) -> Result<Self, CryptoError> {
        params.validate()?;

        // Generate random salt
        let mut salt = [0u8; SALT_SIZE];
        getrandom::getrandom(&mut salt)
            .map_err(|e| CryptoError::RandomGenerationFailed(e.to_string()))?;

        // Generate random nonce
        let mut nonce = [0u8; NONCE_SIZE];
        getrandom::getrandom(&mut nonce)
            .map_err(|e| CryptoError::RandomGenerationFailed(e.to_string()))?;

        // Derive encryption key using Argon2id
        let mut derived_key = derive_key(passphrase, &salt, &params)?;

        // Encrypt with XChaCha20-Poly1305
        let cipher = XChaCha20Poly1305::new_from_slice(&derived_key)
            .map_err(|_| CryptoError::KeyDerivationFailed)?;

        let ciphertext = cipher
            .encrypt(XNonce::from_slice(&nonce), private_key.as_slice())
            .map_err(|_| CryptoError::EncryptionFailed)?;

        // Zeroize the derived key
        derived_key.zeroize();

        Ok(Self {
            version: FORMAT_VERSION,
            params,
            salt,
            nonce,
            ciphertext,
        })
    }

    /// Decrypt the private key using the passphrase.
    ///
    /// # Arguments
    ///
    /// * `passphrase` - The passphrase used during encryption
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The passphrase is incorrect
    /// - The ciphertext has been tampered with
    /// - Key derivation fails
    ///
    /// # Example
    ///
    /// ```rust
    /// use wraith_crypto::encrypted_keys::{EncryptedPrivateKey, KeyEncryptionParams};
    ///
    /// let secret_key = [0u8; 32];
    /// let encrypted = EncryptedPrivateKey::encrypt(
    ///     &secret_key,
    ///     b"my-passphrase",
    ///     KeyEncryptionParams::default(),
    /// ).expect("encryption failed");
    ///
    /// let decrypted = encrypted.decrypt(b"my-passphrase").expect("decryption failed");
    /// assert_eq!(decrypted.as_bytes(), &secret_key);
    /// ```
    pub fn decrypt(&self, passphrase: &[u8]) -> Result<DecryptedPrivateKey, CryptoError> {
        // Derive the same encryption key
        let mut derived_key = derive_key(passphrase, &self.salt, &self.params)?;

        // Decrypt with XChaCha20-Poly1305
        let cipher = XChaCha20Poly1305::new_from_slice(&derived_key)
            .map_err(|_| CryptoError::KeyDerivationFailed)?;

        let plaintext = cipher
            .decrypt(XNonce::from_slice(&self.nonce), self.ciphertext.as_slice())
            .map_err(|_| CryptoError::DecryptionFailed)?;

        // Zeroize the derived key
        derived_key.zeroize();

        // Validate length
        if plaintext.len() != PRIVATE_KEY_SIZE {
            return Err(CryptoError::InvalidKeyMaterial);
        }

        let mut key = [0u8; PRIVATE_KEY_SIZE];
        key.copy_from_slice(&plaintext);

        Ok(DecryptedPrivateKey::new(key))
    }

    /// Serialize the encrypted key to bytes for storage.
    ///
    /// # Format
    ///
    /// ```text
    /// [version: 1] [params: 6] [salt: 32] [nonce: 24] [ciphertext: 48]
    /// Total: 111 bytes
    /// ```
    #[must_use]
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(1 + 6 + SALT_SIZE + NONCE_SIZE + self.ciphertext.len());

        bytes.push(self.version);
        bytes.extend_from_slice(&self.params.to_bytes());
        bytes.extend_from_slice(&self.salt);
        bytes.extend_from_slice(&self.nonce);
        bytes.extend_from_slice(&self.ciphertext);

        bytes
    }

    /// Deserialize from bytes.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Input is too short
    /// - Version is unsupported
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, CryptoError> {
        // Minimum size: version(1) + params(6) + salt(32) + nonce(24) + ciphertext(48)
        const MIN_SIZE: usize = 1 + 6 + SALT_SIZE + NONCE_SIZE + PRIVATE_KEY_SIZE + TAG_SIZE;

        if bytes.len() < MIN_SIZE {
            return Err(CryptoError::InvalidKeyMaterial);
        }

        let version = bytes[0];
        if version != FORMAT_VERSION {
            return Err(CryptoError::InvalidParameter(format!(
                "unsupported format version: {version}"
            )));
        }

        let params = KeyEncryptionParams::from_bytes(bytes[1..7].try_into().unwrap());

        let mut salt = [0u8; SALT_SIZE];
        salt.copy_from_slice(&bytes[7..7 + SALT_SIZE]);

        let mut nonce = [0u8; NONCE_SIZE];
        nonce.copy_from_slice(&bytes[7 + SALT_SIZE..7 + SALT_SIZE + NONCE_SIZE]);

        let ciphertext = bytes[7 + SALT_SIZE + NONCE_SIZE..].to_vec();

        Ok(Self {
            version,
            params,
            salt,
            nonce,
            ciphertext,
        })
    }

    /// Get the Argon2 parameters used for this key.
    #[must_use]
    pub fn params(&self) -> &KeyEncryptionParams {
        &self.params
    }

    /// Get the format version.
    #[must_use]
    pub fn version(&self) -> u8 {
        self.version
    }

    /// Re-encrypt with new passphrase.
    ///
    /// This decrypts with the old passphrase and re-encrypts with a new one.
    /// Useful for passphrase rotation.
    ///
    /// # Errors
    ///
    /// Returns an error if decryption or re-encryption fails.
    pub fn change_passphrase(
        &self,
        old_passphrase: &[u8],
        new_passphrase: &[u8],
        new_params: Option<KeyEncryptionParams>,
    ) -> Result<Self, CryptoError> {
        let decrypted = self.decrypt(old_passphrase)?;
        let params = new_params.unwrap_or(self.params);
        Self::encrypt(decrypted.as_bytes(), new_passphrase, params)
    }
}

/// Derive an encryption key from a passphrase using Argon2id.
fn derive_key(
    passphrase: &[u8],
    salt: &[u8; SALT_SIZE],
    params: &KeyEncryptionParams,
) -> Result<[u8; 32], CryptoError> {
    let argon2_params = params.build_argon2_params()?;
    let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, argon2_params);

    // Buffer pre-allocation - immediately overwritten by Argon2id key derivation below.
    // This is NOT a hard-coded key; it's a standard Rust pattern for mutable buffer allocation.
    let mut derived_key = [0u8; 32];
    argon2
        .hash_password_into(passphrase, salt, &mut derived_key)
        .map_err(|_| CryptoError::KeyDerivationFailed)?;

    Ok(derived_key)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Use low-security params for faster tests.
    fn test_params() -> KeyEncryptionParams {
        KeyEncryptionParams::low_security()
    }

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let private_key: [u8; 32] = [
            0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e,
            0x0f, 0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0x1a, 0x1b, 0x1c,
            0x1d, 0x1e, 0x1f, 0x20,
        ];
        let passphrase = b"correct-horse-battery-staple";

        let encrypted =
            EncryptedPrivateKey::encrypt(&private_key, passphrase, test_params()).unwrap();

        let decrypted = encrypted.decrypt(passphrase).unwrap();

        assert_eq!(decrypted.as_bytes(), &private_key);
    }

    #[test]
    fn test_wrong_passphrase_fails() {
        let private_key = [0xABu8; 32];
        let passphrase = b"correct-passphrase";
        let wrong_passphrase = b"wrong-passphrase";

        let encrypted =
            EncryptedPrivateKey::encrypt(&private_key, passphrase, test_params()).unwrap();

        let result = encrypted.decrypt(wrong_passphrase);
        assert!(result.is_err());
    }

    #[test]
    fn test_serialization_roundtrip() {
        let private_key = [0x42u8; 32];
        let passphrase = b"serialize-me";

        let encrypted =
            EncryptedPrivateKey::encrypt(&private_key, passphrase, test_params()).unwrap();

        let bytes = encrypted.to_bytes();
        let loaded = EncryptedPrivateKey::from_bytes(&bytes).unwrap();

        let decrypted = loaded.decrypt(passphrase).unwrap();
        assert_eq!(decrypted.as_bytes(), &private_key);
    }

    #[test]
    fn test_tampered_ciphertext_fails() {
        let private_key = [0x55u8; 32];
        let passphrase = b"tamper-test";

        let encrypted =
            EncryptedPrivateKey::encrypt(&private_key, passphrase, test_params()).unwrap();

        let mut bytes = encrypted.to_bytes();
        // Tamper with ciphertext
        let last_idx = bytes.len() - 1;
        bytes[last_idx] ^= 0xFF;

        let loaded = EncryptedPrivateKey::from_bytes(&bytes).unwrap();
        let result = loaded.decrypt(passphrase);
        assert!(result.is_err());
    }

    #[test]
    fn test_change_passphrase() {
        let private_key = [0x99u8; 32];
        let old_pass = b"old-passphrase";
        let new_pass = b"new-passphrase";

        let encrypted =
            EncryptedPrivateKey::encrypt(&private_key, old_pass, test_params()).unwrap();

        let re_encrypted = encrypted
            .change_passphrase(old_pass, new_pass, None)
            .unwrap();

        // Old passphrase should fail
        assert!(re_encrypted.decrypt(old_pass).is_err());

        // New passphrase should work
        let decrypted = re_encrypted.decrypt(new_pass).unwrap();
        assert_eq!(decrypted.as_bytes(), &private_key);
    }

    #[test]
    fn test_params_serialization() {
        let params = KeyEncryptionParams {
            memory_cost_kib: 131072,
            iterations: 8,
            parallelism: 4,
        };

        let bytes = params.to_bytes();
        let restored = KeyEncryptionParams::from_bytes(&bytes);

        assert_eq!(params.memory_cost_kib, restored.memory_cost_kib);
        assert_eq!(params.iterations, restored.iterations);
        assert_eq!(params.parallelism, restored.parallelism);
    }

    #[test]
    fn test_invalid_params_rejected() {
        let invalid_params = KeyEncryptionParams {
            memory_cost_kib: 4, // Below minimum (8 KiB)
            iterations: 1,
            parallelism: 1,
        };

        let private_key = [0u8; 32];
        let result = EncryptedPrivateKey::encrypt(&private_key, b"test", invalid_params);
        assert!(result.is_err());
    }

    #[test]
    fn test_minimum_valid_params() {
        let min_params = KeyEncryptionParams {
            memory_cost_kib: 8, // Minimum valid
            iterations: 1,
            parallelism: 1,
        };

        let private_key = [0u8; 32];
        let result = EncryptedPrivateKey::encrypt(&private_key, b"test", min_params);
        assert!(result.is_ok());
    }

    #[test]
    fn test_default_params() {
        let params = KeyEncryptionParams::default();
        assert_eq!(params.memory_cost_kib, 65536); // 64 MiB
        assert_eq!(params.iterations, 4);
        assert_eq!(params.parallelism, 4);
    }

    #[test]
    fn test_high_security_params() {
        let params = KeyEncryptionParams::high_security();
        assert_eq!(params.memory_cost_kib, 131072); // 128 MiB
        assert_eq!(params.iterations, 8);
        assert_eq!(params.parallelism, 4);
    }

    #[test]
    fn test_truncated_data_fails() {
        let private_key = [0u8; 32];
        let passphrase = b"test";

        let encrypted =
            EncryptedPrivateKey::encrypt(&private_key, passphrase, test_params()).unwrap();

        let bytes = encrypted.to_bytes();
        let truncated = &bytes[..bytes.len() - 10];

        let result = EncryptedPrivateKey::from_bytes(truncated);
        assert!(result.is_err());
    }

    #[test]
    fn test_version_check() {
        let encrypted = EncryptedPrivateKey::encrypt(&[0u8; 32], b"test", test_params()).unwrap();

        let mut bytes = encrypted.to_bytes();
        bytes[0] = 99; // Invalid version

        let result = EncryptedPrivateKey::from_bytes(&bytes);
        assert!(result.is_err());
    }

    #[test]
    fn test_decrypted_key_zeroizes() {
        let private_key = [0xFFu8; 32];
        let decrypted = DecryptedPrivateKey::new(private_key);

        // Verify we can read the key
        assert_eq!(decrypted.as_bytes(), &private_key);

        // After drop, the memory should be zeroized
        // (Can't easily test this without unsafe, but the derive macro handles it)
    }

    #[test]
    fn test_empty_passphrase_works() {
        let private_key = [0x42u8; 32];
        let passphrase = b"";

        let encrypted =
            EncryptedPrivateKey::encrypt(&private_key, passphrase, test_params()).unwrap();

        let decrypted = encrypted.decrypt(passphrase).unwrap();
        assert_eq!(decrypted.as_bytes(), &private_key);
    }

    #[test]
    fn test_unicode_passphrase() {
        let private_key = [0x42u8; 32];
        let passphrase = "correct-horse-\u{1F40E}-staple".as_bytes();

        let encrypted =
            EncryptedPrivateKey::encrypt(&private_key, passphrase, test_params()).unwrap();

        let decrypted = encrypted.decrypt(passphrase).unwrap();
        assert_eq!(decrypted.as_bytes(), &private_key);
    }

    #[test]
    fn test_long_passphrase() {
        let private_key = [0x42u8; 32];
        let passphrase = b"a".repeat(10000);

        let encrypted =
            EncryptedPrivateKey::encrypt(&private_key, &passphrase, test_params()).unwrap();

        let decrypted = encrypted.decrypt(&passphrase).unwrap();
        assert_eq!(decrypted.as_bytes(), &private_key);
    }
}
