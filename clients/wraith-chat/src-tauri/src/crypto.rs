// Double Ratchet Algorithm Implementation (Signal Protocol)
//
// Based on: https://signal.org/docs/specifications/doubleratchet/

use chacha20poly1305::{
    ChaCha20Poly1305, Nonce,
    aead::{Aead, KeyInit},
};
use hkdf::Hkdf;
use rand::RngCore;
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use std::collections::HashMap;
use x25519_dalek::{PublicKey, StaticSecret};

const MAX_SKIP: usize = 1000; // Maximum number of skipped messages to store

/// Double Ratchet state for end-to-end encryption
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DoubleRatchet {
    /// Root key for deriving new chain keys
    #[serde(with = "serde_bytes")]
    root_key: Vec<u8>,

    /// Sending chain key
    #[serde(with = "serde_bytes")]
    sending_chain_key: Vec<u8>,

    /// Receiving chain key (optional, set after first message)
    #[serde(with = "serde_bytes_option")]
    receiving_chain_key: Option<Vec<u8>>,

    /// DH sending key pair (private)
    #[serde(with = "serde_bytes")]
    dh_sending_secret: Vec<u8>,

    /// DH sending key pair (public)
    #[serde(with = "serde_bytes")]
    dh_sending_public: Vec<u8>,

    /// DH receiving public key
    #[serde(with = "serde_bytes_option")]
    dh_receiving_public: Option<Vec<u8>>,

    /// Sending chain message number
    sending_chain_index: u32,

    /// Receiving chain message number
    receiving_chain_index: u32,

    /// Previous sending chain length (for skipped messages)
    previous_sending_chain_length: u32,

    /// Skipped message keys (indexed by header + message number)
    skipped_keys: HashMap<String, Vec<u8>>,
}

impl DoubleRatchet {
    /// Initialize a new Double Ratchet from a shared secret
    ///
    /// # Arguments
    /// * `shared_secret` - 32-byte shared secret from key agreement (e.g., X3DH or Noise)
    /// * `remote_public_key` - Optional remote public key if we're the responder
    pub fn new(
        shared_secret: &[u8],
        remote_public_key: Option<&[u8]>,
    ) -> Result<Self, CryptoError> {
        if shared_secret.len() != 32 {
            return Err(CryptoError::InvalidKeyLength);
        }

        // Derive root key from shared secret
        let hkdf = Hkdf::<Sha256>::new(None, shared_secret);
        let mut root_key = vec![0u8; 32];
        hkdf.expand(b"wraith-chat-root-key", &mut root_key)
            .map_err(|_| CryptoError::KdfFailed)?;

        // Generate DH sending key pair
        let mut rng = rand::thread_rng();
        let dh_secret = StaticSecret::random_from_rng(&mut rng);
        let dh_public = PublicKey::from(&dh_secret);

        let dh_sending_secret = dh_secret.to_bytes().to_vec();
        let dh_sending_public = dh_public.as_bytes().to_vec();

        // Initialize sending chain
        let mut sending_chain_key = vec![0u8; 32];
        hkdf.expand(b"wraith-chat-send-chain", &mut sending_chain_key)
            .map_err(|_| CryptoError::KdfFailed)?;

        // If we're the responder, set up receiving chain to match initiator's sending chain
        // The receiving chain must be derived from the same source as the initiator's sending chain
        // so that decryption works. DH ratchet only happens when we send a reply or receive
        // a message with a new DH public key.
        let (receiving_chain_key, dh_receiving_public) = if let Some(remote_pub) = remote_public_key
        {
            // Derive receiving chain from shared secret (same as initiator's sending chain)
            let mut recv_chain_key = vec![0u8; 32];
            hkdf.expand(b"wraith-chat-send-chain", &mut recv_chain_key)
                .map_err(|_| CryptoError::KdfFailed)?;
            (Some(recv_chain_key), Some(remote_pub.to_vec()))
        } else {
            (None, None)
        };

        let ratchet = Self {
            root_key,
            sending_chain_key,
            receiving_chain_key,
            dh_sending_secret,
            dh_sending_public,
            dh_receiving_public,
            sending_chain_index: 0,
            receiving_chain_index: 0,
            previous_sending_chain_length: 0,
            skipped_keys: HashMap::new(),
        };

        Ok(ratchet)
    }

    /// Encrypt a plaintext message
    pub fn encrypt(&mut self, plaintext: &[u8]) -> Result<EncryptedMessage, CryptoError> {
        // Derive message key from sending chain
        let message_key = self.kdf_chain_send()?;

        // Encrypt with ChaCha20-Poly1305
        let mut nonce_bytes = [0u8; 12];
        rand::thread_rng().fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        let cipher = ChaCha20Poly1305::new_from_slice(&message_key)
            .map_err(|_| CryptoError::EncryptionFailed)?;

        let ciphertext = cipher
            .encrypt(nonce, plaintext)
            .map_err(|_| CryptoError::EncryptionFailed)?;

        let encrypted = EncryptedMessage {
            dh_public_key: self.dh_sending_public.clone(),
            message_index: self.sending_chain_index - 1, // Already incremented in kdf_chain_send
            nonce: nonce_bytes.to_vec(),
            ciphertext,
        };

        Ok(encrypted)
    }

    /// Decrypt a ciphertext message
    pub fn decrypt(&mut self, message: &EncryptedMessage) -> Result<Vec<u8>, CryptoError> {
        // Check if we need to perform DH ratchet
        if self.dh_receiving_public.as_ref() != Some(&message.dh_public_key) {
            self.skip_message_keys(message.message_index)?;
            self.dh_ratchet_receive(&message.dh_public_key)?;
        }

        // Handle out-of-order messages
        if message.message_index < self.receiving_chain_index {
            // Try to find skipped message key
            let key_id = self.key_id(&message.dh_public_key, message.message_index);
            if let Some(message_key) = self.skipped_keys.remove(&key_id) {
                return self.decrypt_with_key(&message_key, &message.nonce, &message.ciphertext);
            } else {
                return Err(CryptoError::MessageKeyNotFound);
            }
        }

        if message.message_index > self.receiving_chain_index {
            self.skip_message_keys(message.message_index)?;
        }

        // Derive message key
        let message_key = self.kdf_chain_receive()?;

        // Decrypt
        self.decrypt_with_key(&message_key, &message.nonce, &message.ciphertext)
    }

    /// Perform DH ratchet step (receive)
    fn dh_ratchet_receive(&mut self, remote_public: &[u8]) -> Result<(), CryptoError> {
        if remote_public.len() != 32 {
            return Err(CryptoError::InvalidKeyLength);
        }

        // Update receiving public key
        self.dh_receiving_public = Some(remote_public.to_vec());

        // Perform DH - convert slices to fixed-size arrays
        let remote_pub_array: [u8; 32] = remote_public
            .try_into()
            .map_err(|_| CryptoError::InvalidKeyLength)?;
        let remote_pub = PublicKey::from(remote_pub_array);

        let our_secret_array: [u8; 32] = self
            .dh_sending_secret
            .as_slice()
            .try_into()
            .map_err(|_| CryptoError::InvalidKeyLength)?;
        let our_secret = StaticSecret::from(our_secret_array);
        let dh_output = our_secret.diffie_hellman(&remote_pub);

        // KDF to derive new root key and receiving chain key
        let (new_root_key, new_receiving_chain_key) = self.kdf_ratchet(dh_output.as_bytes())?;

        self.root_key = new_root_key;
        self.receiving_chain_key = Some(new_receiving_chain_key);
        self.receiving_chain_index = 0;

        // Generate new sending key pair
        let new_secret = StaticSecret::random_from_rng(rand::thread_rng());
        let new_public = PublicKey::from(&new_secret);

        self.dh_sending_secret = new_secret.to_bytes().to_vec();
        self.dh_sending_public = new_public.as_bytes().to_vec();

        // Perform DH again with new key pair
        let dh_output2 = new_secret.diffie_hellman(&remote_pub);

        // KDF to derive new root key and sending chain key
        let (new_root_key2, new_sending_chain_key) = self.kdf_ratchet(dh_output2.as_bytes())?;

        self.root_key = new_root_key2;
        self.sending_chain_key = new_sending_chain_key;
        self.previous_sending_chain_length = self.sending_chain_index;
        self.sending_chain_index = 0;

        Ok(())
    }

    /// KDF ratchet: derive new root key and chain key
    fn kdf_ratchet(&self, dh_output: &[u8]) -> Result<(Vec<u8>, Vec<u8>), CryptoError> {
        let mut okm = vec![0u8; 64];
        let hkdf = Hkdf::<Sha256>::new(Some(&self.root_key), dh_output);
        hkdf.expand(b"wraith-chat-ratchet", &mut okm)
            .map_err(|_| CryptoError::KdfFailed)?;

        let root_key = okm[..32].to_vec();
        let chain_key = okm[32..].to_vec();

        Ok((root_key, chain_key))
    }

    /// KDF chain: derive message key from chain key (sending)
    fn kdf_chain_send(&mut self) -> Result<Vec<u8>, CryptoError> {
        let mut okm = vec![0u8; 64];
        let hkdf = Hkdf::<Sha256>::new(None, &self.sending_chain_key);
        hkdf.expand(b"wraith-chat-message-key", &mut okm)
            .map_err(|_| CryptoError::KdfFailed)?;

        self.sending_chain_key = okm[..32].to_vec();
        let message_key = okm[32..].to_vec();
        self.sending_chain_index += 1;

        Ok(message_key)
    }

    /// KDF chain: derive message key from chain key (receiving)
    fn kdf_chain_receive(&mut self) -> Result<Vec<u8>, CryptoError> {
        let chain_key = self
            .receiving_chain_key
            .as_ref()
            .ok_or(CryptoError::NoReceivingChain)?;

        let mut okm = vec![0u8; 64];
        let hkdf = Hkdf::<Sha256>::new(None, chain_key);
        hkdf.expand(b"wraith-chat-message-key", &mut okm)
            .map_err(|_| CryptoError::KdfFailed)?;

        self.receiving_chain_key = Some(okm[..32].to_vec());
        let message_key = okm[32..].to_vec();
        self.receiving_chain_index += 1;

        Ok(message_key)
    }

    /// Skip message keys for out-of-order handling
    fn skip_message_keys(&mut self, until: u32) -> Result<(), CryptoError> {
        // Get the receiving DH public key (required for key ID generation)
        let dh_recv_pub = self
            .dh_receiving_public
            .as_ref()
            .ok_or(CryptoError::NoReceivingChain)?;

        if let Some(receiving_chain_key) = &self.receiving_chain_key {
            let mut temp_chain_key = receiving_chain_key.clone();

            while self.receiving_chain_index < until {
                // Derive message key
                let mut okm = vec![0u8; 64];
                let hkdf = Hkdf::<Sha256>::new(None, &temp_chain_key);
                hkdf.expand(b"wraith-chat-message-key", &mut okm)
                    .map_err(|_| CryptoError::KdfFailed)?;

                temp_chain_key = okm[..32].to_vec();
                let message_key = okm[32..].to_vec();

                // Store skipped key
                let key_id = self.key_id(dh_recv_pub, self.receiving_chain_index);
                self.skipped_keys.insert(key_id, message_key);

                self.receiving_chain_index += 1;

                // Prevent DoS by limiting skipped keys
                if self.skipped_keys.len() > MAX_SKIP {
                    return Err(CryptoError::TooManySkippedKeys);
                }
            }

            self.receiving_chain_key = Some(temp_chain_key);
        }

        Ok(())
    }

    /// Decrypt with a specific message key
    fn decrypt_with_key(
        &self,
        message_key: &[u8],
        nonce: &[u8],
        ciphertext: &[u8],
    ) -> Result<Vec<u8>, CryptoError> {
        let cipher = ChaCha20Poly1305::new_from_slice(message_key)
            .map_err(|_| CryptoError::DecryptionFailed)?;

        let nonce = Nonce::from_slice(nonce);

        cipher
            .decrypt(nonce, ciphertext)
            .map_err(|_| CryptoError::DecryptionFailed)
    }

    /// Generate key ID for skipped message storage
    fn key_id(&self, dh_public: &[u8], index: u32) -> String {
        format!("{}_{}", hex::encode(dh_public), index)
    }

    /// Serialize ratchet state to JSON
    pub fn to_json(&self) -> Result<String, CryptoError> {
        serde_json::to_string(self).map_err(|_| CryptoError::SerializationFailed)
    }

    /// Deserialize ratchet state from JSON
    pub fn from_json(json: &str) -> Result<Self, CryptoError> {
        serde_json::from_str(json).map_err(|_| CryptoError::DeserializationFailed)
    }
}

/// Encrypted message structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptedMessage {
    /// Sender's DH public key
    #[serde(with = "serde_bytes")]
    pub dh_public_key: Vec<u8>,

    /// Message index in the chain
    pub message_index: u32,

    /// Nonce for AEAD
    #[serde(with = "serde_bytes")]
    pub nonce: Vec<u8>,

    /// Ciphertext (includes authentication tag)
    #[serde(with = "serde_bytes")]
    pub ciphertext: Vec<u8>,
}

/// Cryptography errors
#[derive(Debug, thiserror::Error)]
pub enum CryptoError {
    #[error("Invalid key length")]
    InvalidKeyLength,

    #[error("KDF failed")]
    KdfFailed,

    #[error("Encryption failed")]
    EncryptionFailed,

    #[error("Decryption failed")]
    DecryptionFailed,

    #[error("Message key not found")]
    MessageKeyNotFound,

    #[error("No receiving chain initialized")]
    NoReceivingChain,

    #[error("Too many skipped keys (DoS protection)")]
    TooManySkippedKeys,

    #[error("Serialization failed")]
    SerializationFailed,

    #[error("Deserialization failed")]
    DeserializationFailed,
}

mod serde_bytes {
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(bytes: &Vec<u8>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&hex::encode(bytes))
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        hex::decode(&s).map_err(serde::de::Error::custom)
    }
}

mod serde_bytes_option {
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(bytes: &Option<Vec<u8>>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match bytes {
            Some(b) => serializer.serialize_some(&hex::encode(b)),
            None => serializer.serialize_none(),
        }
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<Vec<u8>>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let opt = Option::<String>::deserialize(deserializer)?;
        opt.map(|s| hex::decode(&s).map_err(serde::de::Error::custom))
            .transpose()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encrypt_decrypt() {
        let shared_secret = [0u8; 32];
        let mut alice = DoubleRatchet::new(&shared_secret, None).unwrap();
        let mut bob = DoubleRatchet::new(&shared_secret, Some(&alice.dh_sending_public)).unwrap();

        let plaintext = b"Hello, Bob!";
        let encrypted = alice.encrypt(plaintext).unwrap();
        let decrypted = bob.decrypt(&encrypted).unwrap();

        assert_eq!(plaintext, &decrypted[..]);
    }

    #[test]
    fn test_out_of_order() {
        let shared_secret = [0u8; 32];
        let mut alice = DoubleRatchet::new(&shared_secret, None).unwrap();
        let mut bob = DoubleRatchet::new(&shared_secret, Some(&alice.dh_sending_public)).unwrap();

        let msg1 = alice.encrypt(b"Message 1").unwrap();
        let msg2 = alice.encrypt(b"Message 2").unwrap();
        let msg3 = alice.encrypt(b"Message 3").unwrap();

        // Decrypt out of order
        let dec3 = bob.decrypt(&msg3).unwrap();
        let dec1 = bob.decrypt(&msg1).unwrap();
        let dec2 = bob.decrypt(&msg2).unwrap();

        assert_eq!(b"Message 3", &dec3[..]);
        assert_eq!(b"Message 1", &dec1[..]);
        assert_eq!(b"Message 2", &dec2[..]);
    }

    #[test]
    fn test_serialization() {
        let shared_secret = [0u8; 32];
        let ratchet = DoubleRatchet::new(&shared_secret, None).unwrap();

        let json = ratchet.to_json().unwrap();
        let deserialized = DoubleRatchet::from_json(&json).unwrap();

        assert_eq!(ratchet.root_key, deserialized.root_key);
        assert_eq!(
            ratchet.sending_chain_index,
            deserialized.sending_chain_index
        );
    }
}
