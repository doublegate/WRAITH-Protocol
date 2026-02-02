//! Cryptographic suite negotiation for the WRAITH v2 protocol.
//!
//! Defines four cipher suites with different security and performance
//! trade-offs. Suites are negotiated during the handshake by selecting
//! the strongest suite supported by both peers.

use core::fmt;
use serde::{Deserialize, Serialize};

/// AEAD algorithm identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AeadAlgorithm {
    /// XChaCha20-Poly1305 (256-bit key, 192-bit nonce).
    XChaCha20Poly1305,
    /// AES-256-GCM (256-bit key, 96-bit nonce). Requires hardware AES-NI.
    Aes256Gcm,
}

/// KEM algorithm identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum KemAlgorithm {
    /// Hybrid X25519 + ML-KEM-768 (128-bit classical + NIST Level 3 PQ).
    HybridX25519MlKem768,
    /// Hybrid X25519 + ML-KEM-1024 (128-bit classical + NIST Level 5 PQ).
    HybridX25519MlKem1024,
    /// Classical X25519 only (128-bit security, no PQ protection).
    ClassicalX25519,
}

/// Cryptographic suite for the WRAITH v2 protocol.
///
/// Suites are ordered by security strength (A > B > C > D in terms of
/// PQ security; B may outperform A on hardware with AES-NI).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CryptoSuite {
    /// Suite A: Default. X25519 + ML-KEM-768, XChaCha20-Poly1305, BLAKE3.
    SuiteA,
    /// Suite B: Hardware-accelerated. X25519 + ML-KEM-768, AES-256-GCM, BLAKE3.
    SuiteB,
    /// Suite C: Maximum security. X25519 + ML-KEM-1024, XChaCha20-Poly1305, BLAKE3.
    /// Note: ML-KEM-1024 not yet implemented; falls back to ML-KEM-768.
    SuiteC,
    /// Suite D: Classical only. X25519, XChaCha20-Poly1305, BLAKE3.
    /// Fallback for legacy interoperability or constrained environments.
    SuiteD,
}

impl CryptoSuite {
    /// Negotiate the strongest common suite between local and remote supported sets.
    ///
    /// Both lists are treated as sets of supported suites. The strongest suite
    /// present in both sets is selected, using the priority order:
    /// C > A > B > D (strongest to weakest).
    ///
    /// Returns `None` if no common suite exists.
    #[must_use]
    pub fn negotiate(local: &[CryptoSuite], remote: &[CryptoSuite]) -> Option<CryptoSuite> {
        // Priority order: C (max security) > A (default) > B (hw accel) > D (classical)
        let priority = [
            CryptoSuite::SuiteC,
            CryptoSuite::SuiteA,
            CryptoSuite::SuiteB,
            CryptoSuite::SuiteD,
        ];

        for suite in &priority {
            if local.contains(suite) && remote.contains(suite) {
                return Some(*suite);
            }
        }

        None
    }

    /// Returns `true` if this suite includes post-quantum key exchange.
    #[must_use]
    pub fn supports_post_quantum(self) -> bool {
        matches!(
            self,
            CryptoSuite::SuiteA | CryptoSuite::SuiteB | CryptoSuite::SuiteC
        )
    }

    /// Get the AEAD algorithm used by this suite.
    #[must_use]
    pub fn aead_algorithm(self) -> AeadAlgorithm {
        match self {
            CryptoSuite::SuiteB => AeadAlgorithm::Aes256Gcm,
            _ => AeadAlgorithm::XChaCha20Poly1305,
        }
    }

    /// Get the KEM algorithm used by this suite.
    #[must_use]
    pub fn kem_algorithm(self) -> KemAlgorithm {
        match self {
            CryptoSuite::SuiteA | CryptoSuite::SuiteB => KemAlgorithm::HybridX25519MlKem768,
            CryptoSuite::SuiteC => KemAlgorithm::HybridX25519MlKem1024,
            CryptoSuite::SuiteD => KemAlgorithm::ClassicalX25519,
        }
    }

    /// Return the numeric identifier for wire encoding.
    #[must_use]
    pub fn to_id(self) -> u8 {
        match self {
            CryptoSuite::SuiteA => 0x01,
            CryptoSuite::SuiteB => 0x02,
            CryptoSuite::SuiteC => 0x03,
            CryptoSuite::SuiteD => 0x04,
        }
    }

    /// Parse a suite from its wire identifier.
    ///
    /// Returns `None` for unknown identifiers.
    #[must_use]
    pub fn from_id(id: u8) -> Option<Self> {
        match id {
            0x01 => Some(CryptoSuite::SuiteA),
            0x02 => Some(CryptoSuite::SuiteB),
            0x03 => Some(CryptoSuite::SuiteC),
            0x04 => Some(CryptoSuite::SuiteD),
            _ => None,
        }
    }

    /// Return all defined suites in priority order (strongest first).
    #[must_use]
    pub fn all() -> &'static [CryptoSuite] {
        &[
            CryptoSuite::SuiteC,
            CryptoSuite::SuiteA,
            CryptoSuite::SuiteB,
            CryptoSuite::SuiteD,
        ]
    }
}

impl fmt::Display for CryptoSuite {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CryptoSuite::SuiteA => write!(f, "Suite A (X25519+ML-KEM-768, XChaCha20-Poly1305)"),
            CryptoSuite::SuiteB => write!(f, "Suite B (X25519+ML-KEM-768, AES-256-GCM)"),
            CryptoSuite::SuiteC => write!(f, "Suite C (X25519+ML-KEM-1024, XChaCha20-Poly1305)"),
            CryptoSuite::SuiteD => write!(f, "Suite D (X25519, XChaCha20-Poly1305)"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_negotiate_strongest_common() {
        let local = [CryptoSuite::SuiteA, CryptoSuite::SuiteD];
        let remote = [
            CryptoSuite::SuiteA,
            CryptoSuite::SuiteB,
            CryptoSuite::SuiteD,
        ];

        assert_eq!(
            CryptoSuite::negotiate(&local, &remote),
            Some(CryptoSuite::SuiteA)
        );
    }

    #[test]
    fn test_negotiate_picks_c_over_a() {
        let local = [CryptoSuite::SuiteA, CryptoSuite::SuiteC];
        let remote = [CryptoSuite::SuiteC, CryptoSuite::SuiteA];

        assert_eq!(
            CryptoSuite::negotiate(&local, &remote),
            Some(CryptoSuite::SuiteC)
        );
    }

    #[test]
    fn test_negotiate_no_common() {
        let local = [CryptoSuite::SuiteA];
        let remote = [CryptoSuite::SuiteD];

        assert_eq!(CryptoSuite::negotiate(&local, &remote), None);
    }

    #[test]
    fn test_negotiate_fallback_to_d() {
        let local = [CryptoSuite::SuiteD];
        let remote = [CryptoSuite::SuiteD];

        assert_eq!(
            CryptoSuite::negotiate(&local, &remote),
            Some(CryptoSuite::SuiteD)
        );
    }

    #[test]
    fn test_supports_post_quantum() {
        assert!(CryptoSuite::SuiteA.supports_post_quantum());
        assert!(CryptoSuite::SuiteB.supports_post_quantum());
        assert!(CryptoSuite::SuiteC.supports_post_quantum());
        assert!(!CryptoSuite::SuiteD.supports_post_quantum());
    }

    #[test]
    fn test_aead_algorithm() {
        assert_eq!(
            CryptoSuite::SuiteA.aead_algorithm(),
            AeadAlgorithm::XChaCha20Poly1305
        );
        assert_eq!(
            CryptoSuite::SuiteB.aead_algorithm(),
            AeadAlgorithm::Aes256Gcm
        );
        assert_eq!(
            CryptoSuite::SuiteC.aead_algorithm(),
            AeadAlgorithm::XChaCha20Poly1305
        );
        assert_eq!(
            CryptoSuite::SuiteD.aead_algorithm(),
            AeadAlgorithm::XChaCha20Poly1305
        );
    }

    #[test]
    fn test_kem_algorithm() {
        assert_eq!(
            CryptoSuite::SuiteA.kem_algorithm(),
            KemAlgorithm::HybridX25519MlKem768
        );
        assert_eq!(
            CryptoSuite::SuiteB.kem_algorithm(),
            KemAlgorithm::HybridX25519MlKem768
        );
        assert_eq!(
            CryptoSuite::SuiteC.kem_algorithm(),
            KemAlgorithm::HybridX25519MlKem1024
        );
        assert_eq!(
            CryptoSuite::SuiteD.kem_algorithm(),
            KemAlgorithm::ClassicalX25519
        );
    }

    #[test]
    fn test_suite_id_roundtrip() {
        for suite in CryptoSuite::all() {
            let id = suite.to_id();
            assert_eq!(CryptoSuite::from_id(id), Some(*suite));
        }
    }

    #[test]
    fn test_unknown_id() {
        assert_eq!(CryptoSuite::from_id(0x00), None);
        assert_eq!(CryptoSuite::from_id(0xFF), None);
    }

    #[test]
    fn test_display() {
        let s = alloc::format!("{}", CryptoSuite::SuiteA);
        assert!(s.contains("Suite A"));
    }
}
