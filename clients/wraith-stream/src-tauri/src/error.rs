//! Error types for WRAITH Stream

use serde::Serialize;
use thiserror::Error;

/// Application error type
#[derive(Debug, Error)]
pub enum StreamError {
    #[error("Database error: {0}")]
    Database(String),

    #[error("File system error: {0}")]
    FileSystem(String),

    #[error("Stream not found: {0}")]
    StreamNotFound(String),

    #[error("Segment not found: {0}")]
    SegmentNotFound(String),

    #[error("Transcode error: {0}")]
    Transcode(String),

    #[error("FFmpeg not found - please install FFmpeg")]
    FfmpegNotFound,

    #[error("FFmpeg error: {0}")]
    FfmpegError(String),

    #[error("Invalid video format: {0}")]
    InvalidFormat(String),

    #[error("Playback error: {0}")]
    Playback(String),

    #[error("Subtitle error: {0}")]
    Subtitle(String),

    #[error("Cryptographic error: {0}")]
    Crypto(String),

    #[error("Manifest parse error: {0}")]
    ManifestParse(String),

    #[error("Not initialized: {0}")]
    NotInitialized(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Transcode in progress")]
    TranscodeInProgress,

    #[error("Transcode cancelled")]
    TranscodeCancelled,
}

impl Serialize for StreamError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl From<rusqlite::Error> for StreamError {
    fn from(e: rusqlite::Error) -> Self {
        StreamError::Database(e.to_string())
    }
}

impl From<anyhow::Error> for StreamError {
    fn from(e: anyhow::Error) -> Self {
        StreamError::Database(e.to_string())
    }
}

impl From<chacha20poly1305::Error> for StreamError {
    fn from(_e: chacha20poly1305::Error) -> Self {
        StreamError::Crypto("AEAD encryption/decryption failed".to_string())
    }
}

/// Application result type
pub type StreamResult<T> = Result<T, StreamError>;
