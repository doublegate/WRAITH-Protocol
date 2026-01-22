//! Error types for WRAITH Share

use serde::Serialize;
use thiserror::Error;

/// Application error type
#[derive(Debug, Error)]
pub enum ShareError {
    #[error("Database error: {0}")]
    Database(String),

    #[error("File system error: {0}")]
    FileSystem(String),

    #[error("Cryptographic error: {0}")]
    Crypto(String),

    #[error("Group error: {0}")]
    Group(String),

    #[error("Group not found: {0}")]
    GroupNotFound(String),

    #[error("Member not found: {0}")]
    MemberNotFound(String),

    #[error("File not found: {0}")]
    FileNotFound(String),

    #[error("Version not found: {0}")]
    VersionNotFound(String),

    #[error("Link not found: {0}")]
    LinkNotFound(String),

    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    #[error("Invalid invitation: {0}")]
    InvalidInvitation(String),

    #[error("Invitation expired")]
    InvitationExpired,

    #[error("Access revoked")]
    AccessRevoked,

    #[error("Invalid password")]
    InvalidPassword,

    #[error("Link expired")]
    LinkExpired,

    #[error("Download limit exceeded")]
    DownloadLimitExceeded,

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

impl Serialize for ShareError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl From<rusqlite::Error> for ShareError {
    fn from(e: rusqlite::Error) -> Self {
        ShareError::Database(e.to_string())
    }
}

impl From<anyhow::Error> for ShareError {
    fn from(e: anyhow::Error) -> Self {
        ShareError::Group(e.to_string())
    }
}

impl From<chacha20poly1305::Error> for ShareError {
    fn from(_e: chacha20poly1305::Error) -> Self {
        ShareError::Crypto("AEAD encryption/decryption failed".to_string())
    }
}

impl From<ed25519_dalek::SignatureError> for ShareError {
    fn from(e: ed25519_dalek::SignatureError) -> Self {
        ShareError::Crypto(format!("Signature error: {}", e))
    }
}

/// Application result type
pub type ShareResult<T> = Result<T, ShareError>;
