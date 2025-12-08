//! Error types for the WRAITH Transfer application

use serde::Serialize;
use thiserror::Error;

/// Application error type
#[derive(Debug, Error)]
pub enum AppError {
    #[error("Node error: {0}")]
    Node(String),

    #[error("Node not running")]
    NodeNotRunning,

    #[error("Session error: {0}")]
    Session(String),

    #[error("Transfer error: {0}")]
    Transfer(String),

    #[error("Invalid peer ID: {0}")]
    InvalidPeerId(String),

    #[error("File not found: {0}")]
    FileNotFound(String),

    #[error("IO error: {0}")]
    Io(String),

    #[error("Configuration error: {0}")]
    Config(String),
}

impl Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl From<std::io::Error> for AppError {
    fn from(err: std::io::Error) -> Self {
        AppError::Io(err.to_string())
    }
}

impl From<wraith_core::node::NodeError> for AppError {
    fn from(err: wraith_core::node::NodeError) -> Self {
        AppError::Node(err.to_string())
    }
}
