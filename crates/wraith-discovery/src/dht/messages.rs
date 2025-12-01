//! DHT Protocol Messages
//!
//! This module defines the message types used in the Kademlia DHT protocol:
//! - PING/PONG: Liveness checks and RTT measurement
//! - FIND_NODE: Locate peers close to a target NodeId
//! - FIND_VALUE: Retrieve a stored value or closest peers
//! - STORE: Store a key-value pair in the DHT

use super::node_id::NodeId;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use thiserror::Error;

/// DHT RPC message envelope
///
/// All DHT communication uses this message format. Messages can be
/// encrypted for privacy using the AEAD encryption from wraith-crypto.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DhtMessage {
    /// Ping request for liveness check
    Ping(PingRequest),
    /// Pong response to ping
    Pong(PongResponse),
    /// Find node request
    FindNode(FindNodeRequest),
    /// Found nodes response
    FoundNodes(FoundNodesResponse),
    /// Store value request
    Store(StoreRequest),
    /// Store acknowledgment
    StoreAck(StoreAckResponse),
    /// Find value request
    FindValue(FindValueRequest),
    /// Found value response (either value or peers)
    FoundValue(FoundValueResponse),
}

impl DhtMessage {
    /// Serialize message to bytes
    ///
    /// Uses bincode for compact binary serialization.
    ///
    /// # Errors
    ///
    /// Returns error if serialization fails
    ///
    /// # Examples
    ///
    /// ```
    /// use wraith_discovery::dht::{DhtMessage, PingRequest, NodeId};
    ///
    /// let msg = DhtMessage::Ping(PingRequest {
    ///     sender_id: NodeId::random(),
    ///     sender_addr: "127.0.0.1:8000".parse().unwrap(),
    ///     nonce: 12345,
    /// });
    ///
    /// let bytes = msg.to_bytes().unwrap();
    /// assert!(!bytes.is_empty());
    /// ```
    pub fn to_bytes(&self) -> Result<Vec<u8>, MessageError> {
        bincode::serialize(self).map_err(MessageError::Serialization)
    }

    /// Deserialize message from bytes
    ///
    /// # Arguments
    ///
    /// * `bytes` - Serialized message bytes
    ///
    /// # Errors
    ///
    /// Returns error if deserialization fails
    ///
    /// # Examples
    ///
    /// ```
    /// use wraith_discovery::dht::{DhtMessage, PingRequest, NodeId};
    ///
    /// let msg = DhtMessage::Ping(PingRequest {
    ///     sender_id: NodeId::random(),
    ///     sender_addr: "127.0.0.1:8000".parse().unwrap(),
    ///     nonce: 12345,
    /// });
    ///
    /// let bytes = msg.to_bytes().unwrap();
    /// let decoded = DhtMessage::from_bytes(&bytes).unwrap();
    /// ```
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, MessageError> {
        bincode::deserialize(bytes).map_err(MessageError::Serialization)
    }

    /// Encrypt message for privacy
    ///
    /// Uses XChaCha20-Poly1305 AEAD from wraith-crypto to encrypt the message.
    /// The nonce is prepended to the ciphertext.
    ///
    /// # Arguments
    ///
    /// * `key` - 32-byte encryption key
    ///
    /// # Errors
    ///
    /// Returns error if encryption fails
    pub fn encrypt(&self, key: &[u8; 32]) -> Result<Vec<u8>, MessageError> {
        use wraith_crypto::aead::{AeadKey, Nonce};

        let plaintext = self.to_bytes()?;

        let aead_key = AeadKey::new(*key);
        let nonce = Nonce::generate(&mut rand::thread_rng());

        let ciphertext = aead_key
            .encrypt(&nonce, &plaintext, b"")
            .map_err(|_| MessageError::Encryption)?;

        // Prepend nonce to ciphertext
        let mut encrypted = nonce.as_bytes().to_vec();
        encrypted.extend_from_slice(&ciphertext);

        Ok(encrypted)
    }

    /// Decrypt message
    ///
    /// # Arguments
    ///
    /// * `encrypted` - Encrypted message bytes (nonce + ciphertext)
    /// * `key` - 32-byte decryption key
    ///
    /// # Errors
    ///
    /// Returns error if decryption fails
    pub fn decrypt(encrypted: &[u8], key: &[u8; 32]) -> Result<Self, MessageError> {
        use wraith_crypto::aead::{AeadKey, Nonce};

        if encrypted.len() < 24 {
            return Err(MessageError::TooShort);
        }

        // Extract nonce
        let mut nonce_bytes = [0u8; 24];
        nonce_bytes.copy_from_slice(&encrypted[..24]);
        let nonce = Nonce::from_bytes(nonce_bytes);

        let ciphertext = &encrypted[24..];

        // Decrypt
        let aead_key = AeadKey::new(*key);
        let plaintext = aead_key
            .decrypt(&nonce, ciphertext, b"")
            .map_err(|_| MessageError::Decryption)?;

        Self::from_bytes(&plaintext)
    }

    /// Get the sender's NodeId from a message
    ///
    /// # Returns
    ///
    /// The sender's NodeId if available
    #[must_use]
    pub fn sender_id(&self) -> Option<NodeId> {
        match self {
            Self::Ping(msg) => Some(msg.sender_id),
            Self::Pong(msg) => Some(msg.sender_id),
            Self::FindNode(msg) => Some(msg.sender_id),
            Self::FoundNodes(msg) => Some(msg.sender_id),
            Self::Store(msg) => Some(msg.sender_id),
            Self::StoreAck(msg) => Some(msg.sender_id),
            Self::FindValue(msg) => Some(msg.sender_id),
            Self::FoundValue(resp) => match resp {
                FoundValueResponse::Value { sender_id, .. } => Some(*sender_id),
                FoundValueResponse::Peers { sender_id, .. } => Some(*sender_id),
            },
        }
    }
}

/// Ping request
///
/// Used for liveness checks and RTT measurement.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PingRequest {
    /// Sender's node ID
    pub sender_id: NodeId,
    /// Sender's network address
    pub sender_addr: SocketAddr,
    /// Nonce for matching response
    pub nonce: u64,
}

/// Pong response
///
/// Response to a ping request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PongResponse {
    /// Responder's node ID
    pub sender_id: NodeId,
    /// Echoed nonce from ping request
    pub nonce: u64,
}

/// Find node request
///
/// Requests the K closest nodes to a target NodeId.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FindNodeRequest {
    /// Sender's node ID
    pub sender_id: NodeId,
    /// Sender's network address
    pub sender_addr: SocketAddr,
    /// Target node ID to find
    pub target_id: NodeId,
}

/// Found nodes response
///
/// Returns the K closest nodes known to the responder.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FoundNodesResponse {
    /// Responder's node ID
    pub sender_id: NodeId,
    /// List of closest peers
    pub peers: Vec<CompactPeer>,
}

/// Compact peer representation
///
/// Efficient encoding of peer information for transmission.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompactPeer {
    /// Peer's node ID
    pub id: NodeId,
    /// Peer's network address
    pub addr: SocketAddr,
}

/// Store request
///
/// Requests that a peer store a key-value pair.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoreRequest {
    /// Sender's node ID
    pub sender_id: NodeId,
    /// Sender's network address
    pub sender_addr: SocketAddr,
    /// 32-byte storage key
    pub key: [u8; 32],
    /// Value data to store
    pub value: Vec<u8>,
    /// Time-to-live in seconds
    pub ttl: u64,
}

/// Store acknowledgment
///
/// Confirms whether the value was stored.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoreAckResponse {
    /// Responder's node ID
    pub sender_id: NodeId,
    /// Whether the value was successfully stored
    pub stored: bool,
}

/// Find value request
///
/// Requests a value by key, or the closest nodes if value not found.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FindValueRequest {
    /// Sender's node ID
    pub sender_id: NodeId,
    /// Sender's network address
    pub sender_addr: SocketAddr,
    /// 32-byte key to look up
    pub key: [u8; 32],
}

/// Found value response
///
/// Either returns the value or a list of closer peers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FoundValueResponse {
    /// Value was found
    Value {
        /// Responder's node ID
        sender_id: NodeId,
        /// The stored value
        value: Vec<u8>,
    },
    /// Value not found, here are closer peers
    Peers {
        /// Responder's node ID
        sender_id: NodeId,
        /// List of closer peers
        peers: Vec<CompactPeer>,
    },
}

/// DHT message errors
#[derive(Debug, Error)]
pub enum MessageError {
    /// Serialization error
    #[error("Serialization failed: {0}")]
    Serialization(bincode::Error),

    /// Encryption error
    #[error("Encryption failed")]
    Encryption,

    /// Decryption error
    #[error("Decryption failed")]
    Decryption,

    /// Message too short
    #[error("Message too short to contain nonce")]
    TooShort,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ping_serialization() {
        let msg = DhtMessage::Ping(PingRequest {
            sender_id: NodeId::random(),
            sender_addr: "127.0.0.1:8000".parse().unwrap(),
            nonce: 12345,
        });

        let bytes = msg.to_bytes().unwrap();
        let decoded = DhtMessage::from_bytes(&bytes).unwrap();

        match decoded {
            DhtMessage::Ping(ping) => assert_eq!(ping.nonce, 12345),
            _ => panic!("Wrong message type"),
        }
    }

    #[test]
    fn test_find_node_serialization() {
        let target = NodeId::random();
        let msg = DhtMessage::FindNode(FindNodeRequest {
            sender_id: NodeId::random(),
            sender_addr: "127.0.0.1:8000".parse().unwrap(),
            target_id: target,
        });

        let bytes = msg.to_bytes().unwrap();
        let decoded = DhtMessage::from_bytes(&bytes).unwrap();

        match decoded {
            DhtMessage::FindNode(find) => assert_eq!(find.target_id, target),
            _ => panic!("Wrong message type"),
        }
    }

    #[test]
    fn test_store_serialization() {
        let key = [42u8; 32];
        let value = vec![1, 2, 3, 4, 5];

        let msg = DhtMessage::Store(StoreRequest {
            sender_id: NodeId::random(),
            sender_addr: "127.0.0.1:8000".parse().unwrap(),
            key,
            value: value.clone(),
            ttl: 3600,
        });

        let bytes = msg.to_bytes().unwrap();
        let decoded = DhtMessage::from_bytes(&bytes).unwrap();

        match decoded {
            DhtMessage::Store(store) => {
                assert_eq!(store.key, key);
                assert_eq!(store.value, value);
                assert_eq!(store.ttl, 3600);
            }
            _ => panic!("Wrong message type"),
        }
    }

    #[test]
    fn test_found_value_response() {
        let value_resp = DhtMessage::FoundValue(FoundValueResponse::Value {
            sender_id: NodeId::random(),
            value: vec![1, 2, 3],
        });

        let bytes = value_resp.to_bytes().unwrap();
        let decoded = DhtMessage::from_bytes(&bytes).unwrap();

        match decoded {
            DhtMessage::FoundValue(FoundValueResponse::Value { value, .. }) => {
                assert_eq!(value, vec![1, 2, 3]);
            }
            _ => panic!("Wrong message type"),
        }

        let peers_resp = DhtMessage::FoundValue(FoundValueResponse::Peers {
            sender_id: NodeId::random(),
            peers: vec![CompactPeer {
                id: NodeId::random(),
                addr: "127.0.0.1:8000".parse().unwrap(),
            }],
        });

        let bytes = peers_resp.to_bytes().unwrap();
        let decoded = DhtMessage::from_bytes(&bytes).unwrap();

        match decoded {
            DhtMessage::FoundValue(FoundValueResponse::Peers { peers, .. }) => {
                assert_eq!(peers.len(), 1);
            }
            _ => panic!("Wrong message type"),
        }
    }

    #[test]
    fn test_message_encryption() {
        let msg = DhtMessage::Ping(PingRequest {
            sender_id: NodeId::random(),
            sender_addr: "127.0.0.1:8000".parse().unwrap(),
            nonce: 12345,
        });

        let key = [1u8; 32];

        let encrypted = msg.encrypt(&key).unwrap();
        let decrypted = DhtMessage::decrypt(&encrypted, &key).unwrap();

        match decrypted {
            DhtMessage::Ping(ping) => assert_eq!(ping.nonce, 12345),
            _ => panic!("Wrong message type"),
        }
    }

    #[test]
    fn test_encryption_wrong_key_fails() {
        let msg = DhtMessage::Ping(PingRequest {
            sender_id: NodeId::random(),
            sender_addr: "127.0.0.1:8000".parse().unwrap(),
            nonce: 12345,
        });

        let key1 = [1u8; 32];
        let key2 = [2u8; 32];

        let encrypted = msg.encrypt(&key1).unwrap();
        assert!(DhtMessage::decrypt(&encrypted, &key2).is_err());
    }

    #[test]
    fn test_decrypt_too_short() {
        let key = [1u8; 32];
        let short_data = vec![1, 2, 3];

        let result = DhtMessage::decrypt(&short_data, &key);
        assert!(matches!(result, Err(MessageError::TooShort)));
    }

    #[test]
    fn test_sender_id() {
        let sender = NodeId::random();

        let ping = DhtMessage::Ping(PingRequest {
            sender_id: sender,
            sender_addr: "127.0.0.1:8000".parse().unwrap(),
            nonce: 12345,
        });

        assert_eq!(ping.sender_id(), Some(sender));

        let pong = DhtMessage::Pong(PongResponse {
            sender_id: sender,
            nonce: 12345,
        });

        assert_eq!(pong.sender_id(), Some(sender));
    }

    #[test]
    fn test_all_message_types_roundtrip() {
        let messages = vec![
            DhtMessage::Ping(PingRequest {
                sender_id: NodeId::random(),
                sender_addr: "127.0.0.1:8000".parse().unwrap(),
                nonce: 1,
            }),
            DhtMessage::Pong(PongResponse {
                sender_id: NodeId::random(),
                nonce: 1,
            }),
            DhtMessage::FindNode(FindNodeRequest {
                sender_id: NodeId::random(),
                sender_addr: "127.0.0.1:8000".parse().unwrap(),
                target_id: NodeId::random(),
            }),
            DhtMessage::FoundNodes(FoundNodesResponse {
                sender_id: NodeId::random(),
                peers: vec![],
            }),
            DhtMessage::Store(StoreRequest {
                sender_id: NodeId::random(),
                sender_addr: "127.0.0.1:8000".parse().unwrap(),
                key: [0u8; 32],
                value: vec![],
                ttl: 3600,
            }),
            DhtMessage::StoreAck(StoreAckResponse {
                sender_id: NodeId::random(),
                stored: true,
            }),
            DhtMessage::FindValue(FindValueRequest {
                sender_id: NodeId::random(),
                sender_addr: "127.0.0.1:8000".parse().unwrap(),
                key: [0u8; 32],
            }),
            DhtMessage::FoundValue(FoundValueResponse::Value {
                sender_id: NodeId::random(),
                value: vec![1, 2, 3],
            }),
            DhtMessage::FoundValue(FoundValueResponse::Peers {
                sender_id: NodeId::random(),
                peers: vec![],
            }),
        ];

        for msg in messages {
            let bytes = msg.to_bytes().unwrap();
            let _decoded = DhtMessage::from_bytes(&bytes).unwrap();
            // Successfully roundtripped
        }
    }
}
