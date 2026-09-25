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
        // Map a getrandom failure to a rand_core error. getrandom 0.3+ removed
        // `Error::code()`, so surface the OS error code when present and fall
        // back to rand_core's custom-error range otherwise.
        getrandom::fill(dest).map_err(|e| {
            let code = e
                .raw_os_error()
                .and_then(|c| u32::try_from(c).ok())
                .and_then(core::num::NonZeroU32::new)
                .unwrap_or_else(|| {
                    core::num::NonZeroU32::new(Error::CUSTOM_START)
                        .expect("rand_core CUSTOM_START is non-zero")
                });
            Error::from(code)
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
    getrandom::fill(buf).map_err(|_| CryptoError::RandomFailed)
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
