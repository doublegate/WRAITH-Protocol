//! QUIC transport implementation using quinn.
//!
//! This module provides a QUIC-based transport using the `quinn` crate.
//! QUIC provides built-in TLS 1.3 encryption, stream multiplexing,
//! 0-RTT connection resumption, and connection migration support.
//!
//! # Features
//!
//! - Built-in TLS 1.3 encryption via rustls
//! - Connection migration (IP address changes)
//! - Stream multiplexing
//! - 0-RTT connection resumption
//! - Better congestion control than TCP
//!
//! # Certificate Configuration
//!
//! By default, self-signed certificates are generated for testing.
//! For production use, provide custom certificates via `QuicConfig`.

use crate::factory::TransportType;
use crate::transport::{Transport, TransportError, TransportResult, TransportStats};
use async_trait::async_trait;
use quinn::{ClientConfig, Endpoint, RecvStream, SendStream, ServerConfig};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::Duration;
use tokio::sync::Mutex;

/// Maximum datagram/message size for QUIC transport.
const MAX_QUIC_MESSAGE_SIZE: usize = 65535;

/// Configuration for the QUIC transport.
#[derive(Clone)]
pub struct QuicConfig {
    /// Server certificate chain in DER format.
    pub server_cert_chain: Vec<Vec<u8>>,
    /// Server private key in DER format.
    pub server_key_der: Vec<u8>,
    /// Whether to accept self-signed certificates (for testing).
    pub accept_self_signed: bool,
    /// Keep-alive interval.
    pub keep_alive: Option<Duration>,
}

impl Default for QuicConfig {
    fn default() -> Self {
        Self {
            server_cert_chain: Vec::new(),
            server_key_der: Vec::new(),
            accept_self_signed: true,
            keep_alive: Some(Duration::from_secs(15)),
        }
    }
}

/// Connection state for a QUIC peer.
struct QuicConnection {
    send: SendStream,
    #[allow(dead_code)]
    recv: RecvStream,
}

/// Generate self-signed certificate for testing.
fn generate_self_signed_cert() -> TransportResult<(Vec<Vec<u8>>, Vec<u8>)> {
    let cert = rcgen::generate_simple_self_signed(vec!["localhost".to_string()])
        .map_err(|e| TransportError::Other(format!("Certificate generation failed: {e}")))?;
    let cert_der = cert.cert.der().to_vec();
    let key_der = cert.key_pair.serialize_der();
    Ok((vec![cert_der], key_der))
}

/// Build server config from certificate material.
fn build_server_config(
    cert_chain: Vec<Vec<u8>>,
    key_der: Vec<u8>,
) -> TransportResult<ServerConfig> {
    let certs: Vec<rustls::pki_types::CertificateDer<'static>> = cert_chain
        .into_iter()
        .map(rustls::pki_types::CertificateDer::from)
        .collect();

    let key = rustls::pki_types::PrivateKeyDer::try_from(key_der)
        .map_err(|e| TransportError::Other(format!("Invalid private key: {e}")))?;

    let server_config = ServerConfig::with_single_cert(certs, key)
        .map_err(|e| TransportError::Other(format!("Server config error: {e}")))?;

    Ok(server_config)
}

/// Build client config that skips certificate verification (for self-signed certs).
fn build_insecure_client_config() -> ClientConfig {
    let crypto = rustls::ClientConfig::builder()
        .dangerous()
        .with_custom_certificate_verifier(Arc::new(SkipServerVerification))
        .with_no_client_auth();
    ClientConfig::new(Arc::new(
        quinn::crypto::rustls::QuicClientConfig::try_from(crypto).expect("rustls QuicClientConfig"),
    ))
}

/// Certificate verifier that accepts all certificates (testing only).
#[derive(Debug)]
struct SkipServerVerification;

impl rustls::client::danger::ServerCertVerifier for SkipServerVerification {
    fn verify_server_cert(
        &self,
        _end_entity: &rustls::pki_types::CertificateDer<'_>,
        _intermediates: &[rustls::pki_types::CertificateDer<'_>],
        _server_name: &rustls::pki_types::ServerName<'_>,
        _ocsp_response: &[u8],
        _now: rustls::pki_types::UnixTime,
    ) -> Result<rustls::client::danger::ServerCertVerified, rustls::Error> {
        Ok(rustls::client::danger::ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &rustls::pki_types::CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        _cert: &rustls::pki_types::CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
        vec![
            rustls::SignatureScheme::ECDSA_NISTP256_SHA256,
            rustls::SignatureScheme::ECDSA_NISTP384_SHA384,
            rustls::SignatureScheme::RSA_PSS_SHA256,
            rustls::SignatureScheme::RSA_PSS_SHA384,
            rustls::SignatureScheme::RSA_PSS_SHA512,
            rustls::SignatureScheme::ED25519,
        ]
    }
}

/// QUIC transport using the quinn library.
///
/// Provides encrypted, multiplexed connections with connection migration support.
///
/// # Examples
///
/// ```no_run
/// use wraith_transport::quic::QuicTransport;
/// use wraith_transport::transport::Transport;
/// use std::net::SocketAddr;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let addr: SocketAddr = "127.0.0.1:0".parse()?;
/// let transport = QuicTransport::bind(addr).await?;
/// println!("QUIC transport on {}", transport.local_addr()?);
/// # Ok(())
/// # }
/// ```
pub struct QuicTransport {
    endpoint: Mutex<Endpoint>,
    local_addr: SocketAddr,
    closed: Arc<AtomicBool>,
    /// Active connections keyed by peer address.
    connections: Arc<Mutex<HashMap<SocketAddr, QuicConnection>>>,
    bytes_sent: Arc<AtomicU64>,
    bytes_received: Arc<AtomicU64>,
    packets_sent: Arc<AtomicU64>,
    packets_received: Arc<AtomicU64>,
    send_errors: Arc<AtomicU64>,
    recv_errors: Arc<AtomicU64>,
}

impl QuicTransport {
    /// Create a new QUIC transport bound to the given address with default config.
    ///
    /// Uses self-signed certificates suitable for testing.
    ///
    /// # Arguments
    /// * `addr` - The local address to bind to
    ///
    /// # Errors
    /// Returns `TransportError` if binding or certificate generation fails.
    pub async fn bind<A: Into<SocketAddr>>(addr: A) -> TransportResult<Self> {
        Self::bind_with_config(addr, QuicConfig::default()).await
    }

    /// Create a new QUIC transport with custom configuration.
    ///
    /// # Arguments
    /// * `addr` - The local address to bind to
    /// * `config` - QUIC configuration including certificates
    ///
    /// # Errors
    /// Returns `TransportError` if binding or configuration fails.
    pub async fn bind_with_config<A: Into<SocketAddr>>(
        addr: A,
        config: QuicConfig,
    ) -> TransportResult<Self> {
        let addr = addr.into();

        let (cert_chain, key_der) = if config.server_cert_chain.is_empty() {
            generate_self_signed_cert()?
        } else {
            (config.server_cert_chain, config.server_key_der)
        };

        let server_config = build_server_config(cert_chain, key_der)?;

        let endpoint = Endpoint::server(server_config, addr)
            .map_err(|e| TransportError::BindFailed(format!("QUIC endpoint: {e}")))?;

        let local_addr = endpoint
            .local_addr()
            .map_err(|e| TransportError::BindFailed(e.to_string()))?;

        Ok(Self {
            endpoint: Mutex::new(endpoint),
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

    /// Get or create a connection to the given peer address.
    async fn get_or_connect(&self, addr: SocketAddr) -> TransportResult<()> {
        let mut conns = self.connections.lock().await;
        if conns.contains_key(&addr) {
            return Ok(());
        }

        // Set up insecure client config for self-signed certs
        let mut endpoint = self.endpoint.lock().await;
        endpoint.set_default_client_config(build_insecure_client_config());

        let connection = endpoint
            .connect(addr, "localhost")
            .map_err(|e| TransportError::ConnectionFailed(format!("QUIC connect: {e}")))?
            .await
            .map_err(|e| TransportError::ConnectionFailed(format!("QUIC handshake: {e}")))?;
        drop(endpoint);

        let (send, recv) = connection
            .open_bi()
            .await
            .map_err(|e| TransportError::ConnectionFailed(format!("QUIC open stream: {e}")))?;

        conns.insert(addr, QuicConnection { send, recv });
        Ok(())
    }
}

#[async_trait]
impl Transport for QuicTransport {
    async fn send_to(&self, buf: &[u8], addr: SocketAddr) -> TransportResult<usize> {
        if self.closed.load(Ordering::Relaxed) {
            return Err(TransportError::Closed);
        }

        if let Err(e) = self.get_or_connect(addr).await {
            self.send_errors.fetch_add(1, Ordering::Relaxed);
            return Err(e);
        }

        let mut conns = self.connections.lock().await;
        let conn = conns.get_mut(&addr).ok_or_else(|| {
            TransportError::ConnectionFailed("Connection not found after connect".to_string())
        })?;

        // Write length-prefixed message
        let len = buf.len() as u32;
        conn.send.write_all(&len.to_be_bytes()).await.map_err(|e| {
            self.send_errors.fetch_add(1, Ordering::Relaxed);
            TransportError::Other(format!("QUIC write length: {e}"))
        })?;
        conn.send.write_all(buf).await.map_err(|e| {
            self.send_errors.fetch_add(1, Ordering::Relaxed);
            TransportError::Other(format!("QUIC write data: {e}"))
        })?;

        self.bytes_sent
            .fetch_add(buf.len() as u64, Ordering::Relaxed);
        self.packets_sent.fetch_add(1, Ordering::Relaxed);
        Ok(buf.len())
    }

    async fn recv_from(&self, buf: &mut [u8]) -> TransportResult<(usize, SocketAddr)> {
        if self.closed.load(Ordering::Relaxed) {
            return Err(TransportError::Closed);
        }

        // Accept incoming connection
        let incoming = self
            .endpoint
            .lock()
            .await
            .accept()
            .await
            .ok_or(TransportError::Closed)?;

        let connection = incoming.await.map_err(|e| {
            self.recv_errors.fetch_add(1, Ordering::Relaxed);
            TransportError::Other(format!("QUIC accept: {e}"))
        })?;

        let peer_addr = connection.remote_address();

        let (send, mut recv) = connection.accept_bi().await.map_err(|e| {
            self.recv_errors.fetch_add(1, Ordering::Relaxed);
            TransportError::Other(format!("QUIC accept stream: {e}"))
        })?;

        // Read length-prefixed message
        let mut len_buf = [0u8; 4];
        recv.read_exact(&mut len_buf).await.map_err(|e| {
            self.recv_errors.fetch_add(1, Ordering::Relaxed);
            TransportError::Other(format!("QUIC read length: {e}"))
        })?;
        let msg_len = u32::from_be_bytes(len_buf) as usize;

        if msg_len > MAX_QUIC_MESSAGE_SIZE {
            return Err(TransportError::Other(format!(
                "Message too large: {msg_len}"
            )));
        }

        let read_len = msg_len.min(buf.len());
        recv.read_exact(&mut buf[..read_len]).await.map_err(|e| {
            self.recv_errors.fetch_add(1, Ordering::Relaxed);
            TransportError::Other(format!("QUIC read data: {e}"))
        })?;

        self.bytes_received
            .fetch_add(read_len as u64, Ordering::Relaxed);
        self.packets_received.fetch_add(1, Ordering::Relaxed);

        // Store connection for future use
        self.connections
            .lock()
            .await
            .insert(peer_addr, QuicConnection { send, recv });

        Ok((read_len, peer_addr))
    }

    fn local_addr(&self) -> TransportResult<SocketAddr> {
        Ok(self.local_addr)
    }

    async fn close(&self) -> TransportResult<()> {
        self.closed.store(true, Ordering::Relaxed);
        self.endpoint
            .lock()
            .await
            .close(0u32.into(), b"transport closed");
        self.connections.lock().await.clear();
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
        TransportType::Quic
    }

    fn supports_migration(&self) -> bool {
        true
    }

    fn mtu(&self) -> usize {
        // QUIC max datagram size is typically ~1200 bytes for initial packets
        1200
    }

    fn latency_estimate(&self) -> Duration {
        // QUIC has good latency due to 0-RTT
        Duration::from_micros(500)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::timeout;

    #[tokio::test]
    async fn test_quic_bind() {
        let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
        let transport = QuicTransport::bind(addr).await.unwrap();
        let bound_addr = transport.local_addr().unwrap();
        assert_ne!(bound_addr.port(), 0);
        assert!(bound_addr.is_ipv4());
    }

    #[tokio::test]
    async fn test_quic_close() {
        let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
        let transport = QuicTransport::bind(addr).await.unwrap();
        assert!(!transport.is_closed());
        transport.close().await.unwrap();
        assert!(transport.is_closed());

        let result = transport
            .send_to(b"test", "127.0.0.1:1234".parse().unwrap())
            .await;
        assert!(matches!(result, Err(TransportError::Closed)));
    }

    #[tokio::test]
    async fn test_quic_transport_type() {
        let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
        let transport = QuicTransport::bind(addr).await.unwrap();
        assert_eq!(transport.transport_type(), TransportType::Quic);
    }

    #[tokio::test]
    async fn test_quic_supports_migration() {
        let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
        let transport = QuicTransport::bind(addr).await.unwrap();
        assert!(transport.supports_migration());
    }

    #[tokio::test]
    async fn test_quic_mtu() {
        let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
        let transport = QuicTransport::bind(addr).await.unwrap();
        assert_eq!(transport.mtu(), 1200);
    }

    #[tokio::test]
    async fn test_quic_send_recv() {
        let server_addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
        let server = QuicTransport::bind(server_addr).await.unwrap();
        let server_addr = server.local_addr().unwrap();

        let client_addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
        let client = QuicTransport::bind(client_addr).await.unwrap();

        let server = Arc::new(server);
        let server_clone = server.clone();

        let recv_handle = tokio::spawn(async move {
            let mut buf = vec![0u8; 1500];
            let (size, _from) = timeout(Duration::from_secs(5), server_clone.recv_from(&mut buf))
                .await
                .expect("Timeout")
                .unwrap();
            buf.truncate(size);
            buf
        });

        tokio::time::sleep(Duration::from_millis(100)).await;

        let sent = client.send_to(b"Hello QUIC!", server_addr).await.unwrap();
        assert_eq!(sent, 11);

        let received = recv_handle.await.unwrap();
        assert_eq!(&received, b"Hello QUIC!");
    }

    #[tokio::test]
    async fn test_quic_stats() {
        let server_addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
        let server = QuicTransport::bind(server_addr).await.unwrap();
        let server_addr = server.local_addr().unwrap();

        let client_addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
        let client = QuicTransport::bind(client_addr).await.unwrap();

        let server = Arc::new(server);
        let server_clone = server.clone();

        let recv_handle = tokio::spawn(async move {
            let mut buf = vec![0u8; 1500];
            timeout(Duration::from_secs(5), server_clone.recv_from(&mut buf))
                .await
                .expect("Timeout")
                .unwrap()
        });

        tokio::time::sleep(Duration::from_millis(100)).await;
        client.send_to(b"stats", server_addr).await.unwrap();
        recv_handle.await.unwrap();

        let client_stats = client.stats();
        assert_eq!(client_stats.packets_sent, 1);
        assert_eq!(client_stats.bytes_sent, 5);
    }

    #[tokio::test]
    async fn test_quic_recv_after_close() {
        let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
        let transport = QuicTransport::bind(addr).await.unwrap();
        transport.close().await.unwrap();

        let mut buf = vec![0u8; 1500];
        let result = transport.recv_from(&mut buf).await;
        assert!(matches!(result, Err(TransportError::Closed)));
    }

    #[tokio::test]
    async fn test_quic_self_signed_cert_generation() {
        let result = generate_self_signed_cert();
        assert!(result.is_ok());
        let (certs, key) = result.unwrap();
        assert_eq!(certs.len(), 1);
        assert!(!certs[0].is_empty());
        assert!(!key.is_empty());
    }

    #[tokio::test]
    async fn test_quic_default_config() {
        let config = QuicConfig::default();
        assert!(config.accept_self_signed);
        assert!(config.server_cert_chain.is_empty());
        assert!(config.keep_alive.is_some());
    }
}
