//! Relay server for forwarding packets between peers.

use super::protocol::{NodeId, RelayError, RelayErrorCode, RelayMessage};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::net::UdpSocket;
use tokio::sync::RwLock;

/// Client connection information
#[derive(Debug, Clone)]
struct ClientConnection {
    /// Client's socket address
    addr: SocketAddr,
    /// Client's public key (reserved for future authentication)
    #[allow(dead_code)]
    public_key: [u8; 32],
    /// Last seen time
    last_seen: Instant,
}

impl ClientConnection {
    /// Create a new client connection
    fn new(addr: SocketAddr, public_key: [u8; 32]) -> Self {
        Self {
            addr,
            public_key,
            last_seen: Instant::now(),
        }
    }

    /// Update last seen time
    fn touch(&mut self) {
        self.last_seen = Instant::now();
    }

    /// Check if connection is alive
    fn is_alive(&self, timeout: Duration) -> bool {
        self.last_seen.elapsed() < timeout
    }
}

/// Simple rate limiter
struct RateLimiter {
    /// Packets per client per second
    limit: usize,
    /// Packet counts for each client
    counts: HashMap<NodeId, (Instant, usize)>,
    /// Window duration
    window: Duration,
}

impl RateLimiter {
    /// Create a new rate limiter
    fn new(limit: usize, window: Duration) -> Self {
        Self {
            limit,
            counts: HashMap::new(),
            window,
        }
    }

    /// Check if request is allowed
    fn check(&mut self, node_id: NodeId) -> bool {
        let now = Instant::now();

        let entry = self.counts.entry(node_id).or_insert((now, 0));

        // Reset counter if window expired
        if now.duration_since(entry.0) >= self.window {
            entry.0 = now;
            entry.1 = 0;
        }

        // Check limit
        if entry.1 >= self.limit {
            return false;
        }

        entry.1 += 1;
        true
    }

    /// Clean up expired entries
    fn cleanup(&mut self) {
        let now = Instant::now();
        self.counts
            .retain(|_, (time, _)| now.duration_since(*time) < self.window * 2);
    }
}

/// Relay server configuration
#[derive(Debug, Clone)]
pub struct RelayServerConfig {
    /// Maximum number of concurrent clients
    pub max_clients: usize,
    /// Rate limit (packets per client per second)
    pub rate_limit: usize,
    /// Client timeout duration
    pub client_timeout: Duration,
    /// Cleanup interval
    pub cleanup_interval: Duration,
}

impl Default for RelayServerConfig {
    fn default() -> Self {
        Self {
            max_clients: 10_000,
            rate_limit: 100,
            client_timeout: Duration::from_secs(60),
            cleanup_interval: Duration::from_secs(30),
        }
    }
}

/// DERP-style relay server
pub struct RelayServer {
    /// Bind address
    bind_addr: SocketAddr,
    /// Registered clients (NodeId -> ClientConnection)
    clients: Arc<RwLock<HashMap<NodeId, ClientConnection>>>,
    /// UDP socket
    socket: Arc<UdpSocket>,
    /// Rate limiter
    rate_limiter: Arc<RwLock<RateLimiter>>,
    /// Server configuration
    config: RelayServerConfig,
    /// Server relay ID
    relay_id: [u8; 32],
}

impl RelayServer {
    /// Create a new relay server
    ///
    /// # Arguments
    ///
    /// * `bind_addr` - Address to bind the server to
    ///
    /// # Errors
    ///
    /// Returns error if socket binding fails.
    pub async fn bind(bind_addr: SocketAddr) -> Result<Self, RelayError> {
        Self::bind_with_config(bind_addr, RelayServerConfig::default()).await
    }

    /// Create a new relay server with custom configuration
    ///
    /// # Arguments
    ///
    /// * `bind_addr` - Address to bind the server to
    /// * `config` - Server configuration
    ///
    /// # Errors
    ///
    /// Returns error if socket binding fails.
    pub async fn bind_with_config(
        bind_addr: SocketAddr,
        config: RelayServerConfig,
    ) -> Result<Self, RelayError> {
        let socket = UdpSocket::bind(bind_addr).await?;

        // Generate random relay ID
        let relay_id = {
            let mut id = [0u8; 32];
            use rand::Rng;
            rand::thread_rng().fill(&mut id[..]);
            id
        };

        Ok(Self {
            bind_addr,
            clients: Arc::new(RwLock::new(HashMap::new())),
            socket: Arc::new(socket),
            rate_limiter: Arc::new(RwLock::new(RateLimiter::new(
                config.rate_limit,
                Duration::from_secs(1),
            ))),
            config,
            relay_id,
        })
    }

    /// Run the relay server
    ///
    /// This is the main server loop that processes incoming messages.
    ///
    /// # Errors
    ///
    /// Returns error if socket operations fail.
    pub async fn run(&self) -> Result<(), RelayError> {
        println!(
            "Relay server listening on {} (ID: {:?})",
            self.bind_addr,
            &self.relay_id[..8]
        );

        // Spawn cleanup task
        self.spawn_cleanup_task();

        let mut buf = vec![0u8; 65536];

        loop {
            match self.socket.recv_from(&mut buf).await {
                Ok((len, from)) => {
                    let packet = &buf[..len];

                    if let Ok(msg) = RelayMessage::from_bytes(packet) {
                        self.handle_message(msg, from).await;
                    }
                }
                Err(e) => {
                    eprintln!("Receive error: {e}");
                }
            }
        }
    }

    /// Handle incoming relay message
    async fn handle_message(&self, msg: RelayMessage, from: SocketAddr) {
        match msg {
            RelayMessage::Register {
                node_id,
                public_key,
            } => {
                self.handle_register(node_id, public_key, from).await;
            }
            RelayMessage::SendPacket { dest_id, payload } => {
                // Extract sender's node_id by reverse lookup
                if let Some(sender_id) = self.find_node_id_by_addr(from).await {
                    self.handle_send_packet(sender_id, dest_id, payload, from)
                        .await;
                } else {
                    self.send_error(from, RelayErrorCode::NotRegistered, "Not registered")
                        .await;
                }
            }
            RelayMessage::Keepalive => {
                if let Some(node_id) = self.find_node_id_by_addr(from).await {
                    let mut clients = self.clients.write().await;
                    if let Some(client) = clients.get_mut(&node_id) {
                        client.touch();
                    }
                }
            }
            RelayMessage::Disconnect => {
                if let Some(node_id) = self.find_node_id_by_addr(from).await {
                    let mut clients = self.clients.write().await;
                    clients.remove(&node_id);
                }
            }
            _ => {
                // Ignore other message types
            }
        }
    }

    /// Handle client registration
    async fn handle_register(&self, node_id: NodeId, public_key: [u8; 32], from: SocketAddr) {
        let mut clients = self.clients.write().await;

        // Check if server is full
        if clients.len() >= self.config.max_clients && !clients.contains_key(&node_id) {
            drop(clients);
            self.send_error(from, RelayErrorCode::ServerFull, "Server at capacity")
                .await;
            return;
        }

        // Register or update client
        clients.insert(node_id, ClientConnection::new(from, public_key));

        drop(clients);

        // Send acknowledgment
        let ack = RelayMessage::RegisterAck {
            relay_id: self.relay_id,
            success: true,
            error: None,
        };

        if let Ok(bytes) = ack.to_bytes() {
            let _ = self.socket.send_to(&bytes, from).await;
        }
    }

    /// Handle packet forwarding
    async fn handle_send_packet(
        &self,
        src_id: NodeId,
        dest_id: NodeId,
        payload: Vec<u8>,
        from: SocketAddr,
    ) {
        // Check rate limit
        {
            let mut limiter = self.rate_limiter.write().await;
            if !limiter.check(src_id) {
                drop(limiter);
                self.send_error(from, RelayErrorCode::RateLimited, "Rate limit exceeded")
                    .await;
                return;
            }
        }

        // Find destination client
        let clients = self.clients.read().await;
        if let Some(dest_client) = clients.get(&dest_id) {
            let dest_addr = dest_client.addr;
            drop(clients);

            // Forward packet
            let forward = RelayMessage::RecvPacket { src_id, payload };

            if let Ok(bytes) = forward.to_bytes() {
                let _ = self.socket.send_to(&bytes, dest_addr).await;
            }
        } else {
            drop(clients);
            self.send_error(from, RelayErrorCode::PeerNotFound, "Peer not found")
                .await;
        }
    }

    /// Send error message to client
    async fn send_error(&self, addr: SocketAddr, code: RelayErrorCode, message: &str) {
        let error = RelayMessage::Error {
            code,
            message: message.to_string(),
        };

        if let Ok(bytes) = error.to_bytes() {
            let _ = self.socket.send_to(&bytes, addr).await;
        }
    }

    /// Find node ID by socket address
    async fn find_node_id_by_addr(&self, addr: SocketAddr) -> Option<NodeId> {
        let clients = self.clients.read().await;
        for (node_id, client) in clients.iter() {
            if client.addr == addr {
                return Some(*node_id);
            }
        }
        None
    }

    /// Spawn cleanup task to remove stale clients
    fn spawn_cleanup_task(&self) {
        let clients = self.clients.clone();
        let rate_limiter = self.rate_limiter.clone();
        let timeout = self.config.client_timeout;
        let interval = self.config.cleanup_interval;

        tokio::spawn(async move {
            let mut ticker = tokio::time::interval(interval);

            loop {
                ticker.tick().await;

                // Clean up stale clients
                {
                    let mut clients_guard = clients.write().await;
                    clients_guard.retain(|_, client| client.is_alive(timeout));
                }

                // Clean up rate limiter
                {
                    let mut limiter = rate_limiter.write().await;
                    limiter.cleanup();
                }
            }
        });
    }

    /// Get number of connected clients
    pub async fn client_count(&self) -> usize {
        self.clients.read().await.len()
    }

    /// Get server relay ID
    #[must_use]
    pub fn relay_id(&self) -> [u8; 32] {
        self.relay_id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_relay_server_creation() {
        let addr = "127.0.0.1:0".parse().unwrap();
        let server = RelayServer::bind(addr).await;
        assert!(server.is_ok());
    }

    #[tokio::test]
    async fn test_relay_server_client_count() {
        let addr = "127.0.0.1:0".parse().unwrap();
        let server = RelayServer::bind(addr).await.unwrap();
        assert_eq!(server.client_count().await, 0);
    }

    #[tokio::test]
    async fn test_relay_server_config_default() {
        let config = RelayServerConfig::default();
        assert_eq!(config.max_clients, 10_000);
        assert_eq!(config.rate_limit, 100);
    }

    #[test]
    fn test_client_connection() {
        let addr = "127.0.0.1:8000".parse().unwrap();
        let public_key = [1u8; 32];
        let mut conn = ClientConnection::new(addr, public_key);

        assert!(conn.is_alive(Duration::from_secs(60)));

        conn.touch();
        assert!(conn.is_alive(Duration::from_secs(60)));
    }

    #[test]
    fn test_rate_limiter() {
        let mut limiter = RateLimiter::new(3, Duration::from_secs(1));
        let node_id = [1u8; 32];

        assert!(limiter.check(node_id));
        assert!(limiter.check(node_id));
        assert!(limiter.check(node_id));
        assert!(!limiter.check(node_id)); // Should be rate limited
    }

    #[test]
    fn test_rate_limiter_multiple_nodes() {
        let mut limiter = RateLimiter::new(2, Duration::from_secs(1));
        let node_a = [1u8; 32];
        let node_b = [2u8; 32];

        assert!(limiter.check(node_a));
        assert!(limiter.check(node_a));
        assert!(!limiter.check(node_a)); // a is limited

        assert!(limiter.check(node_b)); // b is independent
        assert!(limiter.check(node_b));
        assert!(!limiter.check(node_b)); // b is limited
    }

    #[test]
    fn test_client_connection_not_alive() {
        let addr = "127.0.0.1:8000".parse().unwrap();
        let conn = ClientConnection::new(addr, [1u8; 32]);
        // With a zero timeout, connection should not be alive
        assert!(!conn.is_alive(Duration::from_secs(0)));
    }

    #[test]
    fn test_client_connection_debug() {
        let addr = "127.0.0.1:8000".parse().unwrap();
        let conn = ClientConnection::new(addr, [1u8; 32]);
        let debug = format!("{:?}", conn);
        assert!(debug.contains("ClientConnection"));
    }

    #[tokio::test]
    async fn test_relay_server_with_config() {
        let config = RelayServerConfig {
            max_clients: 5,
            rate_limit: 10,
            client_timeout: Duration::from_secs(30),
            cleanup_interval: Duration::from_secs(15),
        };
        let addr = "127.0.0.1:0".parse().unwrap();
        let server = RelayServer::bind_with_config(addr, config).await;
        assert!(server.is_ok());
        let server = server.unwrap();
        assert_eq!(server.client_count().await, 0);
    }

    #[tokio::test]
    async fn test_relay_server_relay_id() {
        let addr = "127.0.0.1:0".parse().unwrap();
        let server = RelayServer::bind(addr).await.unwrap();
        let relay_id = server.relay_id();
        // Relay ID should not be all zeros (randomly generated)
        assert_ne!(relay_id, [0u8; 32]);
    }

    #[tokio::test]
    async fn test_relay_server_handle_register() {
        let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
        let server = RelayServer::bind(addr).await.unwrap();
        let server_addr = server.socket.local_addr().unwrap();

        // Send a register message from a client
        let client_socket = tokio::net::UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let register_msg = RelayMessage::Register {
            node_id: [1u8; 32],
            public_key: [2u8; 32],
        };
        let bytes = register_msg.to_bytes().unwrap();
        client_socket.send_to(&bytes, server_addr).await.unwrap();

        // Handle the message directly
        let from = client_socket.local_addr().unwrap();
        server
            .handle_message(
                RelayMessage::Register {
                    node_id: [1u8; 32],
                    public_key: [2u8; 32],
                },
                from,
            )
            .await;

        assert_eq!(server.client_count().await, 1);
    }

    #[tokio::test]
    async fn test_relay_server_handle_register_full() {
        let config = RelayServerConfig {
            max_clients: 1,
            rate_limit: 100,
            client_timeout: Duration::from_secs(60),
            cleanup_interval: Duration::from_secs(30),
        };
        let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
        let server = RelayServer::bind_with_config(addr, config).await.unwrap();

        // Register first client
        let from1: SocketAddr = "127.0.0.1:10001".parse().unwrap();
        server
            .handle_message(
                RelayMessage::Register {
                    node_id: [1u8; 32],
                    public_key: [2u8; 32],
                },
                from1,
            )
            .await;
        assert_eq!(server.client_count().await, 1);

        // Second client should be rejected (server full)
        let from2: SocketAddr = "127.0.0.1:10002".parse().unwrap();
        server
            .handle_message(
                RelayMessage::Register {
                    node_id: [3u8; 32],
                    public_key: [4u8; 32],
                },
                from2,
            )
            .await;
        // Still only 1 client
        assert_eq!(server.client_count().await, 1);
    }

    #[tokio::test]
    async fn test_relay_server_handle_keepalive() {
        let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
        let server = RelayServer::bind(addr).await.unwrap();

        // Register a client first
        let from: SocketAddr = "127.0.0.1:10001".parse().unwrap();
        server
            .handle_message(
                RelayMessage::Register {
                    node_id: [1u8; 32],
                    public_key: [2u8; 32],
                },
                from,
            )
            .await;

        // Send keepalive
        server.handle_message(RelayMessage::Keepalive, from).await;
        assert_eq!(server.client_count().await, 1);
    }

    #[tokio::test]
    async fn test_relay_server_handle_disconnect() {
        let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
        let server = RelayServer::bind(addr).await.unwrap();

        let from: SocketAddr = "127.0.0.1:10001".parse().unwrap();
        server
            .handle_message(
                RelayMessage::Register {
                    node_id: [1u8; 32],
                    public_key: [2u8; 32],
                },
                from,
            )
            .await;
        assert_eq!(server.client_count().await, 1);

        server.handle_message(RelayMessage::Disconnect, from).await;
        assert_eq!(server.client_count().await, 0);
    }

    #[tokio::test]
    async fn test_relay_server_handle_send_not_registered() {
        let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
        let server = RelayServer::bind(addr).await.unwrap();

        // Try sending from unregistered address
        let from: SocketAddr = "127.0.0.1:10001".parse().unwrap();
        server
            .handle_message(
                RelayMessage::SendPacket {
                    dest_id: [2u8; 32],
                    payload: vec![1, 2, 3],
                },
                from,
            )
            .await;
        // No crash, error is sent to client
    }

    #[tokio::test]
    async fn test_relay_server_handle_send_peer_not_found() {
        let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
        let server = RelayServer::bind(addr).await.unwrap();

        // Register sender
        let from: SocketAddr = "127.0.0.1:10001".parse().unwrap();
        server
            .handle_message(
                RelayMessage::Register {
                    node_id: [1u8; 32],
                    public_key: [2u8; 32],
                },
                from,
            )
            .await;

        // Send to non-existent peer
        server
            .handle_message(
                RelayMessage::SendPacket {
                    dest_id: [99u8; 32],
                    payload: vec![1, 2, 3],
                },
                from,
            )
            .await;
        // No crash
    }

    #[tokio::test]
    async fn test_relay_server_handle_send_forward() {
        let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
        let server = RelayServer::bind(addr).await.unwrap();

        // Register sender
        let from_a: SocketAddr = "127.0.0.1:10001".parse().unwrap();
        server
            .handle_message(
                RelayMessage::Register {
                    node_id: [1u8; 32],
                    public_key: [2u8; 32],
                },
                from_a,
            )
            .await;

        // Register destination
        let from_b: SocketAddr = "127.0.0.1:10002".parse().unwrap();
        server
            .handle_message(
                RelayMessage::Register {
                    node_id: [3u8; 32],
                    public_key: [4u8; 32],
                },
                from_b,
            )
            .await;

        // Send from A to B
        server
            .handle_message(
                RelayMessage::SendPacket {
                    dest_id: [3u8; 32],
                    payload: vec![1, 2, 3],
                },
                from_a,
            )
            .await;
        // No crash, packet forwarded
    }

    #[tokio::test]
    async fn test_relay_server_handle_unknown_message() {
        let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
        let server = RelayServer::bind(addr).await.unwrap();

        let from: SocketAddr = "127.0.0.1:10001".parse().unwrap();
        // Send an unexpected message type
        server
            .handle_message(RelayMessage::PeerOnline { peer_id: [1u8; 32] }, from)
            .await;
        // Should be ignored, no crash
    }

    #[tokio::test]
    async fn test_relay_server_keepalive_unregistered() {
        let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
        let server = RelayServer::bind(addr).await.unwrap();

        let from: SocketAddr = "127.0.0.1:10001".parse().unwrap();
        // Keepalive from unregistered client should not crash
        server.handle_message(RelayMessage::Keepalive, from).await;
    }

    #[tokio::test]
    async fn test_relay_server_disconnect_unregistered() {
        let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
        let server = RelayServer::bind(addr).await.unwrap();

        let from: SocketAddr = "127.0.0.1:10001".parse().unwrap();
        server.handle_message(RelayMessage::Disconnect, from).await;
        // No crash
    }

    #[tokio::test]
    async fn test_relay_server_re_register() {
        let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
        let server = RelayServer::bind(addr).await.unwrap();

        let from: SocketAddr = "127.0.0.1:10001".parse().unwrap();
        server
            .handle_message(
                RelayMessage::Register {
                    node_id: [1u8; 32],
                    public_key: [2u8; 32],
                },
                from,
            )
            .await;
        assert_eq!(server.client_count().await, 1);

        // Re-register same node (should update, not add)
        server
            .handle_message(
                RelayMessage::Register {
                    node_id: [1u8; 32],
                    public_key: [3u8; 32],
                },
                from,
            )
            .await;
        assert_eq!(server.client_count().await, 1);
    }

    #[test]
    fn test_relay_server_config_debug() {
        let config = RelayServerConfig::default();
        let debug = format!("{:?}", config);
        assert!(debug.contains("max_clients"));
    }

    #[test]
    fn test_relay_server_config_clone() {
        let config = RelayServerConfig::default();
        let cloned = config.clone();
        assert_eq!(cloned.max_clients, config.max_clients);
    }

    #[test]
    fn test_rate_limiter_cleanup() {
        let mut limiter = RateLimiter::new(10, Duration::from_millis(100));
        let node_id = [1u8; 32];

        limiter.check(node_id);
        assert_eq!(limiter.counts.len(), 1);

        limiter.cleanup();
        assert_eq!(limiter.counts.len(), 1);
    }
}
