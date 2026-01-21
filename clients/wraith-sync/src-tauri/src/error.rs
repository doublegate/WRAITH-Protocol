//! Error types for WRAITH Sync

use serde::Serialize;
use thiserror::Error;

/// Application error type
#[derive(Debug, Error)]
pub enum SyncError {
    #[error("Database error: {0}")]
    Database(String),

    #[error("File system error: {0}")]
    FileSystem(String),

    #[error("Watcher error: {0}")]
    Watcher(String),

    #[error("Sync error: {0}")]
    Sync(String),

    #[error("Node error: {0}")]
    Node(String),

    #[error("Node not running")]
    NodeNotRunning,

    #[error("Folder not found: {0}")]
    FolderNotFound(String),

    #[error("File not found: {0}")]
    FileNotFound(String),

    #[error("Conflict error: {0}")]
    Conflict(String),

    #[error("Version error: {0}")]
    Version(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

impl Serialize for SyncError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl From<rusqlite::Error> for SyncError {
    fn from(e: rusqlite::Error) -> Self {
        SyncError::Database(e.to_string())
    }
}

impl From<notify::Error> for SyncError {
    fn from(e: notify::Error) -> Self {
        SyncError::Watcher(e.to_string())
    }
}

impl From<anyhow::Error> for SyncError {
    fn from(e: anyhow::Error) -> Self {
        SyncError::Sync(e.to_string())
    }
}

/// Application result type
pub type SyncResult<T> = Result<T, SyncError>;
