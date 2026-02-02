//! TCP transport implementation with length-prefixed framing.
//!
//! This module provides a TCP-based transport that implements the `Transport` trait.
//! TCP is useful as a fallback when UDP is blocked by firewalls. Messages are
//! framed using a 4-byte big-endian length prefix to delineate message boundaries
//! over the stream-oriented TCP connection.
//!
//! # Framing Protocol
//!
//! Each message is prefixed with a 4-byte big-endian length header:
//!
//! ```text
//! +------------------+------------------+
//! | Length (4 bytes)  | Payload (N bytes)|
//! +------------------+------------------+
//! ```

use crate::factory::TransportType;
use crate::transport::{Transport, TransportError, TransportResult, TransportStats};
use async_trait::async_trait;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Mutex;

/// Maximum message size for TCP framing (16 MiB).
const MAX_MESSAGE_SIZE: u32 = 16 * 1024 * 1024;

/// TCP transport with length-prefixed framing.
///
/// This transport provides reliable, ordered delivery over TCP connections.
/// It maintains a map of peer connections and accepts incoming connections
/// via a bound listener.
///
/// # Examples
///
/// ```no_run
/// use wraith_transport::tcp::TcpTransport;
/// use wraith_transport::transport::Transport;
/// use std::net::SocketAddr;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let addr: SocketAddr = "127.0.0.1:0".parse()?;
/// let transport = TcpTransport::bind(addr).await?;
/// println!("TCP transport listening on {}", transport.local_addr()?);
/// # Ok(())
/// # }
/// ```
pub struct TcpTransport {
    listener: Arc<TcpListener>,
    local_addr: SocketAddr,
    closed: Arc<AtomicBool>,
    /// Outbound connections keyed by peer address.
    connections: Arc<Mutex<HashMap<SocketAddr, TcpStream>>>,
    /// Buffer of received messages from accepted connections: (data, peer_addr).
    #[allow(clippy::type_complexity)]
    recv_queue: Arc<Mutex<Vec<(Vec<u8>, SocketAddr)>>>,
    bytes_sent: Arc<AtomicU64>,
    bytes_received: Arc<AtomicU64>,
    packets_sent: Arc<AtomicU64>,
    packets_received: Arc<AtomicU64>,
    send_errors: Arc<AtomicU64>,
    recv_errors: Arc<AtomicU64>,
}

impl TcpTransport {
    /// Create a new TCP transport bound to the given address.
    ///
    /// # Arguments
    /// * `addr` - The local address to bind to. Use port 0 for OS-assigned port.
    ///
    /// # Errors
    /// Returns `TransportError::BindFailed` if the TCP listener cannot be created.
    pub async fn bind<A: Into<SocketAddr>>(addr: A) -> TransportResult<Self> {
        let addr = addr.into();
        let listener = TcpListener::bind(addr)
            .await
            .map_err(|e| TransportError::BindFailed(e.to_string()))?;
        let local_addr = listener
            .local_addr()
            .map_err(|e| TransportError::BindFailed(e.to_string()))?;

        Ok(Self {
            listener: Arc::new(listener),
            local_addr,
            closed: Arc::new(AtomicBool::new(false)),
            connections: Arc::new(Mutex::new(HashMap::new())),
            recv_queue: Arc::new(Mutex::new(Vec::new())),
            bytes_sent: Arc::new(AtomicU64::new(0)),
            bytes_received: Arc::new(AtomicU64::new(0)),
            packets_sent: Arc::new(AtomicU64::new(0)),
            packets_received: Arc::new(AtomicU64::new(0)),
            send_errors: Arc::new(AtomicU64::new(0)),
            recv_errors: Arc::new(AtomicU64::new(0)),
        })
    }

    /// Get or create a connection to the given peer address.
    async fn get_or_connect(&self, addr: SocketAddr) -> TransportResult<()> {
        let mut conns = self.connections.lock().await;
        if conns.contains_key(&addr) {
            return Ok(());
        }
        let stream = TcpStream::connect(addr)
            .await
            .map_err(|e| TransportError::ConnectionFailed(e.to_string()))?;
        conns.insert(addr, stream);
        Ok(())
    }

    /// Write a length-prefixed message to a TCP stream.
    async fn write_framed(stream: &mut TcpStream, data: &[u8]) -> TransportResult<usize> {
        let len = data.len() as u32;
        stream
            .write_all(&len.to_be_bytes())
            .await
            .map_err(TransportError::Io)?;
        stream.write_all(data).await.map_err(TransportError::Io)?;
        stream.flush().await.map_err(TransportError::Io)?;
        Ok(data.len())
    }

    /// Read a length-prefixed message from a TCP stream.
    async fn read_framed(stream: &mut TcpStream) -> TransportResult<Vec<u8>> {
        let mut len_buf = [0u8; 4];
        stream
            .read_exact(&mut len_buf)
            .await
            .map_err(TransportError::Io)?;
        let len = u32::from_be_bytes(len_buf);
        if len > MAX_MESSAGE_SIZE {
            return Err(TransportError::Other(format!(
                "Message too large: {} bytes (max {})",
                len, MAX_MESSAGE_SIZE
            )));
        }
        let mut buf = vec![0u8; len as usize];
        stream
            .read_exact(&mut buf)
            .await
            .map_err(TransportError::Io)?;
        Ok(buf)
    }
}

#[async_trait]
impl Transport for TcpTransport {
    async fn send_to(&self, buf: &[u8], addr: SocketAddr) -> TransportResult<usize> {
        if self.closed.load(Ordering::Relaxed) {
            return Err(TransportError::Closed);
        }

        if let Err(e) = self.get_or_connect(addr).await {
            self.send_errors.fetch_add(1, Ordering::Relaxed);
            return Err(e);
        }

        let mut conns = self.connections.lock().await;
        let stream = conns.get_mut(&addr).ok_or_else(|| {
            TransportError::ConnectionFailed("Connection not found after connect".to_string())
        })?;

        match Self::write_framed(stream, buf).await {
            Ok(sent) => {
                self.bytes_sent.fetch_add(sent as u64, Ordering::Relaxed);
                self.packets_sent.fetch_add(1, Ordering::Relaxed);
                Ok(sent)
            }
            Err(e) => {
                self.send_errors.fetch_add(1, Ordering::Relaxed);
                // Remove broken connection
                conns.remove(&addr);
                Err(e)
            }
        }
    }

    async fn recv_from(&self, buf: &mut [u8]) -> TransportResult<(usize, SocketAddr)> {
        if self.closed.load(Ordering::Relaxed) {
            return Err(TransportError::Closed);
        }

        // Check recv queue first
        {
            let mut queue = self.recv_queue.lock().await;
            if let Some((data, addr)) = queue.pop() {
                let len = data.len().min(buf.len());
                buf[..len].copy_from_slice(&data[..len]);
                self.bytes_received.fetch_add(len as u64, Ordering::Relaxed);
                self.packets_received.fetch_add(1, Ordering::Relaxed);
                return Ok((len, addr));
            }
        }

        // Accept a new connection and read from it
        let (mut stream, peer_addr) = self.listener.accept().await.map_err(TransportError::Io)?;

        match Self::read_framed(&mut stream).await {
            Ok(data) => {
                let len = data.len().min(buf.len());
                buf[..len].copy_from_slice(&data[..len]);
                self.bytes_received.fetch_add(len as u64, Ordering::Relaxed);
                self.packets_received.fetch_add(1, Ordering::Relaxed);
                // Store the accepted connection for future use
                self.connections.lock().await.insert(peer_addr, stream);
                Ok((len, peer_addr))
            }
            Err(e) => {
                self.recv_errors.fetch_add(1, Ordering::Relaxed);
                Err(e)
            }
        }
    }

    fn local_addr(&self) -> TransportResult<SocketAddr> {
        Ok(self.local_addr)
    }

    async fn close(&self) -> TransportResult<()> {
        self.closed.store(true, Ordering::Relaxed);
        // Shutdown all connections
        let mut conns = self.connections.lock().await;
        for (_, mut stream) in conns.drain() {
            let _ = stream.shutdown().await;
        }
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
        TransportType::Tcp
    }

    fn mtu(&self) -> usize {
        // TCP handles segmentation, so we can use a large logical MTU
        65535
    }

    fn latency_estimate(&self) -> Duration {
        // TCP has higher latency due to connection setup and acknowledgments
        Duration::from_millis(1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::timeout;

    #[tokio::test]
    async fn test_tcp_bind() {
        let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
        let transport = TcpTransport::bind(addr).await.unwrap();
        let bound_addr = transport.local_addr().unwrap();
        assert_ne!(bound_addr.port(), 0);
        assert!(bound_addr.is_ipv4());
    }

    #[tokio::test]
    async fn test_tcp_close() {
        let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
        let transport = TcpTransport::bind(addr).await.unwrap();
        assert!(!transport.is_closed());
        transport.close().await.unwrap();
        assert!(transport.is_closed());

        let result = transport
            .send_to(b"test", "127.0.0.1:1234".parse().unwrap())
            .await;
        assert!(matches!(result, Err(TransportError::Closed)));
    }

    #[tokio::test]
    async fn test_tcp_transport_type() {
        let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
        let transport = TcpTransport::bind(addr).await.unwrap();
        assert_eq!(transport.transport_type(), TransportType::Tcp);
    }

    #[tokio::test]
    async fn test_tcp_mtu() {
        let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
        let transport = TcpTransport::bind(addr).await.unwrap();
        assert_eq!(transport.mtu(), 65535);
    }

    #[tokio::test]
    async fn test_tcp_send_recv() {
        let server_addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
        let server = TcpTransport::bind(server_addr).await.unwrap();
        let server_addr = server.local_addr().unwrap();

        let client_addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
        let client = TcpTransport::bind(client_addr).await.unwrap();

        let server = Arc::new(server);
        let server_clone = server.clone();

        // Spawn receiver
        let recv_handle = tokio::spawn(async move {
            let mut buf = vec![0u8; 1500];
            let (size, _from) = timeout(Duration::from_secs(2), server_clone.recv_from(&mut buf))
                .await
                .expect("Timeout")
                .unwrap();
            buf.truncate(size);
            buf
        });

        // Small delay for listener to be ready
        tokio::time::sleep(Duration::from_millis(50)).await;

        // Send data
        let sent = client.send_to(b"Hello TCP!", server_addr).await.unwrap();
        assert_eq!(sent, 10);

        // Verify received data
        let received = recv_handle.await.unwrap();
        assert_eq!(&received, b"Hello TCP!");
    }

    #[tokio::test]
    async fn test_tcp_large_message() {
        let server_addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
        let server = TcpTransport::bind(server_addr).await.unwrap();
        let server_addr = server.local_addr().unwrap();

        let client_addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
        let client = TcpTransport::bind(client_addr).await.unwrap();

        let server = Arc::new(server);
        let server_clone = server.clone();

        // Large message (64 KiB)
        let large_data = vec![0xBB; 65536];
        let expected = large_data.clone();

        let recv_handle = tokio::spawn(async move {
            let mut buf = vec![0u8; 70000];
            let (size, _from) = timeout(Duration::from_secs(2), server_clone.recv_from(&mut buf))
                .await
                .expect("Timeout")
                .unwrap();
            buf.truncate(size);
            buf
        });

        tokio::time::sleep(Duration::from_millis(50)).await;

        let sent = client.send_to(&large_data, server_addr).await.unwrap();
        assert_eq!(sent, 65536);

        let received = recv_handle.await.unwrap();
        assert_eq!(received, expected);
    }

    #[tokio::test]
    async fn test_tcp_stats() {
        let server_addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
        let server = TcpTransport::bind(server_addr).await.unwrap();
        let server_addr = server.local_addr().unwrap();

        let client_addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
        let client = TcpTransport::bind(client_addr).await.unwrap();

        let server = Arc::new(server);
        let server_clone = server.clone();

        let recv_handle = tokio::spawn(async move {
            let mut buf = vec![0u8; 1500];
            timeout(Duration::from_secs(2), server_clone.recv_from(&mut buf))
                .await
                .expect("Timeout")
                .unwrap()
        });

        tokio::time::sleep(Duration::from_millis(50)).await;

        client.send_to(b"stats test", server_addr).await.unwrap();
        recv_handle.await.unwrap();

        let client_stats = client.stats();
        assert_eq!(client_stats.packets_sent, 1);
        assert_eq!(client_stats.bytes_sent, 10);

        let server_stats = server.stats();
        assert_eq!(server_stats.packets_received, 1);
        assert_eq!(server_stats.bytes_received, 10);
    }

    #[tokio::test]
    async fn test_tcp_supports_migration() {
        let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
        let transport = TcpTransport::bind(addr).await.unwrap();
        assert!(!transport.supports_migration());
    }

    #[tokio::test]
    async fn test_tcp_recv_after_close() {
        let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
        let transport = TcpTransport::bind(addr).await.unwrap();
        transport.close().await.unwrap();

        let mut buf = vec![0u8; 1500];
        let result = transport.recv_from(&mut buf).await;
        assert!(matches!(result, Err(TransportError::Closed)));
    }
}
