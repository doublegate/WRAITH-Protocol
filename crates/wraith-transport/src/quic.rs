//! QUIC transport implementation (placeholder).
//!
//! This module provides a placeholder for QUIC transport support.
//! Full QUIC implementation will be added in a future phase when
//! higher-level features are complete.
//!
//! QUIC provides:
//! - Built-in TLS 1.3 encryption
//! - Stream multiplexing
//! - 0-RTT connection resumption
//! - Better congestion control than TCP
//! - NAT traversal improvements

use crate::transport::{Transport, TransportError, TransportResult, TransportStats};
use async_trait::async_trait;
use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

/// QUIC transport (placeholder implementation).
///
/// This is a placeholder for future QUIC support. Currently, all operations
/// return `TransportError::Other` with a message indicating QUIC is not yet implemented.
///
/// # Future Implementation
///
/// When fully implemented, this transport will provide:
/// - QUIC protocol support using quinn or quiche
/// - TLS 1.3 encryption
/// - Stream multiplexing
/// - 0-RTT connection establishment
/// - Improved NAT traversal
///
/// # Examples
///
/// ```no_run
/// use wraith_transport::quic::QuicTransport;
/// use std::net::SocketAddr;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let addr: SocketAddr = "127.0.0.1:40000".parse()?;
/// // This will return an error as QUIC is not yet implemented
/// let result = QuicTransport::bind(addr).await;
/// assert!(result.is_err());
/// # Ok(())
/// # }
/// ```
pub struct QuicTransport {
    local_addr: SocketAddr,
    closed: Arc<AtomicBool>,
}

impl QuicTransport {
    /// Create a new QUIC transport (not yet implemented).
    ///
    /// # Arguments
    /// * `addr` - The local address to bind to
    ///
    /// # Errors
    /// Always returns `TransportError::Other` indicating QUIC is not implemented
    ///
    /// # Examples
    /// ```no_run
    /// use wraith_transport::quic::QuicTransport;
    /// use std::net::SocketAddr;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let addr: SocketAddr = "127.0.0.1:40000".parse()?;
    /// let result = QuicTransport::bind(addr).await;
    /// assert!(result.is_err()); // QUIC not yet implemented
    /// # Ok(())
    /// # }
    /// ```
    pub async fn bind<A: Into<SocketAddr>>(_addr: A) -> TransportResult<Self> {
        Err(TransportError::Other(
            "QUIC transport not yet implemented. Will be added in Phase 6.".to_string(),
        ))
    }

    /// Create a placeholder instance for testing (do not use in production).
    #[cfg(test)]
    #[must_use]
    pub fn placeholder(local_addr: SocketAddr) -> Self {
        Self {
            local_addr,
            closed: Arc::new(AtomicBool::new(false)),
        }
    }
}

#[async_trait]
impl Transport for QuicTransport {
    async fn send_to(&self, _buf: &[u8], _addr: SocketAddr) -> TransportResult<usize> {
        if self.closed.load(Ordering::Relaxed) {
            return Err(TransportError::Closed);
        }
        Err(TransportError::Other(
            "QUIC transport not yet implemented".to_string(),
        ))
    }

    async fn recv_from(&self, _buf: &mut [u8]) -> TransportResult<(usize, SocketAddr)> {
        if self.closed.load(Ordering::Relaxed) {
            return Err(TransportError::Closed);
        }
        Err(TransportError::Other(
            "QUIC transport not yet implemented".to_string(),
        ))
    }

    fn local_addr(&self) -> TransportResult<SocketAddr> {
        Ok(self.local_addr)
    }

    async fn close(&self) -> TransportResult<()> {
        self.closed.store(true, Ordering::Relaxed);
        Ok(())
    }

    fn is_closed(&self) -> bool {
        self.closed.load(Ordering::Relaxed)
    }

    fn stats(&self) -> TransportStats {
        TransportStats::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_quic_not_implemented() {
        let addr: SocketAddr = "127.0.0.1:40000".parse().unwrap();
        let result = QuicTransport::bind(addr).await;
        assert!(result.is_err());

        if let Err(TransportError::Other(msg)) = result {
            assert!(msg.contains("not yet implemented"));
        } else {
            panic!("Expected TransportError::Other");
        }
    }

    #[tokio::test]
    async fn test_quic_placeholder_local_addr() {
        let addr: SocketAddr = "127.0.0.1:40000".parse().unwrap();
        let transport = QuicTransport::placeholder(addr);
        assert_eq!(transport.local_addr().unwrap(), addr);
    }

    #[tokio::test]
    async fn test_quic_placeholder_close() {
        let addr: SocketAddr = "127.0.0.1:40000".parse().unwrap();
        let transport = QuicTransport::placeholder(addr);

        assert!(!transport.is_closed());
        transport.close().await.unwrap();
        assert!(transport.is_closed());
    }

    #[tokio::test]
    async fn test_quic_placeholder_operations_fail() {
        let addr: SocketAddr = "127.0.0.1:40000".parse().unwrap();
        let transport = QuicTransport::placeholder(addr);

        let send_result = transport.send_to(b"test", addr).await;
        assert!(matches!(send_result, Err(TransportError::Other(_))));

        let mut buf = vec![0u8; 1500];
        let recv_result = transport.recv_from(&mut buf).await;
        assert!(matches!(recv_result, Err(TransportError::Other(_))));
    }
}
