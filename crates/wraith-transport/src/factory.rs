//! Transport factory for creating transport instances.
//!
//! This module provides a factory pattern for creating different transport
//! implementations based on configuration.

use crate::quic::QuicTransport;
use crate::tcp::TcpTransport;
#[cfg(not(target_os = "linux"))]
use crate::transport::TransportError;
use crate::transport::{Transport, TransportResult};
use crate::udp_async::AsyncUdpTransport;
use crate::websocket::WebSocketTransport;
use std::net::SocketAddr;
use std::sync::Arc;

/// Transport type selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum TransportType {
    /// UDP transport (always available)
    #[default]
    Udp,
    /// QUIC transport using quinn
    Quic,
    /// TCP transport with length-prefixed framing
    Tcp,
    /// WebSocket transport for HTTP proxy traversal
    WebSocket,
    /// io_uring-based network transport (Linux-only)
    IoUring,
    /// AF_XDP kernel bypass transport (Linux-only)
    AfXdp,
}

impl std::fmt::Display for TransportType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Udp => write!(f, "UDP"),
            Self::Quic => write!(f, "QUIC"),
            Self::Tcp => write!(f, "TCP"),
            Self::WebSocket => write!(f, "WebSocket"),
            Self::IoUring => write!(f, "io_uring"),
            Self::AfXdp => write!(f, "AF_XDP"),
        }
    }
}

/// Configuration for creating a transport.
#[derive(Debug, Clone)]
pub struct TransportFactoryConfig {
    /// Type of transport to create
    pub transport_type: TransportType,
    /// Local address to bind to
    pub bind_addr: SocketAddr,
    /// Receive buffer size (bytes)
    pub recv_buffer_size: Option<usize>,
    /// Send buffer size (bytes)
    pub send_buffer_size: Option<usize>,
}

impl TransportFactoryConfig {
    /// Create a new transport configuration.
    ///
    /// # Arguments
    /// * `transport_type` - The type of transport to create
    /// * `bind_addr` - The local address to bind to
    ///
    /// # Examples
    /// ```no_run
    /// use wraith_transport::factory::{TransportFactoryConfig, TransportType};
    /// use std::net::SocketAddr;
    ///
    /// let addr: SocketAddr = "127.0.0.1:40000".parse().unwrap();
    /// let config = TransportFactoryConfig::new(TransportType::Udp, addr);
    /// ```
    #[must_use]
    pub fn new(transport_type: TransportType, bind_addr: SocketAddr) -> Self {
        Self {
            transport_type,
            bind_addr,
            recv_buffer_size: None,
            send_buffer_size: None,
        }
    }

    /// Create a UDP transport configuration.
    ///
    /// # Arguments
    /// * `bind_addr` - The local address to bind to
    ///
    /// # Examples
    /// ```no_run
    /// use wraith_transport::factory::TransportFactoryConfig;
    /// use std::net::SocketAddr;
    ///
    /// let addr: SocketAddr = "127.0.0.1:40000".parse().unwrap();
    /// let config = TransportFactoryConfig::udp(addr);
    /// ```
    #[must_use]
    pub fn udp(bind_addr: SocketAddr) -> Self {
        Self::new(TransportType::Udp, bind_addr)
    }

    /// Create a QUIC transport configuration.
    ///
    /// # Arguments
    /// * `bind_addr` - The local address to bind to
    #[must_use]
    pub fn quic(bind_addr: SocketAddr) -> Self {
        Self::new(TransportType::Quic, bind_addr)
    }

    /// Create a TCP transport configuration.
    ///
    /// # Arguments
    /// * `bind_addr` - The local address to bind to
    #[must_use]
    pub fn tcp(bind_addr: SocketAddr) -> Self {
        Self::new(TransportType::Tcp, bind_addr)
    }

    /// Create a WebSocket transport configuration.
    ///
    /// # Arguments
    /// * `bind_addr` - The local address to bind to
    #[must_use]
    pub fn websocket(bind_addr: SocketAddr) -> Self {
        Self::new(TransportType::WebSocket, bind_addr)
    }

    /// Set custom buffer sizes.
    ///
    /// # Arguments
    /// * `recv_size` - Receive buffer size in bytes
    /// * `send_size` - Send buffer size in bytes
    #[must_use]
    pub fn with_buffer_sizes(mut self, recv_size: usize, send_size: usize) -> Self {
        self.recv_buffer_size = Some(recv_size);
        self.send_buffer_size = Some(send_size);
        self
    }
}

/// Default bind address for transports (0.0.0.0:0 = any interface, OS-assigned port)
const DEFAULT_BIND_ADDR: std::net::SocketAddr = std::net::SocketAddr::V4(
    std::net::SocketAddrV4::new(std::net::Ipv4Addr::UNSPECIFIED, 0),
);

impl Default for TransportFactoryConfig {
    fn default() -> Self {
        Self {
            transport_type: TransportType::Udp,
            bind_addr: DEFAULT_BIND_ADDR,
            recv_buffer_size: None,
            send_buffer_size: None,
        }
    }
}

/// Factory for creating transport instances.
///
/// This factory provides a unified interface for creating different transport
/// implementations based on configuration.
///
/// # Examples
///
/// ```no_run
/// use wraith_transport::factory::{TransportFactory, TransportFactoryConfig, TransportType};
/// use std::net::SocketAddr;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let addr: SocketAddr = "127.0.0.1:40000".parse()?;
/// let config = TransportFactoryConfig::udp(addr);
///
/// let transport = TransportFactory::create(config).await?;
/// println!("Created transport on {}", transport.local_addr()?);
/// # Ok(())
/// # }
/// ```
pub struct TransportFactory;

impl TransportFactory {
    /// Create a transport based on the provided configuration.
    ///
    /// # Arguments
    /// * `config` - Transport configuration
    ///
    /// # Returns
    /// A boxed transport implementing the `Transport` trait
    ///
    /// # Errors
    /// Returns `TransportError` if transport creation fails
    ///
    /// # Examples
    /// ```no_run
    /// use wraith_transport::factory::{TransportFactory, TransportFactoryConfig};
    /// use std::net::SocketAddr;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let addr: SocketAddr = "127.0.0.1:0".parse()?;
    /// let config = TransportFactoryConfig::udp(addr);
    /// let transport = TransportFactory::create(config).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn create(config: TransportFactoryConfig) -> TransportResult<Arc<dyn Transport>> {
        match config.transport_type {
            TransportType::Udp => {
                let transport = AsyncUdpTransport::bind(config.bind_addr).await?;
                Ok(Arc::new(transport))
            }
            TransportType::Quic => {
                let transport = QuicTransport::bind(config.bind_addr).await?;
                Ok(Arc::new(transport))
            }
            TransportType::Tcp => {
                let transport = TcpTransport::bind(config.bind_addr).await?;
                Ok(Arc::new(transport))
            }
            TransportType::WebSocket => {
                let transport = WebSocketTransport::bind(config.bind_addr).await?;
                Ok(Arc::new(transport))
            }
            TransportType::IoUring => {
                #[cfg(target_os = "linux")]
                {
                    let transport =
                        crate::io_uring_net::IoUringTransport::bind(config.bind_addr).await?;
                    Ok(Arc::new(transport))
                }
                #[cfg(not(target_os = "linux"))]
                {
                    Err(TransportError::Other(
                        "io_uring transport is only available on Linux".to_string(),
                    ))
                }
            }
            TransportType::AfXdp => {
                #[cfg(target_os = "linux")]
                {
                    let transport =
                        crate::af_xdp_transport::AfXdpTransport::bind(config.bind_addr).await?;
                    Ok(Arc::new(transport))
                }
                #[cfg(not(target_os = "linux"))]
                {
                    Err(TransportError::Other(
                        "AF_XDP transport is only available on Linux".to_string(),
                    ))
                }
            }
        }
    }

    /// Create a UDP transport with default settings.
    ///
    /// # Arguments
    /// * `bind_addr` - The local address to bind to
    ///
    /// # Errors
    /// Returns `TransportError` if binding fails
    ///
    /// # Examples
    /// ```no_run
    /// use wraith_transport::factory::TransportFactory;
    /// use std::net::SocketAddr;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let addr: SocketAddr = "127.0.0.1:0".parse()?;
    /// let transport = TransportFactory::create_udp(addr).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn create_udp(bind_addr: SocketAddr) -> TransportResult<Arc<dyn Transport>> {
        let config = TransportFactoryConfig::udp(bind_addr);
        Self::create(config).await
    }

    /// Create a QUIC transport with default settings (not yet implemented).
    ///
    /// # Arguments
    /// * `bind_addr` - The local address to bind to
    ///
    /// # Errors
    /// Currently always returns `TransportError::Other` as QUIC is not implemented
    ///
    /// # Examples
    /// ```no_run
    /// use wraith_transport::factory::TransportFactory;
    /// use std::net::SocketAddr;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let addr: SocketAddr = "127.0.0.1:0".parse()?;
    /// let result = TransportFactory::create_quic(addr).await;
    /// assert!(result.is_err()); // QUIC not yet implemented
    /// # Ok(())
    /// # }
    /// ```
    pub async fn create_quic(bind_addr: SocketAddr) -> TransportResult<Arc<dyn Transport>> {
        let config = TransportFactoryConfig::quic(bind_addr);
        Self::create(config).await
    }

    /// Get list of available transport types.
    ///
    /// # Returns
    /// Vector of transport types that can be created
    #[must_use]
    pub fn available_transports() -> Vec<TransportType> {
        let mut transports = vec![
            TransportType::Udp,
            TransportType::Quic,
            TransportType::Tcp,
            TransportType::WebSocket,
        ];
        #[cfg(target_os = "linux")]
        {
            transports.push(TransportType::IoUring);
            transports.push(TransportType::AfXdp);
        }
        transports
    }

    /// Check if a transport type is fully implemented.
    ///
    /// # Arguments
    /// * `transport_type` - The transport type to check
    ///
    /// # Returns
    /// `true` if the transport is fully implemented and functional
    #[must_use]
    pub fn is_implemented(transport_type: TransportType) -> bool {
        match transport_type {
            TransportType::Udp
            | TransportType::Quic
            | TransportType::Tcp
            | TransportType::WebSocket => true,
            #[cfg(target_os = "linux")]
            TransportType::IoUring | TransportType::AfXdp => true,
            #[cfg(not(target_os = "linux"))]
            TransportType::IoUring | TransportType::AfXdp => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_factory_create_udp() {
        let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
        let config = TransportFactoryConfig::udp(addr);

        let transport = TransportFactory::create(config).await.unwrap();
        let bound_addr = transport.local_addr().unwrap();
        assert!(bound_addr.is_ipv4());
        assert_ne!(bound_addr.port(), 0);
    }

    #[tokio::test]
    async fn test_factory_create_udp_shorthand() {
        let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
        let transport = TransportFactory::create_udp(addr).await.unwrap();
        assert!(transport.local_addr().unwrap().is_ipv4());
    }

    #[tokio::test]
    async fn test_factory_create_quic() {
        let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
        let result = TransportFactory::create_quic(addr).await;
        // QUIC is now implemented - should succeed
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_factory_available_transports() {
        let transports = TransportFactory::available_transports();
        assert!(transports.contains(&TransportType::Udp));
        assert!(transports.contains(&TransportType::Quic));
    }

    #[tokio::test]
    async fn test_factory_is_implemented() {
        assert!(TransportFactory::is_implemented(TransportType::Udp));
        assert!(TransportFactory::is_implemented(TransportType::Quic));
        assert!(TransportFactory::is_implemented(TransportType::Tcp));
        assert!(TransportFactory::is_implemented(TransportType::WebSocket));
    }

    #[tokio::test]
    async fn test_config_default() {
        let config = TransportFactoryConfig::default();
        assert_eq!(config.transport_type, TransportType::Udp);
        assert!(config.recv_buffer_size.is_none());
        assert!(config.send_buffer_size.is_none());
    }

    #[tokio::test]
    async fn test_config_with_buffer_sizes() {
        let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
        let config = TransportFactoryConfig::udp(addr).with_buffer_sizes(1024 * 1024, 512 * 1024);

        assert_eq!(config.recv_buffer_size, Some(1024 * 1024));
        assert_eq!(config.send_buffer_size, Some(512 * 1024));
    }

    #[tokio::test]
    async fn test_factory_udp_send_recv() {
        let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
        let server = TransportFactory::create_udp(addr).await.unwrap();
        let server_addr = server.local_addr().unwrap();

        let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
        let client = TransportFactory::create_udp(addr).await.unwrap();

        // Send data
        client.send_to(b"Factory test", server_addr).await.unwrap();

        // Receive data
        let mut buf = vec![0u8; 1500];
        let (size, from) = tokio::time::timeout(
            std::time::Duration::from_secs(1),
            server.recv_from(&mut buf),
        )
        .await
        .expect("Timeout")
        .unwrap();

        assert_eq!(size, 12);
        assert_eq!(&buf[..size], b"Factory test");
        assert_eq!(from, client.local_addr().unwrap());
    }
}
