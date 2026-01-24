# WRAITH Protocol v2 API Reference

**Document Version:** 2.0.0  
**Status:** API Documentation  
**Generated From:** wraith crate v2.0.0  
**Date:** January 2026  

---

## Table of Contents

1. [Crate Overview](#1-crate-overview)
2. [Core Types](#2-core-types)
3. [Cryptographic Types](#3-cryptographic-types)
4. [Transport Types](#4-transport-types)
5. [Session and Stream API](#5-session-and-stream-api)
6. [Configuration Types](#6-configuration-types)
7. [Obfuscation Types](#7-obfuscation-types)
8. [Group Communication API](#8-group-communication-api)
9. [File Transfer API](#9-file-transfer-api)
10. [Error Types](#10-error-types)
11. [Traits](#11-traits)
12. [Feature Flags](#12-feature-flags)

---

## 1. Crate Overview

### 1.1 Module Structure

```
wraith
├── client          Client connection API
├── server          Server listener API
├── session         Session management
├── stream          Stream I/O
├── crypto          Cryptographic primitives
├── transport       Transport abstraction
├── obfuscation     Traffic obfuscation
├── config          Configuration types
├── group           Group communication (feature: "groups")
├── transfer        File transfer API
├── error           Error types
└── prelude         Common imports
```

### 1.2 Prelude

```rust
/// Common imports for WRAITH applications
pub mod prelude {
    pub use crate::{
        Client, Server, Session, Stream,
        Config, Result, Error,
        crypto::{IdentityKeypair, PublicKey},
    };
}
```

---

## 2. Core Types

### 2.1 Client

```rust
/// WRAITH client for initiating connections
/// 
/// The `Client` type provides methods to connect to WRAITH servers.
/// Each connection creates a new `Session` that can be used to
/// open streams and transfer data.
/// 
/// # Example
/// 
/// ```rust
/// use wraith::{Client, Config};
/// use wraith::crypto::{IdentityKeypair, PublicKey};
/// 
/// #[tokio::main]
/// async fn main() -> wraith::Result<()> {
///     let keypair = IdentityKeypair::generate()?;
///     let server_key = PublicKey::from_hex("...")?;
///     
///     let session = Client::connect(
///         "127.0.0.1:8443",
///         keypair,
///         server_key,
///         Config::default(),
///     ).await?;
///     
///     Ok(())
/// }
/// ```
pub struct Client {
    // Private fields
}

impl Client {
    /// Connect to a WRAITH server
    /// 
    /// # Arguments
    /// 
    /// * `addr` - Server address (IP:port or hostname:port)
    /// * `keypair` - Client's identity keypair for authentication
    /// * `server_key` - Expected server public key (for verification)
    /// * `config` - Connection configuration
    /// 
    /// # Returns
    /// 
    /// A new `Session` representing the established connection
    /// 
    /// # Errors
    /// 
    /// * `ErrorKind::ConnectionRefused` - Server unreachable
    /// * `ErrorKind::ConnectionTimeout` - Connection timed out
    /// * `ErrorKind::AuthenticationFailed` - Handshake failed
    /// * `ErrorKind::KeyMismatch` - Server key doesn't match expected
    pub async fn connect(
        addr: impl ToSocketAddrs,
        keypair: IdentityKeypair,
        server_key: PublicKey,
        config: Config,
    ) -> Result<Session>;
    
    /// Connect to a WRAITH server without verifying server key
    /// 
    /// # Security Warning
    /// 
    /// This method should only be used when the server's public key
    /// is not known in advance (e.g., first connection). The returned
    /// session provides the server's key which should be verified
    /// through an out-of-band channel and stored for future connections.
    /// 
    /// # Arguments
    /// 
    /// * `addr` - Server address
    /// * `keypair` - Client's identity keypair
    /// * `config` - Connection configuration
    /// 
    /// # Returns
    /// 
    /// A tuple of (Session, server's PublicKey)
    pub async fn connect_unverified(
        addr: impl ToSocketAddrs,
        keypair: IdentityKeypair,
        config: Config,
    ) -> Result<(Session, PublicKey)>;
    
    /// Resume a previous session using a resumption ticket
    /// 
    /// Session resumption reduces handshake latency from 1.5 RTT to 0.5 RTT
    /// by reusing cryptographic material from a previous session.
    /// 
    /// # Arguments
    /// 
    /// * `ticket` - Resumption ticket from previous session
    /// * `keypair` - Client's identity keypair (must match original)
    /// * `config` - Connection configuration
    /// 
    /// # Returns
    /// 
    /// A new `Session` using resumed cryptographic state
    /// 
    /// # Errors
    /// 
    /// * `ErrorKind::ResumptionFailed` - Server rejected resumption
    /// * `ErrorKind::TicketExpired` - Ticket has expired
    #[cfg(feature = "resumption")]
    pub async fn resume(
        ticket: ResumptionTicket,
        keypair: IdentityKeypair,
        config: Config,
    ) -> Result<Session>;
}
```

### 2.2 Server

```rust
/// WRAITH server for accepting connections
/// 
/// The `Server` type binds to a network address and accepts incoming
/// WRAITH connections. Each accepted connection becomes a `Session`.
/// 
/// # Example
/// 
/// ```rust
/// use wraith::Server;
/// use wraith::crypto::IdentityKeypair;
/// 
/// #[tokio::main]
/// async fn main() -> wraith::Result<()> {
///     let keypair = IdentityKeypair::generate()?;
///     let server = Server::bind("0.0.0.0:8443", keypair).await?;
///     
///     println!("Server public key: {}", server.public_key().to_hex());
///     
///     loop {
///         let session = server.accept().await?;
///         tokio::spawn(handle_session(session));
///     }
/// }
/// ```
pub struct Server {
    // Private fields
}

impl Server {
    /// Bind to an address and start listening
    /// 
    /// # Arguments
    /// 
    /// * `addr` - Address to bind (e.g., "0.0.0.0:8443")
    /// * `keypair` - Server's identity keypair
    /// 
    /// # Returns
    /// 
    /// A `Server` ready to accept connections
    /// 
    /// # Errors
    /// 
    /// * `ErrorKind::AddrInUse` - Address already in use
    /// * `ErrorKind::PermissionDenied` - Insufficient permissions
    pub async fn bind(
        addr: impl ToSocketAddrs,
        keypair: IdentityKeypair,
    ) -> Result<Self>;
    
    /// Bind with custom configuration
    /// 
    /// # Arguments
    /// 
    /// * `addr` - Address to bind
    /// * `keypair` - Server's identity keypair
    /// * `config` - Server configuration
    pub async fn bind_with_config(
        addr: impl ToSocketAddrs,
        keypair: IdentityKeypair,
        config: ServerConfig,
    ) -> Result<Self>;
    
    /// Accept a new connection
    /// 
    /// This method blocks until a new client connects and completes
    /// the handshake successfully.
    /// 
    /// # Returns
    /// 
    /// A `Session` representing the accepted connection
    /// 
    /// # Errors
    /// 
    /// * `ErrorKind::Shutdown` - Server has been shut down
    pub async fn accept(&self) -> Result<Session>;
    
    /// Get the server's public key
    /// 
    /// Clients need this key to verify the server's identity.
    pub fn public_key(&self) -> &PublicKey;
    
    /// Get the bound local address
    pub fn local_addr(&self) -> Result<SocketAddr>;
    
    /// Gracefully shut down the server
    /// 
    /// Stops accepting new connections. Existing sessions continue
    /// until they close naturally or timeout.
    pub async fn shutdown(&self);
    
    /// Get current connection count
    pub fn connection_count(&self) -> usize;
    
    /// Get server statistics
    pub fn stats(&self) -> ServerStats;
}

/// Server statistics
pub struct ServerStats {
    /// Total connections accepted
    pub connections_accepted: u64,
    
    /// Currently active connections
    pub connections_active: usize,
    
    /// Total bytes sent
    pub bytes_sent: u64,
    
    /// Total bytes received
    pub bytes_received: u64,
    
    /// Server uptime
    pub uptime: Duration,
}
```

### 2.3 Session

```rust
/// A secure session with a remote peer
/// 
/// `Session` represents an authenticated, encrypted connection to a peer.
/// Multiple streams can be multiplexed over a single session.
/// 
/// Sessions are created by `Client::connect()` or `Server::accept()`.
pub struct Session {
    // Private fields
}

impl Session {
    /// Open a new bidirectional stream
    /// 
    /// Creates a new stream for sending and receiving data.
    /// The stream is flow-controlled and multiplexed with other streams.
    /// 
    /// # Returns
    /// 
    /// A new `Stream` ready for I/O
    /// 
    /// # Errors
    /// 
    /// * `ErrorKind::SessionClosed` - Session has been closed
    /// * `ErrorKind::StreamLimitReached` - Maximum streams exceeded
    pub async fn open_stream(&self) -> Result<Stream>;
    
    /// Open a stream with custom configuration
    /// 
    /// # Arguments
    /// 
    /// * `config` - Stream configuration (QoS, priority, etc.)
    pub async fn open_stream_with_config(&self, config: StreamConfig) -> Result<Stream>;
    
    /// Accept an incoming stream from the peer
    /// 
    /// Blocks until the peer opens a new stream.
    /// 
    /// # Returns
    /// 
    /// The accepted `Stream`
    /// 
    /// # Errors
    /// 
    /// * `ErrorKind::SessionClosed` - Session has been closed
    pub async fn accept_stream(&self) -> Result<Stream>;
    
    /// Send an unreliable datagram
    /// 
    /// Datagrams are not guaranteed to arrive or arrive in order.
    /// Useful for real-time data where retransmission is undesirable.
    /// 
    /// # Arguments
    /// 
    /// * `data` - Datagram payload (max size: session MTU - overhead)
    /// 
    /// # Errors
    /// 
    /// * `ErrorKind::DatagramTooLarge` - Exceeds maximum datagram size
    #[cfg(feature = "realtime")]
    pub async fn send_datagram(&self, data: &[u8]) -> Result<()>;
    
    /// Receive an unreliable datagram
    /// 
    /// Blocks until a datagram arrives from the peer.
    #[cfg(feature = "realtime")]
    pub async fn recv_datagram(&self) -> Result<Vec<u8>>;
    
    /// Get the session ID
    pub fn id(&self) -> SessionId;
    
    /// Get the peer's public key
    pub fn peer_public_key(&self) -> &PublicKey;
    
    /// Get the local address
    pub fn local_addr(&self) -> SocketAddr;
    
    /// Get the remote address
    pub fn remote_addr(&self) -> SocketAddr;
    
    /// Get current session state
    pub fn state(&self) -> SessionState;
    
    /// Get current round-trip time estimate
    pub fn rtt(&self) -> Duration;
    
    /// Get session statistics
    pub fn stats(&self) -> SessionStats;
    
    /// Get number of active streams
    pub fn stream_count(&self) -> usize;
    
    /// Request a resumption ticket for later session resumption
    /// 
    /// # Returns
    /// 
    /// A `ResumptionTicket` that can be used with `Client::resume()`
    #[cfg(feature = "resumption")]
    pub async fn request_resumption_ticket(&self) -> Result<ResumptionTicket>;
    
    /// Subscribe to session events
    /// 
    /// Returns a channel receiver for session state changes,
    /// errors, and other events.
    pub fn subscribe_events(&self) -> mpsc::Receiver<SessionEvent>;
    
    /// Close the session gracefully
    /// 
    /// Sends a close frame to the peer and waits for acknowledgment.
    /// All streams are closed.
    pub async fn close(&self) -> Result<()>;
    
    /// Close the session immediately
    /// 
    /// Closes without waiting for peer acknowledgment.
    pub fn close_immediate(&self);
    
    /// Check if session is closed
    pub fn is_closed(&self) -> bool;
}

/// Session state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionState {
    /// Connection in progress
    Connecting,
    
    /// Handshake complete, ready for data
    Established,
    
    /// Key ratchet in progress
    Rekeying,
    
    /// Path migration in progress
    Migrating,
    
    /// Graceful shutdown initiated
    Draining,
    
    /// Session closed
    Closed,
}

/// Session events
#[derive(Debug, Clone)]
pub enum SessionEvent {
    /// State changed
    StateChanged(SessionState),
    
    /// Key ratchet completed
    Rekeyed { epoch: u64 },
    
    /// Path migrated
    PathMigrated { old: SocketAddr, new: SocketAddr },
    
    /// Error occurred
    Error(Error),
}

/// Session statistics
pub struct SessionStats {
    /// Bytes sent (application data)
    pub bytes_sent: u64,
    
    /// Bytes received (application data)
    pub bytes_received: u64,
    
    /// Packets sent
    pub packets_sent: u64,
    
    /// Packets received
    pub packets_received: u64,
    
    /// Packets lost
    pub packets_lost: u64,
    
    /// Current congestion window (bytes)
    pub cwnd: u64,
    
    /// Current RTT
    pub rtt: Duration,
    
    /// RTT variance
    pub rtt_var: Duration,
    
    /// Session duration
    pub duration: Duration,
}
```

### 2.4 Stream

```rust
/// A bidirectional stream within a session
/// 
/// `Stream` provides ordered, reliable (by default) byte stream delivery.
/// Implements `AsyncRead` and `AsyncWrite` for standard async I/O.
/// 
/// # Example
/// 
/// ```rust
/// use tokio::io::{AsyncReadExt, AsyncWriteExt};
/// 
/// async fn echo(mut stream: Stream) -> wraith::Result<()> {
///     let mut buf = [0u8; 1024];
///     loop {
///         let n = stream.read(&mut buf).await?;
///         if n == 0 { break; }
///         stream.write_all(&buf[..n]).await?;
///     }
///     Ok(())
/// }
/// ```
pub struct Stream {
    // Private fields
}

impl Stream {
    /// Get the stream ID
    pub fn id(&self) -> StreamId;
    
    /// Get stream priority
    pub fn priority(&self) -> StreamPriority;
    
    /// Set stream priority
    /// 
    /// Higher priority streams get bandwidth preference during congestion.
    pub fn set_priority(&mut self, priority: StreamPriority);
    
    /// Get stream QoS mode
    pub fn qos_mode(&self) -> QosMode;
    
    /// Check if stream is readable
    pub fn is_readable(&self) -> bool;
    
    /// Check if stream is writable
    pub fn is_writable(&self) -> bool;
    
    /// Check if stream is closed
    pub fn is_closed(&self) -> bool;
    
    /// Split stream into read and write halves
    /// 
    /// Allows concurrent reading and writing from different tasks.
    pub fn split(self) -> (ReadHalf, WriteHalf);
    
    /// Reunite split stream halves
    pub fn reunite(read: ReadHalf, write: WriteHalf) -> Result<Self>;
    
    /// Gracefully close the stream
    /// 
    /// Sends FIN to peer, waits for acknowledgment.
    pub async fn close(&mut self) -> Result<()>;
    
    /// Reset the stream
    /// 
    /// Abruptly closes with error code. Peer sees reset.
    pub fn reset(&mut self, error_code: u32);
    
    /// Get stream statistics
    pub fn stats(&self) -> StreamStats;
}

// Async I/O trait implementations
impl AsyncRead for Stream { /* ... */ }
impl AsyncWrite for Stream { /* ... */ }

/// Stream priority levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum StreamPriority {
    /// Lowest priority
    Low = 0,
    
    /// Normal priority (default)
    Normal = 1,
    
    /// High priority
    High = 2,
    
    /// Highest priority (control messages)
    Critical = 3,
}

/// Quality of Service modes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QosMode {
    /// Full reliability, ordered delivery (default)
    Reliable,
    
    /// Lost packets not retransmitted, order preserved
    UnreliableOrdered,
    
    /// Lost packets not retransmitted, may arrive out of order
    UnreliableUnordered,
    
    /// Retransmit until deadline, then drop
    PartiallyReliable { max_retransmit_time: Duration },
}

/// Stream configuration
pub struct StreamConfig {
    /// QoS mode
    pub qos: QosMode,
    
    /// Initial priority
    pub priority: StreamPriority,
    
    /// Maximum buffer size
    pub max_buffer: usize,
}

impl Default for StreamConfig {
    fn default() -> Self {
        Self {
            qos: QosMode::Reliable,
            priority: StreamPriority::Normal,
            max_buffer: 1024 * 1024,  // 1 MB
        }
    }
}
```

---

## 3. Cryptographic Types

### 3.1 Identity Keypair

```rust
/// Ed25519 identity keypair for authentication
/// 
/// Identity keypairs are long-term keys used to authenticate peers.
/// They should be generated once and stored securely.
/// 
/// # Security
/// 
/// - Private key is zeroized on drop
/// - Implements constant-time comparison
/// - Supports encrypted serialization
#[derive(Zeroize, ZeroizeOnDrop)]
pub struct IdentityKeypair {
    // Private fields
}

impl IdentityKeypair {
    /// Generate a new random keypair
    /// 
    /// Uses the system CSPRNG for key generation.
    /// 
    /// # Errors
    /// 
    /// * `ErrorKind::RngFailure` - Random number generation failed
    pub fn generate() -> Result<Self>;
    
    /// Create keypair from secret key bytes
    /// 
    /// # Arguments
    /// 
    /// * `secret` - 32-byte Ed25519 secret key
    /// 
    /// # Errors
    /// 
    /// * `ErrorKind::InvalidKey` - Invalid key bytes
    pub fn from_secret_bytes(secret: &[u8; 32]) -> Result<Self>;
    
    /// Get the public key
    pub fn public_key(&self) -> &PublicKey;
    
    /// Sign a message
    /// 
    /// # Arguments
    /// 
    /// * `message` - Message to sign
    /// 
    /// # Returns
    /// 
    /// 64-byte Ed25519 signature
    pub fn sign(&self, message: &[u8]) -> Signature;
    
    /// Export to encrypted PEM format
    /// 
    /// # Arguments
    /// 
    /// * `password` - Password for encryption (use strong password!)
    /// 
    /// # Returns
    /// 
    /// PEM-encoded encrypted private key
    pub fn to_encrypted_pem(&self, password: &str) -> Result<String>;
    
    /// Import from encrypted PEM format
    /// 
    /// # Arguments
    /// 
    /// * `pem` - PEM-encoded encrypted private key
    /// * `password` - Password for decryption
    pub fn from_encrypted_pem(pem: &str, password: &str) -> Result<Self>;
}

impl Clone for IdentityKeypair {
    /// Clone the keypair
    /// 
    /// # Security Note
    /// 
    /// Cloning copies the private key. Minimize copies of private keys.
    fn clone(&self) -> Self { /* ... */ }
}
```

### 3.2 Public Key

```rust
/// Ed25519 public key
/// 
/// Public keys are used to identify and verify peers.
/// They can be freely shared.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct PublicKey {
    // Private fields
}

impl PublicKey {
    /// Create from raw bytes
    /// 
    /// # Arguments
    /// 
    /// * `bytes` - 32-byte Ed25519 public key
    /// 
    /// # Errors
    /// 
    /// * `ErrorKind::InvalidKey` - Invalid public key bytes
    pub fn from_bytes(bytes: &[u8; 32]) -> Result<Self>;
    
    /// Create from hex string
    /// 
    /// # Arguments
    /// 
    /// * `hex` - 64-character hex string
    pub fn from_hex(hex: &str) -> Result<Self>;
    
    /// Get raw bytes
    pub fn as_bytes(&self) -> &[u8; 32];
    
    /// Convert to hex string
    pub fn to_hex(&self) -> String;
    
    /// Verify a signature
    /// 
    /// # Arguments
    /// 
    /// * `message` - Original message
    /// * `signature` - Signature to verify
    /// 
    /// # Returns
    /// 
    /// `true` if signature is valid, `false` otherwise
    pub fn verify(&self, message: &[u8], signature: &Signature) -> bool;
}

impl Debug for PublicKey {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        // Show abbreviated hex for debugging
        write!(f, "PublicKey({}...)", &self.to_hex()[..16])
    }
}

impl Display for PublicKey {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_hex())
    }
}
```

### 3.3 Signature

```rust
/// Ed25519 signature
#[derive(Clone, PartialEq, Eq)]
pub struct Signature([u8; 64]);

impl Signature {
    /// Create from bytes
    pub fn from_bytes(bytes: &[u8; 64]) -> Self;
    
    /// Get raw bytes
    pub fn as_bytes(&self) -> &[u8; 64];
    
    /// Convert to hex string
    pub fn to_hex(&self) -> String;
    
    /// Create from hex string
    pub fn from_hex(hex: &str) -> Result<Self>;
}
```

---

## 4. Transport Types

### 4.1 Transport Configuration

```rust
/// Transport layer configuration
pub struct TransportConfig {
    /// Transport type to use
    pub transport_type: TransportType,
    
    /// Socket send buffer size (bytes)
    pub send_buffer: usize,
    
    /// Socket receive buffer size (bytes)
    pub recv_buffer: usize,
    
    /// Enable io_uring (Linux only)
    #[cfg(target_os = "linux")]
    pub use_io_uring: bool,
    
    /// Connection timeout
    pub connect_timeout: Duration,
    
    /// Keep-alive interval
    pub keepalive_interval: Option<Duration>,
}

/// Available transport types
#[derive(Debug, Clone)]
pub enum TransportType {
    /// UDP transport (default)
    Udp(UdpConfig),
    
    /// TCP transport
    Tcp(TcpConfig),
    
    /// WebSocket transport
    WebSocket(WebSocketConfig),
    
    /// HTTP/2 transport
    Http2(Http2Config),
    
    /// QUIC transport
    Quic(QuicConfig),
    
    /// AF_XDP kernel bypass (Linux only)
    #[cfg(target_os = "linux")]
    AfXdp(AfXdpConfig),
    
    /// Automatic selection with fallback
    Auto(AutoConfig),
}

/// UDP transport configuration
#[derive(Debug, Clone)]
pub struct UdpConfig {
    /// Enable path MTU discovery
    pub pmtud: bool,
    
    /// Maximum transmission unit
    pub mtu: usize,
}

/// WebSocket transport configuration
#[derive(Debug, Clone)]
pub struct WebSocketConfig {
    /// WebSocket URL (wss:// for TLS)
    pub url: String,
    
    /// Additional HTTP headers
    pub headers: Vec<(String, String)>,
    
    /// WebSocket subprotocol
    pub subprotocol: Option<String>,
}

/// AF_XDP kernel bypass configuration (Linux only)
#[cfg(target_os = "linux")]
#[derive(Debug, Clone)]
pub struct AfXdpConfig {
    /// Network interface name
    pub interface: String,
    
    /// NIC queue ID
    pub queue_id: u32,
    
    /// Frame size
    pub frame_size: usize,
    
    /// Number of frames
    pub num_frames: usize,
    
    /// Enable zero-copy mode
    pub zero_copy: bool,
}
```

---

## 5. Session and Stream API

*(Covered in Core Types section above)*

---

## 6. Configuration Types

### 6.1 Main Configuration

```rust
/// Main WRAITH configuration
/// 
/// Use preset methods (`Config::balanced()`, etc.) for common
/// configurations, then customize as needed.
#[derive(Debug, Clone)]
pub struct Config {
    /// Cryptographic configuration
    pub crypto: CryptoConfig,
    
    /// Transport configuration
    pub transport: TransportConfig,
    
    /// Obfuscation configuration
    pub obfuscation: ObfuscationConfig,
    
    /// Session configuration
    pub session: SessionConfig,
    
    /// Resource limits
    pub resources: ResourceConfig,
    
    /// Debug options
    pub debug: DebugConfig,
}

impl Config {
    /// Maximum performance configuration
    /// 
    /// Use for: Trusted networks, data centers
    /// Features: Kernel bypass ready, minimal obfuscation, large buffers
    pub fn performance() -> Self;
    
    /// Balanced configuration (default)
    /// 
    /// Use for: General internet use
    /// Features: Good security, reasonable performance
    pub fn balanced() -> Self;
    
    /// Maximum stealth configuration
    /// 
    /// Use for: Censored networks, high-security environments
    /// Features: Full obfuscation, cover traffic, probing resistance
    pub fn stealth() -> Self;
    
    /// Resource-constrained configuration
    /// 
    /// Use for: Mobile devices, embedded systems
    /// Features: Small buffers, limited concurrency
    pub fn constrained() -> Self;
    
    /// Metered connection configuration
    /// 
    /// Use for: Limited bandwidth connections
    /// Features: Minimal overhead, no cover traffic
    pub fn metered() -> Self;
    
    // Builder methods
    
    /// Enable post-quantum cryptography
    pub fn with_post_quantum(self, enabled: bool) -> Self;
    
    /// Set transport type
    pub fn with_transport(self, transport: TransportType) -> Self;
    
    /// Set obfuscation configuration
    pub fn with_obfuscation(self, config: ObfuscationConfig) -> Self;
    
    /// Enable cover traffic
    pub fn with_cover_traffic(self, enabled: bool) -> Self;
    
    /// Enable session resumption
    pub fn with_resumption(self, enabled: bool) -> Self;
}

impl Default for Config {
    fn default() -> Self {
        Self::balanced()
    }
}
```

### 6.2 Cryptographic Configuration

```rust
/// Cryptographic configuration
#[derive(Debug, Clone)]
pub struct CryptoConfig {
    /// Cipher suite to use
    pub suite: CipherSuite,
    
    /// Enable post-quantum key exchange
    pub post_quantum: bool,
    
    /// Time interval for DH ratchet (seconds)
    pub ratchet_time_interval: Duration,
    
    /// Packet interval for DH ratchet
    pub ratchet_packet_interval: u64,
}

/// Available cipher suites
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CipherSuite {
    /// Default: XChaCha20-Poly1305 + BLAKE3 + X25519 + ML-KEM-768
    Default,
    
    /// Hardware accelerated: AES-256-GCM + SHA-256 + X25519 + ML-KEM-768
    HardwareAccelerated,
    
    /// Maximum security: XChaCha20-Poly1305 + BLAKE3 + X448 + ML-KEM-1024
    MaximumSecurity,
    
    /// Classical only (no post-quantum)
    ClassicalOnly,
}

impl Default for CryptoConfig {
    fn default() -> Self {
        Self {
            suite: CipherSuite::Default,
            post_quantum: true,
            ratchet_time_interval: Duration::from_secs(120),
            ratchet_packet_interval: 1_000_000,
        }
    }
}
```

### 6.3 Session Configuration

```rust
/// Session configuration
#[derive(Debug, Clone)]
pub struct SessionConfig {
    /// Initial flow control window (bytes)
    pub initial_window: u64,
    
    /// Maximum flow control window (bytes)
    pub max_window: u64,
    
    /// Maximum concurrent streams
    pub max_streams: u16,
    
    /// Chunk size for file transfers (bytes)
    pub chunk_size: usize,
    
    /// Idle timeout
    pub idle_timeout: Duration,
    
    /// Handshake timeout
    pub handshake_timeout: Duration,
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            initial_window: 1024 * 1024,      // 1 MB
            max_window: 16 * 1024 * 1024,     // 16 MB
            max_streams: 1000,
            chunk_size: 256 * 1024,           // 256 KB
            idle_timeout: Duration::from_secs(30),
            handshake_timeout: Duration::from_secs(10),
        }
    }
}
```

---

## 7. Obfuscation Types

### 7.1 Obfuscation Configuration

```rust
/// Traffic obfuscation configuration
#[derive(Debug, Clone)]
pub struct ObfuscationConfig {
    /// Padding configuration
    pub padding: PaddingConfig,
    
    /// Timing obfuscation
    pub timing: TimingConfig,
    
    /// Cover traffic configuration
    pub cover_traffic: CoverTrafficConfig,
    
    /// Enable probing resistance
    pub probing_resistance: bool,
    
    /// Entropy normalization mode
    pub entropy_normalization: EntropyMode,
}

/// Padding configuration
#[derive(Debug, Clone)]
pub struct PaddingConfig {
    /// Size distribution
    pub distribution: PaddingDistribution,
    
    /// Minimum packet size
    pub min_packet: usize,
    
    /// Maximum packet size
    pub max_packet: usize,
}

/// Padding size distributions
#[derive(Debug, Clone)]
pub enum PaddingDistribution {
    /// Uniform random
    Uniform,
    
    /// Matches empirical HTTPS traffic
    HttpsEmpirical,
    
    /// Gaussian (bell curve)
    Gaussian { mean: usize, std_dev: usize },
    
    /// Align to MTU
    MtuAligned,
    
    /// Custom distribution
    Custom(Vec<(usize, f64)>),  // (size, probability)
}

/// Timing obfuscation configuration
#[derive(Debug, Clone)]
pub struct TimingConfig {
    /// Timing mode
    pub mode: TimingMode,
}

/// Timing obfuscation modes
#[derive(Debug, Clone)]
pub enum TimingMode {
    /// No timing obfuscation
    None,
    
    /// Constant packet rate
    ConstantRate { interval: Duration },
    
    /// Random jitter added
    Jittered { max_jitter: Duration },
    
    /// Aggregate into bursts
    BurstShaped { burst_size: usize, burst_interval: Duration },
}

/// Cover traffic configuration
#[derive(Debug, Clone)]
pub struct CoverTrafficConfig {
    /// Enable cover traffic
    pub enabled: bool,
    
    /// Target rate (bytes/second)
    pub target_rate: u64,
    
    /// Minimum interval between cover packets
    pub min_interval: Duration,
    
    /// Maximum interval between cover packets
    pub max_interval: Duration,
    
    /// Size distribution for cover packets
    pub size_distribution: PaddingDistribution,
}
```

---

## 8. Group Communication API

```rust
/// Group communication module (feature: "groups")
#[cfg(feature = "groups")]
pub mod group {
    /// Multi-party group session
    /// 
    /// Enables secure communication among multiple parties
    /// using TreeKEM for efficient group key management.
    pub struct GroupSession {
        // Private fields
    }
    
    impl GroupSession {
        /// Create a new group
        /// 
        /// The creator becomes the group administrator.
        pub async fn create(
            keypair: IdentityKeypair,
            config: GroupConfig,
        ) -> Result<Self>;
        
        /// Join an existing group
        /// 
        /// # Arguments
        /// 
        /// * `keypair` - Your identity keypair
        /// * `invite_token` - Invitation from group admin
        pub async fn join(
            keypair: IdentityKeypair,
            invite_token: &str,
        ) -> Result<Self>;
        
        /// Get group ID
        pub fn id(&self) -> GroupId;
        
        /// Get current member count
        pub fn member_count(&self) -> usize;
        
        /// Get list of members
        pub fn members(&self) -> Vec<MemberInfo>;
        
        /// Generate invite token
        /// 
        /// Only group administrators can generate invites.
        pub fn invite_token(&self) -> Result<String>;
        
        /// Broadcast message to all members
        /// 
        /// Message is encrypted with the group key.
        pub async fn broadcast(&self, data: &[u8]) -> Result<()>;
        
        /// Send message to specific member
        /// 
        /// Message is encrypted specifically for this member.
        pub async fn send_to(&self, member: MemberId, data: &[u8]) -> Result<()>;
        
        /// Receive next message
        /// 
        /// Returns sender and message data.
        pub async fn recv(&self) -> Result<(MemberId, Vec<u8>)>;
        
        /// Subscribe to group events
        pub fn subscribe_events(&self) -> mpsc::Receiver<GroupEvent>;
        
        /// Remove a member (admin only)
        pub async fn remove_member(&self, member: MemberId) -> Result<()>;
        
        /// Leave the group
        pub async fn leave(&self) -> Result<()>;
    }
    
    /// Group configuration
    pub struct GroupConfig {
        /// Maximum number of members
        pub max_members: usize,
        
        /// Group topology
        pub topology: GroupTopology,
        
        /// Key rotation interval
        pub key_rotation_interval: Duration,
    }
    
    /// Group topology options
    pub enum GroupTopology {
        /// Every member connected to every other
        FullMesh,
        
        /// Tree structure (efficient for large groups)
        Tree,
        
        /// Gossip protocol (for very large groups)
        Gossip { fanout: usize },
    }
    
    /// Group events
    pub enum GroupEvent {
        /// Member joined
        MemberJoined(MemberInfo),
        
        /// Member left
        MemberLeft(MemberInfo),
        
        /// Group key rotated
        KeyRotated { epoch: u64 },
        
        /// Message received
        Message { from: MemberId, data: Vec<u8> },
    }
    
    /// Member information
    pub struct MemberInfo {
        /// Member ID
        pub id: MemberId,
        
        /// Member's public key
        pub public_key: PublicKey,
        
        /// Display name (if provided)
        pub display_name: Option<String>,
        
        /// Join time
        pub joined_at: SystemTime,
        
        /// Is administrator
        pub is_admin: bool,
    }
    
    /// Member identifier
    pub struct MemberId([u8; 16]);
}
```

---

## 9. File Transfer API

```rust
/// File transfer module
pub mod transfer {
    /// High-level file transfer API
    pub struct FileTransfer {
        // Private fields
    }
    
    impl FileTransfer {
        /// Send a file
        /// 
        /// # Arguments
        /// 
        /// * `session` - Session to send over
        /// * `path` - Path to file
        /// * `options` - Transfer options
        pub async fn send(
            session: &Session,
            path: impl AsRef<Path>,
            options: TransferOptions,
        ) -> Result<Self>;
        
        /// Receive a file
        /// 
        /// # Arguments
        /// 
        /// * `session` - Session to receive on
        /// * `save_path` - Where to save the file
        /// * `options` - Transfer options
        pub async fn receive(
            session: &Session,
            save_path: impl AsRef<Path>,
            options: TransferOptions,
        ) -> Result<Self>;
        
        /// Resume an interrupted transfer
        pub async fn resume(
            session: &Session,
            state: TransferState,
        ) -> Result<Self>;
        
        /// Get transfer progress
        pub fn progress(&self) -> TransferProgress;
        
        /// Subscribe to progress updates
        pub fn subscribe_progress(&self) -> mpsc::Receiver<TransferProgress>;
        
        /// Cancel the transfer
        pub fn cancel(&self);
        
        /// Wait for completion
        pub async fn await_completion(self) -> Result<()>;
    }
    
    // FileTransfer implements Future for awaiting
    impl Future for FileTransfer {
        type Output = Result<()>;
        
        fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output>;
    }
    
    /// Transfer options
    pub struct TransferOptions {
        /// Enable compression
        pub compression: bool,
        
        /// Enable resume capability
        pub resume: bool,
        
        /// Verify integrity after transfer
        pub verify: bool,
        
        /// Use content-addressed chunks
        pub content_addressed: bool,
    }
    
    impl Default for TransferOptions {
        fn default() -> Self {
            Self {
                compression: true,
                resume: true,
                verify: true,
                content_addressed: false,
            }
        }
    }
    
    /// Transfer progress information
    pub struct TransferProgress {
        /// Bytes transferred so far
        pub bytes_transferred: u64,
        
        /// Total file size
        pub total_bytes: u64,
        
        /// Current transfer speed (bytes/second)
        pub current_speed: u64,
        
        /// Estimated time remaining
        pub estimated_remaining: Duration,
        
        /// Transfer state
        pub state: TransferState,
    }
    
    /// Transfer state
    pub enum TransferState {
        /// Preparing transfer
        Preparing,
        
        /// Actively transferring
        Transferring,
        
        /// Verifying integrity
        Verifying,
        
        /// Transfer complete
        Complete,
        
        /// Transfer failed
        Failed(Error),
        
        /// Transfer cancelled
        Cancelled,
    }
}
```

---

## 10. Error Types

```rust
/// WRAITH error type
#[derive(Debug)]
pub struct Error {
    kind: ErrorKind,
    source: Option<Box<dyn std::error::Error + Send + Sync>>,
}

impl Error {
    /// Get error kind
    pub fn kind(&self) -> ErrorKind;
    
    /// Check if error is retryable
    pub fn is_retryable(&self) -> bool;
    
    /// Check if error indicates closed connection
    pub fn is_closed(&self) -> bool;
}

impl std::error::Error for Error { /* ... */ }
impl Display for Error { /* ... */ }

/// Error categories
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorKind {
    // Connection errors
    ConnectionRefused,
    ConnectionTimeout,
    ConnectionReset,
    ConnectionClosed,
    
    // Authentication errors
    AuthenticationFailed,
    KeyMismatch,
    
    // Protocol errors
    ProtocolViolation,
    DecryptionFailed,
    IntegrityError,
    
    // Stream errors
    StreamClosed,
    StreamReset,
    StreamLimitReached,
    
    // Resource errors
    ResourceExhausted,
    RateLimited,
    
    // I/O errors
    IoError,
    
    // Configuration errors
    InvalidConfig,
    InvalidKey,
    
    // Internal errors
    Internal,
    
    // Other
    Other,
}

/// Result type alias
pub type Result<T> = std::result::Result<T, Error>;
```

---

## 11. Traits

### 11.1 Transport Trait

```rust
/// Transport abstraction trait
/// 
/// Implement this trait to add custom transport support.
pub trait Transport: Send + Sync {
    /// Bind to an address
    async fn bind(&self, addr: SocketAddr) -> Result<Box<dyn TransportListener>>;
    
    /// Connect to an address
    async fn connect(&self, addr: SocketAddr) -> Result<Box<dyn TransportConnection>>;
    
    /// Get maximum transmission unit
    fn mtu(&self) -> usize;
    
    /// Check if transport supports datagrams
    fn supports_datagrams(&self) -> bool;
}

/// Transport listener trait
pub trait TransportListener: Send + Sync {
    /// Accept new connection
    async fn accept(&self) -> Result<(Box<dyn TransportConnection>, SocketAddr)>;
    
    /// Get local address
    fn local_addr(&self) -> Result<SocketAddr>;
}

/// Transport connection trait
pub trait TransportConnection: Send + Sync {
    /// Send data
    async fn send(&self, data: &[u8]) -> Result<()>;
    
    /// Receive data
    async fn recv(&self, buf: &mut [u8]) -> Result<usize>;
    
    /// Get local address
    fn local_addr(&self) -> Result<SocketAddr>;
    
    /// Get remote address
    fn remote_addr(&self) -> Result<SocketAddr>;
}
```

### 11.2 Extension Trait

```rust
/// Protocol extension trait
/// 
/// Implement this trait to add custom protocol extensions.
pub trait Extension: Send + Sync {
    /// Extension identifier (must be unique)
    fn extension_id(&self) -> u16;
    
    /// Extension name (for debugging)
    fn name(&self) -> &str;
    
    /// Negotiate extension parameters
    fn negotiate(&self, offered: &[u8]) -> Result<Vec<u8>>;
    
    /// Process incoming frame
    fn on_frame(&self, frame: &Frame) -> Result<ExtensionAction>;
    
    /// Process outgoing data
    fn on_send(&self, data: &mut [u8]) -> Result<()>;
    
    /// Process incoming data
    fn on_recv(&self, data: &mut [u8]) -> Result<()>;
}

/// Extension action after processing frame
pub enum ExtensionAction {
    /// Continue normal processing
    Continue,
    
    /// Drop the frame
    Drop,
    
    /// Replace with different data
    Replace(Vec<u8>),
}
```

---

## 12. Feature Flags

| Feature | Default | Description |
|---------|---------|-------------|
| `post-quantum` | Yes | Enable ML-KEM-768 hybrid cryptography |
| `groups` | Yes | Enable group communication support |
| `realtime` | Yes | Enable QoS modes and FEC |
| `resumption` | Yes | Enable session resumption |
| `compression` | Yes | Enable LZ4 compression |
| `kernel-bypass` | No | Enable AF_XDP (Linux only, requires root) |
| `wasm` | No | WebAssembly target support |
| `full` | - | Enable all features |

### Cargo.toml Example

```toml
# Minimal configuration
[dependencies]
wraith = { version = "2.0", default-features = false }

# All features
[dependencies]
wraith = { version = "2.0", features = ["full"] }

# Custom selection
[dependencies]
wraith = { version = "2.0", features = ["post-quantum", "groups"] }
```

---

## Document History

| Version | Date | Changes |
|---------|------|---------|
| 2.0.0 | 2026-01 | Initial API reference |

---

*End of WRAITH Protocol v2 API Reference*
