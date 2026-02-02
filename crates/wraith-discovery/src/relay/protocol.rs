//! Relay protocol message definitions.

use serde::{Deserialize, Serialize};
use std::fmt;

/// Node identifier (32-byte public key or derived ID)
pub type NodeId = [u8; 32];

/// Relay protocol messages
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RelayMessage {
    /// Client registers with relay
    Register {
        /// Client's node ID
        node_id: NodeId,
        /// Client's public key for verification
        public_key: [u8; 32],
    },

    /// Relay acknowledges registration
    RegisterAck {
        /// Relay's unique identifier
        relay_id: [u8; 32],
        /// Whether registration succeeded
        success: bool,
        /// Optional error message
        error: Option<String>,
    },

    /// Client sends packet to another peer through relay
    SendPacket {
        /// Destination node ID
        dest_id: NodeId,
        /// Encrypted payload (relay cannot decrypt)
        payload: Vec<u8>,
    },

    /// Relay forwards packet to recipient
    RecvPacket {
        /// Source node ID
        src_id: NodeId,
        /// Encrypted payload
        payload: Vec<u8>,
    },

    /// Notify client that a peer came online
    PeerOnline {
        /// Peer's node ID
        peer_id: NodeId,
    },

    /// Notify client that a peer went offline
    PeerOffline {
        /// Peer's node ID
        peer_id: NodeId,
    },

    /// Keepalive message (no payload)
    Keepalive,

    /// Client disconnects from relay
    Disconnect,

    /// Relay error response
    Error {
        /// Error code
        code: RelayErrorCode,
        /// Human-readable error message
        message: String,
    },
}

/// Relay error codes
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum RelayErrorCode {
    /// Client not registered with relay
    NotRegistered = 1,
    /// Destination peer not found
    PeerNotFound = 2,
    /// Rate limit exceeded
    RateLimited = 3,
    /// Invalid message format
    InvalidMessage = 4,
    /// Server at capacity
    ServerFull = 5,
    /// Authentication failed
    AuthFailed = 6,
    /// Internal server error
    InternalError = 7,
}

impl RelayMessage {
    /// Serialize message to bytes
    pub fn to_bytes(&self) -> Result<Vec<u8>, RelayError> {
        bincode::serde::encode_to_vec(self, bincode::config::standard())
            .map_err(|e| RelayError::Serialization(e.to_string()))
    }

    /// Deserialize message from bytes
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, RelayError> {
        bincode::serde::decode_from_slice(bytes, bincode::config::standard())
            .map(|(msg, _)| msg)
            .map_err(|e| RelayError::Deserialization(e.to_string()))
    }

    /// Get the message type name
    pub fn message_type(&self) -> &'static str {
        match self {
            RelayMessage::Register { .. } => "Register",
            RelayMessage::RegisterAck { .. } => "RegisterAck",
            RelayMessage::SendPacket { .. } => "SendPacket",
            RelayMessage::RecvPacket { .. } => "RecvPacket",
            RelayMessage::PeerOnline { .. } => "PeerOnline",
            RelayMessage::PeerOffline { .. } => "PeerOffline",
            RelayMessage::Keepalive => "Keepalive",
            RelayMessage::Disconnect => "Disconnect",
            RelayMessage::Error { .. } => "Error",
        }
    }
}

/// Relay errors
#[derive(Debug, Clone)]
pub enum RelayError {
    /// Serialization error
    Serialization(String),
    /// Deserialization error
    Deserialization(String),
    /// Network I/O error
    Io(String),
    /// Connection timeout
    Timeout,
    /// Client not registered
    NotRegistered,
    /// Peer not found
    PeerNotFound,
    /// Rate limited
    RateLimited,
    /// Invalid message
    InvalidMessage,
    /// Server full
    ServerFull,
    /// Authentication failed
    AuthFailed,
    /// Internal error
    Internal(String),
}

impl fmt::Display for RelayError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RelayError::Serialization(e) => write!(f, "Serialization error: {e}"),
            RelayError::Deserialization(e) => write!(f, "Deserialization error: {e}"),
            RelayError::Io(e) => write!(f, "I/O error: {e}"),
            RelayError::Timeout => write!(f, "Connection timeout"),
            RelayError::NotRegistered => write!(f, "Client not registered"),
            RelayError::PeerNotFound => write!(f, "Peer not found"),
            RelayError::RateLimited => write!(f, "Rate limited"),
            RelayError::InvalidMessage => write!(f, "Invalid message"),
            RelayError::ServerFull => write!(f, "Server at capacity"),
            RelayError::AuthFailed => write!(f, "Authentication failed"),
            RelayError::Internal(e) => write!(f, "Internal error: {e}"),
        }
    }
}

impl std::error::Error for RelayError {}

impl From<std::io::Error> for RelayError {
    fn from(err: std::io::Error) -> Self {
        RelayError::Io(err.to_string())
    }
}

impl From<RelayErrorCode> for RelayError {
    fn from(code: RelayErrorCode) -> Self {
        match code {
            RelayErrorCode::NotRegistered => RelayError::NotRegistered,
            RelayErrorCode::PeerNotFound => RelayError::PeerNotFound,
            RelayErrorCode::RateLimited => RelayError::RateLimited,
            RelayErrorCode::InvalidMessage => RelayError::InvalidMessage,
            RelayErrorCode::ServerFull => RelayError::ServerFull,
            RelayErrorCode::AuthFailed => RelayError::AuthFailed,
            RelayErrorCode::InternalError => RelayError::Internal("Unknown error".to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_serialization_register() {
        let msg = RelayMessage::Register {
            node_id: [1u8; 32],
            public_key: [2u8; 32],
        };

        let bytes = msg.to_bytes().unwrap();
        let decoded = RelayMessage::from_bytes(&bytes).unwrap();

        assert_eq!(msg, decoded);
    }

    #[test]
    fn test_message_serialization_register_ack() {
        let msg = RelayMessage::RegisterAck {
            relay_id: [3u8; 32],
            success: true,
            error: None,
        };

        let bytes = msg.to_bytes().unwrap();
        let decoded = RelayMessage::from_bytes(&bytes).unwrap();

        assert_eq!(msg, decoded);
    }

    #[test]
    fn test_message_serialization_send_packet() {
        let msg = RelayMessage::SendPacket {
            dest_id: [4u8; 32],
            payload: vec![1, 2, 3, 4, 5],
        };

        let bytes = msg.to_bytes().unwrap();
        let decoded = RelayMessage::from_bytes(&bytes).unwrap();

        assert_eq!(msg, decoded);
    }

    #[test]
    fn test_message_serialization_recv_packet() {
        let msg = RelayMessage::RecvPacket {
            src_id: [5u8; 32],
            payload: vec![6, 7, 8, 9, 10],
        };

        let bytes = msg.to_bytes().unwrap();
        let decoded = RelayMessage::from_bytes(&bytes).unwrap();

        assert_eq!(msg, decoded);
    }

    #[test]
    fn test_message_serialization_peer_online() {
        let msg = RelayMessage::PeerOnline { peer_id: [6u8; 32] };

        let bytes = msg.to_bytes().unwrap();
        let decoded = RelayMessage::from_bytes(&bytes).unwrap();

        assert_eq!(msg, decoded);
    }

    #[test]
    fn test_message_serialization_peer_offline() {
        let msg = RelayMessage::PeerOffline { peer_id: [7u8; 32] };

        let bytes = msg.to_bytes().unwrap();
        let decoded = RelayMessage::from_bytes(&bytes).unwrap();

        assert_eq!(msg, decoded);
    }

    #[test]
    fn test_message_serialization_keepalive() {
        let msg = RelayMessage::Keepalive;

        let bytes = msg.to_bytes().unwrap();
        let decoded = RelayMessage::from_bytes(&bytes).unwrap();

        assert_eq!(msg, decoded);
    }

    #[test]
    fn test_message_serialization_disconnect() {
        let msg = RelayMessage::Disconnect;

        let bytes = msg.to_bytes().unwrap();
        let decoded = RelayMessage::from_bytes(&bytes).unwrap();

        assert_eq!(msg, decoded);
    }

    #[test]
    fn test_message_serialization_error() {
        let msg = RelayMessage::Error {
            code: RelayErrorCode::PeerNotFound,
            message: "Peer not found".to_string(),
        };

        let bytes = msg.to_bytes().unwrap();
        let decoded = RelayMessage::from_bytes(&bytes).unwrap();

        assert_eq!(msg, decoded);
    }

    #[test]
    fn test_message_type() {
        let msg = RelayMessage::Register {
            node_id: [1u8; 32],
            public_key: [2u8; 32],
        };
        assert_eq!(msg.message_type(), "Register");

        let msg = RelayMessage::Keepalive;
        assert_eq!(msg.message_type(), "Keepalive");
    }

    #[test]
    fn test_error_display() {
        let err = RelayError::NotRegistered;
        assert_eq!(err.to_string(), "Client not registered");

        let err = RelayError::Timeout;
        assert_eq!(err.to_string(), "Connection timeout");
    }

    #[test]
    fn test_error_from_code() {
        let err: RelayError = RelayErrorCode::PeerNotFound.into();
        assert!(matches!(err, RelayError::PeerNotFound));

        let err: RelayError = RelayErrorCode::RateLimited.into();
        assert!(matches!(err, RelayError::RateLimited));
    }

    #[test]
    fn test_error_from_all_codes() {
        let err: RelayError = RelayErrorCode::NotRegistered.into();
        assert!(matches!(err, RelayError::NotRegistered));

        let err: RelayError = RelayErrorCode::InvalidMessage.into();
        assert!(matches!(err, RelayError::InvalidMessage));

        let err: RelayError = RelayErrorCode::ServerFull.into();
        assert!(matches!(err, RelayError::ServerFull));

        let err: RelayError = RelayErrorCode::AuthFailed.into();
        assert!(matches!(err, RelayError::AuthFailed));

        let err: RelayError = RelayErrorCode::InternalError.into();
        assert!(matches!(err, RelayError::Internal(_)));
    }

    #[test]
    fn test_error_display_all_variants() {
        assert_eq!(
            RelayError::Serialization("test".to_string()).to_string(),
            "Serialization error: test"
        );
        assert_eq!(
            RelayError::Deserialization("test".to_string()).to_string(),
            "Deserialization error: test"
        );
        assert_eq!(
            RelayError::Io("test".to_string()).to_string(),
            "I/O error: test"
        );
        assert_eq!(RelayError::Timeout.to_string(), "Connection timeout");
        assert_eq!(
            RelayError::NotRegistered.to_string(),
            "Client not registered"
        );
        assert_eq!(RelayError::PeerNotFound.to_string(), "Peer not found");
        assert_eq!(RelayError::RateLimited.to_string(), "Rate limited");
        assert_eq!(RelayError::InvalidMessage.to_string(), "Invalid message");
        assert_eq!(RelayError::ServerFull.to_string(), "Server at capacity");
        assert_eq!(RelayError::AuthFailed.to_string(), "Authentication failed");
        assert_eq!(
            RelayError::Internal("oops".to_string()).to_string(),
            "Internal error: oops"
        );
    }

    #[test]
    fn test_error_from_io_error() {
        let io_err = std::io::Error::new(std::io::ErrorKind::ConnectionRefused, "refused");
        let relay_err: RelayError = io_err.into();
        assert!(matches!(relay_err, RelayError::Io(_)));
        assert!(relay_err.to_string().contains("refused"));
    }

    #[test]
    fn test_error_is_error_trait() {
        let err = RelayError::Timeout;
        let _: &dyn std::error::Error = &err;
    }

    #[test]
    fn test_message_type_all_variants() {
        assert_eq!(
            RelayMessage::RegisterAck {
                relay_id: [0; 32],
                success: true,
                error: None,
            }
            .message_type(),
            "RegisterAck"
        );
        assert_eq!(
            RelayMessage::SendPacket {
                dest_id: [0; 32],
                payload: vec![],
            }
            .message_type(),
            "SendPacket"
        );
        assert_eq!(
            RelayMessage::RecvPacket {
                src_id: [0; 32],
                payload: vec![],
            }
            .message_type(),
            "RecvPacket"
        );
        assert_eq!(
            RelayMessage::PeerOnline { peer_id: [0; 32] }.message_type(),
            "PeerOnline"
        );
        assert_eq!(
            RelayMessage::PeerOffline { peer_id: [0; 32] }.message_type(),
            "PeerOffline"
        );
        assert_eq!(RelayMessage::Disconnect.message_type(), "Disconnect");
        assert_eq!(
            RelayMessage::Error {
                code: RelayErrorCode::InternalError,
                message: "err".to_string(),
            }
            .message_type(),
            "Error"
        );
    }

    #[test]
    fn test_deserialization_invalid_bytes() {
        let result = RelayMessage::from_bytes(&[0xFF, 0xFF, 0xFF]);
        assert!(result.is_err());
        if let Err(RelayError::Deserialization(msg)) = result {
            assert!(!msg.is_empty());
        } else {
            panic!("Expected Deserialization error");
        }
    }

    #[test]
    fn test_register_ack_with_error() {
        let msg = RelayMessage::RegisterAck {
            relay_id: [0u8; 32],
            success: false,
            error: Some("denied".to_string()),
        };
        let bytes = msg.to_bytes().unwrap();
        let decoded = RelayMessage::from_bytes(&bytes).unwrap();
        assert_eq!(msg, decoded);
    }

    #[test]
    fn test_large_payload_roundtrip() {
        let msg = RelayMessage::SendPacket {
            dest_id: [9u8; 32],
            payload: vec![0xAB; 8192],
        };
        let bytes = msg.to_bytes().unwrap();
        let decoded = RelayMessage::from_bytes(&bytes).unwrap();
        assert_eq!(msg, decoded);
    }

    #[test]
    fn test_deserialization_empty_bytes() {
        let result = RelayMessage::from_bytes(&[]);
        assert!(result.is_err());
        if let Err(RelayError::Deserialization(msg)) = result {
            assert!(!msg.is_empty());
        } else {
            panic!("Expected Deserialization error");
        }
    }

    #[test]
    fn test_relay_error_clone() {
        let err = RelayError::Internal("clone test".to_string());
        let cloned = err.clone();
        assert_eq!(err.to_string(), cloned.to_string());
    }

    #[test]
    fn test_relay_error_debug() {
        let err = RelayError::Timeout;
        let debug = format!("{:?}", err);
        assert!(debug.contains("Timeout"));
    }

    #[test]
    fn test_relay_message_clone() {
        let msg = RelayMessage::Register {
            node_id: [1u8; 32],
            public_key: [2u8; 32],
        };
        let cloned = msg.clone();
        assert_eq!(msg, cloned);
    }

    #[test]
    fn test_relay_message_debug() {
        let msg = RelayMessage::Keepalive;
        let debug = format!("{:?}", msg);
        assert!(debug.contains("Keepalive"));
    }

    #[test]
    fn test_relay_error_code_copy() {
        let code = RelayErrorCode::PeerNotFound;
        let copied = code;
        assert_eq!(code, copied);
    }

    #[test]
    fn test_all_error_codes_serialization() {
        let codes = vec![
            RelayErrorCode::NotRegistered,
            RelayErrorCode::PeerNotFound,
            RelayErrorCode::RateLimited,
            RelayErrorCode::InvalidMessage,
            RelayErrorCode::ServerFull,
            RelayErrorCode::AuthFailed,
            RelayErrorCode::InternalError,
        ];
        for code in codes {
            let msg = RelayMessage::Error {
                code,
                message: format!("{code:?}"),
            };
            let bytes = msg.to_bytes().unwrap();
            let decoded = RelayMessage::from_bytes(&bytes).unwrap();
            assert_eq!(msg, decoded);
        }
    }
}
