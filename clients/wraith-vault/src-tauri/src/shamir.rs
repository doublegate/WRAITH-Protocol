//! Shamir Secret Sharing Implementation for WRAITH Vault
//!
//! This module implements Shamir's Secret Sharing scheme (k-of-n threshold scheme)
//! allowing secrets to be split into n shares, where any k shares can reconstruct
//! the original secret.
//!
//! Security properties:
//! - Information-theoretic security: fewer than k shares reveal no information
//! - Perfect reconstruction: exactly k shares perfectly reconstruct the secret
//! - No single point of failure: secret survives loss of (n-k) shares

use crate::error::{VaultError, VaultResult};
use rand::{RngCore, rngs::OsRng};
use serde::{Deserialize, Serialize};

/// Prime modulus for the finite field (256-bit prime)
/// This is the largest 256-bit prime: 2^256 - 189
#[allow(dead_code)]
const PRIME_MODULUS: [u8; 32] = [
    0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
    0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0x43,
];

/// A share of a secret produced by Shamir Secret Sharing
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Share {
    /// Share index (x-coordinate), 1-indexed
    pub index: u8,
    /// Share data (y-coordinate)
    pub data: Vec<u8>,
}

impl Share {
    /// Create a new share
    pub fn new(index: u8, data: Vec<u8>) -> Self {
        Self { index, data }
    }

    /// Encode share as base64 for transmission
    pub fn to_base64(&self) -> String {
        use base64::Engine;
        let mut encoded = vec![self.index];
        encoded.extend(&self.data);
        base64::engine::general_purpose::STANDARD.encode(&encoded)
    }

    /// Decode share from base64
    pub fn from_base64(encoded: &str) -> VaultResult<Self> {
        use base64::Engine;
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(encoded)
            .map_err(|e| VaultError::Shamir(format!("Invalid base64: {}", e)))?;

        if bytes.is_empty() {
            return Err(VaultError::Shamir("Empty share data".to_string()));
        }

        Ok(Self {
            index: bytes[0],
            data: bytes[1..].to_vec(),
        })
    }
}

/// Configuration for secret sharing
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ShamirConfig {
    /// Threshold (k): minimum shares needed for reconstruction
    pub threshold: u8,
    /// Total shares (n): total number of shares created
    pub total_shares: u8,
}

impl ShamirConfig {
    /// Create a new Shamir configuration
    ///
    /// # Arguments
    /// * `threshold` - Minimum shares needed (k)
    /// * `total_shares` - Total shares to create (n)
    ///
    /// # Errors
    /// Returns error if threshold > total_shares or if values are invalid
    pub fn new(threshold: u8, total_shares: u8) -> VaultResult<Self> {
        if threshold == 0 {
            return Err(VaultError::Shamir(
                "Threshold must be at least 1".to_string(),
            ));
        }
        if threshold > total_shares {
            return Err(VaultError::Shamir(
                "Threshold cannot exceed total shares".to_string(),
            ));
        }
        // Note: u8 type already constrains total_shares to max 255

        Ok(Self {
            threshold,
            total_shares,
        })
    }
}

impl Default for ShamirConfig {
    fn default() -> Self {
        Self {
            threshold: 3,
            total_shares: 5,
        }
    }
}

/// Shamir Secret Sharing implementation using GF(256)
///
/// This implementation operates in GF(256) (Galois Field with 256 elements)
/// for efficiency and simplicity. Each byte of the secret is split independently.
pub struct ShamirSecretSharing {
    config: ShamirConfig,
}

impl ShamirSecretSharing {
    /// Create a new Shamir Secret Sharing instance
    pub fn new(config: ShamirConfig) -> Self {
        Self { config }
    }

    /// Split a secret into n shares
    ///
    /// # Arguments
    /// * `secret` - The secret data to split
    ///
    /// # Returns
    /// A vector of n shares
    pub fn split(&self, secret: &[u8]) -> VaultResult<Vec<Share>> {
        if secret.is_empty() {
            return Err(VaultError::Shamir("Cannot split empty secret".to_string()));
        }

        let mut shares: Vec<Share> = (1..=self.config.total_shares)
            .map(|i| Share::new(i, Vec::with_capacity(secret.len())))
            .collect();

        // For each byte of the secret, create polynomial and evaluate at each x
        for &secret_byte in secret {
            // Generate random coefficients for polynomial
            // f(x) = secret + a1*x + a2*x^2 + ... + a(k-1)*x^(k-1)
            let coefficients = self.generate_coefficients(secret_byte)?;

            // Evaluate polynomial at each share's x coordinate
            for share in &mut shares {
                let y = self.evaluate_polynomial(&coefficients, share.index);
                share.data.push(y);
            }
        }

        Ok(shares)
    }

    /// Reconstruct the secret from k or more shares
    ///
    /// # Arguments
    /// * `shares` - Vector of shares (must have at least threshold shares)
    ///
    /// # Returns
    /// The reconstructed secret
    pub fn combine(&self, shares: &[Share]) -> VaultResult<Vec<u8>> {
        if shares.len() < self.config.threshold as usize {
            return Err(VaultError::Shamir(format!(
                "Need at least {} shares, got {}",
                self.config.threshold,
                shares.len()
            )));
        }

        // Check all shares have the same length
        let share_len = shares[0].data.len();
        if shares.iter().any(|s| s.data.len() != share_len) {
            return Err(VaultError::Shamir(
                "All shares must have the same length".to_string(),
            ));
        }

        // Check for duplicate indices
        let mut indices: Vec<u8> = shares.iter().map(|s| s.index).collect();
        indices.sort_unstable();
        indices.dedup();
        if indices.len() != shares.len() {
            return Err(VaultError::Shamir("Duplicate share indices".to_string()));
        }

        // Use only threshold number of shares
        let used_shares = &shares[..self.config.threshold as usize];

        // Reconstruct each byte using Lagrange interpolation
        let mut secret = Vec::with_capacity(share_len);
        for byte_idx in 0..share_len {
            let points: Vec<(u8, u8)> = used_shares
                .iter()
                .map(|s| (s.index, s.data[byte_idx]))
                .collect();

            let reconstructed = self.lagrange_interpolate(&points, 0)?;
            secret.push(reconstructed);
        }

        Ok(secret)
    }

    /// Generate random polynomial coefficients
    fn generate_coefficients(&self, secret_byte: u8) -> VaultResult<Vec<u8>> {
        let mut coefficients = Vec::with_capacity(self.config.threshold as usize);
        coefficients.push(secret_byte); // a0 = secret

        let mut random_bytes = vec![0u8; self.config.threshold as usize - 1];
        OsRng.fill_bytes(&mut random_bytes);
        coefficients.extend(random_bytes);

        Ok(coefficients)
    }

    /// Evaluate polynomial at x using Horner's method in GF(256)
    fn evaluate_polynomial(&self, coefficients: &[u8], x: u8) -> u8 {
        // Horner's method: f(x) = ((a_n * x + a_(n-1)) * x + ...) * x + a_0
        let mut result = 0u8;
        for &coeff in coefficients.iter().rev() {
            result = gf256_add(gf256_mul(result, x), coeff);
        }
        result
    }

    /// Lagrange interpolation to find f(0) in GF(256)
    fn lagrange_interpolate(&self, points: &[(u8, u8)], x_target: u8) -> VaultResult<u8> {
        let mut result = 0u8;

        for (i, &(xi, yi)) in points.iter().enumerate() {
            let mut numerator = 1u8;
            let mut denominator = 1u8;

            for (j, &(xj, _)) in points.iter().enumerate() {
                if i != j {
                    numerator = gf256_mul(numerator, gf256_sub(x_target, xj));
                    denominator = gf256_mul(denominator, gf256_sub(xi, xj));
                }
            }

            if denominator == 0 {
                return Err(VaultError::Shamir(
                    "Division by zero in interpolation".to_string(),
                ));
            }

            let basis = gf256_div(numerator, denominator)?;
            result = gf256_add(result, gf256_mul(yi, basis));
        }

        Ok(result)
    }

    /// Get the current configuration
    pub fn config(&self) -> &ShamirConfig {
        &self.config
    }
}

// GF(256) Arithmetic Operations
// Using AES polynomial: x^8 + x^4 + x^3 + x + 1 (0x11B)

const GF256_GENERATOR: u16 = 0x11B;

/// GF(256) addition (XOR)
#[inline]
fn gf256_add(a: u8, b: u8) -> u8 {
    a ^ b
}

/// GF(256) subtraction (same as addition in characteristic 2)
#[inline]
fn gf256_sub(a: u8, b: u8) -> u8 {
    a ^ b
}

/// GF(256) multiplication using Russian Peasant algorithm
fn gf256_mul(mut a: u8, mut b: u8) -> u8 {
    let mut result: u8 = 0;

    while b != 0 {
        if b & 1 != 0 {
            result ^= a;
        }
        let hi_bit_set = a & 0x80;
        a <<= 1;
        if hi_bit_set != 0 {
            a ^= 0x1B; // Reduce by AES polynomial
        }
        b >>= 1;
    }

    result
}

/// GF(256) division using extended Euclidean algorithm
fn gf256_div(a: u8, b: u8) -> VaultResult<u8> {
    if b == 0 {
        return Err(VaultError::Shamir("Division by zero".to_string()));
    }
    Ok(gf256_mul(a, gf256_inverse(b)))
}

/// GF(256) multiplicative inverse using extended Euclidean algorithm
fn gf256_inverse(a: u8) -> u8 {
    if a == 0 {
        return 0;
    }

    // Use Fermat's little theorem: a^(-1) = a^(254) in GF(256)
    let mut result = a;
    for _ in 0..6 {
        result = gf256_mul(result, result);
        result = gf256_mul(result, a);
    }
    gf256_mul(result, result)
}

/// Create log and exp tables for optimized GF(256) operations
/// These are pre-computed lookup tables for faster multiplication
#[allow(dead_code)]
mod lookup_tables {
    use super::GF256_GENERATOR;

    pub static GF256_LOG: [u8; 256] = {
        let mut log = [0u8; 256];
        let mut x: u16 = 1;
        let mut i = 0u8;
        while i < 255 {
            log[x as usize] = i;
            x <<= 1;
            if x >= 256 {
                x ^= GF256_GENERATOR;
            }
            i += 1;
        }
        log
    };

    pub static GF256_EXP: [u8; 512] = {
        let mut exp = [0u8; 512];
        let mut x: u16 = 1;
        let mut i = 0usize;
        while i < 512 {
            exp[i] = x as u8;
            x <<= 1;
            if x >= 256 {
                x ^= GF256_GENERATOR;
            }
            i += 1;
        }
        exp
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shamir_config_validation() {
        // Valid config
        assert!(ShamirConfig::new(3, 5).is_ok());
        assert!(ShamirConfig::new(1, 1).is_ok());
        assert!(ShamirConfig::new(5, 5).is_ok());

        // Invalid: threshold > total
        assert!(ShamirConfig::new(6, 5).is_err());

        // Invalid: zero threshold
        assert!(ShamirConfig::new(0, 5).is_err());
    }

    #[test]
    fn test_split_and_combine_basic() {
        let config = ShamirConfig::new(3, 5).unwrap();
        let sss = ShamirSecretSharing::new(config);

        let secret = b"Hello, WRAITH Vault!";
        let shares = sss.split(secret).unwrap();

        assert_eq!(shares.len(), 5);
        for share in &shares {
            assert_eq!(share.data.len(), secret.len());
        }

        // Combine with exactly threshold shares
        let reconstructed = sss.combine(&shares[0..3]).unwrap();
        assert_eq!(reconstructed, secret);

        // Combine with more than threshold shares
        let reconstructed = sss.combine(&shares).unwrap();
        assert_eq!(reconstructed, secret);
    }

    #[test]
    fn test_split_and_combine_any_subset() {
        let config = ShamirConfig::new(3, 5).unwrap();
        let sss = ShamirSecretSharing::new(config);

        let secret = b"Test secret 12345";
        let shares = sss.split(secret).unwrap();

        // Try all combinations of 3 shares
        let combinations = vec![
            vec![0, 1, 2],
            vec![0, 1, 3],
            vec![0, 1, 4],
            vec![0, 2, 3],
            vec![0, 2, 4],
            vec![0, 3, 4],
            vec![1, 2, 3],
            vec![1, 2, 4],
            vec![1, 3, 4],
            vec![2, 3, 4],
        ];

        for combo in combinations {
            let subset: Vec<Share> = combo.iter().map(|&i| shares[i].clone()).collect();
            let reconstructed = sss.combine(&subset).unwrap();
            assert_eq!(reconstructed, secret, "Failed for combination {:?}", combo);
        }
    }

    #[test]
    fn test_insufficient_shares() {
        let config = ShamirConfig::new(3, 5).unwrap();
        let sss = ShamirSecretSharing::new(config);

        let secret = b"My secret";
        let shares = sss.split(secret).unwrap();

        // Try with fewer than threshold shares
        let result = sss.combine(&shares[0..2]);
        assert!(result.is_err());
    }

    #[test]
    fn test_empty_secret() {
        let config = ShamirConfig::new(3, 5).unwrap();
        let sss = ShamirSecretSharing::new(config);

        let result = sss.split(b"");
        assert!(result.is_err());
    }

    #[test]
    fn test_share_base64_encoding() {
        let share = Share::new(5, vec![1, 2, 3, 4, 5]);
        let encoded = share.to_base64();
        let decoded = Share::from_base64(&encoded).unwrap();

        assert_eq!(share, decoded);
    }

    #[test]
    fn test_gf256_arithmetic() {
        // Test addition (XOR)
        assert_eq!(gf256_add(0, 0), 0);
        assert_eq!(gf256_add(0xFF, 0xFF), 0);
        assert_eq!(gf256_add(0xAB, 0x00), 0xAB);

        // Test multiplication
        assert_eq!(gf256_mul(0, 0x12), 0);
        assert_eq!(gf256_mul(1, 0x12), 0x12);
        assert_eq!(gf256_mul(2, 2), 4);

        // Test inverse
        for i in 1u8..=255 {
            let inv = gf256_inverse(i);
            assert_eq!(gf256_mul(i, inv), 1, "Inverse failed for {}", i);
        }
    }

    #[test]
    fn test_large_secret() {
        let config = ShamirConfig::new(5, 10).unwrap();
        let sss = ShamirSecretSharing::new(config);

        // 1KB secret
        let mut secret = vec![0u8; 1024];
        OsRng.fill_bytes(&mut secret);

        let shares = sss.split(&secret).unwrap();
        assert_eq!(shares.len(), 10);

        let reconstructed = sss.combine(&shares[0..5]).unwrap();
        assert_eq!(reconstructed, secret);
    }

    #[test]
    fn test_2_of_3_scheme() {
        let config = ShamirConfig::new(2, 3).unwrap();
        let sss = ShamirSecretSharing::new(config);

        let secret = b"Simple 2-of-3 test";
        let shares = sss.split(secret).unwrap();

        // Any 2 shares should work
        assert_eq!(sss.combine(&shares[0..2]).unwrap(), secret);
        assert_eq!(
            sss.combine(&[shares[0].clone(), shares[2].clone()])
                .unwrap(),
            secret
        );
        assert_eq!(sss.combine(&shares[1..3]).unwrap(), secret);
    }

    #[test]
    fn test_duplicate_share_detection() {
        let config = ShamirConfig::new(3, 5).unwrap();
        let sss = ShamirSecretSharing::new(config);

        let secret = b"Duplicate test";
        let shares = sss.split(secret).unwrap();

        // Create shares with duplicate indices
        let duplicate_shares = vec![
            shares[0].clone(),
            shares[0].clone(), // Duplicate!
            shares[2].clone(),
        ];

        let result = sss.combine(&duplicate_shares);
        assert!(result.is_err());
    }

    #[test]
    fn test_mismatched_share_lengths() {
        let config = ShamirConfig::new(2, 3).unwrap();
        let sss = ShamirSecretSharing::new(config);

        let share1 = Share::new(1, vec![1, 2, 3]);
        let share2 = Share::new(2, vec![1, 2]); // Different length

        let result = sss.combine(&[share1, share2]);
        assert!(result.is_err());
    }

    #[test]
    fn test_deterministic_with_same_coefficients() {
        // While the split is random, combining should always give the same result
        let config = ShamirConfig::new(3, 5).unwrap();
        let sss = ShamirSecretSharing::new(config);

        let secret = b"Deterministic reconstruction test";

        // Split multiple times and verify all reconstruct to same secret
        for _ in 0..10 {
            let shares = sss.split(secret).unwrap();
            let reconstructed = sss.combine(&shares[0..3]).unwrap();
            assert_eq!(reconstructed, secret);
        }
    }
}
