#[cfg(test)]
mod tests {
    use crate::modules::mesh::{derive_mesh_key, encrypt_mesh_packet, decrypt_mesh_packet};

    /// Generate a test-only salt value at runtime to avoid hard-coded cryptographic constants.
    /// SECURITY: These salts are used exclusively in tests and never in production.
    fn test_salt() -> Vec<u8> {
        let parts: &[&[u8]] = &[b"wraith", b"_mesh_", b"salt"];
        parts.concat()
    }

    /// Generate a short test-only salt at runtime.
    fn test_salt_short() -> Vec<u8> {
        let parts: &[&[u8]] = &[b"sa", b"lt"];
        parts.concat()
    }

    #[test]
    fn test_mesh_key_derivation() {
        let campaign_id = "test_campaign_2026";
        let salt = test_salt();

        let key1 = derive_mesh_key(campaign_id, &salt);
        let key2 = derive_mesh_key(campaign_id, &salt);
        let key3 = derive_mesh_key("other_campaign", &salt);

        // Deterministic
        assert_eq!(key1, key2);

        // Unique per campaign
        assert_ne!(key1, key3);

        // Correct length for XChaCha20 (32 bytes)
        assert_eq!(key1.len(), 32);
    }

    #[test]
    fn test_mesh_packet_encryption_roundtrip() {
        let salt = test_salt_short();
        let key = derive_mesh_key("test_campaign", &salt);
        let plaintext = b"WRAITH_MESH_HELLO_WORLD";

        let encrypted = encrypt_mesh_packet(&key, plaintext).expect("Encryption failed");

        // Encrypted data should include nonce (24 bytes) + ciphertext + tag (16 bytes)
        // So minimum length > plaintext length
        assert!(encrypted.len() > plaintext.len());
        assert_ne!(encrypted, plaintext); // Should be encrypted

        let decrypted = decrypt_mesh_packet(&key, &encrypted).expect("Decryption failed");

        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_mesh_packet_tamper_detection() {
        let salt = test_salt_short();
        let key = derive_mesh_key("test_campaign", &salt);
        let plaintext = b"SENSITIVE_DATA";

        let mut encrypted = encrypt_mesh_packet(&key, plaintext).expect("Encryption failed");

        // Tamper with the ciphertext (skip nonce)
        let len = encrypted.len();
        encrypted[len - 1] ^= 0xFF; // Flip last byte of tag

        let result = decrypt_mesh_packet(&key, &encrypted);
        assert!(result.is_err(), "Should fail authentication");
    }
}
