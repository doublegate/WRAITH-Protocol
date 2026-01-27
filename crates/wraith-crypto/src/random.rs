//! Secure random number generation.
//!
//! All randomness comes from the operating system CSPRNG.

use crate::CryptoError;
use rand_core::{CryptoRng, Error, RngCore};

/// A secure random number generator backed by the OS CSPRNG.
pub struct SecureRng;

impl SecureRng {
    /// Create a new instance.
    pub fn new() -> Self {
        Self
    }
}

impl Default for SecureRng {
    fn default() -> Self {
        Self::new()
    }
}

impl RngCore for SecureRng {
    fn next_u32(&mut self) -> u32 {
        rand_core::impls::next_u32_via_fill(self)
    }

    fn next_u64(&mut self) -> u64 {
        rand_core::impls::next_u64_via_fill(self)
    }

    fn fill_bytes(&mut self, dest: &mut [u8]) {
        self.try_fill_bytes(dest).expect("Random generation failed")
    }

    fn try_fill_bytes(&mut self, dest: &mut [u8]) -> Result<(), Error> {
        // Map getrandom error to rand_core::Error
        getrandom::getrandom(dest).map_err(|e| {
            // Compiler says e.code() is NonZeroU32
            Error::from(e.code())
        })
    }
}

impl CryptoRng for SecureRng {}

/// Fill a buffer with random bytes from the OS CSPRNG.
///
/// # Errors
///
/// Returns [`CryptoError::RandomFailed`] if the underlying OS CSPRNG fails.
pub fn fill_random(buf: &mut [u8]) -> Result<(), CryptoError> {
    getrandom::getrandom(buf).map_err(|_| CryptoError::RandomFailed)
}

/// Generate a random 32-byte array.
///
/// # Errors
///
/// Returns [`CryptoError::RandomFailed`] if the underlying OS CSPRNG fails.
pub fn random_32() -> Result<[u8; 32], CryptoError> {
    let mut buf = [0u8; 32];
    fill_random(&mut buf)?;
    Ok(buf)
}

/// Generate a random 8-byte array.
///
/// # Errors
///
/// Returns [`CryptoError::RandomFailed`] if the underlying OS CSPRNG fails.
pub fn random_8() -> Result<[u8; 8], CryptoError> {
    let mut buf = [0u8; 8];
    fill_random(&mut buf)?;
    Ok(buf)
}
