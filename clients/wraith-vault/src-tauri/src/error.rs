//! Error types for WRAITH Vault

use serde::Serialize;
use thiserror::Error;

/// Application error type
#[derive(Debug, Error)]
pub enum VaultError {
    #[error("Database error: {0}")]
    Database(String),

    #[error("File system error: {0}")]
    FileSystem(String),

    #[error("Cryptographic error: {0}")]
    Crypto(String),

    #[error("Backup error: {0}")]
    Backup(String),

    #[error("Restore error: {0}")]
    Restore(String),

    #[error("Chunk error: {0}")]
    Chunk(String),

    #[error("Compression error: {0}")]
    Compression(String),

    #[error("Erasure coding error: {0}")]
    ErasureCoding(String),

    #[error("Schedule error: {0}")]
    Schedule(String),

    #[error("Verification error: {0}")]
    Verification(String),

    #[error("Backup not found: {0}")]
    BackupNotFound(String),

    #[error("Chunk not found: {0}")]
    ChunkNotFound(String),

    #[error("Snapshot not found: {0}")]
    SnapshotNotFound(String),

    #[error("Insufficient shards: have {available}, need {required}")]
    InsufficientShards { available: usize, required: usize },

    #[error("Configuration error: {0}")]
    Config(String),

    // Secret storage errors (Phase 24)
    #[error("Secret error: {0}")]
    Secret(String),

    #[error("Secret not found: {0}")]
    SecretNotFound(String),

    #[error("Guardian error: {0}")]
    Guardian(String),

    #[error("Guardian not found: {0}")]
    GuardianNotFound(String),

    #[error("Shard error: {0}")]
    Shard(String),

    #[error("Shard not found: {0}")]
    ShardNotFound(String),

    #[error("Recovery error: {0}")]
    Recovery(String),

    #[error("Recovery session not found: {0}")]
    RecoverySessionNotFound(String),

    #[error("Shamir error: {0}")]
    Shamir(String),

    #[error("Distribution error: {0}")]
    Distribution(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

impl Serialize for VaultError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl From<rusqlite::Error> for VaultError {
    fn from(e: rusqlite::Error) -> Self {
        VaultError::Database(e.to_string())
    }
}

impl From<anyhow::Error> for VaultError {
    fn from(e: anyhow::Error) -> Self {
        VaultError::Backup(e.to_string())
    }
}

impl From<chacha20poly1305::Error> for VaultError {
    fn from(_e: chacha20poly1305::Error) -> Self {
        VaultError::Crypto("AEAD encryption/decryption failed".to_string())
    }
}

impl From<zstd::stream::raw::CParameter> for VaultError {
    fn from(e: zstd::stream::raw::CParameter) -> Self {
        VaultError::Compression(format!("Compression parameter error: {:?}", e))
    }
}

/// Application result type
pub type VaultResult<T> = Result<T, VaultError>;
