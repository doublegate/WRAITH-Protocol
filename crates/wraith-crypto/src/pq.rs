//! Post-Quantum Cryptography using ML-KEM-768 (Kyber).

use core::convert::TryFrom;
use kem::{Decapsulate, Encapsulate};
use ml_kem::kem::{DecapsulationKey, EncapsulationKey};
use ml_kem::{Ciphertext, Encoded, EncodedSizeUser, KemCore, MlKem768};
use rand_core::{CryptoRng, RngCore};

/// ML-KEM-768 Public Key.
pub type PqPublicKey = EncapsulationKey<ml_kem::MlKem768Params>;
/// ML-KEM-768 Private Key.
pub type PqPrivateKey = DecapsulationKey<ml_kem::MlKem768Params>;
/// ML-KEM-768 Ciphertext.
pub type PqCiphertext = Ciphertext<MlKem768>;
/// Shared Secret.
pub type PqSharedSecret = [u8; 32];

/// Generate a new ML-KEM-768 keypair.
/// Returns (encapsulation_key, decapsulation_key).
pub fn generate_keypair<R: RngCore + CryptoRng>(rng: &mut R) -> (PqPublicKey, PqPrivateKey) {
    let (dk, ek) = MlKem768::generate(rng);
    (ek, dk)
}

/// Encapsulate a shared secret to a public key.
/// Returns (ciphertext, shared_secret).
pub fn encapsulate<R: RngCore + CryptoRng>(
    rng: &mut R,
    public_key: &PqPublicKey,
) -> (PqCiphertext, PqSharedSecret) {
    let (ct, ss) = public_key.encapsulate(rng).unwrap();
    let mut out = [0u8; 32];
    out.copy_from_slice(ss.as_slice());
    (ct, out)
}

/// Decapsulate a shared secret from a ciphertext using a private key.
pub fn decapsulate(private_key: &PqPrivateKey, ciphertext: &PqCiphertext) -> PqSharedSecret {
    let ss = private_key.decapsulate(ciphertext).unwrap();
    let mut out = [0u8; 32];
    out.copy_from_slice(ss.as_slice());
    out
}

/// Convert a public key to a byte vector.
pub fn public_key_to_vec(pk: &PqPublicKey) -> alloc::vec::Vec<u8> {
    pk.as_bytes().to_vec()
}

/// Parse a public key from bytes.
pub fn public_key_from_bytes(bytes: &[u8]) -> Result<PqPublicKey, ()> {
    let arr = Encoded::<PqPublicKey>::try_from(bytes).map_err(|_| ())?;
    Ok(PqPublicKey::from_bytes(&arr))
}

/// Parse a ciphertext from bytes.
pub fn ciphertext_from_bytes(bytes: &[u8]) -> Result<PqCiphertext, ()> {
    PqCiphertext::try_from(bytes).map_err(|_| ())
}

/// Convert a ciphertext to a byte vector.
pub fn ciphertext_to_vec(ct: &PqCiphertext) -> alloc::vec::Vec<u8> {
    ct.as_slice().to_vec()
}