//! DHT Node Identity and Distance Metric
//!
//! This module provides the NodeId type, which is a 256-bit identifier used in
//! the Kademlia DHT. NodeIds are derived from public keys using BLAKE3 hashing
//! and use XOR distance metric for routing.
//!
//! # S/Kademlia Sybil Resistance
//!
//! This module implements crypto puzzle-based Sybil resistance as described in
//! the S/Kademlia paper. NodeId generation requires computational work (proof-of-work),
//! making it expensive to generate large numbers of fake identities.

use blake3::Hasher;
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::fmt;

/// S/Kademlia crypto puzzle for Sybil resistance
///
/// Requires finding a nonce such that BLAKE3(public_key || nonce) has
/// a specified number of leading zero bits. This makes NodeId generation
/// computationally expensive, preventing Sybil attacks.
#[derive(Clone, Debug)]
pub struct SybilResistance {
    /// Number of leading zero bits required (default: 20)
    difficulty: usize,
}

impl SybilResistance {
    /// Create a new Sybil resistance configuration
    ///
    /// # Arguments
    ///
    /// * `difficulty` - Number of leading zero bits required (recommended: 16-24)
    ///
    /// # Examples
    ///
    /// ```
    /// use wraith_discovery::dht::SybilResistance;
    ///
    /// let resistance = SybilResistance::new(20);
    /// ```
    #[must_use]
    pub const fn new(difficulty: usize) -> Self {
        Self { difficulty }
    }

    /// Create default configuration (20-bit difficulty)
    ///
    /// A 20-bit puzzle requires ~1 million hash attempts on average,
    /// taking approximately 1-2 seconds on a modern CPU.
    #[must_use]
    pub const fn default() -> Self {
        Self::new(20)
    }

    /// Generate a NodeId with proof-of-work
    ///
    /// Finds a nonce such that BLAKE3(public_key || nonce) has the required
    /// number of leading zero bits. The nonce is returned for verification.
    ///
    /// # Arguments
    ///
    /// * `public_key` - 32-byte public key
    ///
    /// # Returns
    ///
    /// Tuple of (NodeId, nonce, puzzle_hash)
    ///
    /// # Examples
    ///
    /// ```
    /// use wraith_discovery::dht::SybilResistance;
    ///
    /// let pubkey = [42u8; 32];
    /// let resistance = SybilResistance::new(16); // Lower difficulty for testing
    /// let (node_id, nonce, _) = resistance.generate_with_puzzle(&pubkey);
    /// assert!(resistance.verify(&pubkey, &node_id, nonce).is_ok());
    /// ```
    #[must_use]
    pub fn generate_with_puzzle(&self, public_key: &[u8; 32]) -> (NodeId, u64, [u8; 32]) {
        // Start nonce from 0 for proof-of-work puzzle solving
        let mut nonce = 0u64;
        loop {
            let mut hasher = Hasher::new();
            hasher.update(public_key);
            hasher.update(&nonce.to_le_bytes());
            hasher.update(b"wraith-dht-puzzle");
            let hash = hasher.finalize();
            let hash_bytes = *hash.as_bytes();

            // Check if hash meets difficulty requirement
            if Self::count_leading_zeros(&hash_bytes) >= self.difficulty {
                // Derive NodeId from puzzle hash
                let mut node_hasher = Hasher::new();
                node_hasher.update(&hash_bytes);
                node_hasher.update(b"wraith-dht-node-id");
                let node_id_hash = node_hasher.finalize();
                let node_id = NodeId(*node_id_hash.as_bytes());
                return (node_id, nonce, hash_bytes);
            }

            // Increment nonce for next iteration of proof-of-work
            nonce += 1;
        }
    }

    /// Verify a NodeId puzzle solution
    ///
    /// Validates that the nonce produces a hash with sufficient leading zeros
    /// and that the NodeId is correctly derived from the puzzle hash.
    ///
    /// # Arguments
    ///
    /// * `public_key` - 32-byte public key
    /// * `node_id` - Claimed NodeId
    /// * `nonce` - Puzzle nonce
    ///
    /// # Errors
    ///
    /// Returns error if verification fails
    ///
    /// # Examples
    ///
    /// ```
    /// use wraith_discovery::dht::SybilResistance;
    ///
    /// let pubkey = [42u8; 32];
    /// let resistance = SybilResistance::new(16);
    /// let (node_id, nonce, _) = resistance.generate_with_puzzle(&pubkey);
    /// assert!(resistance.verify(&pubkey, &node_id, nonce).is_ok());
    /// ```
    pub fn verify(
        &self,
        public_key: &[u8; 32],
        node_id: &NodeId,
        nonce: u64,
    ) -> Result<(), String> {
        // Recompute puzzle hash
        let mut hasher = Hasher::new();
        hasher.update(public_key);
        hasher.update(&nonce.to_le_bytes());
        hasher.update(b"wraith-dht-puzzle");
        let hash = hasher.finalize();
        let hash_bytes = *hash.as_bytes();

        // Verify difficulty
        if Self::count_leading_zeros(&hash_bytes) < self.difficulty {
            return Err(format!(
                "Puzzle difficulty not met: expected {} zero bits",
                self.difficulty
            ));
        }

        // Verify NodeId derivation
        let mut node_hasher = Hasher::new();
        node_hasher.update(&hash_bytes);
        node_hasher.update(b"wraith-dht-node-id");
        let expected_node_id = NodeId(*node_hasher.finalize().as_bytes());

        if *node_id != expected_node_id {
            return Err("NodeId does not match puzzle solution".to_string());
        }

        Ok(())
    }

    /// Count leading zero bits in a byte array
    fn count_leading_zeros(bytes: &[u8; 32]) -> usize {
        let mut count = 0;
        for byte in bytes {
            if *byte == 0 {
                count += 8;
            } else {
                count += byte.leading_zeros() as usize;
                break;
            }
        }
        count
    }
}

impl Default for SybilResistance {
    fn default() -> Self {
        Self::default()
    }
}

/// 256-bit node identifier for Kademlia DHT
///
/// NodeIds are derived from public keys using BLAKE3 hash function.
/// The XOR metric is used for distance calculation, which provides
/// the symmetric and transitive properties required by Kademlia.
///
/// # S/Kademlia Support
///
/// NodeIds can be generated with proof-of-work puzzles for Sybil resistance.
/// Use `SybilResistance::generate_with_puzzle()` for secure generation.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NodeId([u8; 32]);

impl NodeId {
    /// Number of bits in a NodeId
    pub const BITS: usize = 256;

    /// Generate a random NodeId
    ///
    /// This is primarily used for testing and simulation. Production nodes
    /// should derive their IDs from public keys using `from_public_key`.
    ///
    /// # Examples
    ///
    /// ```
    /// use wraith_discovery::dht::NodeId;
    ///
    /// let id = NodeId::random();
    /// assert_eq!(id.as_bytes().len(), 32);
    /// ```
    #[must_use]
    pub fn random() -> Self {
        let mut rng = rand::thread_rng();
        let mut bytes = [0u8; 32];
        rng.fill(&mut bytes[..]);
        Self(bytes)
    }

    /// Generate NodeId from a public key
    ///
    /// Uses BLAKE3 hash to derive a deterministic 256-bit identifier
    /// from a 32-byte public key. This ensures that a peer's NodeId
    /// is tied to their cryptographic identity.
    ///
    /// # Arguments
    ///
    /// * `public_key` - 32-byte public key (X25519 or Ed25519)
    ///
    /// # Examples
    ///
    /// ```
    /// use wraith_discovery::dht::NodeId;
    ///
    /// let pubkey = [42u8; 32];
    /// let id = NodeId::from_public_key(&pubkey);
    /// ```
    #[must_use]
    pub fn from_public_key(public_key: &[u8; 32]) -> Self {
        let mut hasher = Hasher::new();
        hasher.update(public_key);
        hasher.update(b"wraith-dht-node-id"); // Domain separation
        let hash = hasher.finalize();
        Self(*hash.as_bytes())
    }

    /// Calculate XOR distance to another NodeId
    ///
    /// The XOR metric has the following properties:
    /// - d(x, x) = 0 (identity)
    /// - d(x, y) = d(y, x) (symmetry)
    /// - d(x, y) + d(y, z) >= d(x, z) (triangle inequality)
    ///
    /// These properties make it suitable for Kademlia routing.
    ///
    /// # Arguments
    ///
    /// * `other` - The NodeId to measure distance to
    ///
    /// # Examples
    ///
    /// ```
    /// use wraith_discovery::dht::NodeId;
    ///
    /// let id1 = NodeId::from_bytes([1u8; 32]);
    /// let id2 = NodeId::from_bytes([2u8; 32]);
    /// let distance = id1.distance(&id2);
    /// assert_eq!(distance.as_bytes()[0], 3); // 1 XOR 2 = 3
    /// ```
    #[must_use]
    pub fn distance(&self, other: &NodeId) -> NodeId {
        let mut result = [0u8; 32];
        for (i, byte) in result.iter_mut().enumerate() {
            *byte = self.0[i] ^ other.0[i];
        }
        NodeId(result)
    }

    /// Count leading zero bits in the NodeId
    ///
    /// This is used to determine which k-bucket a node belongs to.
    /// The number of leading zeros indicates the bucket index.
    ///
    /// # Returns
    ///
    /// Number of leading zero bits (0-256)
    ///
    /// # Examples
    ///
    /// ```
    /// use wraith_discovery::dht::NodeId;
    ///
    /// let mut bytes = [0u8; 32];
    /// bytes[0] = 0b00001000; // 4 leading zeros
    /// let id = NodeId::from_bytes(bytes);
    /// assert_eq!(id.leading_zeros(), 4);
    /// ```
    #[must_use]
    pub fn leading_zeros(&self) -> usize {
        let mut count = 0;
        for byte in &self.0 {
            if *byte == 0 {
                count += 8;
            } else {
                count += byte.leading_zeros() as usize;
                break;
            }
        }
        count.min(Self::BITS)
    }

    /// Get the bucket index for this NodeId relative to a local ID
    ///
    /// The bucket index is determined by the position of the first
    /// differing bit in the XOR distance. This is equivalent to
    /// `255 - distance.leading_zeros()` for non-zero distances.
    ///
    /// # Arguments
    ///
    /// * `local_id` - The local node's ID
    ///
    /// # Returns
    ///
    /// Bucket index (0-255), or `None` if `NodeId`s are identical
    ///
    /// # Examples
    ///
    /// ```
    /// use wraith_discovery::dht::NodeId;
    ///
    /// let local = NodeId::from_bytes([0u8; 32]);
    /// let mut remote_bytes = [0u8; 32];
    /// remote_bytes[0] = 0b10000000; // First bit differs
    /// let remote = NodeId::from_bytes(remote_bytes);
    /// assert_eq!(remote.bucket_index(&local), Some(255));
    /// ```
    #[must_use]
    pub fn bucket_index(&self, local_id: &NodeId) -> Option<usize> {
        let distance = self.distance(local_id);
        let leading = distance.leading_zeros();
        if leading == Self::BITS {
            None // Identical NodeIds
        } else {
            Some(Self::BITS - 1 - leading)
        }
    }

    /// Get the raw bytes of the NodeId
    ///
    /// # Examples
    ///
    /// ```
    /// use wraith_discovery::dht::NodeId;
    ///
    /// let id = NodeId::from_bytes([42u8; 32]);
    /// assert_eq!(id.as_bytes(), &[42u8; 32]);
    /// ```
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// Create NodeId from raw bytes
    ///
    /// # Arguments
    ///
    /// * `bytes` - 32-byte array
    ///
    /// # Examples
    ///
    /// ```
    /// use wraith_discovery::dht::NodeId;
    ///
    /// let id = NodeId::from_bytes([1u8; 32]);
    /// assert_eq!(id.as_bytes(), &[1u8; 32]);
    /// ```
    #[must_use]
    pub const fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }
}

impl fmt::Debug for NodeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "NodeId({})", hex::encode(&self.0[..8]))
    }
}

impl fmt::Display for NodeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", hex::encode(&self.0[..8]))
    }
}

impl PartialOrd for NodeId {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for NodeId {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.cmp(&other.0)
    }
}

/// Helper module for hex encoding (simplified)
mod hex {
    pub fn encode(bytes: &[u8]) -> String {
        bytes.iter().map(|b| format!("{b:02x}")).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_node_id_from_public_key() {
        let pubkey = [42u8; 32];
        let id1 = NodeId::from_public_key(&pubkey);
        let id2 = NodeId::from_public_key(&pubkey);
        assert_eq!(id1, id2, "Same pubkey should produce same NodeId");
    }

    #[test]
    fn test_node_id_random_unique() {
        let id1 = NodeId::random();
        let id2 = NodeId::random();
        assert_ne!(id1, id2, "Random NodeIds should be unique");
    }

    #[test]
    fn test_xor_distance() {
        let id1 = NodeId::from_bytes([1u8; 32]);
        let id2 = NodeId::from_bytes([2u8; 32]);
        let distance = id1.distance(&id2);

        // 1 XOR 2 = 3
        assert_eq!(distance.as_bytes()[0], 3);
        for i in 1..32 {
            assert_eq!(distance.as_bytes()[i], 3);
        }
    }

    #[test]
    fn test_xor_distance_symmetry() {
        let id1 = NodeId::random();
        let id2 = NodeId::random();
        assert_eq!(id1.distance(&id2), id2.distance(&id1));
    }

    #[test]
    fn test_xor_distance_identity() {
        let id = NodeId::random();
        let zero = NodeId::from_bytes([0u8; 32]);
        assert_eq!(id.distance(&id), zero);
    }

    #[test]
    fn test_leading_zeros() {
        let mut bytes = [0u8; 32];
        bytes[0] = 0b10000000;
        let id = NodeId::from_bytes(bytes);
        assert_eq!(id.leading_zeros(), 0);

        let mut bytes = [0u8; 32];
        bytes[0] = 0b01000000;
        let id = NodeId::from_bytes(bytes);
        assert_eq!(id.leading_zeros(), 1);

        let mut bytes = [0u8; 32];
        bytes[0] = 0b00000001;
        let id = NodeId::from_bytes(bytes);
        assert_eq!(id.leading_zeros(), 7);

        let mut bytes = [0u8; 32];
        bytes[1] = 0b10000000;
        let id = NodeId::from_bytes(bytes);
        assert_eq!(id.leading_zeros(), 8);

        let zero = NodeId::from_bytes([0u8; 32]);
        assert_eq!(zero.leading_zeros(), 256);
    }

    #[test]
    fn test_bucket_index() {
        let local = NodeId::from_bytes([0u8; 32]);

        // First bit differs (bucket 255)
        let mut bytes = [0u8; 32];
        bytes[0] = 0b10000000;
        let remote = NodeId::from_bytes(bytes);
        assert_eq!(remote.bucket_index(&local), Some(255));

        // Second bit differs (bucket 254)
        let mut bytes = [0u8; 32];
        bytes[0] = 0b01000000;
        let remote = NodeId::from_bytes(bytes);
        assert_eq!(remote.bucket_index(&local), Some(254));

        // Ninth bit differs (bucket 247)
        let mut bytes = [0u8; 32];
        bytes[1] = 0b10000000;
        let remote = NodeId::from_bytes(bytes);
        assert_eq!(remote.bucket_index(&local), Some(247));

        // Identical nodes
        assert_eq!(local.bucket_index(&local), None);
    }

    #[test]
    fn test_bucket_index_all_buckets() {
        let local = NodeId::from_bytes([0u8; 32]);

        for bucket in 0..256 {
            let byte_index = 31 - (bucket / 8);
            let bit_index = bucket % 8;

            let mut bytes = [0u8; 32];
            bytes[byte_index] = 1 << bit_index;

            let remote = NodeId::from_bytes(bytes);
            assert_eq!(remote.bucket_index(&local), Some(bucket));
        }
    }

    #[test]
    fn test_as_bytes() {
        let bytes = [42u8; 32];
        let id = NodeId::from_bytes(bytes);
        assert_eq!(id.as_bytes(), &bytes);
    }

    #[test]
    fn test_ordering() {
        let id1 = NodeId::from_bytes([1u8; 32]);
        let id2 = NodeId::from_bytes([2u8; 32]);
        assert!(id1 < id2);
        assert!(id2 > id1);
        assert_eq!(id1, id1);
    }

    #[test]
    fn test_debug_display() {
        let mut bytes = [0u8; 32];
        bytes[0] = 0xAB;
        bytes[1] = 0xCD;
        bytes[2] = 0xEF;
        bytes[3] = 0x01;
        bytes[4] = 0x23;
        bytes[5] = 0x45;
        bytes[6] = 0x67;
        bytes[7] = 0x89;
        let id = NodeId::from_bytes(bytes);
        let debug_str = format!("{:?}", id);
        let display_str = format!("{}", id);
        assert!(debug_str.contains("abcdef"));
        assert!(display_str.contains("abcdef"));
    }

    // SEC-001: S/Kademlia Sybil Resistance Tests
    #[test]
    fn test_sybil_resistance_generate_and_verify() {
        let pubkey = [42u8; 32];
        let resistance = SybilResistance::new(12); // Lower difficulty for tests
        let (node_id, nonce, puzzle_hash) = resistance.generate_with_puzzle(&pubkey);

        // Verify the solution
        assert!(resistance.verify(&pubkey, &node_id, nonce).is_ok());

        // Verify puzzle hash has sufficient leading zeros
        assert!(SybilResistance::count_leading_zeros(&puzzle_hash) >= 12);
    }

    #[test]
    fn test_sybil_resistance_invalid_nonce() {
        let pubkey = [42u8; 32];
        let resistance = SybilResistance::new(12);
        let (node_id, nonce, _) = resistance.generate_with_puzzle(&pubkey);

        // Wrong nonce should fail verification (nonce + 1 is intentionally invalid test data)
        assert!(resistance.verify(&pubkey, &node_id, nonce + 1).is_err());
    }

    #[test]
    fn test_sybil_resistance_invalid_node_id() {
        let pubkey = [42u8; 32];
        let resistance = SybilResistance::new(12);
        let (_, nonce, _) = resistance.generate_with_puzzle(&pubkey);

        // Wrong NodeId should fail verification
        let wrong_node_id = NodeId::random();
        assert!(resistance.verify(&pubkey, &wrong_node_id, nonce).is_err());
    }

    #[test]
    fn test_sybil_resistance_different_difficulties() {
        let pubkey = [42u8; 32];

        // Test different difficulty levels
        for difficulty in [8, 12, 16] {
            let resistance = SybilResistance::new(difficulty);
            let (node_id, nonce, puzzle_hash) = resistance.generate_with_puzzle(&pubkey);

            // Verify solution
            assert!(resistance.verify(&pubkey, &node_id, nonce).is_ok());

            // Verify difficulty
            assert!(SybilResistance::count_leading_zeros(&puzzle_hash) >= difficulty);
        }
    }

    #[test]
    fn test_sybil_resistance_count_leading_zeros() {
        let mut bytes = [0u8; 32];
        assert_eq!(SybilResistance::count_leading_zeros(&bytes), 256);

        bytes[0] = 0b1000_0000;
        assert_eq!(SybilResistance::count_leading_zeros(&bytes), 0);

        bytes = [0u8; 32];
        bytes[0] = 0b0100_0000;
        assert_eq!(SybilResistance::count_leading_zeros(&bytes), 1);

        bytes = [0u8; 32];
        bytes[0] = 0b0000_0001;
        assert_eq!(SybilResistance::count_leading_zeros(&bytes), 7);

        bytes = [0u8; 32];
        bytes[2] = 0b1000_0000;
        assert_eq!(SybilResistance::count_leading_zeros(&bytes), 16);
    }

    #[test]
    fn test_sybil_resistance_default() {
        let resistance1 = SybilResistance::default();
        let resistance2 = SybilResistance::new(20);

        let pubkey = [42u8; 32];
        let (node_id, nonce, _) = resistance2.generate_with_puzzle(&pubkey);

        // Both should verify the same solution
        assert!(resistance1.verify(&pubkey, &node_id, nonce).is_ok());
    }
}
