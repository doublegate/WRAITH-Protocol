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
        let x25519 = NoiseKeypair::generate()
            .map_err(|e| NodeError::Crypto(wraith_crypto::CryptoError::Handshake(e.to_string())))?;

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
    pub fn public_key(&self) -> &[u8; 32] {
        &self.node_id
    }

    /// Get the node's X25519 public key
    ///
    /// Returns the X25519 public key used in Noise handshakes.
    /// This is the key that identifies the node in sessions.
    pub fn x25519_public_key(&self) -> &[u8; 32] {
        self.x25519.public_key()
    }

    /// Get the X25519 keypair for Noise handshakes
    ///
    /// Returns a reference to the full keypair, including the private key.
    /// This is needed for performing Noise handshakes.
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
}
