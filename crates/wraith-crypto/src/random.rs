//! Secure random number generation.
//!
//! All randomness comes from the operating system CSPRNG.

use crate::CryptoError;

/// Fill a buffer with random bytes from the OS CSPRNG
pub fn fill_random(buf: &mut [u8]) -> Result<(), CryptoError> {
    getrandom::fill(buf).map_err(|_| CryptoError::RandomFailed)
}

/// Generate a random 32-byte array
pub fn random_32() -> Result<[u8; 32], CryptoError> {
    let mut buf = [0u8; 32];
    fill_random(&mut buf)?;
    Ok(buf)
}

/// Generate a random 8-byte array
pub fn random_8() -> Result<[u8; 8], CryptoError> {
    let mut buf = [0u8; 8];
    fill_random(&mut buf)?;
    Ok(buf)
}
