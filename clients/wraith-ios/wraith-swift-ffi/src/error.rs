// Error types for WRAITH iOS FFI

use std::fmt;

/// Result type alias
pub type Result<T> = std::result::Result<T, WraithError>;

/// Error types exposed to Swift via UniFFI
#[derive(Debug, thiserror::Error, uniffi::Error)]
pub enum WraithError {
    #[error("Initialization failed: {message}")]
    InitializationFailed { message: String },

    #[error("Session establishment failed: {message}")]
    SessionFailed { message: String },

    #[error("File transfer failed: {message}")]
    TransferFailed { message: String },

    #[error("Node not started: {message}")]
    NotStarted { message: String },

    #[error("Invalid peer ID: {message}")]
    InvalidPeerId { message: String },

    #[error("Invalid file path: {message}")]
    InvalidFilePath { message: String },

    #[error("Other error: {message}")]
    Other { message: String },
}

impl WraithError {
    /// Create a new initialization error
    pub fn initialization(msg: impl Into<String>) -> Self {
        Self::InitializationFailed {
            message: msg.into(),
        }
    }

    /// Create a new session error
    pub fn session(msg: impl Into<String>) -> Self {
        Self::SessionFailed {
            message: msg.into(),
        }
    }

    /// Create a new transfer error
    pub fn transfer(msg: impl Into<String>) -> Self {
        Self::TransferFailed {
            message: msg.into(),
        }
    }

    /// Create a new not started error
    pub fn not_started(msg: impl Into<String>) -> Self {
        Self::NotStarted {
            message: msg.into(),
        }
    }

    /// Create a new invalid peer ID error
    pub fn invalid_peer_id(msg: impl Into<String>) -> Self {
        Self::InvalidPeerId {
            message: msg.into(),
        }
    }

    /// Create a new invalid file path error
    pub fn invalid_file_path(msg: impl Into<String>) -> Self {
        Self::InvalidFilePath {
            message: msg.into(),
        }
    }

    /// Create a generic error
    pub fn other(msg: impl Into<String>) -> Self {
        Self::Other {
            message: msg.into(),
        }
    }
}

impl From<std::io::Error> for WraithError {
    fn from(err: std::io::Error) -> Self {
        Self::Other {
            message: err.to_string(),
        }
    }
}

impl From<anyhow::Error> for WraithError {
    fn from(err: anyhow::Error) -> Self {
        Self::Other {
            message: err.to_string(),
        }
    }
}
