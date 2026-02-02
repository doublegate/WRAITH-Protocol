//! WebSocket transport implementation for HTTP proxy traversal.
//!
//! This module provides a WebSocket-based transport using `tokio-tungstenite`.
//! WebSocket is useful for traversing HTTP proxies and restrictive firewalls
//! that only allow HTTP/HTTPS traffic. Messages are sent as binary WebSocket frames.
//!
//! # Architecture
//!
//! The transport operates in listener mode, accepting WebSocket upgrade requests
//! from incoming TCP connections. For outbound communication, it establishes
//! WebSocket connections to peer addresses on demand.

use crate::factory::TransportType;
use crate::transport::{Transport, TransportError, TransportResult, TransportStats};
use async_trait::async_trait;
use futures_util::{SinkExt, StreamExt};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::Duration;
use tokio::net::TcpListener;
use tokio::sync::Mutex;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream, accept_async, connect_async};

/// Type alias for a server-side WebSocket stream.
type ServerWsStream = WebSocketStream<tokio::net::TcpStream>;

/// Type alias for a client-side WebSocket stream.
type ClientWsStream = WebSocketStream<MaybeTlsStream<tokio::net::TcpStream>>;

/// Wrapper enum to unify server and client WebSocket streams.
enum WsStream {
    /// Server-side accepted stream.
    Server(ServerWsStream),
    /// Client-side connected stream.
    Client(ClientWsStream),
}

impl WsStream {
    /// Send a binary message.
    async fn send_binary(&mut self, data: Vec<u8>) -> Result<(), TransportError> {
        let msg = Message::Binary(data);
        match self {
            Self::Server(s) => s.send(msg).await,
            Self::Client(s) => s.send(msg).await,
        }
        .map_err(|e| TransportError::Other(format!("WebSocket send error: {e}")))
    }

    /// Receive the next binary message.
    async fn recv_binary(&mut self) -> Result<Vec<u8>, TransportError> {
        loop {
            let msg = match self {
                Self::Server(s) => s.next().await,
                Self::Client(s) => s.next().await,
            };
            match msg {
                Some(Ok(Message::Binary(data))) => return Ok(data),
                Some(Ok(Message::Close(_))) | None => {
                    return Err(TransportError::Closed);
                }
                Some(Ok(_)) => continue, // Skip non-binary messages (ping/pong/text)
                Some(Err(e)) => {
                    return Err(TransportError::Other(format!("WebSocket recv error: {e}")));
                }
            }
        }
    }

    /// Close the stream.
    async fn close_stream(&mut self) -> Result<(), TransportError> {
        match self {
            Self::Server(s) => s.close(None).await,
            Self::Client(s) => s.close(None).await,
        }
        .map_err(|e| TransportError::Other(format!("WebSocket close error: {e}")))
    }
}

/// WebSocket transport for HTTP proxy traversal.
///
/// This transport sends and receives data as binary WebSocket frames,
/// making it suitable for environments where only HTTP/HTTPS traffic is allowed.
///
/// # Examples
///
/// ```no_run
/// use wraith_transport::websocket::WebSocketTransport;
/// use wraith_transport::transport::Transport;
/// use std::net::SocketAddr;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let addr: SocketAddr = "127.0.0.1:0".parse()?;
/// let transport = WebSocketTransport::bind(addr).await?;
/// println!("WebSocket transport on {}", transport.local_addr()?);
/// # Ok(())
/// # }
/// ```
pub struct WebSocketTransport {
    listener: Arc<TcpListener>,
    local_addr: SocketAddr,
    closed: Arc<AtomicBool>,
    /// Outbound WebSocket connections keyed by peer address.
    connections: Arc<Mutex<HashMap<SocketAddr, WsStream>>>,
    bytes_sent: Arc<AtomicU64>,
    bytes_received: Arc<AtomicU64>,
    packets_sent: Arc<AtomicU64>,
    packets_received: Arc<AtomicU64>,
    send_errors: Arc<AtomicU64>,
    recv_errors: Arc<AtomicU64>,
}

impl WebSocketTransport {
    /// Create a new WebSocket transport bound to the given address.
    ///
    /// # Arguments
    /// * `addr` - The local address to bind to. Use port 0 for OS-assigned port.
    ///
    /// # Errors
    /// Returns `TransportError::BindFailed` if the listener cannot be created.
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
            bytes_sent: Arc::new(AtomicU64::new(0)),
            bytes_received: Arc::new(AtomicU64::new(0)),
            packets_sent: Arc::new(AtomicU64::new(0)),
            packets_received: Arc::new(AtomicU64::new(0)),
            send_errors: Arc::new(AtomicU64::new(0)),
            recv_errors: Arc::new(AtomicU64::new(0)),
        })
    }
}

#[async_trait]
impl Transport for WebSocketTransport {
    async fn send_to(&self, buf: &[u8], addr: SocketAddr) -> TransportResult<usize> {
        if self.closed.load(Ordering::Relaxed) {
            return Err(TransportError::Closed);
        }

        let mut conns = self.connections.lock().await;

        // Connect if not already connected
        if let std::collections::hash_map::Entry::Vacant(e) = conns.entry(addr) {
            let url = format!("ws://{addr}");
            let (ws_stream, _) = connect_async(&url)
                .await
                .map_err(|e| TransportError::ConnectionFailed(format!("WebSocket connect: {e}")))?;
            e.insert(WsStream::Client(ws_stream));
        }

        let ws = conns
            .get_mut(&addr)
            .ok_or_else(|| TransportError::ConnectionFailed("Connection not found".to_string()))?;

        let len = buf.len();
        match ws.send_binary(buf.to_vec()).await {
            Ok(()) => {
                self.bytes_sent.fetch_add(len as u64, Ordering::Relaxed);
                self.packets_sent.fetch_add(1, Ordering::Relaxed);
                Ok(len)
            }
            Err(e) => {
                self.send_errors.fetch_add(1, Ordering::Relaxed);
                conns.remove(&addr);
                Err(e)
            }
        }
    }

    async fn recv_from(&self, buf: &mut [u8]) -> TransportResult<(usize, SocketAddr)> {
        if self.closed.load(Ordering::Relaxed) {
            return Err(TransportError::Closed);
        }

        // Accept a new connection and perform WebSocket handshake
        let (tcp_stream, peer_addr) = self.listener.accept().await.map_err(TransportError::Io)?;

        let ws_stream = accept_async(tcp_stream)
            .await
            .map_err(|e| TransportError::Other(format!("WebSocket accept error: {e}")))?;

        let mut ws = WsStream::Server(ws_stream);

        match ws.recv_binary().await {
            Ok(data) => {
                let len = data.len().min(buf.len());
                buf[..len].copy_from_slice(&data[..len]);
                self.bytes_received.fetch_add(len as u64, Ordering::Relaxed);
                self.packets_received.fetch_add(1, Ordering::Relaxed);
                // Store connection
                self.connections.lock().await.insert(peer_addr, ws);
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
        let mut conns = self.connections.lock().await;
        for (_, mut ws) in conns.drain() {
            let _ = ws.close_stream().await;
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
        TransportType::WebSocket
    }

    fn mtu(&self) -> usize {
        // WebSocket frames can be large
        65535
    }

    fn latency_estimate(&self) -> Duration {
        // WebSocket has higher latency due to HTTP upgrade and framing overhead
        Duration::from_millis(5)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::timeout;

    #[tokio::test]
    async fn test_ws_bind() {
        let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
        let transport = WebSocketTransport::bind(addr).await.unwrap();
        let bound_addr = transport.local_addr().unwrap();
        assert_ne!(bound_addr.port(), 0);
        assert!(bound_addr.is_ipv4());
    }

    #[tokio::test]
    async fn test_ws_close() {
        let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
        let transport = WebSocketTransport::bind(addr).await.unwrap();
        assert!(!transport.is_closed());
        transport.close().await.unwrap();
        assert!(transport.is_closed());

        let result = transport
            .send_to(b"test", "127.0.0.1:1234".parse().unwrap())
            .await;
        assert!(matches!(result, Err(TransportError::Closed)));
    }

    #[tokio::test]
    async fn test_ws_transport_type() {
        let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
        let transport = WebSocketTransport::bind(addr).await.unwrap();
        assert_eq!(transport.transport_type(), TransportType::WebSocket);
    }

    #[tokio::test]
    async fn test_ws_send_recv() {
        let server_addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
        let server = WebSocketTransport::bind(server_addr).await.unwrap();
        let server_addr = server.local_addr().unwrap();

        let client_addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
        let client = WebSocketTransport::bind(client_addr).await.unwrap();

        let server = Arc::new(server);
        let server_clone = server.clone();

        let recv_handle = tokio::spawn(async move {
            let mut buf = vec![0u8; 1500];
            let (size, _from) = timeout(Duration::from_secs(2), server_clone.recv_from(&mut buf))
                .await
                .expect("Timeout")
                .unwrap();
            buf.truncate(size);
            buf
        });

        tokio::time::sleep(Duration::from_millis(50)).await;

        let sent = client.send_to(b"Hello WS!", server_addr).await.unwrap();
        assert_eq!(sent, 9);

        let received = recv_handle.await.unwrap();
        assert_eq!(&received, b"Hello WS!");
    }

    #[tokio::test]
    async fn test_ws_stats() {
        let server_addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
        let server = WebSocketTransport::bind(server_addr).await.unwrap();
        let server_addr = server.local_addr().unwrap();

        let client_addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
        let client = WebSocketTransport::bind(client_addr).await.unwrap();

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

        client.send_to(b"stat", server_addr).await.unwrap();
        recv_handle.await.unwrap();

        let client_stats = client.stats();
        assert_eq!(client_stats.packets_sent, 1);
        assert_eq!(client_stats.bytes_sent, 4);
    }

    #[tokio::test]
    async fn test_ws_recv_after_close() {
        let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
        let transport = WebSocketTransport::bind(addr).await.unwrap();
        transport.close().await.unwrap();

        let mut buf = vec![0u8; 1500];
        let result = transport.recv_from(&mut buf).await;
        assert!(matches!(result, Err(TransportError::Closed)));
    }

    #[tokio::test]
    async fn test_ws_supports_migration() {
        let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
        let transport = WebSocketTransport::bind(addr).await.unwrap();
        assert!(!transport.supports_migration());
    }
}
