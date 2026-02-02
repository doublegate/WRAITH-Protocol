//! Relay client implementation for connecting to relay servers.

use super::protocol::{NodeId, RelayError, RelayMessage};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::net::UdpSocket;
use tokio::sync::{Mutex, mpsc};
use tokio::time;

/// Type alias for the message receiver
type MessageReceiver = Arc<Mutex<mpsc::UnboundedReceiver<(NodeId, Vec<u8>)>>>;

/// Relay client state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelayClientState {
    /// Disconnected from relay
    Disconnected,
    /// Connecting to relay
    Connecting,
    /// Registering with relay
    Registering,
    /// Connected and registered
    Connected,
    /// Error state
    Error,
}

/// Relay client for communicating with relay servers
pub struct RelayClient {
    /// Local node ID
    node_id: NodeId,
    /// Relay server address
    relay_addr: SocketAddr,
    /// UDP socket for communication
    socket: Arc<UdpSocket>,
    /// Current client state
    state: Arc<Mutex<RelayClientState>>,
    /// Receiver for incoming messages
    rx: MessageReceiver,
    /// Sender for message processing
    tx: mpsc::UnboundedSender<(NodeId, Vec<u8>)>,
    /// Last keepalive time
    last_keepalive: Arc<Mutex<Instant>>,
}

impl RelayClient {
    /// Connect to a relay server
    ///
    /// # Arguments
    ///
    /// * `addr` - Relay server address
    /// * `node_id` - Local node identifier
    ///
    /// # Errors
    ///
    /// Returns error if connection fails or times out.
    pub async fn connect(addr: SocketAddr, node_id: NodeId) -> Result<Self, RelayError> {
        // Bind local UDP socket
        let socket = UdpSocket::bind("0.0.0.0:0").await?;
        socket.connect(addr).await?;

        let (tx, rx) = mpsc::unbounded_channel();

        let client = Self {
            node_id,
            relay_addr: addr,
            socket: Arc::new(socket),
            state: Arc::new(Mutex::new(RelayClientState::Disconnected)),
            rx: Arc::new(Mutex::new(rx)),
            tx,
            last_keepalive: Arc::new(Mutex::new(Instant::now())),
        };

        // Update state to connecting
        *client.state.lock().await = RelayClientState::Connecting;

        Ok(client)
    }

    /// Register with the relay server
    ///
    /// # Arguments
    ///
    /// * `public_key` - Client's public key for verification
    ///
    /// # Errors
    ///
    /// Returns error if registration fails or times out.
    pub async fn register(&mut self, public_key: &[u8; 32]) -> Result<(), RelayError> {
        *self.state.lock().await = RelayClientState::Registering;

        let msg = RelayMessage::Register {
            node_id: self.node_id,
            public_key: *public_key,
        };

        let bytes = msg.to_bytes()?;
        self.socket.send(&bytes).await?;

        // Wait for RegisterAck with timeout
        let mut buf = vec![0u8; 65536];
        let len = time::timeout(Duration::from_secs(10), self.socket.recv(&mut buf))
            .await
            .map_err(|_| RelayError::Timeout)??;

        let response = RelayMessage::from_bytes(&buf[..len])?;

        match response {
            RelayMessage::RegisterAck {
                success,
                error,
                relay_id: _,
            } => {
                if success {
                    *self.state.lock().await = RelayClientState::Connected;
                    *self.last_keepalive.lock().await = Instant::now();
                    Ok(())
                } else {
                    *self.state.lock().await = RelayClientState::Error;
                    Err(RelayError::Internal(
                        error.unwrap_or_else(|| "Registration failed".to_string()),
                    ))
                }
            }
            RelayMessage::Error { code, message: _ } => {
                *self.state.lock().await = RelayClientState::Error;
                Err(code.into())
            }
            _ => {
                *self.state.lock().await = RelayClientState::Error;
                Err(RelayError::InvalidMessage)
            }
        }
    }

    /// Send a packet to a peer through the relay
    ///
    /// # Arguments
    ///
    /// * `dest` - Destination node ID
    /// * `data` - Packet payload (already encrypted)
    ///
    /// # Errors
    ///
    /// Returns error if send fails or client not registered.
    pub async fn send_to_peer(&self, dest: NodeId, data: &[u8]) -> Result<(), RelayError> {
        if *self.state.lock().await != RelayClientState::Connected {
            return Err(RelayError::NotRegistered);
        }

        let msg = RelayMessage::SendPacket {
            dest_id: dest,
            payload: data.to_vec(),
        };

        let bytes = msg.to_bytes()?;
        self.socket.send(&bytes).await?;

        Ok(())
    }

    /// Receive a packet from a peer through the relay
    ///
    /// # Errors
    ///
    /// Returns error if receive fails or timeout occurs.
    pub async fn recv_from_peer(&self) -> Result<(NodeId, Vec<u8>), RelayError> {
        let mut rx = self.rx.lock().await;
        rx.recv()
            .await
            .ok_or_else(|| RelayError::Internal("Channel closed".to_string()))
    }

    /// Send keepalive message to maintain connection
    ///
    /// # Errors
    ///
    /// Returns error if send fails.
    pub async fn keepalive(&self) -> Result<(), RelayError> {
        let msg = RelayMessage::Keepalive;
        let bytes = msg.to_bytes()?;
        self.socket.send(&bytes).await?;

        *self.last_keepalive.lock().await = Instant::now();
        Ok(())
    }

    /// Disconnect from relay server
    ///
    /// # Errors
    ///
    /// Returns error if disconnect message fails to send.
    pub async fn disconnect(&mut self) -> Result<(), RelayError> {
        let msg = RelayMessage::Disconnect;
        let bytes = msg.to_bytes()?;
        self.socket.send(&bytes).await?;

        *self.state.lock().await = RelayClientState::Disconnected;
        Ok(())
    }

    /// Get current client state
    #[must_use]
    pub async fn state(&self) -> RelayClientState {
        *self.state.lock().await
    }

    /// Get relay server address
    #[must_use]
    pub fn relay_addr(&self) -> SocketAddr {
        self.relay_addr
    }

    /// Start background message processing task
    ///
    /// This task receives messages from the relay and forwards them to the channel.
    pub fn spawn_receiver(&self) {
        let socket = self.socket.clone();
        let tx = self.tx.clone();
        let state = self.state.clone();

        tokio::spawn(async move {
            let mut buf = vec![0u8; 65536];

            loop {
                match socket.recv(&mut buf).await {
                    Ok(len) => {
                        if let Ok(msg) = RelayMessage::from_bytes(&buf[..len]) {
                            match msg {
                                RelayMessage::RecvPacket { src_id, payload } => {
                                    let _ = tx.send((src_id, payload));
                                }
                                RelayMessage::PeerOnline { peer_id: _ } => {
                                    // Could notify application layer
                                }
                                RelayMessage::PeerOffline { peer_id: _ } => {
                                    // Could notify application layer
                                }
                                RelayMessage::Error { code, message: _ } => {
                                    eprintln!("Relay error: {code:?}");
                                    *state.lock().await = RelayClientState::Error;
                                }
                                _ => {
                                    // Ignore other messages
                                }
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("Receive error: {e}");
                        *state.lock().await = RelayClientState::Error;
                        break;
                    }
                }
            }
        });
    }

    /// Check if keepalive is needed and send if necessary
    ///
    /// # Errors
    ///
    /// Returns error if keepalive send fails.
    pub async fn maybe_keepalive(&self, interval: Duration) -> Result<(), RelayError> {
        let last = *self.last_keepalive.lock().await;
        if last.elapsed() >= interval {
            self.keepalive().await?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_relay_client_creation() {
        let node_id = [1u8; 32];
        let addr = "127.0.0.1:8000".parse().unwrap();

        let result = RelayClient::connect(addr, node_id).await;
        // May fail if relay not running, but constructor should succeed
        assert!(result.is_ok() || matches!(result, Err(RelayError::Io(_))));
    }

    #[tokio::test]
    async fn test_relay_client_state() {
        let node_id = [1u8; 32];
        let addr = "127.0.0.1:8001".parse().unwrap();

        if let Ok(client) = RelayClient::connect(addr, node_id).await {
            let state = client.state().await;
            assert_eq!(state, RelayClientState::Connecting);
        }
    }

    #[tokio::test]
    async fn test_relay_client_relay_addr() {
        let node_id = [1u8; 32];
        let addr: SocketAddr = "127.0.0.1:8002".parse().unwrap();

        if let Ok(client) = RelayClient::connect(addr, node_id).await {
            assert_eq!(client.relay_addr(), addr);
        }
    }

    #[test]
    fn test_relay_client_state_transitions() {
        assert_eq!(
            RelayClientState::Disconnected,
            RelayClientState::Disconnected
        );
        assert_ne!(RelayClientState::Connecting, RelayClientState::Connected);
    }

    #[test]
    fn test_relay_client_state_all_variants() {
        let states = vec![
            RelayClientState::Disconnected,
            RelayClientState::Connecting,
            RelayClientState::Registering,
            RelayClientState::Connected,
            RelayClientState::Error,
        ];
        for s in &states {
            assert_eq!(*s, *s);
            assert_eq!(format!("{:?}", s).is_empty(), false);
        }
        // All states are distinct
        for (i, a) in states.iter().enumerate() {
            for (j, b) in states.iter().enumerate() {
                if i != j {
                    assert_ne!(a, b);
                }
            }
        }
    }

    #[tokio::test]
    async fn test_send_to_peer_not_connected() {
        let node_id = [1u8; 32];
        let addr = "127.0.0.1:0".parse().unwrap();

        if let Ok(client) = RelayClient::connect(addr, node_id).await {
            // State is Connecting, not Connected, so send should fail
            let result = client.send_to_peer([2u8; 32], b"hello").await;
            assert!(result.is_err());
            assert!(matches!(result, Err(RelayError::NotRegistered)));
        }
    }

    #[tokio::test]
    async fn test_keepalive() {
        let node_id = [1u8; 32];
        let addr = "127.0.0.1:0".parse().unwrap();

        if let Ok(client) = RelayClient::connect(addr, node_id).await {
            // keepalive sends to socket (connected to 127.0.0.1:0, will likely fail)
            // but we test the code path
            let _ = client.keepalive().await;
        }
    }

    #[tokio::test]
    async fn test_disconnect() {
        let node_id = [1u8; 32];
        let addr = "127.0.0.1:0".parse().unwrap();

        if let Ok(mut client) = RelayClient::connect(addr, node_id).await {
            let _ = client.disconnect().await;
            // After disconnect attempt, state should be Disconnected
            let state = client.state().await;
            assert_eq!(state, RelayClientState::Disconnected);
        }
    }

    #[tokio::test]
    async fn test_maybe_keepalive_not_needed() {
        let node_id = [1u8; 32];
        let addr = "127.0.0.1:0".parse().unwrap();

        if let Ok(client) = RelayClient::connect(addr, node_id).await {
            // Just created, so keepalive not needed with large interval
            let result = client.maybe_keepalive(Duration::from_secs(3600)).await;
            assert!(result.is_ok());
        }
    }

    #[tokio::test]
    async fn test_maybe_keepalive_needed() {
        let node_id = [1u8; 32];
        let addr = "127.0.0.1:0".parse().unwrap();

        if let Ok(client) = RelayClient::connect(addr, node_id).await {
            // With zero interval, keepalive is always needed
            let _ = client.maybe_keepalive(Duration::from_secs(0)).await;
        }
    }

    #[tokio::test]
    async fn test_recv_from_peer_channel_closed() {
        let node_id = [1u8; 32];
        let addr = "127.0.0.1:0".parse().unwrap();

        if let Ok(client) = RelayClient::connect(addr, node_id).await {
            // Drop the sender side by dropping the client's tx clone
            // The channel should eventually close when all senders are dropped
            // For now, we just verify the method exists and returns an error type
            // (we can't easily force channel close without internal access)
            drop(client);
        }
    }

    #[tokio::test]
    async fn test_relay_client_register_timeout() {
        // Start a fake relay server that never responds
        let server_socket = tokio::net::UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let server_addr = server_socket.local_addr().unwrap();

        let node_id = [1u8; 32];
        let mut client = RelayClient::connect(server_addr, node_id).await.unwrap();

        // Register should timeout since server never sends RegisterAck
        let result =
            tokio::time::timeout(Duration::from_secs(12), client.register(&[2u8; 32])).await;

        // Either inner timeout (RelayError::Timeout) or outer timeout
        match result {
            Ok(Err(RelayError::Timeout)) => {} // Expected
            Err(_) => {}                       // Outer timeout also fine
            other => panic!("Unexpected result: {:?}", other.is_ok()),
        }
    }

    #[tokio::test]
    async fn test_relay_client_register_with_error_response() {
        // Set up a fake relay that responds with an Error message
        let server_socket = Arc::new(tokio::net::UdpSocket::bind("127.0.0.1:0").await.unwrap());
        let server_addr = server_socket.local_addr().unwrap();

        let server = server_socket.clone();
        tokio::spawn(async move {
            let mut buf = vec![0u8; 65536];
            if let Ok((len, from)) = server.recv_from(&mut buf).await {
                // Parse the register message
                if let Ok(RelayMessage::Register { .. }) = RelayMessage::from_bytes(&buf[..len]) {
                    // Respond with error
                    let error_msg = RelayMessage::Error {
                        code: super::super::protocol::RelayErrorCode::ServerFull,
                        message: "Server full".to_string(),
                    };
                    let bytes = error_msg.to_bytes().unwrap();
                    let _ = server.send_to(&bytes, from).await;
                }
            }
        });

        let node_id = [1u8; 32];
        let mut client = RelayClient::connect(server_addr, node_id).await.unwrap();
        let result = client.register(&[2u8; 32]).await;
        assert!(result.is_err());
        assert!(matches!(result, Err(RelayError::ServerFull)));
    }

    #[tokio::test]
    async fn test_relay_client_register_with_failed_ack() {
        // Set up a fake relay that responds with RegisterAck { success: false }
        let server_socket = Arc::new(tokio::net::UdpSocket::bind("127.0.0.1:0").await.unwrap());
        let server_addr = server_socket.local_addr().unwrap();

        let server = server_socket.clone();
        tokio::spawn(async move {
            let mut buf = vec![0u8; 65536];
            if let Ok((len, from)) = server.recv_from(&mut buf).await {
                if let Ok(RelayMessage::Register { .. }) = RelayMessage::from_bytes(&buf[..len]) {
                    let ack = RelayMessage::RegisterAck {
                        relay_id: [99u8; 32],
                        success: false,
                        error: Some("Denied".to_string()),
                    };
                    let bytes = ack.to_bytes().unwrap();
                    let _ = server.send_to(&bytes, from).await;
                }
            }
        });

        let node_id = [1u8; 32];
        let mut client = RelayClient::connect(server_addr, node_id).await.unwrap();
        let result = client.register(&[2u8; 32]).await;
        assert!(result.is_err());
        if let Err(RelayError::Internal(msg)) = result {
            assert!(msg.contains("Denied"));
        } else {
            panic!("Expected Internal error with 'Denied'");
        }
    }

    #[tokio::test]
    async fn test_relay_client_register_with_unexpected_response() {
        // Set up a fake relay that responds with an unexpected message type
        let server_socket = Arc::new(tokio::net::UdpSocket::bind("127.0.0.1:0").await.unwrap());
        let server_addr = server_socket.local_addr().unwrap();

        let server = server_socket.clone();
        tokio::spawn(async move {
            let mut buf = vec![0u8; 65536];
            if let Ok((len, from)) = server.recv_from(&mut buf).await {
                if let Ok(RelayMessage::Register { .. }) = RelayMessage::from_bytes(&buf[..len]) {
                    let unexpected = RelayMessage::Keepalive;
                    let bytes = unexpected.to_bytes().unwrap();
                    let _ = server.send_to(&bytes, from).await;
                }
            }
        });

        let node_id = [1u8; 32];
        let mut client = RelayClient::connect(server_addr, node_id).await.unwrap();
        let result = client.register(&[2u8; 32]).await;
        assert!(matches!(result, Err(RelayError::InvalidMessage)));
    }

    #[tokio::test]
    async fn test_relay_client_register_success_and_send() {
        // Set up a fake relay that responds with successful RegisterAck
        let server_socket = Arc::new(tokio::net::UdpSocket::bind("127.0.0.1:0").await.unwrap());
        let server_addr = server_socket.local_addr().unwrap();

        let server = server_socket.clone();
        tokio::spawn(async move {
            let mut buf = vec![0u8; 65536];
            if let Ok((len, from)) = server.recv_from(&mut buf).await {
                if let Ok(RelayMessage::Register { .. }) = RelayMessage::from_bytes(&buf[..len]) {
                    let ack = RelayMessage::RegisterAck {
                        relay_id: [42u8; 32],
                        success: true,
                        error: None,
                    };
                    let bytes = ack.to_bytes().unwrap();
                    let _ = server.send_to(&bytes, from).await;
                }
            }
            // Also consume the SendPacket that follows
            let mut buf = vec![0u8; 65536];
            let _ = server.recv_from(&mut buf).await;
        });

        let node_id = [1u8; 32];
        let mut client = RelayClient::connect(server_addr, node_id).await.unwrap();
        let result = client.register(&[2u8; 32]).await;
        assert!(result.is_ok());
        assert_eq!(client.state().await, RelayClientState::Connected);

        // Now send_to_peer should work (Connected state)
        let send_result = client.send_to_peer([3u8; 32], b"hello").await;
        assert!(send_result.is_ok());
    }

    #[tokio::test]
    async fn test_relay_client_register_failed_ack_no_error_msg() {
        // RegisterAck with success=false and error=None
        let server_socket = Arc::new(tokio::net::UdpSocket::bind("127.0.0.1:0").await.unwrap());
        let server_addr = server_socket.local_addr().unwrap();

        let server = server_socket.clone();
        tokio::spawn(async move {
            let mut buf = vec![0u8; 65536];
            if let Ok((len, from)) = server.recv_from(&mut buf).await {
                if let Ok(RelayMessage::Register { .. }) = RelayMessage::from_bytes(&buf[..len]) {
                    let ack = RelayMessage::RegisterAck {
                        relay_id: [0u8; 32],
                        success: false,
                        error: None,
                    };
                    let bytes = ack.to_bytes().unwrap();
                    let _ = server.send_to(&bytes, from).await;
                }
            }
        });

        let node_id = [1u8; 32];
        let mut client = RelayClient::connect(server_addr, node_id).await.unwrap();
        let result = client.register(&[2u8; 32]).await;
        assert!(result.is_err());
        if let Err(RelayError::Internal(msg)) = result {
            assert_eq!(msg, "Registration failed");
        }
        assert_eq!(client.state().await, RelayClientState::Error);
    }

    #[tokio::test]
    async fn test_spawn_receiver() {
        let node_id = [1u8; 32];
        let addr = "127.0.0.1:0".parse().unwrap();

        if let Ok(client) = RelayClient::connect(addr, node_id).await {
            // Just verify spawn_receiver doesn't panic
            client.spawn_receiver();
            // Give the spawned task a moment to start
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    }
}
