//! Error types for WRAITH Recon
//!
//! Defines all error types used throughout the application, with support
//! for serialization to the frontend via Tauri IPC.

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Main error type for WRAITH Recon operations
#[derive(Debug, Error, Clone, Serialize, Deserialize)]
pub enum ReconError {
    /// Rules of Engagement not loaded
    #[error("Rules of Engagement not loaded")]
    RoENotLoaded,

    /// Invalid Rules of Engagement
    #[error("Invalid Rules of Engagement: {0}")]
    InvalidRoE(String),

    /// Rules of Engagement signature verification failed
    #[error("RoE signature verification failed: {0}")]
    RoESignatureInvalid(String),

    /// Target outside authorized scope
    #[error("Target {target} is outside authorized scope")]
    TargetOutOfScope { target: String },

    /// Engagement window violation
    #[error("Operation attempted outside engagement window: {0}")]
    EngagementWindowViolation(String),

    /// Kill switch activated
    #[error("Kill switch activated: {0}")]
    KillSwitchActivated(String),

    /// Invalid kill switch signal
    #[error("Invalid kill switch signal: {0}")]
    InvalidKillSwitchSignal(String),

    /// Engagement not active
    #[error("No active engagement")]
    EngagementNotActive,

    /// Engagement already active
    #[error("Engagement already active")]
    EngagementAlreadyActive,

    /// Audit chain tampering detected
    #[error("Audit chain integrity violation: {0}")]
    AuditChainTampered(String),

    /// Invalid target specification
    #[error("Invalid target specification: {0}")]
    InvalidTarget(String),

    /// Channel error
    #[error("Channel error: {0}")]
    ChannelError(String),

    /// Network error
    #[error("Network error: {0}")]
    NetworkError(String),

    /// Transport error
    #[error("Transport error: {0}")]
    TransportError(String),

    /// Database error
    #[error("Database error: {0}")]
    DatabaseError(String),

    /// Cryptographic error
    #[error("Cryptographic error: {0}")]
    CryptoError(String),

    /// Configuration error
    #[error("Configuration error: {0}")]
    ConfigError(String),

    /// Permission denied
    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    /// Operation timeout
    #[error("Operation timed out: {0}")]
    Timeout(String),

    /// Resource not found
    #[error("Resource not found: {0}")]
    NotFound(String),

    /// Invalid state
    #[error("Invalid state: {0}")]
    InvalidState(String),

    /// Internal error
    #[error("Internal error: {0}")]
    Internal(String),

    /// IO error
    #[error("IO error: {0}")]
    IoError(String),

    /// WRAITH Protocol error
    #[error("WRAITH Protocol error: {0}")]
    WraithProtocol(String),
}

/// Result type alias for WRAITH Recon operations
pub type Result<T> = std::result::Result<T, ReconError>;

impl From<std::io::Error> for ReconError {
    fn from(err: std::io::Error) -> Self {
        ReconError::IoError(err.to_string())
    }
}

impl From<rusqlite::Error> for ReconError {
    fn from(err: rusqlite::Error) -> Self {
        ReconError::DatabaseError(err.to_string())
    }
}

impl From<serde_json::Error> for ReconError {
    fn from(err: serde_json::Error) -> Self {
        ReconError::ConfigError(format!("JSON error: {}", err))
    }
}

impl From<ipnetwork::IpNetworkError> for ReconError {
    fn from(err: ipnetwork::IpNetworkError) -> Self {
        ReconError::InvalidTarget(format!("Invalid IP network: {}", err))
    }
}

impl From<ed25519_dalek::SignatureError> for ReconError {
    fn from(err: ed25519_dalek::SignatureError) -> Self {
        ReconError::CryptoError(format!("Signature error: {}", err))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = ReconError::TargetOutOfScope {
            target: "192.168.1.1".to_string(),
        };
        assert!(err.to_string().contains("192.168.1.1"));
        assert!(err.to_string().contains("outside authorized scope"));
    }

    #[test]
    fn test_error_serialization() {
        let err = ReconError::KillSwitchActivated("Emergency halt".to_string());
        let json = serde_json::to_string(&err).unwrap();
        assert!(json.contains("KillSwitchActivated"));
        assert!(json.contains("Emergency halt"));
    }

    #[test]
    fn test_error_from_io() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let recon_err: ReconError = io_err.into();
        assert!(matches!(recon_err, ReconError::IoError(_)));
    }
}
