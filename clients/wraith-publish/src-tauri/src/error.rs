//! Error types for WRAITH Publish

use serde::Serialize;
use thiserror::Error;

/// Application error type
#[derive(Debug, Error)]
pub enum PublishError {
    #[error("Database error: {0}")]
    Database(String),

    #[error("File system error: {0}")]
    FileSystem(String),

    #[error("Cryptographic error: {0}")]
    Crypto(String),

    #[error("Article not found: {0}")]
    ArticleNotFound(String),

    #[error("Draft not found: {0}")]
    DraftNotFound(String),

    #[error("Image not found: {0}")]
    ImageNotFound(String),

    #[error("Invalid content: {0}")]
    InvalidContent(String),

    #[error("Search error: {0}")]
    Search(String),

    #[error("Publishing error: {0}")]
    Publishing(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

impl Serialize for PublishError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl From<rusqlite::Error> for PublishError {
    fn from(e: rusqlite::Error) -> Self {
        PublishError::Database(e.to_string())
    }
}

impl From<anyhow::Error> for PublishError {
    fn from(e: anyhow::Error) -> Self {
        PublishError::Publishing(e.to_string())
    }
}

impl From<chacha20poly1305::Error> for PublishError {
    fn from(_e: chacha20poly1305::Error) -> Self {
        PublishError::Crypto("AEAD encryption/decryption failed".to_string())
    }
}

impl From<ed25519_dalek::SignatureError> for PublishError {
    fn from(e: ed25519_dalek::SignatureError) -> Self {
        PublishError::Crypto(format!("Signature error: {}", e))
    }
}

/// Application result type
pub type PublishResult<T> = Result<T, PublishError>;
