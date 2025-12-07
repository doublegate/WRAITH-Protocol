//! Two-node test fixture for integration testing
//!
//! Provides a reusable test infrastructure for testing two-node scenarios:
//! - Session establishment
//! - File transfers
//! - Protocol interactions
//!
//! # Example
//!
//! ```no_run
//! use wraith_integration_tests::fixtures::TwoNodeFixture;
//!
//! #[tokio::test]
//! async fn test_basic_transfer() {
//!     let mut fixture = TwoNodeFixture::new().await.unwrap();
//!     fixture.establish_session().await.unwrap();
//!
//!     let transfer_id = fixture.send_file(Path::new("/tmp/test.bin")).await.unwrap();
//!     // ... test transfer ...
//!
//!     fixture.cleanup().await.unwrap();
//! }
//! ```

use std::net::SocketAddr;
use std::path::Path;
use std::sync::atomic::{AtomicU16, Ordering};
use std::time::Duration;
use tokio::net::UdpSocket;
use wraith_core::node::{Identity, Node, NodeConfig, NodeError};

/// Global port allocator for concurrent test execution
static NEXT_PORT: AtomicU16 = AtomicU16::new(19000);

/// Allocate a pair of unique ports for a test
fn allocate_port_pair() -> (u16, u16) {
    let base = NEXT_PORT.fetch_add(2, Ordering::SeqCst);
    (base, base + 1)
}

/// Two-node test fixture
///
/// Provides lifecycle management for a pair of WRAITH nodes that can
/// communicate with each other for testing purposes.
pub struct TwoNodeFixture {
    pub initiator: Node,
    pub responder: Node,
    pub initiator_addr: SocketAddr,
    pub responder_addr: SocketAddr,
    initiator_identity: Identity,
    responder_identity: Identity,
    session_established: bool,
}

impl TwoNodeFixture {
    /// Create a new two-node fixture with random identities
    ///
    /// Automatically allocates unique ports to avoid conflicts with
    /// concurrent test execution.
    ///
    /// # Errors
    ///
    /// Returns `NodeError::TransportInit` if UDP sockets cannot be bound
    /// (e.g., ports already in use).
    pub async fn new() -> Result<Self, NodeError> {
        Self::new_with_config(NodeConfig::default(), NodeConfig::default()).await
    }

    /// Create a new two-node fixture with custom configurations
    ///
    /// Useful for testing specific transport or obfuscation settings.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use wraith_core::node::NodeConfig;
    /// use wraith_integration_tests::fixtures::TwoNodeFixture;
    ///
    /// async fn example() -> Result<(), Box<dyn std::error::Error>> {
    ///     let mut initiator_config = NodeConfig::default();
    ///     initiator_config.transport.enable_af_xdp = false;
    ///
    ///     let mut responder_config = NodeConfig::default();
    ///     responder_config.transport.worker_threads = 2;
    ///
    ///     let fixture = TwoNodeFixture::new_with_config(
    ///         initiator_config,
    ///         responder_config
    ///     ).await?;
    ///     Ok(())
    /// }
    /// ```
    pub async fn new_with_config(
        mut initiator_config: NodeConfig,
        mut responder_config: NodeConfig,
    ) -> Result<Self, NodeError> {
        // Allocate unique ports
        let (initiator_port, responder_port) = allocate_port_pair();
        let initiator_addr: SocketAddr = format!("127.0.0.1:{}", initiator_port).parse().unwrap();
        let responder_addr: SocketAddr = format!("127.0.0.1:{}", responder_port).parse().unwrap();

        // Verify ports are available
        Self::verify_port_available(initiator_addr).await?;
        Self::verify_port_available(responder_addr).await?;

        // Configure transport for local testing
        initiator_config.listen_addr = initiator_addr;
        initiator_config.transport.enable_xdp = false; // Disable AF_XDP for tests
        initiator_config.transport.enable_io_uring = false; // Disable io_uring for portability
        initiator_config.transport.connection_timeout = Duration::from_secs(5);
        initiator_config.transport.idle_timeout = Duration::from_secs(10);

        responder_config.listen_addr = responder_addr;
        responder_config.transport.enable_xdp = false;
        responder_config.transport.enable_io_uring = false;
        responder_config.transport.connection_timeout = Duration::from_secs(5);
        responder_config.transport.idle_timeout = Duration::from_secs(10);

        // Create identities
        let initiator_identity = Identity::generate()
            .map_err(|e| NodeError::Crypto(wraith_crypto::CryptoError::Handshake(e.to_string())))?;
        let responder_identity = Identity::generate()
            .map_err(|e| NodeError::Crypto(wraith_crypto::CryptoError::Handshake(e.to_string())))?;

        // Create nodes
        let initiator =
            Node::new_from_identity(initiator_identity.clone(), initiator_config).await?;
        let responder =
            Node::new_from_identity(responder_identity.clone(), responder_config).await?;

        Ok(Self {
            initiator,
            responder,
            initiator_addr,
            responder_addr,
            initiator_identity,
            responder_identity,
            session_established: false,
        })
    }

    /// Verify that a port is available for binding
    async fn verify_port_available(addr: SocketAddr) -> Result<(), NodeError> {
        UdpSocket::bind(addr)
            .await
            .map_err(|e| {
                NodeError::TransportInit(format!("Port {} already in use: {}", addr.port(), e))
            })
            .map(|_| ())
    }

    /// Start both nodes
    ///
    /// Must be called before establishing sessions or transferring files.
    pub async fn start(&mut self) -> Result<(), NodeError> {
        self.initiator.start().await?;
        self.responder.start().await?;
        Ok(())
    }

    /// Establish a session between initiator and responder
    ///
    /// Performs the Noise_XX handshake and sets up session crypto.
    ///
    /// # Errors
    ///
    /// Returns `NodeError::SessionEstablishment` if the handshake fails.
    pub async fn establish_session(&mut self) -> Result<(), NodeError> {
        if self.session_established {
            return Ok(());
        }

        // Start nodes if not already running
        if !self.initiator.is_running() || !self.responder.is_running() {
            self.start().await?;
        }

        // Establish session from initiator to responder using establish_session_with_addr
        // Note: Sessions are keyed by X25519 public keys, not Ed25519 node IDs
        self.initiator
            .establish_session_with_addr(
                self.responder_identity.x25519_public_key(),
                self.responder_addr,
            )
            .await?;

        self.session_established = true;
        Ok(())
    }

    /// Send a file from initiator to responder
    ///
    /// Automatically establishes a session if not already established.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the file to send
    ///
    /// # Returns
    ///
    /// The transfer ID for tracking progress
    ///
    /// # Errors
    ///
    /// Returns `NodeError::Transfer` if the file cannot be read or
    /// the transfer cannot be initiated.
    pub async fn send_file(&mut self, path: &Path) -> Result<[u8; 32], NodeError> {
        // Ensure session is established
        if !self.session_established {
            self.establish_session().await?;
        }

        // Initiate file transfer - send_file takes (file_path, peer_id)
        // Note: Sessions are keyed by X25519 public keys, not Ed25519 node IDs
        let transfer_id = self
            .initiator
            .send_file(path, self.responder_identity.x25519_public_key())
            .await?;

        Ok(transfer_id)
    }

    /// Wait for a transfer to complete with timeout
    ///
    /// # Arguments
    ///
    /// * `transfer_id` - The transfer to wait for
    /// * `timeout` - Maximum time to wait
    ///
    /// # Errors
    ///
    /// Returns `NodeError::Timeout` if the transfer does not complete
    /// within the specified timeout.
    pub async fn wait_for_transfer(
        &self,
        transfer_id: &[u8; 32],
        timeout: Duration,
    ) -> Result<(), NodeError> {
        tokio::time::timeout(timeout, self.initiator.wait_for_transfer(*transfer_id))
            .await
            .map_err(|_| NodeError::Timeout("Transfer did not complete in time".to_string()))?
    }

    /// Get transfer progress as a percentage (0.0 to 100.0)
    pub async fn get_transfer_progress(&self, transfer_id: &[u8; 32]) -> Option<f64> {
        self.initiator
            .get_transfer_progress(transfer_id)
            .await
            .map(|progress| progress.progress_percent)
    }

    /// Get the responder's peer ID (X25519 public key)
    ///
    /// Note: Sessions are keyed by X25519 public keys, not Ed25519 node IDs
    pub fn responder_peer_id(&self) -> [u8; 32] {
        *self.responder_identity.x25519_public_key()
    }

    /// Get the initiator's peer ID (X25519 public key)
    ///
    /// Note: Sessions are keyed by X25519 public keys, not Ed25519 node IDs
    pub fn initiator_peer_id(&self) -> [u8; 32] {
        *self.initiator_identity.x25519_public_key()
    }

    /// Get the number of active sessions on the initiator
    pub async fn initiator_active_sessions(&self) -> usize {
        self.initiator.active_sessions().await.len()
    }

    /// Get the number of active sessions on the responder
    pub async fn responder_active_sessions(&self) -> usize {
        self.responder.active_sessions().await.len()
    }

    /// Clean up resources and stop both nodes
    ///
    /// Should be called at the end of each test to ensure clean shutdown.
    pub async fn cleanup(self) -> Result<(), NodeError> {
        // Close all sessions
        // Note: Sessions are keyed by X25519 public keys, not Ed25519 node IDs
        if self.session_established {
            let _ = self
                .initiator
                .close_session(self.responder_identity.x25519_public_key())
                .await;
            let _ = self
                .responder
                .close_session(self.initiator_identity.x25519_public_key())
                .await;
        }

        // Stop nodes if running
        if self.initiator.is_running() {
            self.initiator.stop().await?;
        }
        if self.responder.is_running() {
            self.responder.stop().await?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[tokio::test]
    async fn test_fixture_creation() {
        let fixture = TwoNodeFixture::new().await.unwrap();
        assert!(!fixture.session_established);
        assert_ne!(fixture.initiator_addr.port(), fixture.responder_addr.port());
        fixture.cleanup().await.unwrap();
    }

    #[tokio::test]
    async fn test_fixture_session_establishment() {
        let mut fixture = TwoNodeFixture::new().await.unwrap();
        fixture.establish_session().await.unwrap();
        assert!(fixture.session_established);

        // Verify session exists
        assert_eq!(fixture.initiator_active_sessions().await, 1);

        fixture.cleanup().await.unwrap();
    }

    #[tokio::test]
    async fn test_fixture_file_transfer() {
        let mut fixture = TwoNodeFixture::new().await.unwrap();

        // Create temporary test file
        let mut temp_file = NamedTempFile::new().unwrap();
        let test_data = b"Hello, WRAITH Protocol!";
        temp_file.write_all(test_data).unwrap();
        temp_file.flush().unwrap();

        // Start nodes and establish session
        fixture.start().await.unwrap();
        fixture.establish_session().await.unwrap();

        // Send file
        let transfer_id = fixture.send_file(temp_file.path()).await.unwrap();

        // Verify transfer was initiated
        let progress = fixture.get_transfer_progress(&transfer_id).await;
        assert!(progress.is_some());

        fixture.cleanup().await.unwrap();
    }

    #[tokio::test]
    async fn test_fixture_concurrent_port_allocation() {
        // Create multiple fixtures concurrently to test port allocation
        let fixture1 = TwoNodeFixture::new().await.unwrap();
        let fixture2 = TwoNodeFixture::new().await.unwrap();
        let fixture3 = TwoNodeFixture::new().await.unwrap();

        // All should have different ports
        let ports = vec![
            fixture1.initiator_addr.port(),
            fixture1.responder_addr.port(),
            fixture2.initiator_addr.port(),
            fixture2.responder_addr.port(),
            fixture3.initiator_addr.port(),
            fixture3.responder_addr.port(),
        ];

        // Check all ports are unique
        for (i, &port1) in ports.iter().enumerate() {
            for &port2 in ports.iter().skip(i + 1) {
                assert_ne!(port1, port2, "Ports should be unique");
            }
        }

        fixture1.cleanup().await.unwrap();
        fixture2.cleanup().await.unwrap();
        fixture3.cleanup().await.unwrap();
    }

    #[tokio::test]
    async fn test_fixture_cleanup() {
        let mut fixture = TwoNodeFixture::new().await.unwrap();
        fixture.establish_session().await.unwrap();

        // Cleanup should succeed
        fixture.cleanup().await.unwrap();

        // Note: After cleanup, fixture is consumed, so we can't check state
    }
}
