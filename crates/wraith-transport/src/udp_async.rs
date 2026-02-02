//! Async UDP transport implementation.
//!
//! This module provides an async UDP transport implementation using Tokio
//! that implements the `Transport` trait.

use crate::factory::TransportType;
use crate::transport::{Transport, TransportError, TransportResult, TransportStats};
use async_trait::async_trait;
use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use tokio::net::UdpSocket;

/// Async UDP transport using Tokio.
///
/// This transport provides high-performance async UDP communication
/// with statistics tracking and graceful shutdown support.
///
/// # Examples
///
/// ```no_run
/// use wraith_transport::udp_async::AsyncUdpTransport;
/// use wraith_transport::transport::Transport;
/// use std::net::SocketAddr;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let addr: SocketAddr = "127.0.0.1:40000".parse()?;
/// let transport = AsyncUdpTransport::bind(addr).await?;
///
/// // Send data
/// transport.send_to(b"Hello!", "127.0.0.1:50000".parse()?).await?;
///
/// // Receive data
/// let mut buf = vec![0u8; 1500];
/// let (size, from) = transport.recv_from(&mut buf).await?;
/// println!("Received {} bytes from {}", size, from);
/// # Ok(())
/// # }
/// ```
#[derive(Clone)]
pub struct AsyncUdpTransport {
    socket: Arc<UdpSocket>,
    closed: Arc<AtomicBool>,
    bytes_sent: Arc<AtomicU64>,
    bytes_received: Arc<AtomicU64>,
    packets_sent: Arc<AtomicU64>,
    packets_received: Arc<AtomicU64>,
    send_errors: Arc<AtomicU64>,
    recv_errors: Arc<AtomicU64>,
}

impl AsyncUdpTransport {
    /// Create a new async UDP transport bound to the given address.
    ///
    /// # Arguments
    /// * `addr` - The local address to bind to. Use "0.0.0.0:0" for automatic port selection.
    ///
    /// # Errors
    /// Returns `TransportError` if binding fails
    ///
    /// # Examples
    /// ```no_run
    /// use wraith_transport::udp_async::AsyncUdpTransport;
    /// use wraith_transport::transport::Transport;
    /// use std::net::SocketAddr;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let addr: SocketAddr = "127.0.0.1:0".parse()?;
    /// let transport = AsyncUdpTransport::bind(addr).await?;
    /// println!("Listening on {}", transport.local_addr()?);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn bind<A: Into<SocketAddr>>(addr: A) -> TransportResult<Self> {
        let addr = addr.into();

        // Create socket using socket2 for advanced options
        let domain = if addr.is_ipv4() {
            socket2::Domain::IPV4
        } else {
            socket2::Domain::IPV6
        };

        let socket2 =
            socket2::Socket::new(domain, socket2::Type::DGRAM, Some(socket2::Protocol::UDP))
                .map_err(|e| TransportError::BindFailed(e.to_string()))?;

        // Set buffer sizes for high-throughput operation
        socket2
            .set_recv_buffer_size(2 * 1024 * 1024)
            .map_err(|e| TransportError::BindFailed(e.to_string()))?; // 2MB
        socket2
            .set_send_buffer_size(2 * 1024 * 1024)
            .map_err(|e| TransportError::BindFailed(e.to_string()))?; // 2MB

        // Bind to address
        socket2
            .bind(&addr.into())
            .map_err(|e| TransportError::BindFailed(e.to_string()))?;

        // Convert to std socket, then to tokio socket
        socket2
            .set_nonblocking(true)
            .map_err(|e| TransportError::BindFailed(e.to_string()))?;
        let std_socket: std::net::UdpSocket = socket2.into();
        let socket = UdpSocket::from_std(std_socket)
            .map_err(|e| TransportError::BindFailed(e.to_string()))?;

        Ok(Self {
            socket: Arc::new(socket),
            closed: Arc::new(AtomicBool::new(false)),
            bytes_sent: Arc::new(AtomicU64::new(0)),
            bytes_received: Arc::new(AtomicU64::new(0)),
            packets_sent: Arc::new(AtomicU64::new(0)),
            packets_received: Arc::new(AtomicU64::new(0)),
            send_errors: Arc::new(AtomicU64::new(0)),
            recv_errors: Arc::new(AtomicU64::new(0)),
        })
    }

    /// Create from an existing Tokio UdpSocket.
    ///
    /// # Arguments
    /// * `socket` - An already-bound Tokio UdpSocket
    #[must_use]
    pub fn from_socket(socket: UdpSocket) -> Self {
        Self {
            socket: Arc::new(socket),
            closed: Arc::new(AtomicBool::new(false)),
            bytes_sent: Arc::new(AtomicU64::new(0)),
            bytes_received: Arc::new(AtomicU64::new(0)),
            packets_sent: Arc::new(AtomicU64::new(0)),
            packets_received: Arc::new(AtomicU64::new(0)),
            send_errors: Arc::new(AtomicU64::new(0)),
            recv_errors: Arc::new(AtomicU64::new(0)),
        }
    }
}

#[async_trait]
impl Transport for AsyncUdpTransport {
    async fn send_to(&self, buf: &[u8], addr: SocketAddr) -> TransportResult<usize> {
        if self.closed.load(Ordering::Relaxed) {
            return Err(TransportError::Closed);
        }

        match self.socket.send_to(buf, addr).await {
            Ok(sent) => {
                self.bytes_sent.fetch_add(sent as u64, Ordering::Relaxed);
                self.packets_sent.fetch_add(1, Ordering::Relaxed);
                Ok(sent)
            }
            Err(e) => {
                self.send_errors.fetch_add(1, Ordering::Relaxed);
                Err(TransportError::Io(e))
            }
        }
    }

    async fn recv_from(&self, buf: &mut [u8]) -> TransportResult<(usize, SocketAddr)> {
        if self.closed.load(Ordering::Relaxed) {
            return Err(TransportError::Closed);
        }

        match self.socket.recv_from(buf).await {
            Ok((size, addr)) => {
                self.bytes_received
                    .fetch_add(size as u64, Ordering::Relaxed);
                self.packets_received.fetch_add(1, Ordering::Relaxed);
                Ok((size, addr))
            }
            Err(e) => {
                self.recv_errors.fetch_add(1, Ordering::Relaxed);
                Err(TransportError::Io(e))
            }
        }
    }

    fn local_addr(&self) -> TransportResult<SocketAddr> {
        self.socket.local_addr().map_err(TransportError::Io)
    }

    async fn close(&self) -> TransportResult<()> {
        self.closed.store(true, Ordering::Relaxed);
        Ok(())
    }

    fn is_closed(&self) -> bool {
        self.closed.load(Ordering::Relaxed)
    }

    fn stats(&self) -> TransportStats {
        TransportStats {
            bytes_sent: self.bytes_sent.load(Ordering::Relaxed),
            bytes_received: self.bytes_received.load(Ordering::Relaxed),
            packets_sent: self.packets_sent.load(Ordering::Relaxed),
            packets_received: self.packets_received.load(Ordering::Relaxed),
            send_errors: self.send_errors.load(Ordering::Relaxed),
            recv_errors: self.recv_errors.load(Ordering::Relaxed),
        }
    }

    fn transport_type(&self) -> TransportType {
        TransportType::Udp
    }

    fn mtu(&self) -> usize {
        // Standard Ethernet MTU minus IP/UDP headers
        1472
    }

    fn latency_estimate(&self) -> std::time::Duration {
        std::time::Duration::from_micros(100)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use tokio::time::timeout;

    #[tokio::test]
    async fn test_udp_bind() {
        let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
        let transport = AsyncUdpTransport::bind(addr).await.unwrap();
        let bound_addr = transport.local_addr().unwrap();
        assert_ne!(bound_addr.port(), 0);
        assert!(bound_addr.is_ipv4());
    }

    #[tokio::test]
    async fn test_udp_send_recv() {
        let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
        let server = AsyncUdpTransport::bind(addr).await.unwrap();
        let server_addr = server.local_addr().unwrap();

        let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
        let client = AsyncUdpTransport::bind(addr).await.unwrap();

        // Send from client to server
        let sent = client
            .send_to(b"Hello, WRAITH!", server_addr)
            .await
            .unwrap();
        assert_eq!(sent, 14);

        // Receive on server
        let mut buf = vec![0u8; 1500];
        let (size, from) = timeout(Duration::from_secs(1), server.recv_from(&mut buf))
            .await
            .expect("Timeout")
            .unwrap();

        assert_eq!(size, 14);
        assert_eq!(&buf[..size], b"Hello, WRAITH!");
        assert_eq!(from, client.local_addr().unwrap());
    }

    #[tokio::test]
    async fn test_udp_stats() {
        let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
        let server = AsyncUdpTransport::bind(addr).await.unwrap();
        let server_addr = server.local_addr().unwrap();

        let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
        let client = AsyncUdpTransport::bind(addr).await.unwrap();

        // Initial stats should be zero
        let stats = client.stats();
        assert_eq!(stats.packets_sent, 0);
        assert_eq!(stats.bytes_sent, 0);

        // Send a packet
        client.send_to(b"Test", server_addr).await.unwrap();

        // Check stats updated
        let stats = client.stats();
        assert_eq!(stats.packets_sent, 1);
        assert_eq!(stats.bytes_sent, 4);

        // Receive on server
        let mut buf = vec![0u8; 1500];
        timeout(Duration::from_secs(1), server.recv_from(&mut buf))
            .await
            .expect("Timeout")
            .unwrap();

        let stats = server.stats();
        assert_eq!(stats.packets_received, 1);
        assert_eq!(stats.bytes_received, 4);
    }

    #[tokio::test]
    async fn test_udp_close() {
        let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
        let transport = AsyncUdpTransport::bind(addr).await.unwrap();

        assert!(!transport.is_closed());

        transport.close().await.unwrap();

        assert!(transport.is_closed());

        // Operations after close should fail
        let result = transport
            .send_to(b"test", "127.0.0.1:1234".parse().unwrap())
            .await;
        assert!(matches!(result, Err(TransportError::Closed)));
    }

    #[tokio::test]
    async fn test_udp_large_packet() {
        let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
        let server = AsyncUdpTransport::bind(addr).await.unwrap();
        let server_addr = server.local_addr().unwrap();

        let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
        let client = AsyncUdpTransport::bind(addr).await.unwrap();

        // Send a large packet (close to MTU)
        let large_data = vec![0xAA; 1400];
        let sent = client.send_to(&large_data, server_addr).await.unwrap();
        assert_eq!(sent, 1400);

        // Receive on server
        let mut buf = vec![0u8; 2000];
        let (size, _) = timeout(Duration::from_secs(1), server.recv_from(&mut buf))
            .await
            .expect("Timeout")
            .unwrap();

        assert_eq!(size, 1400);
        assert_eq!(&buf[..size], &large_data[..]);
    }

    #[tokio::test]
    async fn test_udp_multiple_packets() {
        let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
        let server = AsyncUdpTransport::bind(addr).await.unwrap();
        let server_addr = server.local_addr().unwrap();

        let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
        let client = AsyncUdpTransport::bind(addr).await.unwrap();

        // Send multiple packets
        for i in 0..10 {
            let data = format!("Packet {}", i);
            client.send_to(data.as_bytes(), server_addr).await.unwrap();
        }

        // Receive all packets
        let mut buf = vec![0u8; 1500];
        for _ in 0..10 {
            let result = timeout(Duration::from_secs(1), server.recv_from(&mut buf)).await;
            assert!(result.is_ok(), "Timeout receiving packet");
        }

        // Verify stats
        let client_stats = client.stats();
        assert_eq!(client_stats.packets_sent, 10);

        let server_stats = server.stats();
        assert_eq!(server_stats.packets_received, 10);
    }

    #[tokio::test]
    async fn test_udp_ipv6() {
        let addr: SocketAddr = "[::1]:0".parse().unwrap();
        let transport = AsyncUdpTransport::bind(addr).await.unwrap();
        let bound_addr = transport.local_addr().unwrap();

        assert!(bound_addr.is_ipv6());
        assert_ne!(bound_addr.port(), 0);
    }

    #[tokio::test]
    async fn test_udp_ipv6_send_recv() {
        let addr: SocketAddr = "[::1]:0".parse().unwrap();
        let server = AsyncUdpTransport::bind(addr).await.unwrap();
        let server_addr = server.local_addr().unwrap();

        let addr: SocketAddr = "[::1]:0".parse().unwrap();
        let client = AsyncUdpTransport::bind(addr).await.unwrap();

        client.send_to(b"IPv6 test", server_addr).await.unwrap();

        let mut buf = vec![0u8; 1500];
        let (size, _) = timeout(Duration::from_secs(1), server.recv_from(&mut buf))
            .await
            .expect("Timeout")
            .unwrap();

        assert_eq!(size, 9);
        assert_eq!(&buf[..size], b"IPv6 test");
    }

    #[tokio::test]
    async fn test_udp_from_socket() {
        let std_socket = std::net::UdpSocket::bind("127.0.0.1:0").unwrap();
        std_socket.set_nonblocking(true).unwrap();

        let tokio_socket = tokio::net::UdpSocket::from_std(std_socket).unwrap();
        let transport = AsyncUdpTransport::from_socket(tokio_socket);

        assert!(!transport.is_closed());
        let addr = transport.local_addr().unwrap();
        assert!(addr.is_ipv4());
    }

    #[tokio::test]
    async fn test_udp_empty_packet() {
        let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
        let server = AsyncUdpTransport::bind(addr).await.unwrap();
        let server_addr = server.local_addr().unwrap();

        let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
        let client = AsyncUdpTransport::bind(addr).await.unwrap();

        // Send empty packet
        let sent = client.send_to(&[], server_addr).await.unwrap();
        assert_eq!(sent, 0);

        // Receive empty packet
        let mut buf = vec![0u8; 1500];
        let (size, _) = timeout(Duration::from_secs(1), server.recv_from(&mut buf))
            .await
            .expect("Timeout")
            .unwrap();
        assert_eq!(size, 0);
    }

    #[tokio::test]
    async fn test_udp_stats_errors() {
        let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
        let transport = AsyncUdpTransport::bind(addr).await.unwrap();

        // Close transport
        transport.close().await.unwrap();

        // Attempt send after close - should increment error counter
        let result = transport
            .send_to(b"test", "127.0.0.1:1234".parse().unwrap())
            .await;
        assert!(matches!(result, Err(TransportError::Closed)));

        // Note: We can't easily test recv errors in the same way since close
        // is checked before the actual recv operation
    }

    #[tokio::test]
    async fn test_udp_concurrent_operations() {
        let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
        let server = AsyncUdpTransport::bind(addr).await.unwrap();
        let server_addr = server.local_addr().unwrap();

        let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
        let client = AsyncUdpTransport::bind(addr).await.unwrap();

        // Spawn concurrent sends
        let send_handles: Vec<_> = (0..5)
            .map(|i| {
                let client = client.clone();
                tokio::spawn(async move {
                    let data = format!("Concurrent {}", i);
                    client.send_to(data.as_bytes(), server_addr).await
                })
            })
            .collect();

        // Wait for all sends
        for handle in send_handles {
            handle.await.unwrap().unwrap();
        }

        // Receive all packets
        let mut buf = vec![0u8; 1500];
        for _ in 0..5 {
            timeout(Duration::from_secs(1), server.recv_from(&mut buf))
                .await
                .expect("Timeout")
                .unwrap();
        }

        // Verify stats
        let client_stats = client.stats();
        assert_eq!(client_stats.packets_sent, 5);
    }

    #[tokio::test]
    async fn test_udp_recv_after_close() {
        let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
        let transport = AsyncUdpTransport::bind(addr).await.unwrap();

        transport.close().await.unwrap();

        let mut buf = vec![0u8; 1500];
        let result = transport.recv_from(&mut buf).await;
        assert!(matches!(result, Err(TransportError::Closed)));
    }
}
