//! Transport trait abstraction for multiple transport backends.
//!
//! This module defines the core `Transport` trait that abstracts over different
//! network transport implementations (UDP, QUIC, etc.). This allows the WRAITH
//! protocol to work with multiple transport layers without changing application code.

use async_trait::async_trait;
use std::io;
use std::net::SocketAddr;

/// Transport layer errors
#[derive(Debug, thiserror::Error)]
pub enum TransportError {
    /// I/O error from underlying transport
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),

    /// Transport is closed
    #[error("Transport is closed")]
    Closed,

    /// Address binding failed
    #[error("Failed to bind to address: {0}")]
    BindFailed(String),

    /// Connection failed
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),

    /// Invalid configuration
    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),

    /// Transport-specific error
    #[error("Transport error: {0}")]
    Other(String),
}

/// Result type for transport operations
pub type TransportResult<T> = Result<T, TransportError>;

/// Async transport trait for network communication.
///
/// This trait provides a uniform interface for different transport backends
/// (UDP, QUIC, etc.) allowing the WRAITH protocol to work with multiple
/// transport implementations.
///
/// # Examples
///
/// ```no_run
/// use wraith_transport::transport::Transport;
/// use wraith_transport::udp_async::AsyncUdpTransport;
/// use std::net::SocketAddr;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let addr: SocketAddr = "127.0.0.1:40000".parse()?;
/// let transport = AsyncUdpTransport::bind(addr).await?;
///
/// // Send data
/// transport.send_to(b"Hello, WRAITH!", "127.0.0.1:50000".parse()?).await?;
///
/// // Receive data
/// let mut buf = vec![0u8; 1500];
/// let (size, from) = transport.recv_from(&mut buf).await?;
/// println!("Received {} bytes from {}", size, from);
/// # Ok(())
/// # }
/// ```
#[async_trait]
pub trait Transport: Send + Sync {
    /// Send data to a remote address.
    ///
    /// # Arguments
    /// * `buf` - The data to send
    /// * `addr` - The destination address
    ///
    /// # Returns
    /// The number of bytes sent
    ///
    /// # Errors
    /// Returns `TransportError` if the send operation fails
    async fn send_to(&self, buf: &[u8], addr: SocketAddr) -> TransportResult<usize>;

    /// Receive data from the transport.
    ///
    /// Fills `buf` with received data and returns the number of bytes
    /// received and the sender's address.
    ///
    /// # Arguments
    /// * `buf` - Buffer to receive data into
    ///
    /// # Returns
    /// A tuple of (bytes_received, sender_address)
    ///
    /// # Errors
    /// Returns `TransportError` if the receive operation fails
    async fn recv_from(&self, buf: &mut [u8]) -> TransportResult<(usize, SocketAddr)>;

    /// Get the local address this transport is bound to.
    ///
    /// # Errors
    /// Returns `TransportError` if the address cannot be determined
    fn local_addr(&self) -> TransportResult<SocketAddr>;

    /// Close the transport and release resources.
    ///
    /// After calling this method, all subsequent operations should
    /// return `TransportError::Closed`.
    ///
    /// # Errors
    /// Returns `TransportError` if closing fails
    async fn close(&self) -> TransportResult<()>;

    /// Check if the transport is closed.
    fn is_closed(&self) -> bool;

    /// Get transport statistics (optional).
    ///
    /// Returns transport-specific statistics like bytes sent/received,
    /// packet counts, error rates, etc.
    fn stats(&self) -> TransportStats {
        TransportStats::default()
    }
}

/// Transport statistics
#[derive(Debug, Clone, Default)]
pub struct TransportStats {
    /// Total bytes sent
    pub bytes_sent: u64,
    /// Total bytes received
    pub bytes_received: u64,
    /// Total packets sent
    pub packets_sent: u64,
    /// Total packets received
    pub packets_received: u64,
    /// Send errors
    pub send_errors: u64,
    /// Receive errors
    pub recv_errors: u64,
}

impl TransportStats {
    /// Create new empty statistics
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a successful send
    pub fn record_send(&mut self, bytes: usize) {
        self.bytes_sent += bytes as u64;
        self.packets_sent += 1;
    }

    /// Record a successful receive
    pub fn record_recv(&mut self, bytes: usize) {
        self.bytes_received += bytes as u64;
        self.packets_received += 1;
    }

    /// Record a send error
    pub fn record_send_error(&mut self) {
        self.send_errors += 1;
    }

    /// Record a receive error
    pub fn record_recv_error(&mut self) {
        self.recv_errors += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transport_stats() {
        let mut stats = TransportStats::new();
        assert_eq!(stats.bytes_sent, 0);
        assert_eq!(stats.packets_sent, 0);

        stats.record_send(100);
        assert_eq!(stats.bytes_sent, 100);
        assert_eq!(stats.packets_sent, 1);

        stats.record_recv(200);
        assert_eq!(stats.bytes_received, 200);
        assert_eq!(stats.packets_received, 1);

        stats.record_send_error();
        assert_eq!(stats.send_errors, 1);

        stats.record_recv_error();
        assert_eq!(stats.recv_errors, 1);
    }

    #[test]
    fn test_transport_stats_multiple_operations() {
        let mut stats = TransportStats::new();

        // Multiple sends
        for i in 1..=10 {
            stats.record_send(100);
            assert_eq!(stats.packets_sent, i);
            assert_eq!(stats.bytes_sent, i * 100);
        }

        // Multiple receives
        for i in 1..=5 {
            stats.record_recv(50);
            assert_eq!(stats.packets_received, i);
            assert_eq!(stats.bytes_received, i * 50);
        }
    }

    #[test]
    fn test_transport_stats_default() {
        let stats1 = TransportStats::new();
        let stats2 = TransportStats::default();

        assert_eq!(stats1.bytes_sent, stats2.bytes_sent);
        assert_eq!(stats1.packets_sent, stats2.packets_sent);
    }

    #[test]
    fn test_transport_error_display() {
        let err = TransportError::Closed;
        assert_eq!(err.to_string(), "Transport is closed");

        let err = TransportError::BindFailed("test".to_string());
        assert!(err.to_string().contains("Failed to bind"));

        let err = TransportError::ConnectionFailed("test".to_string());
        assert!(err.to_string().contains("Connection failed"));

        let err = TransportError::InvalidConfig("test".to_string());
        assert!(err.to_string().contains("Invalid configuration"));

        let err = TransportError::Other("test error".to_string());
        assert_eq!(err.to_string(), "Transport error: test error");
    }

    #[test]
    fn test_transport_error_from_io() {
        let io_err = std::io::Error::other("test");
        let transport_err = TransportError::from(io_err);

        assert!(matches!(transport_err, TransportError::Io(_)));
    }

    #[test]
    fn test_transport_stats_clone() {
        let mut stats1 = TransportStats::new();
        stats1.record_send(100);
        stats1.record_recv(200);

        let stats2 = stats1.clone();
        assert_eq!(stats1.bytes_sent, stats2.bytes_sent);
        assert_eq!(stats1.bytes_received, stats2.bytes_received);
    }
}
