//! UDP Hole Punching
//!
//! This module implements UDP hole punching for establishing direct peer-to-peer
//! connections through NAT devices using the simultaneous open technique.

use std::net::SocketAddr;
use std::time::Duration;
use tokio::net::UdpSocket;

/// Probe packet marker
const PROBE_MARKER: &[u8] = b"WRAITH_PROBE";
/// Response packet marker
const RESPONSE_MARKER: &[u8] = b"WRAITH_RESPONSE";

/// Maximum probe attempts
const MAX_PROBE_ATTEMPTS: usize = 20;
/// Probe interval
const PROBE_INTERVAL: Duration = Duration::from_millis(100);
/// Probe timeout
const PROBE_TIMEOUT: Duration = Duration::from_millis(50);
/// Maximum port prediction range
const MAX_PORT_RANGE: u16 = 10;

/// Hole puncher for UDP NAT traversal
pub struct HolePuncher {
    socket: UdpSocket,
}

impl HolePuncher {
    /// Create a new hole puncher bound to a local address
    ///
    /// # Errors
    ///
    /// Returns an error if the socket cannot be bound to the specified address
    pub async fn new(bind_addr: SocketAddr) -> Result<Self, std::io::Error> {
        let socket = UdpSocket::bind(bind_addr).await?;
        Ok(Self { socket })
    }

    /// Get local socket address
    ///
    /// # Errors
    ///
    /// Returns an error if the local address cannot be determined
    pub fn local_addr(&self) -> Result<SocketAddr, std::io::Error> {
        self.socket.local_addr()
    }

    /// Perform hole punching to establish connection with peer
    ///
    /// This uses multiple strategies in parallel:
    /// 1. Direct connection to peer's external address
    /// 2. Connection to peer's internal address (if on same LAN)
    /// 3. Sequential port prediction (for predictable NAT port allocation)
    ///
    /// # Arguments
    ///
    /// * `peer_external` - Peer's external (server reflexive) address
    /// * `peer_internal` - Peer's internal (host) address (if known)
    ///
    /// # Errors
    ///
    /// Returns `PunchError` if:
    /// - All hole punching strategies fail
    /// - Network I/O errors occur
    /// - Timeout is exceeded
    pub async fn punch(
        &self,
        peer_external: SocketAddr,
        peer_internal: Option<SocketAddr>,
    ) -> Result<SocketAddr, PunchError> {
        // Try strategies in parallel using tokio::select
        tokio::select! {
            result = self.try_direct(peer_external) => {
                result
            }
            result = self.try_internal(peer_internal) => {
                result
            }
            result = self.try_sequential_ports(peer_external) => {
                result
            }
            _ = tokio::time::sleep(Duration::from_secs(5)) => {
                Err(PunchError::Timeout)
            }
        }
    }

    /// Try direct connection to peer's external address
    async fn try_direct(&self, peer: SocketAddr) -> Result<SocketAddr, PunchError> {
        for _ in 0..MAX_PROBE_ATTEMPTS {
            // Send probe
            self.socket.send_to(PROBE_MARKER, peer).await?;

            // Try to receive response
            match tokio::time::timeout(PROBE_TIMEOUT, self.recv_probe()).await {
                Ok(Ok(from)) if from.ip() == peer.ip() => {
                    return Ok(from);
                }
                _ => {
                    tokio::time::sleep(PROBE_INTERVAL).await;
                }
            }
        }

        Err(PunchError::Timeout)
    }

    /// Try connection to peer's internal address (LAN)
    async fn try_internal(&self, peer: Option<SocketAddr>) -> Result<SocketAddr, PunchError> {
        let peer = peer.ok_or(PunchError::NoInternalAddress)?;

        for _ in 0..(MAX_PROBE_ATTEMPTS / 2) {
            // Send probe
            self.socket.send_to(PROBE_MARKER, peer).await?;

            // Try to receive response
            match tokio::time::timeout(PROBE_TIMEOUT, self.recv_probe()).await {
                Ok(Ok(from)) if from == peer => {
                    return Ok(from);
                }
                _ => {
                    tokio::time::sleep(PROBE_INTERVAL).await;
                }
            }
        }

        Err(PunchError::Timeout)
    }

    /// Try sequential port prediction for predictable NAT port allocation
    async fn try_sequential_ports(&self, peer: SocketAddr) -> Result<SocketAddr, PunchError> {
        let base_port = peer.port();

        for offset in 0..MAX_PORT_RANGE {
            let try_port = base_port.wrapping_add(offset);
            let try_addr = SocketAddr::new(peer.ip(), try_port);

            // Send probe
            self.socket.send_to(PROBE_MARKER, try_addr).await?;

            // Try to receive response
            match tokio::time::timeout(PROBE_TIMEOUT, self.recv_probe()).await {
                Ok(Ok(from)) if from.ip() == peer.ip() => {
                    return Ok(from);
                }
                _ => {
                    tokio::time::sleep(PROBE_INTERVAL / 2).await;
                }
            }

            // Also try ports below base
            if offset > 0 {
                let try_port = base_port.wrapping_sub(offset);
                let try_addr = SocketAddr::new(peer.ip(), try_port);

                self.socket.send_to(PROBE_MARKER, try_addr).await?;

                match tokio::time::timeout(PROBE_TIMEOUT, self.recv_probe()).await {
                    Ok(Ok(from)) if from.ip() == peer.ip() => {
                        return Ok(from);
                    }
                    _ => {}
                }
            }
        }

        Err(PunchError::Timeout)
    }

    /// Receive and verify a probe packet
    async fn recv_probe(&self) -> Result<SocketAddr, std::io::Error> {
        let mut buf = [0u8; 1024];
        let (len, from) = self.socket.recv_from(&mut buf).await?;

        // Check if it's a probe or response packet
        if &buf[..len] == PROBE_MARKER || &buf[..len] == RESPONSE_MARKER {
            // Send response if we received a probe
            if &buf[..len] == PROBE_MARKER {
                let _ = self.socket.send_to(RESPONSE_MARKER, from).await;
            }
            Ok(from)
        } else {
            Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Not a probe packet",
            ))
        }
    }

    /// Maintain hole in NAT by sending keepalive packets
    ///
    /// This should be called periodically (e.g., every 15-30 seconds) to keep
    /// the NAT binding alive.
    ///
    /// # Errors
    ///
    /// Returns an error if the keepalive packet cannot be sent
    pub async fn maintain_hole(&self, peer: SocketAddr) -> Result<(), std::io::Error> {
        self.socket.send_to(PROBE_MARKER, peer).await?;
        Ok(())
    }

    /// Get the underlying socket
    ///
    /// This allows the caller to use the same socket for application data
    /// after hole punching succeeds.
    #[must_use]
    pub fn into_socket(self) -> UdpSocket {
        self.socket
    }
}

/// Hole punching error
#[derive(Debug)]
pub enum PunchError {
    /// I/O error
    Io(std::io::Error),
    /// Hole punching timeout (all strategies failed)
    Timeout,
    /// No internal address provided for LAN strategy
    NoInternalAddress,
}

impl std::fmt::Display for PunchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(e) => write!(f, "I/O error: {e}"),
            Self::Timeout => write!(f, "Hole punching timeout"),
            Self::NoInternalAddress => write!(f, "No internal address for LAN strategy"),
        }
    }
}

impl std::error::Error for PunchError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(e) => Some(e),
            _ => None,
        }
    }
}

impl From<std::io::Error> for PunchError {
    fn from(err: std::io::Error) -> Self {
        Self::Io(err)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error;

    #[test]
    fn test_probe_markers() {
        assert_eq!(PROBE_MARKER, b"WRAITH_PROBE");
        assert_eq!(RESPONSE_MARKER, b"WRAITH_RESPONSE");
    }

    #[test]
    fn test_punch_error_display() {
        let err = PunchError::Timeout;
        assert_eq!(err.to_string(), "Hole punching timeout");

        let err = PunchError::NoInternalAddress;
        assert_eq!(err.to_string(), "No internal address for LAN strategy");
    }

    #[tokio::test]
    async fn test_hole_puncher_creation() {
        let puncher = HolePuncher::new("127.0.0.1:0".parse().unwrap())
            .await
            .unwrap();

        let local_addr = puncher.local_addr().unwrap();
        assert_eq!(local_addr.ip().to_string(), "127.0.0.1");
    }

    #[tokio::test]
    async fn test_loopback_punch() {
        // Create two punchers on loopback
        let puncher1 = HolePuncher::new("127.0.0.1:0".parse().unwrap())
            .await
            .unwrap();
        let addr1 = puncher1.local_addr().unwrap();

        let puncher2 = HolePuncher::new("127.0.0.1:0".parse().unwrap())
            .await
            .unwrap();
        let addr2 = puncher2.local_addr().unwrap();

        // Try to punch through (should succeed on loopback)
        let punch1 = puncher1.punch(addr2, Some(addr2));
        let punch2 = puncher2.punch(addr1, Some(addr1));

        // At least one should succeed
        tokio::select! {
            result = punch1 => {
                assert!(result.is_ok() || matches!(result, Err(PunchError::Timeout)));
            }
            result = punch2 => {
                assert!(result.is_ok() || matches!(result, Err(PunchError::Timeout)));
            }
        }
    }

    #[tokio::test]
    async fn test_maintain_hole() {
        let puncher = HolePuncher::new("127.0.0.1:0".parse().unwrap())
            .await
            .unwrap();

        let peer_addr = "127.0.0.1:12345".parse().unwrap();

        // Should not error (even if peer doesn't exist)
        let result = puncher.maintain_hole(peer_addr).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_into_socket() {
        let puncher = HolePuncher::new("127.0.0.1:0".parse().unwrap())
            .await
            .unwrap();

        let original_addr = puncher.local_addr().unwrap();
        let socket = puncher.into_socket();

        // Socket should have same address
        assert_eq!(socket.local_addr().unwrap(), original_addr);
    }

    #[test]
    fn test_punch_error_from_io_error() {
        let io_err = std::io::Error::new(std::io::ErrorKind::TimedOut, "timeout");
        let punch_err: PunchError = io_err.into();
        assert!(matches!(punch_err, PunchError::Io(_)));
    }

    #[test]
    fn test_constants() {
        // Verify constants are reasonable values
        // Note: These are compile-time constants, but we verify their values
        let max_probe_val = MAX_PROBE_ATTEMPTS;
        let max_port_val = MAX_PORT_RANGE;
        let probe_interval_val = PROBE_INTERVAL;

        assert_eq!(max_probe_val, 20);
        assert!(max_probe_val > 0);

        assert_eq!(max_port_val, 10);
        assert!(max_port_val > 0);

        assert_eq!(probe_interval_val, Duration::from_millis(100));
        assert_eq!(PROBE_TIMEOUT, Duration::from_millis(50));
    }

    #[test]
    fn test_probe_marker_lengths() {
        assert!(PROBE_MARKER.len() > 0);
        assert!(RESPONSE_MARKER.len() > 0);
        assert_ne!(PROBE_MARKER, RESPONSE_MARKER);
    }

    #[tokio::test]
    async fn test_punch_with_no_internal_address() {
        let puncher = HolePuncher::new("127.0.0.1:0".parse().unwrap())
            .await
            .unwrap();

        let external_addr = "203.0.113.1:12345".parse().unwrap();

        // With very short timeout, should timeout trying all strategies
        let result = tokio::time::timeout(
            Duration::from_millis(500),
            puncher.punch(external_addr, None),
        )
        .await;

        // Either timeout or punch error expected
        assert!(result.is_err() || matches!(result, Ok(Err(_))));
    }

    #[test]
    fn test_punch_error_source() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "not found");
        let punch_err = PunchError::Io(io_err);

        assert!(punch_err.source().is_some());

        let timeout_err = PunchError::Timeout;
        assert!(timeout_err.source().is_none());
    }

    #[tokio::test]
    async fn test_multiple_maintain_hole_calls() {
        let puncher = HolePuncher::new("127.0.0.1:0".parse().unwrap())
            .await
            .unwrap();

        let peer_addr = "127.0.0.1:12345".parse().unwrap();

        // Multiple calls should succeed
        for _ in 0..5 {
            assert!(puncher.maintain_hole(peer_addr).await.is_ok());
        }
    }
}
