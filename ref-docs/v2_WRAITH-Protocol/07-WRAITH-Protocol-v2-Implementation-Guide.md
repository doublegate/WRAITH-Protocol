# WRAITH Protocol v2 Implementation Guide

**Document Version:** 2.0.0  
**Status:** Implementation Reference  
**Target Audience:** Developers and Integrators  
**Date:** January 2026  

---

## Table of Contents

1. [Introduction](#1-introduction)
2. [Getting Started](#2-getting-started)
3. [Core Concepts](#3-core-concepts)
4. [Basic Usage](#4-basic-usage)
5. [Session Management](#5-session-management)
6. [Stream Operations](#6-stream-operations)
7. [File Transfer](#7-file-transfer)
8. [Group Communication](#8-group-communication)
9. [Transport Configuration](#9-transport-configuration)
10. [Obfuscation Configuration](#10-obfuscation-configuration)
11. [Error Handling](#11-error-handling)
12. [Performance Tuning](#12-performance-tuning)
13. [Platform-Specific Notes](#13-platform-specific-notes)
14. [Testing and Debugging](#14-testing-and-debugging)
15. [Security Best Practices](#15-security-best-practices)

---

## 1. Introduction

### 1.1 Purpose

This guide provides practical implementation guidance for developers integrating WRAITH Protocol v2 into their applications. It covers common use cases, code patterns, configuration options, and best practices.

### 1.2 Prerequisites

- Rust 2021 edition (1.70+)
- Familiarity with async/await patterns (Tokio runtime)
- Basic understanding of cryptographic concepts
- Target platform: Linux (primary), Windows, macOS, or WASM

### 1.3 Crate Overview

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                         WRAITH v2 Crate Structure                           │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  wraith                        Main crate (re-exports all public API)      │
│  ├── wraith-core               Protocol core (sessions, streams, frames)   │
│  ├── wraith-crypto             Cryptographic primitives                    │
│  ├── wraith-transport          Transport abstraction layer                 │
│  ├── wraith-obfuscation        Traffic obfuscation                         │
│  ├── wraith-group              Group communication extension               │
│  └── wraith-cli                Command-line interface                      │
│                                                                             │
│  Feature flags:                                                            │
│  • post-quantum     Enable ML-KEM-768 hybrid crypto (default: on)          │
│  • groups           Enable group communication (default: on)               │
│  • realtime         Enable QoS/FEC extensions (default: on)                │
│  • kernel-bypass    Enable AF_XDP (Linux only, default: off)               │
│  • wasm             WebAssembly target support                             │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## 2. Getting Started

### 2.1 Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
wraith = "2.0"
tokio = { version = "1.32", features = ["full"] }

# Optional: For enhanced features
wraith = { version = "2.0", features = ["post-quantum", "groups", "realtime"] }
```

### 2.2 Key Generation

```rust
//! key_generation.rs
//! 
//! Generate and manage WRAITH identity keys.
//! 
//! Identity keys are long-term Ed25519 keypairs used for authentication.
//! They should be generated once and stored securely.

use wraith::crypto::{IdentityKeypair, SecretKey};
use wraith::Result;
use std::fs;
use std::path::Path;

/// Generate a new identity keypair
/// 
/// # Returns
/// A new randomly-generated Ed25519 keypair
/// 
/// # Example
/// ```rust
/// let keypair = generate_identity()?;
/// println!("Public key: {}", keypair.public_key().to_hex());
/// ```
pub fn generate_identity() -> Result<IdentityKeypair> {
    IdentityKeypair::generate()
}

/// Save keypair to file (encrypted with password)
/// 
/// # Arguments
/// * `keypair` - The keypair to save
/// * `path` - File path for the encrypted key
/// * `password` - Password for encryption (use strong password!)
/// 
/// # Security
/// - Key is encrypted with Argon2id + XChaCha20-Poly1305
/// - File permissions set to 0600 (owner read/write only)
pub fn save_identity(
    keypair: &IdentityKeypair,
    path: impl AsRef<Path>,
    password: &str,
) -> Result<()> {
    let encrypted = keypair.to_encrypted_pem(password)?;
    fs::write(&path, encrypted)?;
    
    // Set restrictive permissions (Unix only)
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600))?;
    }
    
    Ok(())
}

/// Load keypair from encrypted file
/// 
/// # Arguments
/// * `path` - Path to encrypted key file
/// * `password` - Password for decryption
/// 
/// # Errors
/// Returns error if file not found, wrong password, or corrupted file
pub fn load_identity(
    path: impl AsRef<Path>,
    password: &str,
) -> Result<IdentityKeypair> {
    let encrypted = fs::read_to_string(path)?;
    IdentityKeypair::from_encrypted_pem(&encrypted, password)
}

/// Example: First-time setup
fn main() -> Result<()> {
    let key_path = "identity.key";
    
    // Check if key exists
    if Path::new(key_path).exists() {
        println!("Loading existing identity...");
        let keypair = load_identity(key_path, "my-secure-password")?;
        println!("Public key: {}", keypair.public_key().to_hex());
    } else {
        println!("Generating new identity...");
        let keypair = generate_identity()?;
        save_identity(&keypair, key_path, "my-secure-password")?;
        println!("New public key: {}", keypair.public_key().to_hex());
        println!("Share this public key with peers to establish connections.");
    }
    
    Ok(())
}
```

### 2.3 Minimal Example

```rust
//! minimal_example.rs
//! 
//! Minimal WRAITH v2 client-server example demonstrating basic connectivity.

use wraith::{Client, Server, Config, Result};
use tokio;

#[tokio::main]
async fn main() -> Result<()> {
    // Server side
    let server_keypair = wraith::crypto::IdentityKeypair::generate()?;
    let server_public = server_keypair.public_key().clone();
    
    // Start server
    let server = Server::bind("127.0.0.1:8443", server_keypair).await?;
    println!("Server listening on 127.0.0.1:8443");
    
    // Spawn server handler
    tokio::spawn(async move {
        while let Ok(session) = server.accept().await {
            println!("Client connected!");
            
            // Echo received data back
            let mut stream = session.accept_stream().await.unwrap();
            let mut buf = vec![0u8; 4096];
            while let Ok(n) = stream.read(&mut buf).await {
                if n == 0 { break; }
                stream.write_all(&buf[..n]).await.unwrap();
            }
        }
    });
    
    // Client side
    let client_keypair = wraith::crypto::IdentityKeypair::generate()?;
    
    // Connect to server
    let config = Config::default();
    let session = Client::connect(
        "127.0.0.1:8443",
        client_keypair,
        server_public,  // Server's public key (obtained out-of-band)
        config,
    ).await?;
    
    println!("Connected to server!");
    
    // Send and receive data
    let mut stream = session.open_stream().await?;
    stream.write_all(b"Hello, WRAITH!").await?;
    
    let mut response = vec![0u8; 1024];
    let n = stream.read(&mut response).await?;
    println!("Received: {}", String::from_utf8_lossy(&response[..n]));
    
    Ok(())
}
```

---

## 3. Core Concepts

### 3.1 Architecture Overview

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                        WRAITH v2 Object Hierarchy                           │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  Application                                                               │
│      │                                                                      │
│      ▼                                                                      │
│  ┌───────────────────────────────────────────────────────────────────────┐ │
│  │ Server / Client                                                       │ │
│  │                                                                        │ │
│  │ • Listens for / initiates connections                                 │ │
│  │ • Manages transport binding                                           │ │
│  │ • Holds identity keypair                                              │ │
│  └───────────────────────────────┬───────────────────────────────────────┘ │
│                                  │                                         │
│                                  │ accept() / connect()                    │
│                                  ▼                                         │
│  ┌───────────────────────────────────────────────────────────────────────┐ │
│  │ Session                                                               │ │
│  │                                                                        │ │
│  │ • One per peer connection                                             │ │
│  │ • Handles crypto handshake and ratcheting                             │ │
│  │ • Manages multiple streams                                            │ │
│  │ • Provides connection-level operations                                │ │
│  └───────────────────────────────┬───────────────────────────────────────┘ │
│                                  │                                         │
│                                  │ open_stream() / accept_stream()         │
│                                  ▼                                         │
│  ┌───────────────────────────────────────────────────────────────────────┐ │
│  │ Stream                                                                │ │
│  │                                                                        │ │
│  │ • Bidirectional byte stream                                           │ │
│  │ • Flow controlled                                                     │ │
│  │ • Multiplexed over session                                            │ │
│  │ • AsyncRead + AsyncWrite                                              │ │
│  └───────────────────────────────────────────────────────────────────────┘ │
│                                                                             │
│  Additional Components:                                                    │
│                                                                             │
│  • GroupSession: Multi-party communication (see Section 8)                 │
│  • FileTransfer: High-level file transfer API (see Section 7)              │
│  • Datagram: Unreliable message delivery (for real-time)                   │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

### 3.2 Configuration Hierarchy

```rust
//! Configuration structure and inheritance

use wraith::config::*;

/// Top-level configuration
pub struct Config {
    /// Cryptographic settings
    pub crypto: CryptoConfig,
    
    /// Transport settings
    pub transport: TransportConfig,
    
    /// Obfuscation settings
    pub obfuscation: ObfuscationConfig,
    
    /// Session management
    pub session: SessionConfig,
    
    /// Resource limits
    pub resources: ResourceConfig,
}

/// Use preset profiles for common scenarios
impl Config {
    /// Maximum performance, minimal obfuscation
    /// Use: Trusted networks, data centers
    pub fn performance() -> Self { /* ... */ }
    
    /// Balanced security and performance
    /// Use: General internet use
    pub fn balanced() -> Self { /* ... */ }
    
    /// Maximum obfuscation, moderate performance
    /// Use: Censored networks, high-security environments
    pub fn stealth() -> Self { /* ... */ }
    
    /// Minimal resource usage
    /// Use: Mobile devices, embedded systems
    pub fn constrained() -> Self { /* ... */ }
    
    /// Bandwidth-aware mode
    /// Use: Metered connections
    pub fn metered() -> Self { /* ... */ }
}

// Example: Start with preset, customize as needed
let config = Config::stealth()
    .with_transport(TransportType::WebSocket)
    .with_cover_traffic(false);  // Disable for metered connection
```

---

## 4. Basic Usage

### 4.1 Creating a Server

```rust
//! server_example.rs
//! 
//! Complete server implementation with connection handling.

use wraith::{Server, Session, Config, Result};
use wraith::crypto::IdentityKeypair;
use std::sync::Arc;
use tokio::sync::Semaphore;

/// Server configuration
struct ServerConfig {
    /// Listen address
    bind_addr: String,
    
    /// Maximum concurrent connections
    max_connections: usize,
    
    /// Identity keypair
    keypair: IdentityKeypair,
}

/// Run the WRAITH server
async fn run_server(config: ServerConfig) -> Result<()> {
    // Create server with identity
    let wraith_config = Config::balanced();
    let server = Server::bind(&config.bind_addr, config.keypair)
        .with_config(wraith_config)
        .await?;
    
    println!("WRAITH server listening on {}", config.bind_addr);
    println!("Server public key: {}", server.public_key().to_hex());
    
    // Connection limiter
    let semaphore = Arc::new(Semaphore::new(config.max_connections));
    
    // Accept loop
    loop {
        // Wait for connection slot
        let permit = semaphore.clone().acquire_owned().await?;
        
        // Accept new session
        let session = server.accept().await?;
        let peer = session.peer_public_key().to_hex();
        println!("New connection from: {}", peer);
        
        // Spawn handler for this session
        tokio::spawn(async move {
            if let Err(e) = handle_session(session).await {
                eprintln!("Session error: {}", e);
            }
            drop(permit);  // Release connection slot
        });
    }
}

/// Handle a single client session
async fn handle_session(session: Session) -> Result<()> {
    // Accept streams from this client
    loop {
        match session.accept_stream().await {
            Ok(stream) => {
                // Spawn stream handler
                tokio::spawn(async move {
                    if let Err(e) = handle_stream(stream).await {
                        eprintln!("Stream error: {}", e);
                    }
                });
            }
            Err(e) if e.is_closed() => {
                println!("Client disconnected");
                break;
            }
            Err(e) => return Err(e),
        }
    }
    Ok(())
}

/// Handle a single stream (implement your application protocol here)
async fn handle_stream(mut stream: wraith::Stream) -> Result<()> {
    // Example: Simple echo protocol
    let mut buf = vec![0u8; 65536];
    
    loop {
        let n = stream.read(&mut buf).await?;
        if n == 0 {
            break;  // Stream closed
        }
        
        // Echo back
        stream.write_all(&buf[..n]).await?;
    }
    
    Ok(())
}
```

### 4.2 Creating a Client

```rust
//! client_example.rs
//! 
//! Complete client implementation with reconnection logic.

use wraith::{Client, Session, Config, Result};
use wraith::crypto::{IdentityKeypair, PublicKey};
use std::time::Duration;
use tokio::time::sleep;

/// Client configuration
struct ClientConfig {
    /// Server address
    server_addr: String,
    
    /// Server's public key (obtained out-of-band)
    server_public_key: PublicKey,
    
    /// Client identity
    keypair: IdentityKeypair,
    
    /// Reconnection attempts
    max_retries: u32,
}

/// Connect to server with retry logic
async fn connect_with_retry(config: &ClientConfig) -> Result<Session> {
    let wraith_config = Config::balanced()
        .with_post_quantum(true);  // Enable PQ crypto
    
    let mut attempt = 0;
    loop {
        attempt += 1;
        println!("Connection attempt {} to {}", attempt, config.server_addr);
        
        match Client::connect(
            &config.server_addr,
            config.keypair.clone(),
            config.server_public_key.clone(),
            wraith_config.clone(),
        ).await {
            Ok(session) => {
                println!("Connected successfully!");
                return Ok(session);
            }
            Err(e) if attempt < config.max_retries => {
                eprintln!("Connection failed: {}. Retrying...", e);
                // Exponential backoff
                let delay = Duration::from_secs(2u64.pow(attempt.min(5)));
                sleep(delay).await;
            }
            Err(e) => return Err(e),
        }
    }
}

/// Example client application
async fn run_client(config: ClientConfig) -> Result<()> {
    // Connect to server
    let session = connect_with_retry(&config).await?;
    
    // Open a stream for application data
    let mut stream = session.open_stream().await?;
    
    // Send request
    let request = b"GET /data HTTP/1.1\r\n\r\n";
    stream.write_all(request).await?;
    
    // Read response
    let mut response = Vec::new();
    stream.read_to_end(&mut response).await?;
    
    println!("Response: {} bytes", response.len());
    
    // Graceful shutdown
    session.close().await?;
    
    Ok(())
}
```

---

## 5. Session Management

### 5.1 Session Lifecycle

```rust
//! session_lifecycle.rs
//! 
//! Understanding and managing session states.

use wraith::{Session, SessionState, SessionEvent};

/// Monitor session state changes
async fn monitor_session(session: &Session) {
    let mut events = session.subscribe_events();
    
    while let Some(event) = events.recv().await {
        match event {
            SessionEvent::StateChanged(new_state) => {
                println!("Session state: {:?}", new_state);
                match new_state {
                    SessionState::Established => {
                        println!("  ✓ Handshake complete, ready for data");
                    }
                    SessionState::Rekeying => {
                        println!("  ↻ Key ratchet in progress");
                    }
                    SessionState::Migrating => {
                        println!("  → Path migration in progress");
                    }
                    SessionState::Draining => {
                        println!("  ↓ Graceful shutdown initiated");
                    }
                    SessionState::Closed => {
                        println!("  × Session closed");
                        break;
                    }
                }
            }
            SessionEvent::Rekeyed { epoch } => {
                println!("Key ratchet complete, epoch: {}", epoch);
            }
            SessionEvent::PathMigrated { old, new } => {
                println!("Migrated from {} to {}", old, new);
            }
            SessionEvent::Error(e) => {
                eprintln!("Session error: {}", e);
            }
        }
    }
}

/// Session information queries
fn print_session_info(session: &Session) {
    println!("Session Information:");
    println!("  Session ID: {}", session.id());
    println!("  Peer: {}", session.peer_public_key().to_hex());
    println!("  Local addr: {}", session.local_addr());
    println!("  Remote addr: {}", session.remote_addr());
    println!("  State: {:?}", session.state());
    println!("  RTT: {:?}", session.rtt());
    println!("  Bytes sent: {}", session.stats().bytes_sent);
    println!("  Bytes recv: {}", session.stats().bytes_received);
    println!("  Active streams: {}", session.stream_count());
}
```

### 5.2 Session Resumption

```rust
//! session_resumption.rs
//! 
//! Save and resume sessions for faster reconnection.

use wraith::{Session, ResumptionTicket, Config, Client};

/// Enable resumption in config
fn config_with_resumption() -> Config {
    Config::balanced()
        .with_resumption(true)
        .with_resumption_lifetime(Duration::from_secs(3600))  // 1 hour
}

/// Save ticket after successful connection
async fn save_resumption_ticket(session: &Session) -> Option<ResumptionTicket> {
    // Request ticket from server
    match session.request_resumption_ticket().await {
        Ok(ticket) => {
            println!("Resumption ticket obtained, valid until: {:?}", 
                     ticket.expires());
            Some(ticket)
        }
        Err(e) => {
            eprintln!("Failed to get resumption ticket: {}", e);
            None
        }
    }
}

/// Resume session using saved ticket
async fn resume_session(
    ticket: ResumptionTicket,
    keypair: IdentityKeypair,
) -> Result<Session> {
    let config = config_with_resumption();
    
    // Try resumption first (0.5 RTT)
    match Client::resume(ticket.clone(), keypair.clone(), config.clone()).await {
        Ok(session) => {
            println!("Session resumed successfully (0.5 RTT handshake)");
            Ok(session)
        }
        Err(e) => {
            println!("Resumption failed: {}. Falling back to full handshake.", e);
            
            // Fall back to full connection (1.5 RTT)
            Client::connect(
                &ticket.server_addr(),
                keypair,
                ticket.server_public_key(),
                config,
            ).await
        }
    }
}
```

---

## 6. Stream Operations

### 6.1 Stream Types

```rust
//! stream_types.rs
//! 
//! Different stream configurations for various use cases.

use wraith::{Session, Stream, StreamConfig, QosMode};

/// Open stream with specific QoS mode
async fn open_typed_stream(
    session: &Session,
    stream_type: StreamType,
) -> Result<Stream> {
    let config = match stream_type {
        StreamType::Reliable => {
            // Full reliability, ordered delivery
            // Use for: File transfer, messaging, RPC
            StreamConfig::default()
        }
        
        StreamType::UnreliableOrdered => {
            // Lost packets not retransmitted, but ordering preserved
            // Use for: Live video, game state updates
            StreamConfig::default()
                .with_qos(QosMode::UnreliableOrdered)
        }
        
        StreamType::UnreliableUnordered => {
            // Lowest latency, no guarantees
            // Use for: VoIP, real-time sensor data
            StreamConfig::default()
                .with_qos(QosMode::UnreliableUnordered)
        }
        
        StreamType::PartiallyReliable => {
            // Retransmit up to deadline, then drop
            // Use for: Interactive video, gaming
            StreamConfig::default()
                .with_qos(QosMode::PartiallyReliable {
                    max_retransmit_time: Duration::from_millis(100),
                })
        }
    };
    
    session.open_stream_with_config(config).await
}

/// Stream priority configuration
async fn open_prioritized_streams(session: &Session) -> Result<()> {
    // High priority for control messages
    let control = session.open_stream_with_config(
        StreamConfig::default().with_priority(StreamPriority::High)
    ).await?;
    
    // Normal priority for data
    let data = session.open_stream_with_config(
        StreamConfig::default().with_priority(StreamPriority::Normal)
    ).await?;
    
    // Low priority for bulk transfer
    let bulk = session.open_stream_with_config(
        StreamConfig::default().with_priority(StreamPriority::Low)
    ).await?;
    
    // During congestion, high priority streams get bandwidth first
    Ok(())
}
```

### 6.2 Stream I/O Patterns

```rust
//! stream_io.rs
//! 
//! Common I/O patterns for WRAITH streams.

use wraith::Stream;
use tokio::io::{AsyncReadExt, AsyncWriteExt, BufReader, BufWriter};

/// Buffered I/O for small messages
async fn buffered_io(stream: Stream) -> Result<()> {
    let (read_half, write_half) = stream.split();
    
    let mut reader = BufReader::new(read_half);
    let mut writer = BufWriter::new(write_half);
    
    // Read line-delimited messages
    let mut line = String::new();
    reader.read_line(&mut line).await?;
    
    // Write with buffering
    writer.write_all(b"Response\n").await?;
    writer.flush().await?;  // Don't forget to flush!
    
    Ok(())
}

/// Zero-copy I/O for large data
async fn zerocopy_transfer(
    source: &mut impl AsyncRead,
    dest: &mut Stream,
) -> Result<u64> {
    // Use tokio's copy which optimizes for zero-copy when possible
    tokio::io::copy(source, dest).await
}

/// Bidirectional copy (proxy pattern)
async fn bidirectional_copy(
    stream1: Stream,
    stream2: Stream,
) -> Result<()> {
    let (r1, w1) = stream1.split();
    let (r2, w2) = stream2.split();
    
    // Copy in both directions concurrently
    let (result1, result2) = tokio::join!(
        tokio::io::copy(r1, w2),
        tokio::io::copy(r2, w1),
    );
    
    result1?;
    result2?;
    Ok(())
}

/// Chunked reading for message protocols
async fn read_message(stream: &mut Stream) -> Result<Vec<u8>> {
    // Read 4-byte length prefix
    let mut len_buf = [0u8; 4];
    stream.read_exact(&mut len_buf).await?;
    let len = u32::from_be_bytes(len_buf) as usize;
    
    // Validate length
    if len > MAX_MESSAGE_SIZE {
        return Err(Error::MessageTooLarge);
    }
    
    // Read message body
    let mut message = vec![0u8; len];
    stream.read_exact(&mut message).await?;
    
    Ok(message)
}

/// Write with length prefix
async fn write_message(stream: &mut Stream, data: &[u8]) -> Result<()> {
    if data.len() > MAX_MESSAGE_SIZE {
        return Err(Error::MessageTooLarge);
    }
    
    // Write length prefix
    let len = (data.len() as u32).to_be_bytes();
    stream.write_all(&len).await?;
    
    // Write message body
    stream.write_all(data).await?;
    
    Ok(())
}
```

---

## 7. File Transfer

### 7.1 High-Level File Transfer API

```rust
//! file_transfer.rs
//! 
//! High-level file transfer API with resume support.

use wraith::transfer::{FileTransfer, TransferOptions, TransferProgress};
use std::path::Path;

/// Send a file with progress reporting
async fn send_file_with_progress(
    session: &Session,
    path: impl AsRef<Path>,
) -> Result<()> {
    let options = TransferOptions::default()
        .with_compression(true)     // Enable LZ4 compression
        .with_resume(true)          // Enable resume on interruption
        .with_verify(true);         // Verify integrity after transfer
    
    // Create transfer
    let transfer = FileTransfer::send(session, path, options).await?;
    
    // Monitor progress
    let mut progress_rx = transfer.subscribe_progress();
    let transfer_handle = tokio::spawn(async move {
        transfer.await
    });
    
    while let Some(progress) = progress_rx.recv().await {
        print_progress(&progress);
    }
    
    // Wait for completion
    transfer_handle.await??;
    println!("\nTransfer complete!");
    
    Ok(())
}

fn print_progress(progress: &TransferProgress) {
    let percent = (progress.bytes_transferred as f64 / progress.total_bytes as f64) * 100.0;
    let speed_mbps = progress.current_speed as f64 / 1_000_000.0;
    
    print!("\r{:.1}% ({:.2} MB/s) ETA: {:?}    ",
           percent, speed_mbps, progress.estimated_remaining);
}

/// Receive a file
async fn receive_file(
    session: &Session,
    save_path: impl AsRef<Path>,
) -> Result<()> {
    let options = TransferOptions::default()
        .with_resume(true);
    
    // Accept incoming file transfer
    let transfer = session.accept_file_transfer().await?;
    
    println!("Receiving: {} ({} bytes)", 
             transfer.filename(), 
             transfer.size());
    
    // Save to specified path
    transfer.save_to(save_path, options).await?;
    
    println!("File saved successfully!");
    
    Ok(())
}

/// Resume interrupted transfer
async fn resume_transfer(
    session: &Session,
    partial_path: impl AsRef<Path>,
) -> Result<()> {
    // Load partial transfer state
    let state = TransferState::load(&partial_path)?;
    
    // Resume from where we left off
    let transfer = FileTransfer::resume(session, state).await?;
    
    println!("Resuming from byte {}", transfer.bytes_transferred());
    
    transfer.await?;
    
    Ok(())
}
```

### 7.2 Chunked Transfer for Large Files

```rust
//! chunked_transfer.rs
//! 
//! Content-addressed chunked transfer for deduplication and verification.

use wraith::transfer::{ChunkedTransfer, Chunk, MerkleTree};
use blake3::Hash;

/// Send file with content-addressed chunks
/// 
/// Benefits:
/// - Automatic deduplication of repeated content
/// - Per-chunk verification
/// - Efficient resume (only retransmit failed chunks)
async fn send_chunked(
    session: &Session,
    path: impl AsRef<Path>,
) -> Result<()> {
    // Build Merkle tree from file
    let tree = MerkleTree::from_file(&path).await?;
    println!("File hash: {}", tree.root().to_hex());
    println!("Chunks: {}", tree.chunk_count());
    
    // Send chunk manifest first
    let transfer = ChunkedTransfer::new(session, tree).await?;
    
    // Server may already have some chunks (deduplication)
    let needed = transfer.needed_chunks().await?;
    println!("Chunks to send: {} (dedup saved {})", 
             needed.len(), 
             tree.chunk_count() - needed.len());
    
    // Send only needed chunks
    for chunk_id in needed {
        let data = tree.read_chunk(chunk_id)?;
        transfer.send_chunk(chunk_id, &data).await?;
    }
    
    // Finalize transfer
    transfer.finalize().await?;
    
    Ok(())
}

/// Receive and verify chunked file
async fn receive_chunked(
    session: &Session,
    save_path: impl AsRef<Path>,
    expected_root: Hash,
) -> Result<()> {
    // Receive manifest
    let transfer = session.accept_chunked_transfer().await?;
    
    // Verify root hash matches expected
    if transfer.root_hash() != expected_root {
        return Err(Error::HashMismatch);
    }
    
    // Receive chunks (verified individually)
    let mut file = ChunkedFile::create(&save_path, transfer.tree()).await?;
    
    while let Some(chunk) = transfer.recv_chunk().await? {
        // Each chunk is verified against Merkle tree
        file.write_chunk(chunk.id, &chunk.data).await?;
    }
    
    // Verify complete file
    file.verify_complete()?;
    
    Ok(())
}
```

---

## 8. Group Communication

### 8.1 Creating and Managing Groups

```rust
//! group_communication.rs
//! 
//! Multi-party group communication using TreeKEM.

use wraith::group::{GroupSession, GroupConfig, GroupEvent, MemberId};

/// Create a new group
async fn create_group(keypair: IdentityKeypair) -> Result<GroupSession> {
    let config = GroupConfig::default()
        .with_max_members(1000)
        .with_topology(GroupTopology::FullMesh);  // or Tree, Gossip
    
    let group = GroupSession::create(keypair, config).await?;
    
    println!("Group created: {}", group.id());
    println!("Invite token: {}", group.invite_token());
    
    Ok(group)
}

/// Join an existing group
async fn join_group(
    keypair: IdentityKeypair,
    invite_token: &str,
) -> Result<GroupSession> {
    let group = GroupSession::join(keypair, invite_token).await?;
    
    println!("Joined group: {}", group.id());
    println!("Members: {}", group.member_count());
    
    Ok(group)
}

/// Handle group events
async fn handle_group_events(group: &GroupSession) {
    let mut events = group.subscribe_events();
    
    while let Some(event) = events.recv().await {
        match event {
            GroupEvent::MemberJoined(member) => {
                println!("+ {} joined", member.display_name());
            }
            GroupEvent::MemberLeft(member) => {
                println!("- {} left", member.display_name());
            }
            GroupEvent::KeyRotated { epoch } => {
                println!("Group key rotated, epoch: {}", epoch);
            }
            GroupEvent::Message { from, data } => {
                println!("{}: {}", from.display_name(), 
                         String::from_utf8_lossy(&data));
            }
        }
    }
}

/// Send message to group
async fn send_to_group(group: &GroupSession, message: &[u8]) -> Result<()> {
    // Message is encrypted with group key and sent to all members
    group.broadcast(message).await?;
    Ok(())
}

/// Send to specific member
async fn send_to_member(
    group: &GroupSession,
    member: MemberId,
    message: &[u8],
) -> Result<()> {
    // Encrypted specifically for this member
    group.send_to(member, message).await?;
    Ok(())
}
```

---

## 9. Transport Configuration

### 9.1 Available Transports

```rust
//! transport_configuration.rs
//! 
//! Configure different transport options.

use wraith::transport::*;

/// Available transport types
pub enum TransportType {
    /// UDP (default, best performance)
    /// Use when: Direct connectivity, performance critical
    Udp(UdpConfig),
    
    /// TCP (reliable fallback)
    /// Use when: UDP blocked, firewall traversal
    Tcp(TcpConfig),
    
    /// WebSocket (web compatibility)
    /// Use when: Browser clients, HTTP-only networks
    WebSocket(WebSocketConfig),
    
    /// HTTP/2 (disguised as web traffic)
    /// Use when: Deep inspection networks
    Http2(Http2Config),
    
    /// QUIC (modern alternative)
    /// Use when: Need QUIC benefits, compatible networks
    Quic(QuicConfig),
    
    /// AF_XDP (kernel bypass, Linux only)
    /// Use when: Maximum performance, server environments
    #[cfg(target_os = "linux")]
    AfXdp(AfXdpConfig),
}

/// Configure transport with automatic fallback
fn config_with_fallback() -> TransportConfig {
    TransportConfig::auto()
        // Try transports in order until one works
        .with_fallback_chain(vec![
            TransportType::Udp(UdpConfig::default()),
            TransportType::Quic(QuicConfig::default()),
            TransportType::WebSocket(WebSocketConfig::default()),
            TransportType::Tcp(TcpConfig::default()),
        ])
        // Timeout before trying next transport
        .with_fallback_timeout(Duration::from_secs(5))
}

/// WebSocket configuration for browser compatibility
fn websocket_config() -> TransportConfig {
    TransportConfig::websocket()
        .with_url("wss://example.com/wraith")
        .with_headers(vec![
            ("User-Agent", "Mozilla/5.0 ..."),
            ("Accept", "text/html,application/json"),
        ])
        // Looks like legitimate WebSocket traffic
        .with_subprotocol("graphql-transport-ws")
}

/// Kernel bypass for maximum throughput (Linux)
#[cfg(target_os = "linux")]
fn kernel_bypass_config() -> TransportConfig {
    TransportConfig::af_xdp()
        .with_interface("eth0")
        .with_queue_count(4)  // Use 4 NIC queues
        .with_zero_copy(true)
        // Requires: CAP_NET_RAW, CAP_BPF, or root
}
```

---

## 10. Obfuscation Configuration

### 10.1 Obfuscation Profiles

```rust
//! obfuscation_configuration.rs
//! 
//! Configure traffic obfuscation for different environments.

use wraith::obfuscation::*;

/// Stealth configuration for censored networks
fn stealth_config() -> ObfuscationConfig {
    ObfuscationConfig::stealth()
        // Packet size distribution matching HTTPS
        .with_padding(PaddingConfig {
            distribution: PaddingDistribution::HttpsEmpirical,
            min_packet: 64,
            max_packet: 1472,
        })
        // Timing obfuscation
        .with_timing(TimingConfig {
            mode: TimingMode::Jittered {
                max_jitter: Duration::from_millis(50),
            },
        })
        // Cover traffic during idle
        .with_cover_traffic(CoverTrafficConfig {
            enabled: true,
            target_rate: 10_000,  // 10 KB/s minimum
            distribution: PaddingDistribution::HttpsEmpirical,
        })
        // Probing resistance
        .with_probing_resistance(true)
}

/// Performance configuration (minimal obfuscation)
fn performance_config() -> ObfuscationConfig {
    ObfuscationConfig::performance()
        // Minimal padding (just to reach MTU for efficiency)
        .with_padding(PaddingConfig {
            distribution: PaddingDistribution::MtuAligned,
            min_packet: 64,
            max_packet: 9000,  // Jumbo frames if supported
        })
        // No timing obfuscation
        .with_timing(TimingConfig {
            mode: TimingMode::None,
        })
        // No cover traffic
        .with_cover_traffic(CoverTrafficConfig {
            enabled: false,
            ..Default::default()
        })
}

/// Custom obfuscation profile
fn custom_obfuscation() -> ObfuscationConfig {
    // Measure target traffic and match it
    let target_distribution = measure_https_traffic("target_network.pcap");
    
    ObfuscationConfig::custom()
        .with_padding(PaddingConfig {
            distribution: PaddingDistribution::Custom(target_distribution),
            ..Default::default()
        })
        .with_timing(TimingConfig {
            mode: TimingMode::MatchTarget {
                samples: load_timing_samples("target_timing.csv"),
            },
        })
}
```

---

## 11. Error Handling

### 11.1 Error Types and Handling

```rust
//! error_handling.rs
//! 
//! Comprehensive error handling for WRAITH applications.

use wraith::{Error, ErrorKind, Result};

/// WRAITH error categories
fn handle_error(error: Error) {
    match error.kind() {
        // Connection errors - may be recoverable with retry
        ErrorKind::ConnectionRefused => {
            eprintln!("Server not reachable. Check address and firewall.");
        }
        ErrorKind::ConnectionTimeout => {
            eprintln!("Connection timed out. Network may be slow or blocked.");
        }
        ErrorKind::ConnectionReset => {
            eprintln!("Connection reset by peer. Server may have crashed.");
        }
        
        // Authentication errors - not recoverable without user action
        ErrorKind::AuthenticationFailed => {
            eprintln!("Authentication failed. Check keys and server identity.");
        }
        ErrorKind::KeyMismatch => {
            eprintln!("Server key doesn't match expected. Possible MITM attack!");
        }
        
        // Protocol errors - usually indicates bug or attack
        ErrorKind::ProtocolViolation => {
            eprintln!("Protocol violation. Connection terminated.");
        }
        ErrorKind::DecryptionFailed => {
            eprintln!("Decryption failed. Possible tampering or key issue.");
        }
        
        // Resource errors - may be recoverable after wait
        ErrorKind::ResourceExhausted => {
            eprintln!("Resource limit reached. Try again later.");
        }
        ErrorKind::RateLimited => {
            eprintln!("Rate limited. Slow down requests.");
        }
        
        // Internal errors - should not happen in production
        ErrorKind::Internal => {
            eprintln!("Internal error: {}. Please report this bug.", error);
        }
        
        _ => {
            eprintln!("Error: {}", error);
        }
    }
}

/// Retry wrapper with exponential backoff
async fn with_retry<T, F, Fut>(
    mut operation: F,
    max_retries: u32,
) -> Result<T>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T>>,
{
    let mut attempt = 0;
    
    loop {
        match operation().await {
            Ok(value) => return Ok(value),
            Err(e) if e.is_retryable() && attempt < max_retries => {
                attempt += 1;
                let delay = Duration::from_millis(100 * 2u64.pow(attempt));
                tokio::time::sleep(delay).await;
            }
            Err(e) => return Err(e),
        }
    }
}
```

---

## 12. Performance Tuning

### 12.1 Buffer and Memory Tuning

```rust
//! performance_tuning.rs
//! 
//! Optimize WRAITH for maximum performance.

use wraith::config::*;

/// High-performance configuration
fn high_performance_config() -> Config {
    Config::performance()
        // Larger buffers for throughput
        .with_session(SessionConfig {
            // Initial flow control window
            initial_window: 16 * 1024 * 1024,  // 16 MB
            
            // Maximum window size
            max_window: 64 * 1024 * 1024,  // 64 MB
            
            // Chunk size for file transfers
            chunk_size: 1024 * 1024,  // 1 MB chunks
            
            // Maximum concurrent streams
            max_streams: 1000,
            
            ..Default::default()
        })
        // Transport tuning
        .with_transport(TransportConfig {
            // Socket buffer sizes
            send_buffer: 4 * 1024 * 1024,  // 4 MB
            recv_buffer: 4 * 1024 * 1024,  // 4 MB
            
            // Use io_uring on Linux
            #[cfg(target_os = "linux")]
            use_io_uring: true,
            
            ..Default::default()
        })
        // Congestion control tuning
        .with_congestion(CongestionConfig {
            algorithm: CongestionAlgorithm::Bbr,  // BBR for high-bandwidth
            initial_cwnd: 32,  // Aggressive initial window
            ..Default::default()
        })
}

/// Memory-constrained configuration
fn constrained_config() -> Config {
    Config::constrained()
        .with_session(SessionConfig {
            initial_window: 256 * 1024,  // 256 KB
            max_window: 1024 * 1024,     // 1 MB max
            chunk_size: 64 * 1024,       // 64 KB chunks
            max_streams: 10,
            ..Default::default()
        })
        .with_resources(ResourceConfig {
            max_memory: 16 * 1024 * 1024,  // 16 MB total
            buffer_pool_size: 100,          // Small pool
            ..Default::default()
        })
}
```

### 12.2 Benchmarking

```rust
//! benchmarking.rs
//! 
//! Measure WRAITH performance.

use wraith::benchmark::*;
use std::time::Instant;

/// Throughput benchmark
async fn benchmark_throughput(session: &Session) -> BenchmarkResult {
    let data = vec![0u8; 1024 * 1024];  // 1 MB chunks
    let total_bytes = 1024 * 1024 * 1024;  // 1 GB total
    let iterations = total_bytes / data.len();
    
    let mut stream = session.open_stream().await?;
    
    let start = Instant::now();
    
    for _ in 0..iterations {
        stream.write_all(&data).await?;
    }
    stream.flush().await?;
    
    let elapsed = start.elapsed();
    let throughput_mbps = (total_bytes as f64 * 8.0) / elapsed.as_secs_f64() / 1_000_000.0;
    
    BenchmarkResult {
        total_bytes,
        elapsed,
        throughput_mbps,
    }
}

/// Latency benchmark
async fn benchmark_latency(session: &Session, iterations: u32) -> LatencyResult {
    let mut latencies = Vec::with_capacity(iterations as usize);
    
    let mut stream = session.open_stream().await?;
    let ping = b"PING";
    let mut pong = [0u8; 4];
    
    for _ in 0..iterations {
        let start = Instant::now();
        
        stream.write_all(ping).await?;
        stream.read_exact(&mut pong).await?;
        
        latencies.push(start.elapsed());
    }
    
    latencies.sort();
    
    LatencyResult {
        min: latencies[0],
        p50: latencies[latencies.len() / 2],
        p99: latencies[latencies.len() * 99 / 100],
        max: latencies[latencies.len() - 1],
    }
}
```

---

## 13. Platform-Specific Notes

### 13.1 Linux

```rust
//! linux_specific.rs

#[cfg(target_os = "linux")]
mod linux {
    use wraith::transport::AfXdpTransport;
    
    /// Setup for kernel bypass (requires root or CAP_NET_RAW + CAP_BPF)
    pub fn setup_kernel_bypass() -> Result<AfXdpTransport> {
        // Load BPF program
        // Attach to NIC
        // Initialize XSK socket
        AfXdpTransport::new(AfXdpConfig {
            interface: "eth0".to_string(),
            queue_id: 0,
            frame_size: 4096,
            num_frames: 4096,
            zero_copy: true,
        })
    }
    
    /// Use io_uring for async I/O
    pub fn enable_io_uring(config: &mut TransportConfig) {
        config.use_io_uring = true;
        config.io_uring_entries = 256;  // Ring buffer size
    }
    
    /// Set socket options for performance
    pub fn optimize_socket(socket: &UdpSocket) -> Result<()> {
        use std::os::unix::io::AsRawFd;
        
        let fd = socket.as_raw_fd();
        
        // Enable busy polling
        unsafe {
            libc::setsockopt(
                fd,
                libc::SOL_SOCKET,
                libc::SO_BUSY_POLL,
                &50i32 as *const _ as *const _,
                4,
            );
        }
        
        Ok(())
    }
}
```

### 13.2 WASM/Browser

```rust
//! wasm_specific.rs

#[cfg(target_arch = "wasm32")]
mod wasm {
    use wraith::transport::WebSocketTransport;
    use wasm_bindgen::prelude::*;
    
    /// Initialize WRAITH for browser environment
    #[wasm_bindgen]
    pub async fn init_wraith() -> Result<JsValue, JsValue> {
        // Use WebSocket transport only
        let config = Config::default()
            .with_transport(TransportType::WebSocket(
                WebSocketConfig::default()
            ));
        
        Ok(JsValue::TRUE)
    }
    
    /// Connect from browser
    #[wasm_bindgen]
    pub async fn connect(
        url: &str,
        server_public_key: &str,
    ) -> Result<JsValue, JsValue> {
        let keypair = IdentityKeypair::generate()?;
        let server_pk = PublicKey::from_hex(server_public_key)?;
        
        let session = Client::connect(url, keypair, server_pk, config).await?;
        
        // Return handle for JavaScript
        Ok(session.into())
    }
}
```

---

## 14. Testing and Debugging

### 14.1 Unit Testing

```rust
//! testing.rs

#[cfg(test)]
mod tests {
    use wraith::testing::*;
    
    #[tokio::test]
    async fn test_basic_connection() {
        // Create test environment with simulated network
        let env = TestEnvironment::new();
        
        // Create server and client
        let server = env.create_server().await;
        let client = env.create_client().await;
        
        // Connect
        let session = client.connect(&server).await.unwrap();
        
        // Verify connection
        assert_eq!(session.state(), SessionState::Established);
    }
    
    #[tokio::test]
    async fn test_network_conditions() {
        let env = TestEnvironment::new()
            // Simulate poor network
            .with_latency(Duration::from_millis(100))
            .with_packet_loss(0.01)  // 1% loss
            .with_bandwidth(1_000_000);  // 1 Mbps
        
        let server = env.create_server().await;
        let client = env.create_client().await;
        
        // Connection should still work
        let session = client.connect(&server).await.unwrap();
        
        // Data transfer should work (with retransmissions)
        let mut stream = session.open_stream().await.unwrap();
        stream.write_all(b"test data").await.unwrap();
    }
}
```

### 14.2 Debugging

```rust
//! debugging.rs

use tracing::{info, debug, trace, span, Level};
use tracing_subscriber;

/// Enable detailed logging
fn setup_logging() {
    tracing_subscriber::fmt()
        .with_max_level(Level::TRACE)
        .with_target(true)
        .with_thread_ids(true)
        .init();
}

/// Enable protocol tracing
fn enable_protocol_trace(config: &mut Config) {
    config.debug.trace_handshake = true;
    config.debug.trace_frames = true;
    config.debug.trace_crypto = true;  // Caution: logs key material!
}

/// Capture packets for analysis
fn enable_packet_capture(config: &mut Config, path: &str) {
    config.debug.pcap_output = Some(path.to_string());
    // Creates pcap file compatible with Wireshark
}
```

---

## 15. Security Best Practices

### 15.1 Key Management

```rust
//! security_best_practices.rs

/// Secure key storage recommendations
mod key_management {
    /// DON'T: Store keys in plaintext
    // let key_bytes = keypair.secret_bytes();  // BAD
    // fs::write("key.bin", key_bytes);         // BAD
    
    /// DO: Use encrypted storage
    pub fn store_key_securely(
        keypair: &IdentityKeypair,
        password: &str,
    ) -> Result<()> {
        // Encrypt with strong KDF
        let encrypted = keypair.to_encrypted_pem(password)?;
        
        // Store with restrictive permissions
        let path = dirs::data_dir()
            .ok_or(Error::NoDataDir)?
            .join("wraith")
            .join("identity.pem");
        
        fs::create_dir_all(path.parent().unwrap())?;
        fs::write(&path, encrypted)?;
        
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&path, fs::Permissions::from_mode(0o600))?;
        }
        
        Ok(())
    }
    
    /// DO: Use hardware security where available
    #[cfg(feature = "hsm")]
    pub fn use_hardware_key() -> Result<IdentityKeypair> {
        // PKCS#11 integration for HSMs
        let pkcs11 = Pkcs11::new()?;
        pkcs11.generate_keypair(KeyType::Ed25519)
    }
}

/// Secure configuration
mod secure_config {
    /// Always verify server identity
    pub fn secure_client_config() -> Config {
        Config::balanced()
            // Pin server's public key
            .with_server_key_pinning(true)
            // Enable post-quantum crypto
            .with_post_quantum(true)
            // Verify certificates (if using TLS transport)
            .with_certificate_verification(true)
    }
}
```

---

## Document History

| Version | Date | Changes |
|---------|------|---------|
| 2.0.0 | 2026-01 | Initial implementation guide |

---

*End of WRAITH Protocol v2 Implementation Guide*
