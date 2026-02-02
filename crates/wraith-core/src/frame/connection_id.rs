//! 128-bit Connection ID for the WRAITH v2 protocol.
//!
//! Expands the v1 64-bit `ConnectionId` to a 128-bit value for stronger
//! collision resistance and improved privacy. Provides migration helpers
//! for converting v1 connection IDs (zero-extended).

use crate::session::ConnectionId;
use rand::Rng;

/// 128-bit Connection ID for v2 session demultiplexing.
///
/// The v2 Connection ID is 16 bytes (128 bits), providing:
/// - Cryptographically random generation with negligible collision probability
/// - v1 migration support via zero-extension
/// - Rotating CID support using XOR with sequence-derived masks
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ConnectionIdV2([u8; 16]);

impl ConnectionIdV2 {
    /// Size of a v2 connection ID in bytes.
    pub const SIZE: usize = 16;

    /// Invalid (all-zero) connection ID.
    pub const INVALID: Self = Self([0u8; 16]);

    /// Handshake initiation connection ID (all 0xFF).
    pub const HANDSHAKE: Self = Self([0xFF; 16]);

    /// Version negotiation connection ID.
    pub const VERSION_NEGOTIATION: Self = Self([
        0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
        0xFE,
    ]);

    /// Stateless reset connection ID.
    pub const STATELESS_RESET: Self = Self([
        0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
        0xFD,
    ]);

    /// Generate a cryptographically random connection ID.
    ///
    /// Uses `getrandom` for OS-level randomness.
    #[must_use]
    pub fn generate() -> Self {
        let mut bytes = [0u8; 16];
        rand::thread_rng().fill(&mut bytes);
        // Ensure we don't accidentally generate a special value
        let cid = Self(bytes);
        if cid.is_special() || cid == Self::INVALID {
            // Extremely unlikely, but handle gracefully by flipping a bit
            let mut retry = bytes;
            retry[0] ^= 0x01;
            Self(retry)
        } else {
            cid
        }
    }

    /// Create from raw bytes.
    #[must_use]
    pub const fn from_bytes(bytes: [u8; 16]) -> Self {
        Self(bytes)
    }

    /// Get raw bytes.
    #[must_use]
    pub const fn to_bytes(self) -> [u8; 16] {
        self.0
    }

    /// Get a reference to the raw bytes.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8; 16] {
        &self.0
    }

    /// Create a v2 connection ID from a v1 64-bit connection ID.
    ///
    /// Zero-extends the v1 CID into the high 8 bytes, with the low 8 bytes
    /// set to zero. This makes migrated v1 CIDs detectable.
    #[must_use]
    pub fn from_v1(v1: ConnectionId) -> Self {
        let mut bytes = [0u8; 16];
        bytes[..8].copy_from_slice(&v1.to_bytes());
        // Low 8 bytes remain zero -- indicates v1 migration
        Self(bytes)
    }

    /// Check if this connection ID was migrated from v1.
    ///
    /// A migrated v1 CID has all-zero lower 8 bytes and non-zero upper 8 bytes.
    #[must_use]
    pub fn is_migrated_v1(&self) -> bool {
        let upper_nonzero = self.0[..8] != [0u8; 8];
        let lower_zero = self.0[8..] == [0u8; 8];
        upper_nonzero && lower_zero
    }

    /// Extract the original v1 connection ID if this was migrated.
    ///
    /// Returns `None` if this is not a migrated v1 connection ID.
    #[must_use]
    pub fn to_v1(&self) -> Option<ConnectionId> {
        if self.is_migrated_v1() {
            let mut bytes = [0u8; 8];
            bytes.copy_from_slice(&self.0[..8]);
            Some(ConnectionId::from_bytes(bytes))
        } else {
            None
        }
    }

    /// Check if this is a special connection ID (handshake, version negotiation, etc.).
    #[must_use]
    pub fn is_special(&self) -> bool {
        *self == Self::HANDSHAKE
            || *self == Self::VERSION_NEGOTIATION
            || *self == Self::STATELESS_RESET
    }

    /// Check if this is a valid (non-zero, non-special) connection ID.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        *self != Self::INVALID && !self.is_special()
    }

    /// Create a rotating connection ID by XOR-ing with a sequence-derived mask.
    ///
    /// XORs the lower 8 bytes with the sequence number to prevent tracking.
    #[must_use]
    pub fn rotate(self, seq_num: u64) -> Self {
        let mut bytes = self.0;
        let seq_bytes = seq_num.to_be_bytes();
        for i in 0..8 {
            bytes[8 + i] ^= seq_bytes[i];
        }
        Self(bytes)
    }

    /// Serialize to a byte slice (writes 16 bytes).
    pub fn write_to(&self, buf: &mut [u8]) {
        buf[..16].copy_from_slice(&self.0);
    }

    /// Deserialize from a byte slice (reads 16 bytes).
    ///
    /// # Panics
    ///
    /// Panics if `buf.len() < 16`.
    #[must_use]
    pub fn read_from(buf: &[u8]) -> Self {
        let mut bytes = [0u8; 16];
        bytes.copy_from_slice(&buf[..16]);
        Self(bytes)
    }
}

impl core::fmt::Display for ConnectionIdV2 {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        for byte in &self.0 {
            write!(f, "{byte:02x}")?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_is_valid() {
        let cid = ConnectionIdV2::generate();
        assert!(cid.is_valid());
        assert_ne!(cid, ConnectionIdV2::INVALID);
    }

    #[test]
    fn test_generate_uniqueness() {
        let cid1 = ConnectionIdV2::generate();
        let cid2 = ConnectionIdV2::generate();
        assert_ne!(cid1, cid2);
    }

    #[test]
    fn test_from_bytes_roundtrip() {
        let bytes = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16];
        let cid = ConnectionIdV2::from_bytes(bytes);
        assert_eq!(cid.to_bytes(), bytes);
    }

    #[test]
    fn test_special_values() {
        assert!(!ConnectionIdV2::INVALID.is_valid());
        assert!(ConnectionIdV2::HANDSHAKE.is_special());
        assert!(ConnectionIdV2::VERSION_NEGOTIATION.is_special());
        assert!(ConnectionIdV2::STATELESS_RESET.is_special());
        assert!(!ConnectionIdV2::HANDSHAKE.is_valid());
    }

    #[test]
    fn test_from_v1() {
        let v1 = ConnectionId::from_bytes([0x12, 0x34, 0x56, 0x78, 0x9A, 0xBC, 0xDE, 0xF0]);
        let v2 = ConnectionIdV2::from_v1(v1);

        // Upper 8 bytes should match v1
        assert_eq!(&v2.to_bytes()[..8], &v1.to_bytes());
        // Lower 8 bytes should be zero
        assert_eq!(&v2.to_bytes()[8..], &[0u8; 8]);
    }

    #[test]
    fn test_is_migrated_v1() {
        let v1 = ConnectionId::from_bytes([1, 2, 3, 4, 5, 6, 7, 8]);
        let migrated = ConnectionIdV2::from_v1(v1);
        assert!(migrated.is_migrated_v1());

        // A fully random CID should not appear migrated (extremely unlikely)
        let random = ConnectionIdV2::generate();
        // Can't assert !is_migrated_v1 for random since lower 8 could theoretically be zero,
        // but it's astronomically unlikely. Test the logic instead.
        let non_migrated =
            ConnectionIdV2::from_bytes([1, 2, 3, 4, 5, 6, 7, 8, 0, 0, 0, 0, 0, 0, 0, 1]);
        assert!(!non_migrated.is_migrated_v1());
        let _ = random; // use variable
    }

    #[test]
    fn test_to_v1_roundtrip() {
        let original_v1 =
            ConnectionId::from_bytes([0xAB, 0xCD, 0xEF, 0x01, 0x23, 0x45, 0x67, 0x89]);
        let v2 = ConnectionIdV2::from_v1(original_v1);
        let recovered = v2.to_v1().expect("should be migrated v1");
        assert_eq!(recovered, original_v1);
    }

    #[test]
    fn test_to_v1_non_migrated() {
        let cid =
            ConnectionIdV2::from_bytes([1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16]);
        assert!(cid.to_v1().is_none());
    }

    #[test]
    fn test_rotate() {
        let cid =
            ConnectionIdV2::from_bytes([1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16]);
        let rotated = cid.rotate(0x1111111111111111);

        // Upper 8 bytes unchanged
        assert_eq!(&cid.to_bytes()[..8], &rotated.to_bytes()[..8]);
        // Lower 8 bytes changed
        assert_ne!(&cid.to_bytes()[8..], &rotated.to_bytes()[8..]);

        // Double rotation with same seq restores original
        let restored = rotated.rotate(0x1111111111111111);
        assert_eq!(cid, restored);
    }

    #[test]
    fn test_write_read_roundtrip() {
        let cid = ConnectionIdV2::generate();
        let mut buf = [0u8; 16];
        cid.write_to(&mut buf);
        let recovered = ConnectionIdV2::read_from(&buf);
        assert_eq!(cid, recovered);
    }

    #[test]
    fn test_display() {
        let cid = ConnectionIdV2::from_bytes([
            0x01, 0x23, 0x45, 0x67, 0x89, 0xAB, 0xCD, 0xEF, 0x01, 0x23, 0x45, 0x67, 0x89, 0xAB,
            0xCD, 0xEF,
        ]);
        let s = format!("{cid}");
        assert_eq!(s, "0123456789abcdef0123456789abcdef");
    }

    #[test]
    fn test_invalid_cid_not_migrated() {
        // All zeros should not count as migrated (upper bytes are zero)
        assert!(!ConnectionIdV2::INVALID.is_migrated_v1());
    }

    #[test]
    fn test_size_constant() {
        assert_eq!(ConnectionIdV2::SIZE, 16);
        assert_eq!(ConnectionIdV2::INVALID.to_bytes().len(), 16);
    }

    #[test]
    fn test_generate_many_unique() {
        let mut cids = std::collections::HashSet::new();
        for _ in 0..100 {
            let cid = ConnectionIdV2::generate();
            assert!(cids.insert(cid), "Duplicate CID generated");
        }
    }
}
