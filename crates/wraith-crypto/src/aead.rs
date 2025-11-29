//! XChaCha20-Poly1305 AEAD encryption.

use crate::CryptoError;
use chacha20poly1305::{
    XChaCha20Poly1305,
    aead::{Aead, KeyInit},
};

/// AEAD cipher for packet encryption
pub struct AeadCipher {
    cipher: XChaCha20Poly1305,
}

impl AeadCipher {
    /// Create a new AEAD cipher with the given key
    pub fn new(key: &[u8; 32]) -> Self {
        Self {
            cipher: XChaCha20Poly1305::new(key.into()),
        }
    }

    /// Encrypt plaintext with the given nonce and associated data
    pub fn encrypt(
        &self,
        nonce: &[u8; 24],
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

    /// Decrypt ciphertext with the given nonce and associated data
    pub fn decrypt(
        &self,
        nonce: &[u8; 24],
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

impl Drop for AeadCipher {
    fn drop(&mut self) {
        // Zeroize is handled by the underlying cipher
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
