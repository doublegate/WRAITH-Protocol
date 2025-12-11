//! Identity management for WRAITH nodes
//!
//! This module provides identity types and functionality for node identification
//! and cryptographic key management.
//!
//! # Key Types
//!
//! WRAITH nodes use two key types:
//! - **Ed25519**: For node identity (node ID derived from public key)
//! - **X25519**: For Noise handshakes (session establishment)
//!
//! # Example
//!
//! ```
//! use wraith_core::node::identity::Identity;
//!
//! let identity = Identity::generate().expect("Failed to generate identity");
//! println!("Node ID: {:?}", hex::encode(identity.public_key()));
//! ```

use crate::node::error::{NodeError, Result};
use wraith_crypto::noise::NoiseKeypair;
use wraith_crypto::signatures::SigningKey as Ed25519SigningKey;

/// Transfer ID (32-byte unique identifier)
///
/// Used to uniquely identify file transfers across the network.
/// Generated randomly for each new transfer.
pub type TransferId = [u8; 32];

/// Peer ID (32-byte unique identifier)
///
/// Used to identify peers in the network (typically an X25519 public key).
pub type PeerId = [u8; 32];

/// Error type for parsing hex-encoded identifiers
#[derive(Debug, thiserror::Error)]
pub enum ParseError {
    /// Invalid hexadecimal encoding
    #[error("Invalid hex encoding: {0}")]
    InvalidHex(#[from] hex::FromHexError),

    /// Invalid length for the identifier
    #[error("Invalid length: expected {expected} bytes, got {actual}")]
    InvalidLength {
        /// Expected number of bytes
        expected: usize,
        /// Actual number of bytes
        actual: usize,
    },
}

/// Parse a peer ID from hex string (with optional 0x prefix)
///
/// Accepts hex strings with or without "0x" prefix and converts them to a 32-byte array.
///
/// # Arguments
///
/// * `input` - Hex string (with or without "0x" prefix)
///
/// # Errors
///
/// Returns an error if:
/// - The input is not valid hexadecimal
/// - The decoded bytes are not exactly 32 bytes
///
/// # Example
///
/// ```
/// use wraith_core::node::identity::parse_peer_id;
///
/// // With 0x prefix
/// let peer_id = parse_peer_id("0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef").unwrap();
/// assert_eq!(peer_id.len(), 32);
///
/// // Without prefix
/// let peer_id = parse_peer_id("1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef").unwrap();
/// assert_eq!(peer_id.len(), 32);
/// ```
pub fn parse_peer_id(input: &str) -> std::result::Result<PeerId, ParseError> {
    parse_fixed_array(input, "Peer ID")
}

/// Parse a transfer ID from hex string (with optional 0x prefix)
///
/// Accepts hex strings with or without "0x" prefix and converts them to a 32-byte array.
///
/// # Arguments
///
/// * `input` - Hex string (with or without "0x" prefix)
///
/// # Errors
///
/// Returns an error if:
/// - The input is not valid hexadecimal
/// - The decoded bytes are not exactly 32 bytes
///
/// # Example
///
/// ```
/// use wraith_core::node::identity::parse_transfer_id;
///
/// // With 0x prefix
/// let transfer_id = parse_transfer_id("0xabcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890").unwrap();
/// assert_eq!(transfer_id.len(), 32);
///
/// // Without prefix
/// let transfer_id = parse_transfer_id("abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890").unwrap();
/// assert_eq!(transfer_id.len(), 32);
/// ```
pub fn parse_transfer_id(input: &str) -> std::result::Result<TransferId, ParseError> {
    parse_fixed_array(input, "Transfer ID")
}

/// Internal helper to parse a fixed-size array from hex string
fn parse_fixed_array<const N: usize>(
    input: &str,
    _name: &str,
) -> std::result::Result<[u8; N], ParseError> {
    let input = input.trim();
    let bytes = if input.starts_with("0x") || input.starts_with("0X") {
        hex::decode(&input[2..])?
    } else {
        hex::decode(input)?
    };

    if bytes.len() != N {
        return Err(ParseError::InvalidLength {
            expected: N,
            actual: bytes.len(),
        });
    }

    let mut result = [0u8; N];
    result.copy_from_slice(&bytes);
    Ok(result)
}

/// Node identity containing cryptographic keypairs
///
/// The identity combines an Ed25519 keypair (for node identification) with
/// an X25519 keypair (for Noise handshakes). The node ID is derived from
/// the Ed25519 public key.
///
/// # Security
///
/// - Ed25519 provides 128-bit security for signatures
/// - X25519 provides 128-bit security for key exchange
/// - Both keypairs are generated using a cryptographically secure RNG
///
/// # Example
///
/// ```
/// use wraith_core::node::identity::Identity;
///
/// // Generate a new random identity
/// let identity = Identity::generate().expect("Failed to generate identity");
///
/// // Access the node ID (Ed25519 public key)
/// let node_id = identity.public_key();
/// assert_eq!(node_id.len(), 32);
///
/// // Access the X25519 keypair for Noise handshakes
/// let _noise_keypair = identity.x25519_keypair();
/// ```
#[derive(Clone)]
pub struct Identity {
    /// Node ID (derived from Ed25519 public key)
    node_id: [u8; 32],

    /// X25519 keypair for Noise handshakes
    x25519: NoiseKeypair,
}

impl Identity {
    /// Generate a random identity
    ///
    /// Creates a new identity with randomly generated Ed25519 and X25519 keypairs.
    /// The node ID is derived from the Ed25519 public key.
    ///
    /// # Errors
    ///
    /// Returns an error if key generation fails (e.g., insufficient entropy).
    ///
    /// # Example
    ///
    /// ```
    /// use wraith_core::node::identity::Identity;
    ///
    /// let identity = Identity::generate().expect("Failed to generate identity");
    /// assert_eq!(identity.public_key().len(), 32);
    /// ```
    pub fn generate() -> Result<Self> {
        use rand_core::OsRng;

        // Generate Ed25519 keypair and extract public key as node ID
        let ed25519 = Ed25519SigningKey::generate(&mut OsRng);
        let node_id = ed25519.verifying_key().to_bytes();
        // Note: We don't store the signing key, only use the public key as node ID

        // Generate X25519 keypair for Noise handshakes
        let x25519 = NoiseKeypair::generate().map_err(|e| NodeError::Crypto(e.to_string()))?;

        Ok(Self { node_id, x25519 })
    }

    /// Create identity from existing components
    ///
    /// This is useful for restoring a previously saved identity or for testing.
    ///
    /// # Arguments
    ///
    /// * `node_id` - 32-byte node identifier
    /// * `x25519` - X25519 keypair for Noise handshakes
    ///
    /// # Example
    ///
    /// ```
    /// use wraith_core::node::identity::Identity;
    /// use wraith_crypto::noise::NoiseKeypair;
    ///
    /// let x25519 = NoiseKeypair::generate().unwrap();
    /// let node_id = [0u8; 32];
    /// let identity = Identity::from_components(node_id, x25519);
    /// ```
    pub fn from_components(node_id: [u8; 32], x25519: NoiseKeypair) -> Self {
        Self { node_id, x25519 }
    }

    /// Get the node's public key (node ID)
    ///
    /// Returns the Ed25519 public key used as the node's unique identifier.
    ///
    /// # Note
    ///
    /// For session lookups, use [`Self::x25519_public_key`] instead, since
    /// sessions are keyed by X25519 public keys from the Noise handshake.
    #[must_use]
    pub fn public_key(&self) -> &[u8; 32] {
        &self.node_id
    }

    /// Get the node's X25519 public key
    ///
    /// Returns the X25519 public key used in Noise handshakes.
    /// This is the key that identifies the node in sessions.
    #[must_use]
    pub fn x25519_public_key(&self) -> &[u8; 32] {
        self.x25519.public_key()
    }

    /// Get the X25519 keypair for Noise handshakes
    ///
    /// Returns a reference to the full keypair, including the private key.
    /// This is needed for performing Noise handshakes.
    #[must_use]
    pub fn x25519_keypair(&self) -> &NoiseKeypair {
        &self.x25519
    }
}

impl std::fmt::Debug for Identity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Identity")
            .field("node_id", &hex::encode(&self.node_id[..8]))
            .field(
                "x25519_public",
                &hex::encode(&self.x25519.public_key()[..8]),
            )
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_identity_generation() {
        let identity = Identity::generate().unwrap();
        assert_eq!(identity.public_key().len(), 32);
        assert_eq!(identity.x25519_public_key().len(), 32);
    }

    #[test]
    fn test_identity_unique() {
        let id1 = Identity::generate().unwrap();
        let id2 = Identity::generate().unwrap();

        // Each identity should be unique
        assert_ne!(id1.public_key(), id2.public_key());
        assert_ne!(id1.x25519_public_key(), id2.x25519_public_key());
    }

    #[test]
    fn test_identity_from_components() {
        let x25519 = NoiseKeypair::generate().unwrap();
        let x25519_pub = *x25519.public_key();
        let node_id = [42u8; 32];

        let identity = Identity::from_components(node_id, x25519);

        assert_eq!(*identity.public_key(), node_id);
        assert_eq!(*identity.x25519_public_key(), x25519_pub);
    }

    #[test]
    fn test_identity_debug() {
        let identity = Identity::generate().unwrap();
        let debug = format!("{:?}", identity);

        assert!(debug.contains("Identity"));
        assert!(debug.contains("node_id"));
        assert!(debug.contains("x25519_public"));
    }

    #[test]
    fn test_identity_clone() {
        let identity = Identity::generate().unwrap();
        let cloned = identity.clone();

        assert_eq!(identity.public_key(), cloned.public_key());
        assert_eq!(identity.x25519_public_key(), cloned.x25519_public_key());
    }

    #[test]
    fn test_transfer_id_type() {
        let transfer_id: TransferId = [0u8; 32];
        assert_eq!(transfer_id.len(), 32);
    }

    #[test]
    fn test_parse_peer_id_with_prefix() {
        let input = "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef";
        let result = parse_peer_id(input).unwrap();
        assert_eq!(result.len(), 32);
        assert_eq!(result[0], 0x12);
        assert_eq!(result[1], 0x34);
    }

    #[test]
    fn test_parse_peer_id_without_prefix() {
        let input = "1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef";
        let result = parse_peer_id(input).unwrap();
        assert_eq!(result.len(), 32);
        assert_eq!(result[0], 0x12);
        assert_eq!(result[1], 0x34);
    }

    #[test]
    fn test_parse_peer_id_uppercase_prefix() {
        let input = "0X1234567890ABCDEF1234567890ABCDEF1234567890ABCDEF1234567890ABCDEF";
        let result = parse_peer_id(input).unwrap();
        assert_eq!(result.len(), 32);
    }

    #[test]
    fn test_parse_peer_id_invalid_length() {
        let input = "0x1234"; // Too short
        let result = parse_peer_id(input);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ParseError::InvalidLength { .. }
        ));
    }

    #[test]
    fn test_parse_peer_id_invalid_hex() {
        let input = "0xzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz";
        let result = parse_peer_id(input);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ParseError::InvalidHex(_)));
    }

    #[test]
    fn test_parse_transfer_id_with_prefix() {
        let input = "0xabcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890";
        let result = parse_transfer_id(input).unwrap();
        assert_eq!(result.len(), 32);
        assert_eq!(result[0], 0xab);
        assert_eq!(result[1], 0xcd);
    }

    #[test]
    fn test_parse_transfer_id_without_prefix() {
        let input = "abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890";
        let result = parse_transfer_id(input).unwrap();
        assert_eq!(result.len(), 32);
        assert_eq!(result[0], 0xab);
        assert_eq!(result[1], 0xcd);
    }

    #[test]
    fn test_parse_transfer_id_whitespace() {
        let input = " \t0xabcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890  \n";
        let result = parse_transfer_id(input).unwrap();
        assert_eq!(result.len(), 32);
    }
}
