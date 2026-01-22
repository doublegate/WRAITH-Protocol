//! Ed25519 Content Signatures
//!
//! Provides cryptographic signing and verification for published content.
//! Ensures content authenticity and author attribution.

use crate::error::{PublishError, PublishResult};
use ed25519_dalek::{Signature, SigningKey, VerifyingKey};
use serde::{Deserialize, Serialize};

/// Signed content with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignedContent {
    /// The raw content bytes
    pub content: Vec<u8>,
    /// Ed25519 signature over the content
    pub signature: Vec<u8>,
    /// Public key of the signer (verifying key)
    pub public_key: Vec<u8>,
    /// Content ID (BLAKE3 hash)
    pub cid: String,
    /// Timestamp of signing
    pub signed_at: i64,
}

/// Content signer for creating and verifying signatures
pub struct ContentSigner {
    signing_key: Option<SigningKey>,
}

impl ContentSigner {
    /// Create a new content signer with a signing key
    pub fn new(signing_key: Option<SigningKey>) -> Self {
        Self { signing_key }
    }

    /// Sign content with the local signing key
    pub fn sign(&self, content: &[u8]) -> PublishResult<SignedContent> {
        let signing_key = self
            .signing_key
            .as_ref()
            .ok_or_else(|| PublishError::Crypto("No signing key available".to_string()))?;

        // Compute content ID (CID)
        let cid = hex::encode(&blake3::hash(content).as_bytes()[..32]);

        // Create signature
        use ed25519_dalek::Signer;
        let signature = signing_key.sign(content);

        // Get public key
        let verifying_key = signing_key.verifying_key();

        Ok(SignedContent {
            content: content.to_vec(),
            signature: signature.to_bytes().to_vec(),
            public_key: verifying_key.as_bytes().to_vec(),
            cid,
            signed_at: chrono::Utc::now().timestamp(),
        })
    }

    /// Verify a signed content's signature
    pub fn verify(signed_content: &SignedContent) -> PublishResult<bool> {
        // Reconstruct verifying key
        let public_key_bytes: [u8; 32] = signed_content.public_key[..32]
            .try_into()
            .map_err(|_| PublishError::Crypto("Invalid public key length".to_string()))?;

        let verifying_key = VerifyingKey::from_bytes(&public_key_bytes)?;

        // Reconstruct signature
        let signature_bytes: [u8; 64] = signed_content.signature[..64]
            .try_into()
            .map_err(|_| PublishError::Crypto("Invalid signature length".to_string()))?;

        let signature = Signature::from_bytes(&signature_bytes);

        // Verify
        use ed25519_dalek::Verifier;
        match verifying_key.verify(&signed_content.content, &signature) {
            Ok(()) => Ok(true),
            Err(_) => Ok(false),
        }
    }

    /// Verify content against a specific CID
    pub fn verify_cid(content: &[u8], expected_cid: &str) -> bool {
        let computed_cid = hex::encode(&blake3::hash(content).as_bytes()[..32]);
        computed_cid == expected_cid
    }
}

/// Signature metadata for UI display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignatureInfo {
    /// Whether the signature is valid
    pub valid: bool,
    /// Public key fingerprint (truncated for display)
    pub signer_fingerprint: String,
    /// When the content was signed
    pub signed_at: i64,
    /// Content ID
    pub cid: String,
}

impl SignatureInfo {
    /// Create signature info from signed content
    pub fn from_signed_content(signed_content: &SignedContent) -> PublishResult<Self> {
        let valid = ContentSigner::verify(signed_content)?;

        // Create fingerprint from public key hash
        let fingerprint = hex::encode(&blake3::hash(&signed_content.public_key).as_bytes()[..8]);

        Ok(Self {
            valid,
            signer_fingerprint: fingerprint,
            signed_at: signed_content.signed_at,
            cid: signed_content.cid.clone(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::rngs::OsRng;

    #[test]
    fn test_sign_and_verify() {
        let signing_key = SigningKey::generate(&mut OsRng);
        let signer = ContentSigner::new(Some(signing_key));

        let content = b"Hello, WRAITH Publish!";
        let signed = signer.sign(content).unwrap();

        assert!(ContentSigner::verify(&signed).unwrap());
        assert!(!signed.cid.is_empty());
    }

    #[test]
    fn test_verify_cid() {
        let content = b"Test content";
        let cid = hex::encode(&blake3::hash(content).as_bytes()[..32]);

        assert!(ContentSigner::verify_cid(content, &cid));
        assert!(!ContentSigner::verify_cid(b"Different content", &cid));
    }

    #[test]
    fn test_tampered_content_fails_verification() {
        let signing_key = SigningKey::generate(&mut OsRng);
        let signer = ContentSigner::new(Some(signing_key));

        let content = b"Original content";
        let mut signed = signer.sign(content).unwrap();

        // Tamper with content
        signed.content = b"Tampered content".to_vec();

        assert!(!ContentSigner::verify(&signed).unwrap());
    }

    #[test]
    fn test_signature_info() {
        let signing_key = SigningKey::generate(&mut OsRng);
        let signer = ContentSigner::new(Some(signing_key));

        let content = b"Test content for signature info";
        let signed = signer.sign(content).unwrap();

        let info = SignatureInfo::from_signed_content(&signed).unwrap();

        assert!(info.valid);
        assert!(!info.signer_fingerprint.is_empty());
        assert_eq!(info.cid, signed.cid);
    }

    #[test]
    fn test_no_signing_key_error() {
        let signer = ContentSigner::new(None);
        let result = signer.sign(b"content");

        assert!(result.is_err());
    }
}
