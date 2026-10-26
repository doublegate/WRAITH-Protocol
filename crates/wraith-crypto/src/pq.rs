//! Post-Quantum Cryptography using ML-KEM-768 (Kyber).

use core::convert::TryFrom;
use kem::Decapsulate;
use ml_kem::{B32, Ciphertext, DecapsulationKey, EncapsulationKey, Key, KeyExport, MlKem768, Seed};
use rand_core::{CryptoRng, RngCore};
use zeroize::Zeroize;

/// ML-KEM-768 Public Key.
pub type PqPublicKey = EncapsulationKey<MlKem768>;
/// ML-KEM-768 Private Key.
pub type PqPrivateKey = DecapsulationKey<MlKem768>;
/// ML-KEM-768 Ciphertext.
pub type PqCiphertext = Ciphertext<MlKem768>;
/// Shared Secret.
pub type PqSharedSecret = [u8; 32];

/// Generate a new ML-KEM-768 keypair.
/// Returns (encapsulation_key, decapsulation_key).
pub fn generate_keypair<R: RngCore + CryptoRng>(rng: &mut R) -> (PqPublicKey, PqPrivateKey) {
    let mut seed = Seed::default();
    rng.fill_bytes(seed.as_mut_slice());
    let dk = PqPrivateKey::from_seed(seed);
    seed.zeroize();
    let ek = dk.encapsulation_key().clone();
    (ek, dk)
}

/// Encapsulate a shared secret to a public key.
/// Returns (ciphertext, shared_secret).
pub fn encapsulate<R: RngCore + CryptoRng>(
    rng: &mut R,
    public_key: &PqPublicKey,
) -> (PqCiphertext, PqSharedSecret) {
    let mut m = B32::default();
    rng.fill_bytes(m.as_mut_slice());
    let (ct, ss) = public_key.encapsulate_deterministic(&m);
    m.zeroize();
    let mut out = [0u8; 32];
    out.copy_from_slice(ss.as_slice());
    (ct, out)
}

/// Decapsulate a shared secret from a ciphertext using a private key.
pub fn decapsulate(private_key: &PqPrivateKey, ciphertext: &PqCiphertext) -> PqSharedSecret {
    let ss = private_key.decapsulate(ciphertext);
    let mut out = [0u8; 32];
    out.copy_from_slice(ss.as_slice());
    out
}

/// Convert a public key to a byte vector.
pub fn public_key_to_vec(pk: &PqPublicKey) -> alloc::vec::Vec<u8> {
    pk.to_bytes().to_vec()
}

/// Error returned when parsing PQ key material from bytes fails.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PqParseError;

impl core::fmt::Display for PqParseError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("invalid post-quantum key material")
    }
}

/// Parse a public key from bytes.
pub fn public_key_from_bytes(bytes: &[u8]) -> Result<PqPublicKey, PqParseError> {
    let key_bytes = Key::<PqPublicKey>::try_from(bytes).map_err(|_| PqParseError)?;
    PqPublicKey::new(&key_bytes).map_err(|_| PqParseError)
}

/// Parse a ciphertext from bytes.
pub fn ciphertext_from_bytes(bytes: &[u8]) -> Result<PqCiphertext, PqParseError> {
    PqCiphertext::try_from(bytes).map_err(|_| PqParseError)
}

/// Convert a ciphertext to a byte vector.
pub fn ciphertext_to_vec(ct: &PqCiphertext) -> alloc::vec::Vec<u8> {
    ct.as_slice().to_vec()
}
