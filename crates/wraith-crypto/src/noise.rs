//! Noise\_XX handshake protocol for mutual authentication with identity hiding.
//!
//! Implements the Noise\_XX pattern using the snow library:
//! - Pattern: `XX` (mutual authentication, identity hiding)
//! - DH: `25519` (Curve25519)
//! - Cipher: `ChaChaPoly` (ChaCha20-Poly1305)
//! - Hash: `BLAKE2s` (for snow compatibility; BLAKE3 for application KDF)
//!
//! ## Message Flow
//!
//! ```text
//! Message 1: Initiator → Responder: e
//! Message 2: Responder → Initiator: e, ee, s, es
//! Message 3: Initiator → Responder: s, se
//! ```
//!
//! After message 3, both parties have:
//! - Authenticated each other's static keys
//! - Established shared symmetric keys for encryption
//! - Perfect forward secrecy (ephemeral keys forgotten)
//!
//! ## Security Properties
//!
//! - Identity hiding: Static keys encrypted after first DH
//! - Forward secrecy: Compromise of static keys doesn't reveal past sessions
//! - Mutual authentication: Both parties prove knowledge of static keys

use crate::random::SecureRng;
use crate::ratchet::{DoubleRatchet, MessageHeader};
use crate::x25519::{PrivateKey, PublicKey};
use crate::{CryptoError, SessionKeys};
use alloc::string::String;
use alloc::string::ToString;
use alloc::vec::Vec;
use alloc::{format, vec};
use core::fmt;
use snow::{Builder, HandshakeState};
use zeroize::Zeroize;

/// Noise protocol pattern used by WRAITH.
const NOISE_PATTERN: &str = "Noise_XX_25519_ChaChaPoly_BLAKE2s";

/// Maximum handshake message size.
/// Message 1: 32 (e) + 0 payload + 0 tag = 32 bytes
/// Message 2: 32 (e) + 32 (s) + 16 (tag) + 16 (tag) = 96 bytes
/// Message 3: 32 (s) + 16 (tag) + 16 (tag) = 64 bytes
/// Add buffer for optional payloads
const MAX_HANDSHAKE_MSG_SIZE: usize = 256;

/// Role in the Noise handshake.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    /// Initiates the handshake (sends message 1)
    Initiator,
    /// Responds to handshake (receives message 1)
    Responder,
}

/// State of the handshake.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HandshakePhase {
    /// Initial state, ready to start
    Initial,
    /// After message 1 (initiator sent, responder received)
    Message1Complete,
    /// After message 2 (responder sent, initiator received)
    Message2Complete,
    /// Handshake complete, transport ready
    Complete,
}

/// Error types for Noise operations.
#[derive(Debug, Clone)]
pub enum NoiseError {
    /// Invalid handshake state for this operation
    InvalidState,
    /// Handshake message was invalid
    InvalidMessage,
    /// Decryption failed (bad MAC or corrupted data)
    DecryptionFailed,
    /// Key derivation failed
    KeyDerivationFailed,
    /// Snow library error
    SnowError(String),
}

impl fmt::Display for NoiseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NoiseError::InvalidState => write!(f, "Invalid handshake state"),
            NoiseError::InvalidMessage => write!(f, "Invalid handshake message"),
            NoiseError::DecryptionFailed => write!(f, "Decryption failed"),
            NoiseError::KeyDerivationFailed => write!(f, "Key derivation failed"),
            NoiseError::SnowError(e) => write!(f, "Snow error: {e}"),
        }
    }
}

impl core::error::Error for NoiseError {}

impl From<snow::Error> for NoiseError {
    fn from(e: snow::Error) -> Self {
        NoiseError::SnowError(e.to_string())
    }
}

impl From<NoiseError> for CryptoError {
    fn from(e: NoiseError) -> Self {
        CryptoError::HandshakeFailed(e.to_string())
    }
}

/// Static keypair for Noise handshakes.
///
/// This is the long-term identity key used across multiple sessions.
pub struct NoiseKeypair {
    private: Vec<u8>,
    public: [u8; 32],
}

impl NoiseKeypair {
    /// Generate a new random keypair.
    ///
    /// # Errors
    ///
    /// Returns `NoiseError::SnowError` if:
    /// - The Noise pattern string fails to parse (should not happen with valid constant)
    /// - Keypair generation fails due to RNG issues
    pub fn generate() -> Result<Self, NoiseError> {
        let builder = Builder::new(
            NOISE_PATTERN
                .parse()
                .map_err(|e| NoiseError::SnowError(format!("Pattern parse error: {e:?}")))?,
        );

        let keypair = builder
            .generate_keypair()
            .map_err(|e| NoiseError::SnowError(format!("Keypair generation error: {e:?}")))?;

        let mut public = [0u8; 32];
        public.copy_from_slice(&keypair.public);

        Ok(Self {
            private: keypair.private,
            public,
        })
    }

    /// Create from existing key bytes.
    ///
    /// # Errors
    ///
    /// This function is infallible for valid 32-byte input but returns `Result`
    /// for API consistency with `generate()`.
    pub fn from_bytes(private: [u8; 32]) -> Result<Self, NoiseError> {
        // Derive public key from private using X25519
        // The public key is private * basepoint on Curve25519
        use crate::x25519::PrivateKey;

        let x25519_private = PrivateKey::from_bytes(private);
        let public = x25519_private.public_key().to_bytes();

        Ok(Self {
            private: private.to_vec(),
            public,
        })
    }

    /// Get the public key bytes.
    #[must_use]
    pub fn public_key(&self) -> &[u8; 32] {
        &self.public
    }

    /// Get the private key bytes.
    ///
    /// # Security
    ///
    /// Handle with extreme care - this is the long-term identity key.
    #[must_use]
    pub fn private_key(&self) -> &[u8] {
        &self.private
    }
}

impl Drop for NoiseKeypair {
    fn drop(&mut self) {
        self.private.zeroize();
    }
}

impl Clone for NoiseKeypair {
    fn clone(&self) -> Self {
        Self {
            private: self.private.clone(),
            public: self.public,
        }
    }
}

/// `Noise_XX` handshake session.
///
/// Manages the 3-message handshake pattern for mutual authentication.
pub struct NoiseHandshake {
    state: HandshakeState,
    role: Role,
    phase: HandshakePhase,
}

impl NoiseHandshake {
    /// Create a new handshake as the initiator.
    ///
    /// The initiator sends the first message and must know their own static key.
    ///
    /// # Errors
    ///
    /// Returns `NoiseError::SnowError` if:
    /// - The Noise pattern string fails to parse
    /// - The local private key is invalid
    /// - Handshake state initialization fails
    pub fn new_initiator(local_keypair: &NoiseKeypair) -> Result<Self, NoiseError> {
        let builder = Builder::new(
            NOISE_PATTERN
                .parse()
                .map_err(|e| NoiseError::SnowError(format!("Pattern parse error: {e:?}")))?,
        );

        let state = builder
            .local_private_key(&local_keypair.private)
            .map_err(|e| NoiseError::SnowError(format!("Key error: {e:?}")))?
            .build_initiator()
            .map_err(|e| NoiseError::SnowError(format!("Build error: {e:?}")))?;

        Ok(Self {
            state,
            role: Role::Initiator,
            phase: HandshakePhase::Initial,
        })
    }

    /// Create a new handshake as the responder.
    ///
    /// The responder waits for the first message and must know their own static key.
    ///
    /// # Errors
    ///
    /// Returns `NoiseError::SnowError` if:
    /// - The Noise pattern string fails to parse
    /// - The local private key is invalid
    /// - Handshake state initialization fails
    pub fn new_responder(local_keypair: &NoiseKeypair) -> Result<Self, NoiseError> {
        let builder = Builder::new(
            NOISE_PATTERN
                .parse()
                .map_err(|e| NoiseError::SnowError(format!("Pattern parse error: {e:?}")))?,
        );

        let state = builder
            .local_private_key(&local_keypair.private)
            .map_err(|e| NoiseError::SnowError(format!("Key error: {e:?}")))?
            .build_responder()
            .map_err(|e| NoiseError::SnowError(format!("Build error: {e:?}")))?;

        Ok(Self {
            state,
            role: Role::Responder,
            phase: HandshakePhase::Initial,
        })
    }

    /// Get the current handshake phase.
    #[must_use]
    pub fn phase(&self) -> HandshakePhase {
        self.phase
    }

    /// Get the role of this handshake.
    #[must_use]
    pub fn role(&self) -> Role {
        self.role
    }

    /// Check if the handshake is complete.
    #[must_use]
    pub fn is_complete(&self) -> bool {
        self.phase == HandshakePhase::Complete
    }

    /// Write the next handshake message.
    ///
    /// Returns the message bytes to send to the peer.
    /// Optionally includes a payload (typically empty during handshake).
    ///
    /// # Errors
    ///
    /// Returns `NoiseError::InvalidState` if called in the wrong phase for the current role.
    /// Returns `NoiseError::SnowError` if the underlying snow library fails.
    pub fn write_message(&mut self, payload: &[u8]) -> Result<Vec<u8>, NoiseError> {
        // Validate state
        match (self.role, self.phase) {
            (Role::Initiator, HandshakePhase::Initial | HandshakePhase::Message2Complete) => {}
            (Role::Responder, HandshakePhase::Message1Complete) => {}
            _ => return Err(NoiseError::InvalidState),
        }

        let mut message = vec![0u8; MAX_HANDSHAKE_MSG_SIZE];
        let len = self.state.write_message(payload, &mut message)?;
        message.truncate(len);

        // Update phase
        self.phase = match self.phase {
            HandshakePhase::Initial => HandshakePhase::Message1Complete,
            HandshakePhase::Message1Complete => HandshakePhase::Message2Complete,
            HandshakePhase::Message2Complete | HandshakePhase::Complete => HandshakePhase::Complete,
        };

        Ok(message)
    }

    /// Read a handshake message from the peer.
    ///
    /// Returns any payload included in the message.
    ///
    /// # Errors
    ///
    /// Returns `NoiseError::InvalidState` if called in the wrong phase for the current role.
    /// Returns `NoiseError::SnowError` if decryption or verification fails.
    pub fn read_message(&mut self, message: &[u8]) -> Result<Vec<u8>, NoiseError> {
        // Validate state
        match (self.role, self.phase) {
            (Role::Responder, HandshakePhase::Initial | HandshakePhase::Message2Complete) => {}
            (Role::Initiator, HandshakePhase::Message1Complete) => {}
            _ => return Err(NoiseError::InvalidState),
        }

        let mut payload = vec![0u8; MAX_HANDSHAKE_MSG_SIZE];
        let len = self.state.read_message(message, &mut payload)?;
        payload.truncate(len);

        // Update phase
        self.phase = match self.phase {
            HandshakePhase::Initial => HandshakePhase::Message1Complete,
            HandshakePhase::Message1Complete => HandshakePhase::Message2Complete,
            HandshakePhase::Message2Complete | HandshakePhase::Complete => HandshakePhase::Complete,
        };

        Ok(payload)
    }

    /// Get the remote peer's static public key (available after message 2/3).
    #[must_use]
    pub fn get_remote_static(&self) -> Option<[u8; 32]> {
        self.state.get_remote_static().map(|key| {
            let mut arr = [0u8; 32];
            arr.copy_from_slice(key);
            arr
        })
    }

    /// Complete the handshake and transition to transport mode.
    ///
    /// Returns the transport state for encrypted communication.
    ///
    /// # Errors
    ///
    /// Returns `NoiseError::InvalidState` if the handshake is not yet complete.
    /// Returns `NoiseError::SnowError` if transport mode initialization fails.
    pub fn into_transport(
        self,
        local_ratchet_key: Option<PrivateKey>,
        peer_ratchet_key: Option<PublicKey>,
    ) -> Result<NoiseTransport, NoiseError> {
        if self.phase != HandshakePhase::Complete {
            return Err(NoiseError::InvalidState);
        }

        let h = self.state.get_handshake_hash();
        let mut root_key = [0u8; 32];
        root_key.copy_from_slice(h);

        let mut rng = SecureRng::new();
        let ratchet = match self.role {
            Role::Initiator => {
                let peer = peer_ratchet_key.ok_or(NoiseError::InvalidState)?;
                DoubleRatchet::new_initiator(&mut rng, &root_key, peer)
            }
            Role::Responder => {
                let local = local_ratchet_key.ok_or(NoiseError::InvalidState)?;
                DoubleRatchet::new_responder(&root_key, local)
            }
        };

        Ok(NoiseTransport {
            ratchet,
            role: self.role,
        })
    }

    /// Complete the handshake and extract session keys.
    ///
    /// This extracts the symmetric keys for use with custom AEAD.
    ///
    /// # Errors
    ///
    /// Returns `NoiseError::InvalidState` if the handshake is not yet complete.
    pub fn into_session_keys(self) -> Result<SessionKeys, NoiseError> {
        if self.phase != HandshakePhase::Complete {
            return Err(NoiseError::InvalidState);
        }

        // Get the handshake hash (h) for key derivation
        let h = self.state.get_handshake_hash();

        // Use BLAKE3 to derive separate keys from the handshake hash
        // This provides domain separation between send/recv/chain keys
        // Both parties derive the SAME two directional keys, then assign based on role
        let mut key_i_to_r = [0u8; 32]; // Key for initiator → responder direction
        let mut key_r_to_i = [0u8; 32]; // Key for responder → initiator direction
        let mut chain_key = [0u8; 32];

        // Derive keys using BLAKE3 keyed mode with consistent labels
        // Both parties derive the same keys from the same handshake hash
        derive_key(h, b"wraith_i_to_r", &mut key_i_to_r);
        derive_key(h, b"wraith_r_to_i", &mut key_r_to_i);
        derive_key(h, b"wraith_chain", &mut chain_key);

        // Assign send/recv based on role
        // Initiator: send = i_to_r, recv = r_to_i
        // Responder: send = r_to_i, recv = i_to_r
        let (send_key, recv_key) = match self.role {
            Role::Initiator => (key_i_to_r, key_r_to_i),
            Role::Responder => (key_r_to_i, key_i_to_r),
        };

        Ok(SessionKeys {
            send_key,
            recv_key,
            chain_key,
        })
    }
}

/// Derive a key using BLAKE3 keyed mode.
fn derive_key(ikm: &[u8], context: &[u8], output: &mut [u8; 32]) {
    use crate::hash::hkdf;
    hkdf(context, ikm, b"wraith", output);
}

/// Noise transport state for post-handshake encrypted communication.
///
/// After the handshake completes, use this for bidirectional encryption.
pub struct NoiseTransport {
    ratchet: DoubleRatchet,
    role: Role,
}

impl NoiseTransport {
    /// Encrypt a message.
    ///
    /// The payload is encrypted and authenticated.
    ///
    /// # Errors
    ///
    /// Returns [`NoiseError::SnowError`] if encryption fails.
    pub fn write_message(&mut self, payload: &[u8]) -> Result<Vec<u8>, NoiseError> {
        let mut rng = SecureRng::new();
        let (header, ciphertext) = self
            .ratchet
            .encrypt(&mut rng, payload)
            .map_err(|_| NoiseError::SnowError("Ratchet encryption failed".into()))?;

        let mut message = header.to_bytes().to_vec();
        message.extend_from_slice(&ciphertext);
        Ok(message)
    }

    /// Decrypt a message.
    ///
    /// Verifies the authentication tag before returning plaintext.
    ///
    /// # Errors
    ///
    /// Returns [`NoiseError::InvalidMessage`] if the message is too short.
    /// Returns [`NoiseError::SnowError`] if decryption or authentication fails.
    pub fn read_message(&mut self, message: &[u8]) -> Result<Vec<u8>, NoiseError> {
        if message.len() < 40 {
            return Err(NoiseError::InvalidMessage);
        }
        let mut rng = SecureRng::new();
        let header =
            MessageHeader::from_bytes(&message[..40]).map_err(|_| NoiseError::InvalidMessage)?;
        let ciphertext = &message[40..];

        self.ratchet
            .decrypt(&mut rng, &header, ciphertext)
            .map_err(|_| NoiseError::DecryptionFailed)
    }

    /// Get the role this transport was created with.
    #[must_use]
    pub fn role(&self) -> Role {
        self.role
    }

    /// Force a DH ratchet step (rotate sending key).
    pub fn rekey_dh(&mut self) {
        let mut rng = SecureRng::new();
        self.ratchet.force_dh_step(&mut rng);
    }

    /// Mix external key material (e.g. PQ KEM) into the ratchet.
    pub fn mix_key(&mut self, data: &[u8]) {
        self.ratchet.mix_into_root(data);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keypair_generation() {
        let keypair = NoiseKeypair::generate().unwrap();
        assert_ne!(keypair.public_key(), &[0u8; 32]);
        assert_ne!(keypair.private_key(), &[0u8; 32]);
    }

    #[test]
    fn test_keypair_from_bytes() {
        let original = NoiseKeypair::generate().unwrap();
        let mut private_bytes = [0u8; 32];
        private_bytes.copy_from_slice(original.private_key());

        let restored = NoiseKeypair::from_bytes(private_bytes).unwrap();
        assert_eq!(original.public_key(), restored.public_key());
    }

    #[test]
    fn test_full_handshake() {
        let initiator_keypair = NoiseKeypair::generate().unwrap();
        let responder_keypair = NoiseKeypair::generate().unwrap();

        let mut initiator = NoiseHandshake::new_initiator(&initiator_keypair).unwrap();
        let mut responder = NoiseHandshake::new_responder(&responder_keypair).unwrap();

        // Message 1: Initiator → Responder
        assert_eq!(initiator.phase(), HandshakePhase::Initial);
        let msg1 = initiator.write_message(&[]).unwrap();
        assert_eq!(initiator.phase(), HandshakePhase::Message1Complete);

        assert_eq!(responder.phase(), HandshakePhase::Initial);
        let _payload1 = responder.read_message(&msg1).unwrap();
        assert_eq!(responder.phase(), HandshakePhase::Message1Complete);

        // Message 2: Responder → Initiator
        let msg2 = responder.write_message(&[]).unwrap();
        assert_eq!(responder.phase(), HandshakePhase::Message2Complete);

        let _payload2 = initiator.read_message(&msg2).unwrap();
        assert_eq!(initiator.phase(), HandshakePhase::Message2Complete);

        // Message 3: Initiator → Responder
        let msg3 = initiator.write_message(&[]).unwrap();
        assert_eq!(initiator.phase(), HandshakePhase::Complete);
        assert!(initiator.is_complete());

        let _payload3 = responder.read_message(&msg3).unwrap();
        assert_eq!(responder.phase(), HandshakePhase::Complete);
        assert!(responder.is_complete());

        // Verify remote static keys
        assert_eq!(
            initiator.get_remote_static().unwrap(),
            *responder_keypair.public_key()
        );
        assert_eq!(
            responder.get_remote_static().unwrap(),
            *initiator_keypair.public_key()
        );
    }

    #[test]
    fn test_transport_encryption() {
        let initiator_keypair = NoiseKeypair::generate().unwrap();
        let responder_keypair = NoiseKeypair::generate().unwrap();

        let mut initiator = NoiseHandshake::new_initiator(&initiator_keypair).unwrap();
        let mut responder = NoiseHandshake::new_responder(&responder_keypair).unwrap();

        // Complete handshake
        let msg1 = initiator.write_message(&[]).unwrap();
        responder.read_message(&msg1).unwrap();

        let msg2 = responder.write_message(&[]).unwrap();
        initiator.read_message(&msg2).unwrap();

        let msg3 = initiator.write_message(&[]).unwrap();
        responder.read_message(&msg3).unwrap();

        // Ratchet keys
        let mut rng = SecureRng::new();
        let resp_ratchet_priv = PrivateKey::generate(&mut rng);
        let resp_ratchet_pub = resp_ratchet_priv.public_key();

        // Transition to transport mode
        // Initiator needs Responder's public ratchet key
        let mut initiator_transport = initiator
            .into_transport(None, Some(resp_ratchet_pub))
            .unwrap();
        // Responder needs its own private ratchet key
        let mut responder_transport = responder
            .into_transport(Some(resp_ratchet_priv), None)
            .unwrap();

        // Test bidirectional encryption
        let plaintext1 = b"secret message from initiator";
        let ciphertext1 = initiator_transport.write_message(plaintext1).unwrap();
        let decrypted1 = responder_transport.read_message(&ciphertext1).unwrap();
        assert_eq!(decrypted1, plaintext1);

        let plaintext2 = b"secret message from responder";
        let ciphertext2 = responder_transport.write_message(plaintext2).unwrap();
        let decrypted2 = initiator_transport.read_message(&ciphertext2).unwrap();
        assert_eq!(decrypted2, plaintext2);
    }
}
