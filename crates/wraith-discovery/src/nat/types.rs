//! NAT Type Detection
//!
//! This module implements NAT type detection using STUN-like probing to classify
//! NAT devices into categories that determine the best traversal strategy.

use super::stun::StunClient;
use std::net::{IpAddr, SocketAddr};

/// NAT type classification
///
/// Different NAT types require different traversal strategies:
/// - Open: No NAT, direct connection possible
/// - Full Cone: Easy to traverse, any external host can send
/// - Restricted Cone: Moderate difficulty, requires simultaneous open
/// - Port Restricted Cone: Moderate difficulty, requires simultaneous open
/// - Symmetric: Hardest to traverse, requires birthday attack or relay
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NatType {
    /// No NAT detected, public IP address
    Open,
    /// Full Cone NAT - any external host can send to mapped port
    FullCone,
    /// Restricted Cone NAT - only contacted IPs can send
    RestrictedCone,
    /// Port Restricted Cone NAT - only contacted IP:port can send
    PortRestrictedCone,
    /// Symmetric NAT - different mapping per destination
    Symmetric,
    /// Unknown NAT type (detection failed)
    Unknown,
}

impl std::fmt::Display for NatType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Open => write!(f, "Open (No NAT)"),
            Self::FullCone => write!(f, "Full Cone NAT"),
            Self::RestrictedCone => write!(f, "Restricted Cone NAT"),
            Self::PortRestrictedCone => write!(f, "Port Restricted Cone NAT"),
            Self::Symmetric => write!(f, "Symmetric NAT"),
            Self::Unknown => write!(f, "Unknown NAT Type"),
        }
    }
}

/// NAT detection error
#[derive(Debug)]
pub enum NatError {
    /// I/O error during detection
    Io(std::io::Error),
    /// STUN server timeout
    Timeout,
    /// Invalid response from STUN server
    InvalidResponse,
    /// No STUN servers available
    NoServers,
}

impl std::fmt::Display for NatError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(e) => write!(f, "I/O error: {e}"),
            Self::Timeout => write!(f, "STUN server timeout"),
            Self::InvalidResponse => write!(f, "Invalid STUN response"),
            Self::NoServers => write!(f, "No STUN servers available"),
        }
    }
}

impl std::error::Error for NatError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(e) => Some(e),
            _ => None,
        }
    }
}

impl From<std::io::Error> for NatError {
    fn from(err: std::io::Error) -> Self {
        Self::Io(err)
    }
}

impl From<super::stun::StunError> for NatError {
    fn from(err: super::stun::StunError) -> Self {
        match err {
            super::stun::StunError::Io(e) => Self::Io(e),
            super::stun::StunError::Timeout => Self::Timeout,
            _ => Self::InvalidResponse,
        }
    }
}

/// NAT type detector
///
/// Uses multiple STUN servers to probe NAT behavior and classify the NAT type.
pub struct NatDetector {
    stun_servers: Vec<SocketAddr>,
}

impl NatDetector {
    /// Create a new NAT detector with default STUN servers
    ///
    /// Uses Google's public STUN servers. Note that these IPs are hardcoded
    /// and may change. In production, implement DNS resolution for:
    /// - stun.l.google.com:19302
    /// - stun1.l.google.com:19302
    /// - stun2.l.google.com:19302
    ///
    /// TODO: Implement DNS-based STUN server resolution
    #[must_use]
    pub fn new() -> Self {
        Self {
            stun_servers: vec![
                // Google Public STUN servers (hardcoded IPs)
                // stun.l.google.com:19302
                "74.125.250.129:19302"
                    .parse()
                    .expect("valid STUN server address"),
                // stun1.l.google.com:19302
                "74.125.250.130:19302"
                    .parse()
                    .expect("valid STUN server address"),
            ],
        }
    }

    /// Create a NAT detector with custom STUN servers
    #[must_use]
    pub fn with_servers(servers: Vec<SocketAddr>) -> Self {
        Self {
            stun_servers: servers,
        }
    }

    /// Detect NAT type using STUN probing
    ///
    /// This performs a series of STUN queries to classify the NAT device:
    /// 1. Query local address to check if it's public
    /// 2. Query from same socket to different servers (check for symmetric NAT)
    /// 3. Query from different sockets to same server (check port mapping)
    ///
    /// # Errors
    ///
    /// Returns `NatError` if:
    /// - No STUN servers are configured
    /// - All STUN queries fail
    /// - Network I/O errors occur
    pub async fn detect(&self) -> Result<NatType, NatError> {
        if self.stun_servers.is_empty() {
            return Err(NatError::NoServers);
        }

        // Test 1: Create socket and get external address from first STUN server
        let client1 = StunClient::bind("0.0.0.0:0").await?;
        let local_addr1 = client1.local_addr()?;

        let external1 = client1.get_mapped_address(self.stun_servers[0]).await?;

        // Check if we have a public IP (no NAT)
        if Self::is_public_ip(&local_addr1.ip()) && local_addr1.ip() == external1.ip() {
            return Ok(NatType::Open);
        }

        // Test 2: Query from same socket to different server
        if self.stun_servers.len() > 1 {
            let external2 = client1.get_mapped_address(self.stun_servers[1]).await?;

            // If external addresses differ, it's symmetric NAT
            if external1 != external2 {
                return Ok(NatType::Symmetric);
            }
        }

        // Test 3: Use different local socket, same server
        let client2 = StunClient::bind("0.0.0.0:0").await?;
        let external3 = client2.get_mapped_address(self.stun_servers[0]).await?;

        // Check if external port changes with local port
        if external1.port() != external3.port() {
            // Port changes -> Port Restricted Cone or Symmetric
            // We already ruled out Symmetric above, so it's Port Restricted
            return Ok(NatType::PortRestrictedCone);
        }

        // Test 4: Check if we can receive from different IP
        // This would require a more complex STUN implementation with CHANGE-REQUEST
        // For now, we default to Restricted Cone as it's the most common

        Ok(NatType::RestrictedCone)
    }

    /// Check if an IP address is public (not private/loopback/link-local)
    fn is_public_ip(ip: &IpAddr) -> bool {
        match ip {
            IpAddr::V4(ipv4) => !ipv4.is_private() && !ipv4.is_loopback() && !ipv4.is_link_local(),
            IpAddr::V6(ipv6) => !ipv6.is_loopback() && !ipv6.is_multicast(),
        }
    }
}

impl Default for NatDetector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_nat_type_display() {
        assert_eq!(NatType::Open.to_string(), "Open (No NAT)");
        assert_eq!(NatType::FullCone.to_string(), "Full Cone NAT");
        assert_eq!(NatType::Symmetric.to_string(), "Symmetric NAT");
    }

    #[test]
    fn test_is_public_ip() {
        // Private IPs
        assert!(!NatDetector::is_public_ip(&"192.168.1.1".parse().unwrap()));
        assert!(!NatDetector::is_public_ip(&"10.0.0.1".parse().unwrap()));
        assert!(!NatDetector::is_public_ip(&"172.16.0.1".parse().unwrap()));

        // Loopback
        assert!(!NatDetector::is_public_ip(&"127.0.0.1".parse().unwrap()));

        // Link-local
        assert!(!NatDetector::is_public_ip(&"169.254.1.1".parse().unwrap()));

        // Public IPs
        assert!(NatDetector::is_public_ip(&"8.8.8.8".parse().unwrap()));
        assert!(NatDetector::is_public_ip(&"203.0.113.1".parse().unwrap()));
    }

    #[test]
    fn test_nat_detector_creation() {
        let detector = NatDetector::new();
        assert_eq!(detector.stun_servers.len(), 2);

        let custom_servers = vec!["1.1.1.1:3478".parse().unwrap()];
        let detector = NatDetector::with_servers(custom_servers);
        assert_eq!(detector.stun_servers.len(), 1);
    }

    #[test]
    fn test_nat_error_display() {
        let err = NatError::Timeout;
        assert_eq!(err.to_string(), "STUN server timeout");

        let err = NatError::NoServers;
        assert_eq!(err.to_string(), "No STUN servers available");
    }

    #[test]
    fn test_nat_error_from_io_error() {
        let io_err = std::io::Error::new(std::io::ErrorKind::TimedOut, "timeout");
        let nat_err: NatError = io_err.into();
        assert!(matches!(nat_err, NatError::Io(_)));
    }

    #[test]
    fn test_nat_error_from_stun_error() {
        use super::super::stun::StunError;

        let stun_timeout: NatError = StunError::Timeout.into();
        assert!(matches!(stun_timeout, NatError::Timeout));

        let stun_invalid: NatError = StunError::InvalidMagicCookie.into();
        assert!(matches!(stun_invalid, NatError::InvalidResponse));
    }

    #[test]
    fn test_nat_type_equality() {
        assert_eq!(NatType::Open, NatType::Open);
        assert_ne!(NatType::Open, NatType::Symmetric);
        assert_ne!(NatType::FullCone, NatType::RestrictedCone);
    }

    #[test]
    fn test_is_public_ip_ipv6() {
        // IPv6 loopback
        assert!(!NatDetector::is_public_ip(&"::1".parse().unwrap()));

        // IPv6 multicast
        assert!(!NatDetector::is_public_ip(&"ff02::1".parse().unwrap()));

        // IPv6 public (Google DNS)
        assert!(NatDetector::is_public_ip(
            &"2001:4860:4860::8888".parse().unwrap()
        ));
    }

    #[test]
    fn test_is_public_ip_edge_cases() {
        // Class E (reserved)
        assert!(NatDetector::is_public_ip(&"240.0.0.1".parse().unwrap())); // Technically reserved but not checked

        // Documentation ranges
        assert!(NatDetector::is_public_ip(&"203.0.113.1".parse().unwrap()));
        assert!(NatDetector::is_public_ip(&"198.51.100.1".parse().unwrap()));
    }

    #[test]
    fn test_nat_detector_default() {
        let detector = NatDetector::default();
        assert_eq!(detector.stun_servers.len(), 2);
    }

    #[test]
    fn test_nat_detector_empty_servers() {
        let detector = NatDetector::with_servers(vec![]);
        assert_eq!(detector.stun_servers.len(), 0);
    }

    #[tokio::test]
    async fn test_nat_detect_no_servers() {
        let detector = NatDetector::with_servers(vec![]);
        let result = detector.detect().await;
        assert!(matches!(result, Err(NatError::NoServers)));
    }

    #[test]
    fn test_nat_type_all_variants() {
        // Ensure all NAT types can be created and compared
        let types = vec![
            NatType::Open,
            NatType::FullCone,
            NatType::RestrictedCone,
            NatType::PortRestrictedCone,
            NatType::Symmetric,
            NatType::Unknown,
        ];

        for typ in types {
            let display = typ.to_string();
            assert!(!display.is_empty());
        }
    }
}
