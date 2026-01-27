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

use crate::{CryptoError, SessionKeys};
use snow::{Builder, HandshakeState, TransportState};
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

impl std::fmt::Display for NoiseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NoiseError::InvalidState => write!(f, "Invalid handshake state"),
            NoiseError::InvalidMessage => write!(f, "Invalid handshake message"),
            NoiseError::DecryptionFailed => write!(f, "Decryption failed"),
            NoiseError::KeyDerivationFailed => write!(f, "Key derivation failed"),
            NoiseError::SnowError(e) => write!(f, "Snow error: {e}"),
        }
    }
}

impl std::error::Error for NoiseError {}

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
    pub fn into_transport(self) -> Result<NoiseTransport, NoiseError> {
        if self.phase != HandshakePhase::Complete {
            return Err(NoiseError::InvalidState);
        }

        let transport = self.state.into_transport_mode()?;
        Ok(NoiseTransport {
            transport,
            role: self.role,
            msg_count: 0,
            last_rekey: std::time::Instant::now(),
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
    transport: TransportState,
    role: Role,
    msg_count: u64,
    last_rekey: std::time::Instant,
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
        let mut message = vec![0u8; payload.len() + 16]; // payload + tag
        let len = self.transport.write_message(payload, &mut message)?;
        message.truncate(len);
        self.msg_count += 1;
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
        if message.len() < 16 {
            return Err(NoiseError::InvalidMessage);
        }
        let mut payload = vec![0u8; message.len() - 16];
        let len = self.transport.read_message(message, &mut payload)?;
        payload.truncate(len);
        Ok(payload)
    }

    /// Get the role this transport was created with.
    #[must_use]
    pub fn role(&self) -> Role {
        self.role
    }

    /// Rekey the sending cipher (for forward secrecy).
    pub fn rekey_send(&mut self) {
        self.transport.rekey_outgoing();
        self.msg_count = 0;
        self.last_rekey = std::time::Instant::now();
    }

    /// Rekey the receiving cipher.
    pub fn rekey_recv(&mut self) {
        self.transport.rekey_incoming();
    }

    /// Check if rekeying is needed (1M packets or 2 minutes)
    pub fn should_rekey(&self) -> bool {
        self.msg_count >= 1_000_000 || self.last_rekey.elapsed().as_secs() >= 120
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
    fn test_handshake_with_payloads() {
        let initiator_keypair = NoiseKeypair::generate().unwrap();
        let responder_keypair = NoiseKeypair::generate().unwrap();

        let mut initiator = NoiseHandshake::new_initiator(&initiator_keypair).unwrap();
        let mut responder = NoiseHandshake::new_responder(&responder_keypair).unwrap();

        // Message 1 with payload
        let payload1 = b"hello from initiator";
        let msg1 = initiator.write_message(payload1).unwrap();
        let received1 = responder.read_message(&msg1).unwrap();
        assert_eq!(received1, payload1);

        // Message 2 with payload
        let payload2 = b"hello from responder";
        let msg2 = responder.write_message(payload2).unwrap();
        let received2 = initiator.read_message(&msg2).unwrap();
        assert_eq!(received2, payload2);

        // Message 3 with payload
        let payload3 = b"final message";
        let msg3 = initiator.write_message(payload3).unwrap();
        let received3 = responder.read_message(&msg3).unwrap();
        assert_eq!(received3, payload3);
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

        // Transition to transport mode
        let mut initiator_transport = initiator.into_transport().unwrap();
        let mut responder_transport = responder.into_transport().unwrap();

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

    #[test]
    fn test_session_keys_derivation() {
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

        // Extract session keys
        let initiator_keys = initiator.into_session_keys().unwrap();
        let responder_keys = responder.into_session_keys().unwrap();

        // Initiator's send key should match responder's recv key
        assert_eq!(initiator_keys.send_key, responder_keys.recv_key);
        // Initiator's recv key should match responder's send key
        assert_eq!(initiator_keys.recv_key, responder_keys.send_key);
        // Chain keys should match
        assert_eq!(initiator_keys.chain_key, responder_keys.chain_key);
    }

    #[test]
    fn test_invalid_state_errors() {
        let keypair = NoiseKeypair::generate().unwrap();

        // Initiator can't read message 1
        let mut initiator = NoiseHandshake::new_initiator(&keypair).unwrap();
        assert!(initiator.read_message(&[0u8; 32]).is_err());

        // Responder can't write message 1
        let mut responder = NoiseHandshake::new_responder(&keypair).unwrap();
        assert!(responder.write_message(&[]).is_err());
    }

    #[test]
    fn test_transport_rekey() {
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

        let mut initiator_transport = initiator.into_transport().unwrap();
        let mut responder_transport = responder.into_transport().unwrap();

        // Send a message before rekey
        let msg_before = b"before rekey";
        let ct1 = initiator_transport.write_message(msg_before).unwrap();
        let pt1 = responder_transport.read_message(&ct1).unwrap();
        assert_eq!(pt1, msg_before);

        // Rekey both sides
        initiator_transport.rekey_send();
        responder_transport.rekey_recv();

        // Send a message after rekey
        let msg_after = b"after rekey";
        let ct2 = initiator_transport.write_message(msg_after).unwrap();
        let pt2 = responder_transport.read_message(&ct2).unwrap();
        assert_eq!(pt2, msg_after);
    }
}
