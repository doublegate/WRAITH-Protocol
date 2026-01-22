//! Error types for WRAITH Mesh
//!
//! Provides unified error handling for network monitoring, DHT inspection,
//! and diagnostic operations.

use thiserror::Error;

/// Result type alias for Mesh operations
pub type MeshResult<T> = Result<T, MeshError>;

/// Mesh-specific errors
#[derive(Debug, Error)]
pub enum MeshError {
    /// Network monitoring error
    #[error("Network error: {0}")]
    Network(String),

    /// DHT inspection error
    #[error("DHT error: {0}")]
    Dht(String),

    /// Diagnostic operation error
    #[error("Diagnostic error: {0}")]
    Diagnostic(String),

    /// Database error
    #[error("Database error: {0}")]
    Database(String),

    /// I/O error
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Serialization error
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// State not initialized
    #[error("State not initialized: {0}")]
    NotInitialized(String),

    /// Peer not found
    #[error("Peer not found: {0}")]
    PeerNotFound(String),

    /// Operation timeout
    #[error("Operation timeout: {0}")]
    Timeout(String),
}

impl From<rusqlite::Error> for MeshError {
    fn from(err: rusqlite::Error) -> Self {
        MeshError::Database(err.to_string())
    }
}

impl serde::Serialize for MeshError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}
