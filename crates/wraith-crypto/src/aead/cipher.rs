//! Core AEAD cipher types and operations.
//!
//! Provides `XChaCha20-Poly1305` AEAD encryption with:
//! - 256-bit keys
//! - 192-bit nonces (extended nonce for safe random generation)
//! - 128-bit authentication tags
//! - Associated data authentication
//! - In-place encryption/decryption for zero-copy operations

use crate::CryptoError;
use alloc::vec::Vec;
use chacha20poly1305::{
    XChaCha20Poly1305,
    aead::{Aead, AeadInPlace, KeyInit},
};
use rand_core::{CryptoRng, RngCore};
use subtle::ConstantTimeEq;
use zeroize::ZeroizeOnDrop;

/// Authentication tag size (16 bytes / 128 bits).
pub const TAG_SIZE: usize = 16;

/// XChaCha20-Poly1305 nonce size (24 bytes / 192 bits).
pub const NONCE_SIZE: usize = 24;

/// AEAD key size (32 bytes / 256 bits).
pub const KEY_SIZE: usize = 32;

/// XChaCha20-Poly1305 nonce (24 bytes).
///
/// The extended 192-bit nonce allows safe random nonce generation
/// without risk of collision (birthday bound is 2^96 messages).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Nonce([u8; NONCE_SIZE]);

impl Nonce {
    /// Create a nonce from raw bytes.
    #[must_use]
    pub fn from_bytes(bytes: [u8; NONCE_SIZE]) -> Self {
        Self(bytes)
    }

    /// Create a nonce from a slice.
    #[must_use]
    pub fn from_slice(slice: &[u8]) -> Option<Self> {
        if slice.len() != NONCE_SIZE {
            return None;
        }
        let mut bytes = [0u8; NONCE_SIZE];
        bytes.copy_from_slice(slice);
        Some(Self(bytes))
    }

    /// Generate a random nonce.
    #[must_use]
    pub fn generate<R: RngCore + CryptoRng>(rng: &mut R) -> Self {
        let mut bytes = [0u8; NONCE_SIZE];
        rng.fill_bytes(&mut bytes);
        Self(bytes)
    }

    /// Create a nonce from a counter value.
    ///
    /// The counter is placed in the first 8 bytes (little-endian),
    /// with the remaining 16 bytes available for session ID or salt.
    #[must_use]
    pub fn from_counter(counter: u64, salt: &[u8; 16]) -> Self {
        let mut bytes = [0u8; NONCE_SIZE];
        bytes[..8].copy_from_slice(&counter.to_le_bytes());
        bytes[8..].copy_from_slice(salt);
        Self(bytes)
    }

    /// Get raw bytes.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8; NONCE_SIZE] {
        &self.0
    }

    /// Get as a reference for chacha20poly1305.
    pub(crate) fn as_generic(&self) -> &chacha20poly1305::XNonce {
        chacha20poly1305::XNonce::from_slice(&self.0)
    }
}

impl Default for Nonce {
    fn default() -> Self {
        Self([0u8; NONCE_SIZE])
    }
}

/// Authentication tag (16 bytes).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Tag([u8; TAG_SIZE]);

impl Tag {
    /// Create a tag from raw bytes.
    #[must_use]
    pub fn from_bytes(bytes: [u8; TAG_SIZE]) -> Self {
        Self(bytes)
    }

    /// Create from slice.
    #[must_use]
    pub fn from_slice(slice: &[u8]) -> Option<Self> {
        if slice.len() != TAG_SIZE {
            return None;
        }
        let mut bytes = [0u8; TAG_SIZE];
        bytes.copy_from_slice(slice);
        Some(Self(bytes))
    }

    /// Get raw bytes.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8; TAG_SIZE] {
        &self.0
    }
}

/// AEAD encryption key (32 bytes).
///
/// Wraps the raw key material and provides encryption/decryption methods.
/// Key is zeroized on drop.
#[derive(Clone, ZeroizeOnDrop)]
pub struct AeadKey([u8; KEY_SIZE]);

impl AeadKey {
    /// Create a key from raw bytes.
    #[must_use]
    pub fn new(bytes: [u8; KEY_SIZE]) -> Self {
        Self(bytes)
    }

    /// Create from slice.
    ///
    /// # Errors
    ///
    /// Returns `CryptoError::InvalidKeyLength` if slice length is not 32 bytes.
    pub fn from_slice(slice: &[u8]) -> Result<Self, CryptoError> {
        if slice.len() != KEY_SIZE {
            return Err(CryptoError::InvalidKeyLength {
                expected: KEY_SIZE,
                actual: slice.len(),
            });
        }
        let mut bytes = [0u8; KEY_SIZE];
        bytes.copy_from_slice(slice);
        Ok(Self(bytes))
    }

    /// Generate a random key.
    #[must_use]
    pub fn generate<R: RngCore + CryptoRng>(rng: &mut R) -> Self {
        let mut bytes = [0u8; KEY_SIZE];
        rng.fill_bytes(&mut bytes);
        Self(bytes)
    }

    /// Get raw key bytes.
    ///
    /// # Security
    ///
    /// Handle with extreme care - this exposes the raw key material.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8; KEY_SIZE] {
        &self.0
    }

    /// Compute key commitment for key-committing AEAD.
    ///
    /// Returns a 16-byte commitment that binds the ciphertext to this specific key.
    /// This prevents key-commitment attacks where an attacker crafts ciphertexts
    /// that decrypt validly under multiple keys.
    ///
    /// The commitment is computed as: `BLAKE3(key || "wraith-key-commitment")[0..16]`
    ///
    /// # Security
    ///
    /// The key commitment is computed using BLAKE3, which is a cryptographic hash
    /// function with constant-time properties when used with fixed-length inputs.
    #[must_use]
    pub fn commitment(&self) -> [u8; 16] {
        let mut hasher = blake3::Hasher::new();
        hasher.update(&self.0);
        hasher.update(b"wraith-key-commitment");
        let hash = hasher.finalize();
        let mut commitment = [0u8; 16];
        commitment.copy_from_slice(&hash.as_bytes()[..16]);
        commitment
    }

    /// Verify key commitment in constant time.
    ///
    /// Returns `true` if the provided commitment matches this key's commitment.
    /// Uses constant-time comparison to prevent timing attacks.
    #[must_use]
    pub fn verify_commitment(&self, commitment: &[u8; 16]) -> bool {
        let expected = self.commitment();
        expected.ct_eq(commitment).into()
    }

    /// Encrypt plaintext with associated data.
    ///
    /// Returns ciphertext with appended authentication tag (`plaintext.len()` + 16 bytes).
    ///
    /// # Errors
    ///
    /// Returns `CryptoError::EncryptionFailed` if AEAD encryption fails.
    pub fn encrypt(
        &self,
        nonce: &Nonce,
        plaintext: &[u8],
        aad: &[u8],
    ) -> Result<Vec<u8>, CryptoError> {
        let cipher = XChaCha20Poly1305::new((&self.0).into());

        cipher
            .encrypt(
                nonce.as_generic(),
                chacha20poly1305::aead::Payload {
                    msg: plaintext,
                    aad,
                },
            )
            .map_err(|_| CryptoError::EncryptionFailed)
    }

    /// Decrypt ciphertext with associated data.
    ///
    /// Input must include the authentication tag at the end.
    ///
    /// # Errors
    ///
    /// Returns `CryptoError::DecryptionFailed` on authentication failure.
    pub fn decrypt(
        &self,
        nonce: &Nonce,
        ciphertext_and_tag: &[u8],
        aad: &[u8],
    ) -> Result<Vec<u8>, CryptoError> {
        if ciphertext_and_tag.len() < TAG_SIZE {
            return Err(CryptoError::DecryptionFailed);
        }

        let cipher = XChaCha20Poly1305::new((&self.0).into());

        cipher
            .decrypt(
                nonce.as_generic(),
                chacha20poly1305::aead::Payload {
                    msg: ciphertext_and_tag,
                    aad,
                },
            )
            .map_err(|_| CryptoError::DecryptionFailed)
    }

    /// Encrypt in-place, returning the authentication tag.
    ///
    /// The buffer is modified in-place to contain the ciphertext.
    /// Returns the authentication tag separately.
    ///
    /// # Errors
    ///
    /// Returns `CryptoError::EncryptionFailed` if AEAD encryption fails.
    pub fn encrypt_in_place(
        &self,
        nonce: &Nonce,
        buffer: &mut [u8],
        aad: &[u8],
    ) -> Result<Tag, CryptoError> {
        let cipher = XChaCha20Poly1305::new((&self.0).into());

        let tag = cipher
            .encrypt_in_place_detached(nonce.as_generic(), aad, buffer)
            .map_err(|_| CryptoError::EncryptionFailed)?;

        let mut tag_bytes = [0u8; TAG_SIZE];
        tag_bytes.copy_from_slice(&tag);
        Ok(Tag(tag_bytes))
    }

    /// Decrypt in-place, verifying the authentication tag.
    ///
    /// The buffer is modified in-place to contain the plaintext.
    ///
    /// # Errors
    ///
    /// Returns `CryptoError::DecryptionFailed` on authentication failure.
    pub fn decrypt_in_place(
        &self,
        nonce: &Nonce,
        buffer: &mut [u8],
        tag: &Tag,
        aad: &[u8],
    ) -> Result<(), CryptoError> {
        let cipher = XChaCha20Poly1305::new((&self.0).into());

        cipher
            .decrypt_in_place_detached(
                nonce.as_generic(),
                aad,
                buffer,
                chacha20poly1305::Tag::from_slice(&tag.0),
            )
            .map_err(|_| CryptoError::DecryptionFailed)
    }
}

/// Cached AEAD cipher that holds both the key and pre-constructed cipher instance.
///
/// Avoids calling `XChaCha20Poly1305::new()` on every encrypt/decrypt invocation.
/// Use this when performing multiple operations with the same key.
#[derive(Clone)]
pub struct CachedAeadCipher {
    cipher: XChaCha20Poly1305,
}

impl CachedAeadCipher {
    /// Create a cached cipher from an AEAD key.
    #[must_use]
    pub fn new(key: &AeadKey) -> Self {
        Self {
            cipher: XChaCha20Poly1305::new((&key.0).into()),
        }
    }

    /// Create from raw key bytes.
    #[must_use]
    pub fn from_bytes(key: &[u8; KEY_SIZE]) -> Self {
        Self {
            cipher: XChaCha20Poly1305::new(key.into()),
        }
    }

    /// Encrypt plaintext with associated data.
    ///
    /// # Errors
    ///
    /// Returns `CryptoError::EncryptionFailed` if AEAD encryption fails.
    pub fn encrypt(
        &self,
        nonce: &Nonce,
        plaintext: &[u8],
        aad: &[u8],
    ) -> Result<Vec<u8>, CryptoError> {
        self.cipher
            .encrypt(
                nonce.as_generic(),
                chacha20poly1305::aead::Payload {
                    msg: plaintext,
                    aad,
                },
            )
            .map_err(|_| CryptoError::EncryptionFailed)
    }

    /// Decrypt ciphertext with associated data.
    ///
    /// # Errors
    ///
    /// Returns `CryptoError::DecryptionFailed` on authentication failure.
    pub fn decrypt(
        &self,
        nonce: &Nonce,
        ciphertext_and_tag: &[u8],
        aad: &[u8],
    ) -> Result<Vec<u8>, CryptoError> {
        if ciphertext_and_tag.len() < TAG_SIZE {
            return Err(CryptoError::DecryptionFailed);
        }

        self.cipher
            .decrypt(
                nonce.as_generic(),
                chacha20poly1305::aead::Payload {
                    msg: ciphertext_and_tag,
                    aad,
                },
            )
            .map_err(|_| CryptoError::DecryptionFailed)
    }
}

/// AEAD cipher for packet encryption (legacy API).
///
/// Use `AeadKey` directly for new code.
pub struct AeadCipher {
    cipher: XChaCha20Poly1305,
}

impl AeadCipher {
    /// Create a new AEAD cipher with the given key.
    #[must_use]
    pub fn new(key: &[u8; KEY_SIZE]) -> Self {
        Self {
            cipher: XChaCha20Poly1305::new(key.into()),
        }
    }

    /// Encrypt plaintext with the given nonce and associated data.
    ///
    /// # Errors
    ///
    /// Returns `CryptoError::EncryptionFailed` if AEAD encryption fails.
    pub fn encrypt(
        &self,
        nonce: &[u8; NONCE_SIZE],
        plaintext: &[u8],
        aad: &[u8],
    ) -> Result<Vec<u8>, CryptoError> {
        use chacha20poly1305::aead::Payload;

        let payload = Payload {
            msg: plaintext,
            aad,
        };

        self.cipher
            .encrypt(nonce.into(), payload)
            .map_err(|_| CryptoError::EncryptionFailed)
    }

    /// Decrypt ciphertext with the given nonce and associated data.
    ///
    /// # Errors
    ///
    /// Returns `CryptoError::DecryptionFailed` on authentication failure.
    pub fn decrypt(
        &self,
        nonce: &[u8; NONCE_SIZE],
        ciphertext: &[u8],
        aad: &[u8],
    ) -> Result<Vec<u8>, CryptoError> {
        use chacha20poly1305::aead::Payload;

        let payload = Payload {
            msg: ciphertext,
            aad,
        };

        self.cipher
            .decrypt(nonce.into(), payload)
            .map_err(|_| CryptoError::DecryptionFailed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand_core::OsRng;

    #[test]
    fn test_aead_roundtrip() {
        let key = [0x42u8; 32];
        let nonce = [0x00u8; 24];
        let plaintext = b"Hello, WRAITH!";
        let aad = b"additional data";

        let cipher = AeadCipher::new(&key);

        let ciphertext = cipher.encrypt(&nonce, plaintext, aad).unwrap();
        let decrypted = cipher.decrypt(&nonce, &ciphertext, aad).unwrap();

        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_aead_tamper_detection() {
        let key = [0x42u8; 32];
        let nonce = [0x00u8; 24];
        let plaintext = b"Hello, WRAITH!";
        let aad = b"additional data";

        let cipher = AeadCipher::new(&key);

        let mut ciphertext = cipher.encrypt(&nonce, plaintext, aad).unwrap();
        ciphertext[0] ^= 0xFF; // Tamper with ciphertext

        assert!(cipher.decrypt(&nonce, &ciphertext, aad).is_err());
    }

    #[test]
    fn test_aead_key_encrypt_decrypt() {
        let key = AeadKey::generate(&mut OsRng);
        let nonce = Nonce::generate(&mut OsRng);
        let plaintext = b"secret message";
        let aad = b"header";

        let ciphertext = key.encrypt(&nonce, plaintext, aad).unwrap();
        assert_eq!(ciphertext.len(), plaintext.len() + TAG_SIZE);

        let decrypted = key.decrypt(&nonce, &ciphertext, aad).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_aead_wrong_key_fails() {
        let key1 = AeadKey::generate(&mut OsRng);
        let key2 = AeadKey::generate(&mut OsRng);
        let nonce = Nonce::generate(&mut OsRng);

        let ciphertext = key1.encrypt(&nonce, b"secret", b"").unwrap();
        assert!(key2.decrypt(&nonce, &ciphertext, b"").is_err());
    }

    #[test]
    fn test_aead_wrong_nonce_fails() {
        let key = AeadKey::generate(&mut OsRng);
        let nonce1 = Nonce::generate(&mut OsRng);
        let nonce2 = Nonce::generate(&mut OsRng);

        let ciphertext = key.encrypt(&nonce1, b"secret", b"").unwrap();
        assert!(key.decrypt(&nonce2, &ciphertext, b"").is_err());
    }

    #[test]
    fn test_aead_wrong_aad_fails() {
        let key = AeadKey::generate(&mut OsRng);
        let nonce = Nonce::generate(&mut OsRng);

        let ciphertext = key.encrypt(&nonce, b"secret", b"aad1").unwrap();
        assert!(key.decrypt(&nonce, &ciphertext, b"aad2").is_err());
    }

    #[test]
    fn test_aead_in_place() {
        let key = AeadKey::generate(&mut OsRng);
        let nonce = Nonce::generate(&mut OsRng);
        let plaintext = b"hello world";
        let mut buffer = plaintext.to_vec();

        let tag = key.encrypt_in_place(&nonce, &mut buffer, b"").unwrap();
        assert_ne!(&buffer, plaintext);

        key.decrypt_in_place(&nonce, &mut buffer, &tag, b"")
            .unwrap();
        assert_eq!(&buffer, plaintext);
    }

    #[test]
    fn test_nonce_from_counter() {
        let salt = [0x42u8; 16];
        let nonce1 = Nonce::from_counter(0, &salt);
        let nonce2 = Nonce::from_counter(1, &salt);
        let nonce3 = Nonce::from_counter(0, &salt);

        assert_ne!(nonce1.as_bytes(), nonce2.as_bytes());
        assert_eq!(nonce1.as_bytes(), nonce3.as_bytes());
    }

    #[test]
    fn test_key_commitment() {
        let key = AeadKey::generate(&mut OsRng);
        let commitment = key.commitment();

        assert!(key.verify_commitment(&commitment));

        let mut wrong_commitment = commitment;
        wrong_commitment[0] ^= 0xFF;
        assert!(!key.verify_commitment(&wrong_commitment));
    }
}
