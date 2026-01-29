//! # WRAITH Files
//!
//! File transfer engine for the WRAITH protocol.
//!
//! This crate provides:
//! - File chunking with configurable chunk size
//! - BLAKE3 tree hashing for integrity verification
//! - Transfer state machine with resume support
//! - Parallel chunk processing

#![warn(missing_docs)]
#![warn(clippy::all)]

pub mod chunker;
pub mod hasher;
pub mod transfer;
pub mod tree_hash;

// Linux-only high-performance file I/O using io_uring
#[cfg(target_os = "linux")]
pub mod async_file;
#[cfg(target_os = "linux")]
pub mod io_uring;

/// Default chunk size (1 MiB).
/// Larger chunks reduce per-transfer overhead (Merkle tree nodes, session
/// scheduling, chunk requests) by 4x compared to the previous 256 KiB default.
/// BLAKE3 throughput is equally high at 1 MiB (~4.86 GiB/s).
pub const DEFAULT_CHUNK_SIZE: usize = 1024 * 1024;

/// File metadata for transfers
#[derive(Debug, Clone)]
pub struct FileMetadata {
    /// File name
    pub name: String,
    /// File size in bytes
    pub size: u64,
    /// BLAKE3 hash of entire file
    pub hash: [u8; 32],
    /// Number of chunks
    pub chunk_count: u64,
}
