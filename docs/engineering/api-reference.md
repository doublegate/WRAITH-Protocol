# WRAITH Protocol API Reference

**Document Version:** 1.0.0
**Last Updated:** 2025-11-28
**Status:** Engineering Documentation

---

## Overview

This document provides a comprehensive reference for the WRAITH Protocol's public APIs. The protocol is implemented as a Rust workspace with multiple crates, each exposing specific functionality.

**Crate Organization:**
- **wraith-core:** Core session management, framing
- **wraith-crypto:** Cryptographic primitives
- **wraith-transport:** Network transport layer
- **wraith-obfuscation:** Traffic obfuscation (padding, timing, mimicry)
- **wraith-discovery:** DHT and peer discovery
- **wraith-files:** File transfer operations
- **wraith-ffi:** FFI bindings (C, JNI, UniFFI)
- **wraith-cli:** Command-line interface

---

## wraith-core

### Session Management

#### `Session`

Represents an established secure session with a peer.

```rust
pub struct Session {
    // Private fields
}

impl Session {
    /// Creates a new session after successful handshake.
    ///
    /// # Arguments
    ///
    /// * `keys` - Symmetric encryption keys derived from handshake
    /// * `peer_id` - Peer's long-term public key
    /// * `socket` - UDP socket for communication
    ///
    /// # Returns
    ///
    /// A new `Session` instance.
    pub fn new(
        keys: SymmetricKeys,
        peer_id: PublicKey,
        socket: Arc<UdpSocket>,
    ) -> Self;

    /// Sends a data frame to the peer.
    ///
    /// # Arguments
    ///
    /// * `payload` - Plaintext data to send (max 1200 bytes recommended)
    ///
    /// # Returns
    ///
    /// Ok(()) on success, or SessionError on failure.
    ///
    /// # Errors
    ///
    /// - `SessionError::PayloadTooLarge` if payload exceeds max size
    /// - `SessionError::EncryptionFailed` if encryption fails
    /// - `SessionError::NetworkError` if socket send fails
    pub async fn send(&mut self, payload: &[u8]) -> Result<(), SessionError>;

    /// Receives the next data frame from the peer.
    ///
    /// # Returns
    ///
    /// The decrypted payload bytes, or SessionError on failure.
    ///
    /// # Errors
    ///
    /// - `SessionError::Timeout` if no frame received within timeout
    /// - `SessionError::DecryptionFailed` if frame authentication fails
    /// - `SessionError::ReplayAttack` if nonce is reused
    pub async fn recv(&mut self) -> Result<Vec<u8>, SessionError>;

    /// Closes the session gracefully.
    ///
    /// Sends a TERMINATE frame to the peer and cleans up resources.
    ///
    /// # Returns
    ///
    /// Ok(()) if session closed successfully.
    pub async fn close(self) -> Result<(), SessionError>;

    /// Returns the peer's public key.
    pub fn peer_id(&self) -> &PublicKey;

    /// Returns session statistics.
    pub fn stats(&self) -> SessionStats;
}
```

#### `SessionStats`

Statistics about an active session.

```rust
#[derive(Debug, Clone)]
pub struct SessionStats {
    /// Bytes sent
    pub bytes_sent: u64,
    /// Bytes received
    pub bytes_received: u64,
    /// Packets sent
    pub packets_sent: u64,
    /// Packets received
    pub packets_received: u64,
    /// Round-trip time estimate
    pub rtt: Duration,
    /// Session establishment time
    pub established_at: Instant,
}
```

### Frame Types

#### `Frame`

Represents a protocol frame.

```rust
#[derive(Debug, Clone)]
pub struct Frame {
    pub frame_type: FrameType,
    pub payload: Vec<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameType {
    Handshake = 0x00,
    Data = 0x01,
    Ack = 0x02,
    PathChallenge = 0x03,
    PathResponse = 0x04,
    Terminate = 0x05,
}

impl Frame {
    /// Parses a frame from raw bytes.
    ///
    /// # Arguments
    ///
    /// * `data` - Raw frame bytes (minimum 3 bytes)
    ///
    /// # Returns
    ///
    /// Parsed `Frame` or `FrameError`.
    ///
    /// # Errors
    ///
    /// - `FrameError::TooShort` if data < 3 bytes
    /// - `FrameError::InvalidType` if frame type is unknown
    /// - `FrameError::InvalidLength` if length field doesn't match data
    pub fn parse(data: &[u8]) -> Result<Self, FrameError>;

    /// Serializes frame to bytes.
    ///
    /// # Returns
    ///
    /// Frame encoded as bytes (3-byte header + payload).
    pub fn to_bytes(&self) -> Vec<u8>;
}
```

### Error Types

```rust
#[derive(Debug, Error)]
pub enum SessionError {
    #[error("handshake timeout after {0:?}")]
    HandshakeTimeout(Duration),

    #[error("payload too large: {size} bytes (max {max})")]
    PayloadTooLarge { size: usize, max: usize },

    #[error("decryption failed")]
    DecryptionFailed,

    #[error("replay attack detected")]
    ReplayAttack,

    #[error("network error: {0}")]
    NetworkError(#[from] std::io::Error),
}

#[derive(Debug, Error)]
pub enum FrameError {
    #[error("frame too short: expected {expected}, got {actual}")]
    TooShort { expected: usize, actual: usize },

    #[error("invalid frame type: {0}")]
    InvalidType(u8),

    #[error("invalid length field")]
    InvalidLength,
}
```

---

## wraith-crypto

### Noise Protocol

#### `NoiseHandshake`

Performs Noise_XX handshake pattern.

```rust
pub struct NoiseHandshake {
    // Private fields
}

impl NoiseHandshake {
    /// Creates a new handshake initiator.
    ///
    /// # Arguments
    ///
    /// * `static_keypair` - Local long-term Ed25519 keypair
    ///
    /// # Returns
    ///
    /// A new `NoiseHandshake` in initiator mode.
    pub fn new_initiator(static_keypair: Keypair) -> Self;

    /// Creates a new handshake responder.
    ///
    /// # Arguments
    ///
    /// * `static_keypair` - Local long-term Ed25519 keypair
    ///
    /// # Returns
    ///
    /// A new `NoiseHandshake` in responder mode.
    pub fn new_responder(static_keypair: Keypair) -> Self;

    /// Generates the first handshake message (initiator only).
    ///
    /// # Returns
    ///
    /// Handshake message 1 (ephemeral key, ~32 bytes).
    pub fn write_message_1(&mut self) -> Result<Vec<u8>, NoiseError>;

    /// Processes handshake message 1 (responder only).
    ///
    /// # Arguments
    ///
    /// * `message` - Message 1 from initiator
    ///
    /// # Returns
    ///
    /// Ok(()) if message is valid.
    pub fn read_message_1(&mut self, message: &[u8]) -> Result<(), NoiseError>;

    /// Generates handshake message 2 (responder only).
    ///
    /// # Returns
    ///
    /// Handshake message 2 (ephemeral + static keys, ~96 bytes).
    pub fn write_message_2(&mut self) -> Result<Vec<u8>, NoiseError>;

    /// Processes handshake message 2 (initiator only).
    ///
    /// # Arguments
    ///
    /// * `message` - Message 2 from responder
    ///
    /// # Returns
    ///
    /// Ok(()) if message is valid.
    pub fn read_message_2(&mut self, message: &[u8]) -> Result<(), NoiseError>;

    /// Generates handshake message 3 (initiator only).
    ///
    /// # Returns
    ///
    /// Handshake message 3 (static key, ~64 bytes).
    pub fn write_message_3(&mut self) -> Result<Vec<u8>, NoiseError>;

    /// Processes handshake message 3 (responder only).
    ///
    /// # Arguments
    ///
    /// * `message` - Message 3 from initiator
    ///
    /// # Returns
    ///
    /// Ok(()) if message is valid.
    pub fn read_message_3(&mut self, message: &[u8]) -> Result<(), NoiseError>;

    /// Finalizes handshake and derives session keys.
    ///
    /// # Returns
    ///
    /// Symmetric encryption keys for the session.
    pub fn finalize(self) -> Result<SymmetricKeys, NoiseError>;
}
```

### Symmetric Encryption

#### `SymmetricKeys`

Session encryption keys.

```rust
#[derive(Zeroize, ZeroizeOnDrop)]
pub struct SymmetricKeys {
    // Private fields (zeroized on drop)
}

impl SymmetricKeys {
    /// Encrypts plaintext using XChaCha20-Poly1305 AEAD.
    ///
    /// # Arguments
    ///
    /// * `plaintext` - Data to encrypt
    ///
    /// # Returns
    ///
    /// Ciphertext with 16-byte authentication tag appended.
    pub fn encrypt(&mut self, plaintext: &[u8]) -> Vec<u8>;

    /// Decrypts ciphertext using XChaCha20-Poly1305 AEAD.
    ///
    /// # Arguments
    ///
    /// * `ciphertext` - Encrypted data with auth tag
    ///
    /// # Returns
    ///
    /// Plaintext if authentication succeeds, or CryptoError.
    ///
    /// # Errors
    ///
    /// - `CryptoError::AuthenticationFailed` if tag is invalid
    /// - `CryptoError::ReplayAttack` if nonce is reused
    pub fn decrypt(&mut self, ciphertext: &[u8]) -> Result<Vec<u8>, CryptoError>;
}
```

### Hashing

#### `Blake3Hash`

BLAKE3 hash function.

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Blake3Hash([u8; 32]);

impl Blake3Hash {
    /// Computes BLAKE3 hash of data.
    ///
    /// # Arguments
    ///
    /// * `data` - Data to hash
    ///
    /// # Returns
    ///
    /// 32-byte BLAKE3 hash.
    pub fn hash(data: &[u8]) -> Self;

    /// Computes keyed BLAKE3 hash (for HMAC-like uses).
    ///
    /// # Arguments
    ///
    /// * `key` - 32-byte key
    /// * `data` - Data to hash
    ///
    /// # Returns
    ///
    /// 32-byte keyed hash.
    pub fn keyed_hash(key: &[u8; 32], data: &[u8]) -> Self;

    /// Derives key material using BLAKE3 in KDF mode.
    ///
    /// # Arguments
    ///
    /// * `context` - KDF context string
    /// * `ikm` - Input key material
    /// * `length` - Output length in bytes
    ///
    /// # Returns
    ///
    /// Derived key material of specified length.
    pub fn derive_key(context: &str, ikm: &[u8], length: usize) -> Vec<u8>;

    /// Returns hash as byte array.
    pub fn as_bytes(&self) -> &[u8; 32];

    /// Returns hash as hex string.
    pub fn to_hex(&self) -> String;
}
```

### Key Generation

#### `Keypair`

Ed25519 keypair.

```rust
#[derive(Clone, Zeroize, ZeroizeOnDrop)]
pub struct Keypair {
    // Private fields (zeroized on drop)
}

impl Keypair {
    /// Generates a new random Ed25519 keypair.
    ///
    /// # Returns
    ///
    /// A new `Keypair` with cryptographically secure random keys.
    pub fn generate() -> Self;

    /// Creates a keypair from a seed.
    ///
    /// # Arguments
    ///
    /// * `seed` - 32-byte seed
    ///
    /// # Returns
    ///
    /// Deterministic `Keypair` derived from seed.
    pub fn from_seed(seed: &[u8; 32]) -> Self;

    /// Returns the public key.
    pub fn public(&self) -> &PublicKey;

    /// Signs a message.
    ///
    /// # Arguments
    ///
    /// * `message` - Message to sign
    ///
    /// # Returns
    ///
    /// 64-byte Ed25519 signature.
    pub fn sign(&self, message: &[u8]) -> Signature;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PublicKey([u8; 32]);

impl PublicKey {
    /// Verifies a signature.
    ///
    /// # Arguments
    ///
    /// * `message` - Original message
    /// * `signature` - 64-byte signature
    ///
    /// # Returns
    ///
    /// true if signature is valid, false otherwise.
    pub fn verify(&self, message: &[u8], signature: &Signature) -> bool;

    /// Converts public key to bytes.
    pub fn as_bytes(&self) -> &[u8; 32];

    /// Creates public key from bytes.
    pub fn from_bytes(bytes: [u8; 32]) -> Self;
}
```

### Elligator2

#### `Elligator2`

Elligator2 encoding for Curve25519 points.

```rust
pub struct Elligator2;

impl Elligator2 {
    /// Encodes a Curve25519 public key to a random-looking representative.
    ///
    /// # Arguments
    ///
    /// * `pubkey` - Curve25519 public key
    ///
    /// # Returns
    ///
    /// 32-byte representative indistinguishable from random,
    /// or None if point is not encodable (~50% of points).
    pub fn encode(pubkey: &[u8; 32]) -> Option<[u8; 32]>;

    /// Decodes an Elligator2 representative to a Curve25519 public key.
    ///
    /// # Arguments
    ///
    /// * `representative` - 32-byte Elligator2 representative
    ///
    /// # Returns
    ///
    /// Curve25519 public key.
    pub fn decode(representative: &[u8; 32]) -> [u8; 32];
}
```

---

## wraith-transport

### UDP Transport

#### `UdpTransport`

UDP-based transport layer.

```rust
pub struct UdpTransport {
    // Private fields
}

impl UdpTransport {
    /// Creates a new UDP transport.
    ///
    /// # Arguments
    ///
    /// * `bind_addr` - Local address to bind (e.g., "0.0.0.0:41641")
    ///
    /// # Returns
    ///
    /// New `UdpTransport` instance.
    ///
    /// # Errors
    ///
    /// - `TransportError::BindFailed` if socket bind fails
    pub async fn bind(bind_addr: SocketAddr) -> Result<Self, TransportError>;

    /// Sends a packet.
    ///
    /// # Arguments
    ///
    /// * `dest` - Destination address
    /// * `data` - Packet data
    ///
    /// # Returns
    ///
    /// Number of bytes sent.
    pub async fn send_to(
        &self,
        dest: SocketAddr,
        data: &[u8],
    ) -> Result<usize, TransportError>;

    /// Receives a packet.
    ///
    /// # Returns
    ///
    /// (data, source address) tuple.
    pub async fn recv_from(&self) -> Result<(Vec<u8>, SocketAddr), TransportError>;

    /// Returns the local socket address.
    pub fn local_addr(&self) -> SocketAddr;
}
```

### AF_XDP Transport

#### `XdpTransport`

High-performance AF_XDP transport (Linux-only).

```rust
#[cfg(target_os = "linux")]
pub struct XdpTransport {
    // Private fields
}

#[cfg(target_os = "linux")]
impl XdpTransport {
    /// Creates a new AF_XDP transport.
    ///
    /// # Arguments
    ///
    /// * `interface` - Network interface name (e.g., "eth0")
    /// * `queue_id` - RX queue ID (typically 0)
    ///
    /// # Returns
    ///
    /// New `XdpTransport` instance.
    ///
    /// # Errors
    ///
    /// - `TransportError::PermissionDenied` if missing CAP_NET_RAW
    /// - `TransportError::InterfaceNotFound` if interface doesn't exist
    /// - `TransportError::XdpNotSupported` if driver doesn't support XDP
    ///
    /// # Safety
    ///
    /// Requires CAP_NET_RAW, CAP_NET_ADMIN, CAP_BPF capabilities.
    pub fn new(interface: &str, queue_id: u32) -> Result<Self, TransportError>;

    /// Sends a packet (zero-copy if possible).
    ///
    /// # Arguments
    ///
    /// * `data` - Packet data
    ///
    /// # Returns
    ///
    /// Ok(()) if packet queued successfully.
    pub fn send(&mut self, data: &[u8]) -> Result<(), TransportError>;

    /// Receives a packet (zero-copy).
    ///
    /// # Returns
    ///
    /// Reference to received packet (valid until next recv call).
    pub fn recv(&mut self) -> Result<&[u8], TransportError>;
}
```

---

## wraith-discovery

### DHT

#### `DhtNode`

Kademlia DHT node.

```rust
pub struct DhtNode {
    // Private fields
}

impl DhtNode {
    /// Creates a new DHT node.
    ///
    /// # Arguments
    ///
    /// * `config` - DHT configuration
    ///
    /// # Returns
    ///
    /// New `DhtNode` instance.
    pub fn new(config: DhtConfig) -> Self;

    /// Bootstraps DHT by connecting to known nodes.
    ///
    /// # Arguments
    ///
    /// * `bootstrap_nodes` - List of known DHT node addresses
    ///
    /// # Returns
    ///
    /// Ok(()) when bootstrap succeeds.
    pub async fn bootstrap(&mut self, bootstrap_nodes: &[SocketAddr]) -> Result<()>;

    /// Stores a value in the DHT.
    ///
    /// # Arguments
    ///
    /// * `key` - 20-byte DHT key
    /// * `value` - Value to store (max 1024 bytes)
    ///
    /// # Returns
    ///
    /// Ok(()) when stored on k nodes.
    pub async fn put(&mut self, key: &[u8; 20], value: Vec<u8>) -> Result<()>;

    /// Retrieves a value from the DHT.
    ///
    /// # Arguments
    ///
    /// * `key` - 20-byte DHT key
    ///
    /// # Returns
    ///
    /// Value if found, or None.
    pub async fn get(&mut self, key: &[u8; 20]) -> Result<Option<Vec<u8>>>;

    /// Announces file availability to the DHT.
    ///
    /// # Arguments
    ///
    /// * `group_secret` - Group secret key
    /// * `file_hash` - BLAKE3 file hash
    /// * `endpoints` - Peer endpoints (IP:port)
    ///
    /// # Returns
    ///
    /// Ok(()) when announcement succeeds.
    pub async fn announce(
        &mut self,
        group_secret: &[u8; 32],
        file_hash: &Blake3Hash,
        endpoints: Vec<SocketAddr>,
    ) -> Result<()>;

    /// Searches for file peers in the DHT.
    ///
    /// # Arguments
    ///
    /// * `group_secret` - Group secret key
    /// * `file_hash` - BLAKE3 file hash
    ///
    /// # Returns
    ///
    /// List of peer endpoints offering the file.
    pub async fn find_peers(
        &mut self,
        group_secret: &[u8; 32],
        file_hash: &Blake3Hash,
    ) -> Result<Vec<SocketAddr>>;
}

#[derive(Debug, Clone)]
pub struct DhtConfig {
    /// Replication factor (default: 20)
    pub k: usize,
    /// Concurrent lookup requests (default: 3)
    pub alpha: usize,
    /// Node ID
    pub node_id: [u8; 20],
}
```

---

## wraith-files

### File Transfer

#### `FileTransfer`

High-level file transfer API.

```rust
pub struct FileTransfer {
    // Private fields
}

impl FileTransfer {
    /// Creates a new file transfer session.
    ///
    /// # Arguments
    ///
    /// * `session` - Established WRAITH session
    /// * `config` - Transfer configuration
    ///
    /// # Returns
    ///
    /// New `FileTransfer` instance.
    pub fn new(session: Session, config: TransferConfig) -> Self;

    /// Sends a file to the peer.
    ///
    /// # Arguments
    ///
    /// * `file_path` - Path to file to send
    /// * `progress` - Optional progress callback
    ///
    /// # Returns
    ///
    /// Ok(()) when transfer completes successfully.
    ///
    /// # Errors
    ///
    /// - `TransferError::FileNotFound` if file doesn't exist
    /// - `TransferError::PermissionDenied` if can't read file
    /// - `TransferError::ConnectionLost` if peer disconnects
    pub async fn send_file<P: AsRef<Path>, F>(
        &mut self,
        file_path: P,
        progress: Option<F>,
    ) -> Result<(), TransferError>
    where
        F: Fn(TransferProgress);

    /// Receives a file from the peer.
    ///
    /// # Arguments
    ///
    /// * `output_path` - Where to save received file
    /// * `progress` - Optional progress callback
    ///
    /// # Returns
    ///
    /// File metadata when transfer completes.
    ///
    /// # Errors
    ///
    /// - `TransferError::PermissionDenied` if can't write file
    /// - `TransferError::IntegrityCheckFailed` if hash doesn't match
    pub async fn recv_file<P: AsRef<Path>, F>(
        &mut self,
        output_path: P,
        progress: Option<F>,
    ) -> Result<FileMetadata, TransferError>
    where
        F: Fn(TransferProgress);
}

#[derive(Debug, Clone)]
pub struct TransferProgress {
    pub bytes_transferred: u64,
    pub total_bytes: u64,
    pub throughput: f64,  // bytes/sec
    pub eta: Option<Duration>,
}

#[derive(Debug, Clone)]
pub struct FileMetadata {
    pub file_hash: Blake3Hash,
    pub size: u64,
    pub chunk_count: u32,
}
```

---

## Error Handling

All errors implement `std::error::Error` and derive from `thiserror::Error`.

**Common error handling pattern:**
```rust
use wraith_core::{Session, SessionError};

async fn example() -> Result<(), Box<dyn std::error::Error>> {
    let session = Session::new(/* ... */);

    match session.send(b"hello").await {
        Ok(()) => println!("Sent successfully"),
        Err(SessionError::NetworkError(e)) => {
            eprintln!("Network error: {}", e);
            // Retry logic
        }
        Err(e) => return Err(e.into()),
    }

    Ok(())
}
```

---

## wraith-core: Transfer Session

### TransferSession

Manages file transfer state with multi-peer coordination.

```rust
pub struct TransferSession {
    /// Transfer ID (unique identifier)
    pub id: [u8; 32],
    /// Transfer direction
    pub direction: Direction,
    /// File path
    pub file_path: PathBuf,
    /// File size in bytes
    pub file_size: u64,
    /// Chunk size in bytes
    pub chunk_size: usize,
    /// Total number of chunks
    pub total_chunks: u64,
    // ... private fields
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransferState {
    /// Transfer initializing
    Initializing,
    /// Performing handshake
    Handshaking,
    /// Actively transferring
    Transferring,
    /// Transfer paused (can resume)
    Paused,
    /// Completing final verification
    Completing,
    /// Transfer complete
    Complete,
    /// Transfer failed
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Send,
    Receive,
}

impl TransferSession {
    /// Creates a new send transfer session.
    ///
    /// # Arguments
    ///
    /// * `id` - Unique 32-byte transfer identifier
    /// * `file_path` - Path to the file being sent
    /// * `file_size` - Total file size in bytes
    /// * `chunk_size` - Size of each chunk in bytes
    pub fn new_send(
        id: [u8; 32],
        file_path: PathBuf,
        file_size: u64,
        chunk_size: usize,
    ) -> Self;

    /// Creates a new receive transfer session.
    ///
    /// # Arguments
    ///
    /// * `id` - Unique 32-byte transfer identifier
    /// * `file_path` - Path where the received file will be saved
    /// * `file_size` - Expected total file size in bytes
    /// * `chunk_size` - Size of each chunk in bytes
    pub fn new_receive(
        id: [u8; 32],
        file_path: PathBuf,
        file_size: u64,
        chunk_size: usize,
    ) -> Self;

    /// Starts the transfer.
    pub fn start(&mut self);

    /// Pauses the transfer (can resume).
    pub fn pause(&mut self);

    /// Resumes a paused transfer.
    pub fn resume(&mut self);

    /// Marks a chunk as transferred.
    ///
    /// Updates both transferred and missing chunk sets for O(1) operations.
    pub fn mark_chunk_transferred(&mut self, chunk_index: u64, chunk_size: usize);

    /// Returns transfer progress (0.0 to 1.0).
    pub fn progress(&self) -> f64;

    /// Returns transfer speed in bytes/sec.
    pub fn speed(&self) -> Option<f64>;

    /// Returns ETA in seconds.
    pub fn eta(&self) -> Option<f64>;

    /// Returns missing chunk indices.
    ///
    /// # Performance
    ///
    /// O(m) where m is the number of missing chunks,
    /// NOT O(n) where n is total chunks.
    pub fn missing_chunks(&self) -> Vec<u64>;

    /// Returns missing chunk count.
    ///
    /// O(1) operation.
    pub fn missing_count(&self) -> u64;

    /// Checks if a specific chunk is missing.
    ///
    /// O(1) lookup.
    pub fn is_chunk_missing(&self, chunk_index: u64) -> bool;

    // Multi-peer coordination
    /// Adds a peer to the transfer.
    pub fn add_peer(&mut self, peer_id: [u8; 32]);

    /// Removes a peer, returning their assigned chunks.
    pub fn remove_peer(&mut self, peer_id: &[u8; 32]) -> Option<HashSet<u64>>;

    /// Assigns a chunk to a specific peer.
    pub fn assign_chunk_to_peer(&mut self, peer_id: &[u8; 32], chunk_index: u64) -> bool;

    /// Returns the next unassigned chunk to request.
    pub fn next_chunk_to_request(&self) -> Option<u64>;

    /// Returns aggregate download speed from all peers.
    pub fn aggregate_peer_speed(&self) -> f64;

    /// Returns current state.
    pub fn state(&self) -> TransferState;

    /// Checks if transfer is complete.
    pub fn is_complete(&self) -> bool;
}
```

**Example:**
```rust
use wraith_core::transfer::{TransferSession, Direction};
use std::path::PathBuf;

// Create receive session for 1 GB file
let mut session = TransferSession::new_receive(
    [1u8; 32],
    PathBuf::from("/tmp/received.dat"),
    1_000_000_000,  // 1 GB
    256 * 1024,     // 256 KB chunks
);

session.start();

// Add multiple peers for parallel download
session.add_peer([2u8; 32]);
session.add_peer([3u8; 32]);

// Assign chunks to peers
session.assign_chunk_to_peer(&[2u8; 32], 0);
session.assign_chunk_to_peer(&[3u8; 32], 1);

// Mark chunks as received
session.mark_chunk_transferred(0, 256 * 1024);
session.mark_chunk_transferred(1, 256 * 1024);

println!("Progress: {:.1}%", session.progress() * 100.0);
println!("Speed: {:.1} MB/s", session.speed().unwrap_or(0.0) / 1_000_000.0);
println!("ETA: {:.0}s", session.eta().unwrap_or(0.0));
```

---

## wraith-files: File Chunking

### FileChunker

Reads files in chunks with seek support for random access.

```rust
pub struct FileChunker {
    // Private fields
}

impl FileChunker {
    /// Creates a new chunker for a file.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the file to chunk
    /// * `chunk_size` - Size of each chunk in bytes
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be opened.
    pub fn new<P: AsRef<Path>>(path: P, chunk_size: usize) -> io::Result<Self>;

    /// Creates a chunker with default chunk size (256 KB).
    pub fn with_default_size<P: AsRef<Path>>(path: P) -> io::Result<Self>;

    /// Returns total number of chunks.
    pub fn num_chunks(&self) -> u64;

    /// Returns chunk size in bytes.
    pub fn chunk_size(&self) -> usize;

    /// Returns total file size.
    pub fn total_size(&self) -> u64;

    /// Reads next chunk sequentially.
    ///
    /// Returns None when all chunks have been read.
    pub fn read_chunk(&mut self) -> io::Result<Option<Vec<u8>>>;

    /// Seeks to a specific chunk index.
    pub fn seek_to_chunk(&mut self, chunk_index: u64) -> io::Result<()>;

    /// Reads a specific chunk by index.
    pub fn read_chunk_at(&mut self, chunk_index: u64) -> io::Result<Vec<u8>>;

    /// Returns chunk metadata including BLAKE3 hash.
    pub fn chunk_info(&mut self, chunk_index: u64) -> io::Result<ChunkInfo>;
}

#[derive(Debug, Clone)]
pub struct ChunkInfo {
    /// Chunk index
    pub index: u64,
    /// Byte offset in file
    pub offset: u64,
    /// Chunk size in bytes
    pub size: usize,
    /// BLAKE3 hash of chunk
    pub hash: [u8; 32],
}
```

### FileReassembler

Reassembles files from out-of-order chunks.

```rust
pub struct FileReassembler {
    // Private fields
}

impl FileReassembler {
    /// Creates a new reassembler.
    ///
    /// Pre-allocates the file to the expected size for faster writes.
    ///
    /// # Arguments
    ///
    /// * `path` - Path where the file will be written
    /// * `total_size` - Expected total file size in bytes
    /// * `chunk_size` - Size of each chunk in bytes
    ///
    /// # Performance
    ///
    /// Initialization is O(n) where n is total_chunks, but subsequent
    /// missing_chunks() queries are O(m) where m is missing chunks.
    pub fn new<P: AsRef<Path>>(
        path: P,
        total_size: u64,
        chunk_size: usize,
    ) -> io::Result<Self>;

    /// Writes a chunk at a specific index.
    ///
    /// Supports out-of-order chunk writes for parallel downloads.
    pub fn write_chunk(&mut self, chunk_index: u64, data: &[u8]) -> io::Result<()>;

    /// Checks if a chunk has been received.
    pub fn has_chunk(&self, chunk_index: u64) -> bool;

    /// Returns missing chunk indices.
    ///
    /// # Performance
    ///
    /// O(m) where m is missing chunks, not O(n) total chunks.
    pub fn missing_chunks(&self) -> Vec<u64>;

    /// Returns missing chunks in sorted order.
    pub fn missing_chunks_sorted(&self) -> Vec<u64>;

    /// Returns count of missing chunks (O(1)).
    pub fn missing_count(&self) -> u64;

    /// Checks if a specific chunk is missing (O(1)).
    pub fn is_chunk_missing(&self, chunk_index: u64) -> bool;

    /// Returns number of received chunks.
    pub fn received_count(&self) -> u64;

    /// Returns progress (0.0 to 1.0).
    pub fn progress(&self) -> f64;

    /// Checks if all chunks have been received.
    pub fn is_complete(&self) -> bool;

    /// Syncs file to disk.
    pub fn sync(&mut self) -> io::Result<()>;

    /// Finalizes the file (fails if incomplete).
    pub fn finalize(self) -> io::Result<()>;
}
```

**Example:**
```rust
use wraith_files::chunker::{FileChunker, FileReassembler};
use wraith_files::DEFAULT_CHUNK_SIZE;

// Chunking a file
let mut chunker = FileChunker::new("/path/to/file.dat", DEFAULT_CHUNK_SIZE)?;
println!("File has {} chunks", chunker.num_chunks());

// Read specific chunk
let chunk_data = chunker.read_chunk_at(5)?;

// Reassembly
let mut reassembler = FileReassembler::new(
    "/path/to/output.dat",
    chunker.total_size(),
    DEFAULT_CHUNK_SIZE,
)?;

// Write chunks (can be out-of-order)
reassembler.write_chunk(5, &chunk_data)?;
reassembler.write_chunk(0, &other_chunk)?;

// Check progress
println!("Progress: {:.1}%", reassembler.progress() * 100.0);
println!("Missing: {} chunks", reassembler.missing_count());

// Finalize when complete
if reassembler.is_complete() {
    reassembler.finalize()?;
}
```

---

## wraith-files: Tree Hashing

### FileTreeHash

BLAKE3 Merkle tree hash for file integrity verification.

```rust
#[derive(Debug, Clone)]
pub struct FileTreeHash {
    /// Merkle root hash
    pub root: [u8; 32],
    /// Chunk hashes (leaf nodes)
    pub chunks: Vec<[u8; 32]>,
}

impl FileTreeHash {
    /// Creates a new tree hash.
    pub fn new(root: [u8; 32], chunks: Vec<[u8; 32]>) -> Self;

    /// Returns number of chunks.
    pub fn chunk_count(&self) -> usize;

    /// Verifies a chunk against its expected hash.
    pub fn verify_chunk(&self, chunk_index: usize, chunk_data: &[u8]) -> bool;

    /// Returns chunk hash at index.
    pub fn get_chunk_hash(&self, chunk_index: usize) -> Option<&[u8; 32]>;
}

/// Computes tree hash for a file.
///
/// # Arguments
///
/// * `path` - Path to the file
/// * `chunk_size` - Size of each chunk in bytes
///
/// # Performance
///
/// Single-pass file read with streaming hash computation.
/// Throughput: >3 GB/s (memory), >1 GB/s (file I/O).
pub fn compute_tree_hash<P: AsRef<Path>>(
    path: P,
    chunk_size: usize,
) -> io::Result<FileTreeHash>;

/// Computes Merkle root from leaf hashes.
///
/// # Performance
///
/// Pre-allocates each tree level to minimize allocations.
pub fn compute_merkle_root(leaves: &[[u8; 32]]) -> [u8; 32];

/// Verifies a chunk against its expected hash in a tree.
pub fn verify_chunk(
    chunk_index: usize,
    chunk_data: &[u8],
    tree: &FileTreeHash,
) -> bool;

/// Computes tree hash from in-memory data.
pub fn compute_tree_hash_from_data(data: &[u8], chunk_size: usize) -> FileTreeHash;
```

### IncrementalTreeHasher

Streaming tree hasher for data received incrementally.

```rust
pub struct IncrementalTreeHasher {
    // Private fields
}

impl IncrementalTreeHasher {
    /// Creates a new incremental hasher.
    ///
    /// # Arguments
    ///
    /// * `chunk_size` - Size of each chunk in bytes
    pub fn new(chunk_size: usize) -> Self;

    /// Updates with new data.
    ///
    /// Data is buffered until a complete chunk is accumulated,
    /// at which point it's hashed and added to the chunk list.
    ///
    /// # Performance
    ///
    /// Uses slice-based hashing to avoid allocation in the hot path.
    pub fn update(&mut self, data: &[u8]);

    /// Returns number of complete chunks processed.
    pub fn chunk_count(&self) -> usize;

    /// Returns buffered byte count (not yet hashed).
    pub fn buffered_bytes(&self) -> usize;

    /// Finalizes and returns the tree hash.
    ///
    /// Hashes any remaining buffered data and computes Merkle root.
    pub fn finalize(self) -> FileTreeHash;
}
```

**Example:**
```rust
use wraith_files::tree_hash::{
    compute_tree_hash, verify_chunk, IncrementalTreeHasher,
};
use wraith_files::DEFAULT_CHUNK_SIZE;

// Compute tree hash for a file
let tree = compute_tree_hash("/path/to/file.dat", DEFAULT_CHUNK_SIZE)?;
println!("Root hash: {:?}", tree.root);
println!("Chunks: {}", tree.chunk_count());

// Verify a chunk
let chunk_data = vec![0u8; DEFAULT_CHUNK_SIZE];
if tree.verify_chunk(0, &chunk_data) {
    println!("Chunk 0 verified");
} else {
    println!("Chunk 0 FAILED verification");
}

// Incremental hashing (for streaming data)
let mut hasher = IncrementalTreeHasher::new(DEFAULT_CHUNK_SIZE);

// Feed data as it arrives
hasher.update(&received_data_part1);
hasher.update(&received_data_part2);
hasher.update(&received_data_part3);

// Finalize and get tree hash
let tree = hasher.finalize();
println!("Root hash: {:?}", tree.root);
```

---

## Feature Flags

**Available features:**
```toml
[features]
default = ["af-xdp", "io-uring"]

# Kernel bypass features (Linux-only)
af-xdp = ["libbpf-sys"]
io-uring = ["io-uring"]

# Network features
tls-relay = ["rustls", "tokio-rustls"]

# Experimental features
experimental = []
```

---

## See Also

- [Development Guide](development-guide.md)
- [Coding Standards](coding-standards.md)
- [Protocol Overview](../architecture/protocol-overview.md)
- [Security Model](../architecture/security-model.md)
