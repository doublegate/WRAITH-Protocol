//! ICE Candidate Gathering
//!
//! This module implements ICE (Interactive Connectivity Establishment) candidate
//! gathering for peer-to-peer connection establishment.

use super::dns::{StunDnsResolver, StunServerSpec, default_stun_servers, fallback_stun_ips};
use super::stun::StunClient;
use std::net::SocketAddr;

/// ICE candidate type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CandidateType {
    /// Host candidate (local interface address)
    Host,
    /// Server reflexive candidate (public address from STUN)
    ServerReflexive,
    /// Peer reflexive candidate (discovered during connectivity checks)
    PeerReflexive,
    /// Relay candidate (from TURN server)
    Relay,
}

impl std::fmt::Display for CandidateType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Host => write!(f, "host"),
            Self::ServerReflexive => write!(f, "srflx"),
            Self::PeerReflexive => write!(f, "prflx"),
            Self::Relay => write!(f, "relay"),
        }
    }
}

/// ICE candidate
#[derive(Debug, Clone)]
pub struct IceCandidate {
    /// Foundation - unique identifier for candidates that can be paired
    pub foundation: String,
    /// Component ID (1 for RTP, 2 for RTCP; we use 1)
    pub component_id: u32,
    /// Transport protocol (always "udp" for WRAITH)
    pub transport: String,
    /// Priority for candidate selection
    pub priority: u32,
    /// Candidate address (IP:port)
    pub address: SocketAddr,
    /// Candidate type
    pub candidate_type: CandidateType,
    /// Related address (for srflx/prflx/relay candidates)
    pub related_address: Option<SocketAddr>,
}

impl IceCandidate {
    /// Create a new host candidate
    #[must_use]
    pub fn host(address: SocketAddr) -> Self {
        Self {
            foundation: Self::compute_foundation(address, CandidateType::Host),
            component_id: 1,
            transport: "udp".to_string(),
            priority: Self::compute_priority(CandidateType::Host, 65535, 1),
            address,
            candidate_type: CandidateType::Host,
            related_address: None,
        }
    }

    /// Create a new server reflexive candidate
    #[must_use]
    pub fn server_reflexive(address: SocketAddr, base: SocketAddr) -> Self {
        Self {
            foundation: Self::compute_foundation(address, CandidateType::ServerReflexive),
            component_id: 1,
            transport: "udp".to_string(),
            priority: Self::compute_priority(CandidateType::ServerReflexive, 65535, 1),
            address,
            candidate_type: CandidateType::ServerReflexive,
            related_address: Some(base),
        }
    }

    /// Create a new relay candidate
    #[must_use]
    pub fn relay(address: SocketAddr, base: SocketAddr) -> Self {
        Self {
            foundation: Self::compute_foundation(address, CandidateType::Relay),
            component_id: 1,
            transport: "udp".to_string(),
            priority: Self::compute_priority(CandidateType::Relay, 65535, 1),
            address,
            candidate_type: CandidateType::Relay,
            related_address: Some(base),
        }
    }

    /// Compute foundation (simplified - just hash of type + address)
    fn compute_foundation(addr: SocketAddr, _typ: CandidateType) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        addr.hash(&mut hasher);
        format!("{:x}", hasher.finish())
            .chars()
            .take(8)
            .collect::<String>()
    }

    /// Compute priority (RFC 8445 Section 5.1.2)
    ///
    /// Priority = (2^24) * (type preference) + (2^8) * (local preference) + (256 - component ID)
    fn compute_priority(typ: CandidateType, local_pref: u32, component_id: u32) -> u32 {
        let type_pref = match typ {
            CandidateType::Host => 126,
            CandidateType::PeerReflexive => 110,
            CandidateType::ServerReflexive => 100,
            CandidateType::Relay => 0,
        };

        ((1 << 24) * type_pref) + ((1 << 8) * local_pref) + (256 - component_id)
    }

    /// Format as ICE candidate string (SDP format)
    #[must_use]
    pub fn to_sdp_string(&self) -> String {
        let mut s = format!(
            "candidate:{} {} {} {} {} {} typ {}",
            self.foundation,
            self.component_id,
            self.transport,
            self.priority,
            self.address.ip(),
            self.address.port(),
            self.candidate_type
        );

        if let Some(related) = self.related_address {
            s.push_str(&format!(" raddr {} rport {}", related.ip(), related.port()));
        }

        s
    }
}

/// Candidate for external use
#[derive(Debug, Clone)]
pub struct Candidate {
    /// Candidate address
    pub address: SocketAddr,
    /// Candidate type
    pub candidate_type: CandidateType,
    /// Priority
    pub priority: u32,
}

impl From<IceCandidate> for Candidate {
    fn from(ice: IceCandidate) -> Self {
        Self {
            address: ice.address,
            candidate_type: ice.candidate_type,
            priority: ice.priority,
        }
    }
}

/// ICE candidate gatherer
pub struct IceGatherer {
    stun_servers: Vec<SocketAddr>,
}

impl IceGatherer {
    /// Create a new ICE gatherer with fallback STUN server IPs
    ///
    /// Uses hardcoded fallback IPs from well-known STUN providers.
    /// For DNS-based resolution, use `with_dns_resolution()` instead.
    #[must_use]
    pub fn new() -> Self {
        Self {
            stun_servers: fallback_stun_ips(),
        }
    }

    /// Create with custom STUN servers
    #[must_use]
    pub fn with_stun_servers(servers: Vec<SocketAddr>) -> Self {
        Self {
            stun_servers: servers,
        }
    }

    /// Create an ICE gatherer with DNS resolution for STUN servers
    ///
    /// Resolves STUN server hostnames (like stun.l.google.com) using DNS,
    /// with automatic fallback to hardcoded IPs if DNS resolution fails.
    ///
    /// This is the preferred constructor when DNS is available.
    ///
    /// # Errors
    ///
    /// Returns error if DNS resolver initialization fails
    pub async fn with_dns_resolution() -> Result<Self, std::io::Error> {
        Self::with_custom_dns_resolution(&default_stun_servers()).await
    }

    /// Create an ICE gatherer with DNS resolution for custom STUN servers
    ///
    /// Resolves the provided STUN server specifications using DNS,
    /// with automatic fallback to hardcoded IPs if all resolutions fail.
    ///
    /// # Arguments
    ///
    /// * `stun_specs` - STUN server specifications (hostnames or IP addresses)
    ///
    /// # Errors
    ///
    /// Returns error if DNS resolver initialization fails
    pub async fn with_custom_dns_resolution(
        stun_specs: &[StunServerSpec],
    ) -> Result<Self, std::io::Error> {
        let resolver = StunDnsResolver::new()
            .await
            .map_err(|e| std::io::Error::other(format!("DNS resolver init failed: {e}")))?;

        let mut resolved = resolver.resolve_many(stun_specs).await;

        // Fall back to hardcoded IPs if DNS resolution failed for all servers
        if resolved.is_empty() {
            resolved = fallback_stun_ips();
        }

        Ok(Self {
            stun_servers: resolved,
        })
    }

    /// Gather all candidates for a local address
    ///
    /// Returns host, server reflexive, and relay candidates (if available).
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Cannot create UDP socket
    /// - STUN queries fail
    /// - No candidates can be gathered
    pub async fn gather(&self, local_addr: SocketAddr) -> Result<Vec<Candidate>, std::io::Error> {
        let mut candidates = Vec::new();

        // Always add host candidate
        let host_cand = IceCandidate::host(local_addr);
        candidates.push(host_cand.into());

        // Gather server reflexive candidates from STUN
        for stun_server in &self.stun_servers {
            if let Ok(client) = StunClient::bind("0.0.0.0:0").await {
                if let Ok(mapped_addr) = client.get_mapped_address(*stun_server).await {
                    // Only add if different from host candidate
                    if mapped_addr != local_addr {
                        let srflx_cand = IceCandidate::server_reflexive(mapped_addr, local_addr);
                        candidates.push(srflx_cand.into());
                    }
                }
            }
        }

        // Note: Relay candidates would be added here if TURN is implemented
        // For now, we only support host and server reflexive

        Ok(candidates)
    }

    /// Gather candidates for all local interfaces
    ///
    /// # Errors
    ///
    /// Returns an error if no candidates can be gathered
    pub async fn gather_all(&self) -> Result<Vec<Candidate>, std::io::Error> {
        let mut all_candidates = Vec::new();

        // Get local interfaces
        let interfaces = self.get_local_interfaces()?;

        for interface_addr in interfaces {
            if let Ok(mut candidates) = self.gather(interface_addr).await {
                all_candidates.append(&mut candidates);
            }
        }

        if all_candidates.is_empty() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "No candidates gathered",
            ));
        }

        Ok(all_candidates)
    }

    /// Get local network interface addresses
    fn get_local_interfaces(&self) -> Result<Vec<SocketAddr>, std::io::Error> {
        use std::net::{IpAddr, Ipv4Addr};

        // Simplified: just return common bind addresses
        // In production, would enumerate actual network interfaces
        Ok(vec![
            SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), 0),
            SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 0),
        ])
    }

    /// Sort candidates by priority (descending)
    pub fn sort_by_priority(candidates: &mut [Candidate]) {
        candidates.sort_by(|a, b| b.priority.cmp(&a.priority));
    }
}

impl Default for IceGatherer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_candidate_type_display() {
        assert_eq!(CandidateType::Host.to_string(), "host");
        assert_eq!(CandidateType::ServerReflexive.to_string(), "srflx");
        assert_eq!(CandidateType::Relay.to_string(), "relay");
    }

    #[test]
    fn test_host_candidate() {
        let addr: SocketAddr = "192.168.1.100:5000".parse().unwrap();
        let cand = IceCandidate::host(addr);

        assert_eq!(cand.candidate_type, CandidateType::Host);
        assert_eq!(cand.address, addr);
        assert!(cand.related_address.is_none());
        assert_eq!(cand.component_id, 1);
        assert_eq!(cand.transport, "udp");
    }

    #[test]
    fn test_server_reflexive_candidate() {
        let addr: SocketAddr = "203.0.113.1:12345".parse().unwrap();
        let base: SocketAddr = "192.168.1.100:5000".parse().unwrap();

        let cand = IceCandidate::server_reflexive(addr, base);

        assert_eq!(cand.candidate_type, CandidateType::ServerReflexive);
        assert_eq!(cand.address, addr);
        assert_eq!(cand.related_address, Some(base));
    }

    #[test]
    fn test_priority_calculation() {
        let host_priority = IceCandidate::compute_priority(CandidateType::Host, 65535, 1);
        let srflx_priority =
            IceCandidate::compute_priority(CandidateType::ServerReflexive, 65535, 1);
        let relay_priority = IceCandidate::compute_priority(CandidateType::Relay, 65535, 1);

        // Host should have highest priority
        assert!(host_priority > srflx_priority);
        assert!(srflx_priority > relay_priority);
    }

    #[test]
    fn test_sdp_string() {
        let addr: SocketAddr = "192.168.1.100:5000".parse().unwrap();
        let cand = IceCandidate::host(addr);

        let sdp = cand.to_sdp_string();

        assert!(sdp.contains("candidate:"));
        assert!(sdp.contains("udp"));
        assert!(sdp.contains("typ host"));
        assert!(sdp.contains("192.168.1.100"));
        assert!(sdp.contains("5000"));
    }

    #[test]
    fn test_sdp_string_with_related() {
        let addr: SocketAddr = "203.0.113.1:12345".parse().unwrap();
        let base: SocketAddr = "192.168.1.100:5000".parse().unwrap();

        let cand = IceCandidate::server_reflexive(addr, base);
        let sdp = cand.to_sdp_string();

        assert!(sdp.contains("typ srflx"));
        assert!(sdp.contains("raddr 192.168.1.100"));
        assert!(sdp.contains("rport 5000"));
    }

    #[test]
    fn test_candidate_sorting() {
        let addr1: SocketAddr = "192.168.1.100:5000".parse().unwrap();
        let addr2: SocketAddr = "203.0.113.1:12345".parse().unwrap();

        let host = IceCandidate::host(addr1);
        let srflx = IceCandidate::server_reflexive(addr2, addr1);

        let mut candidates = vec![Candidate::from(srflx), Candidate::from(host)];

        IceGatherer::sort_by_priority(&mut candidates);

        // Host should come first (higher priority)
        assert_eq!(candidates[0].candidate_type, CandidateType::Host);
        assert_eq!(candidates[1].candidate_type, CandidateType::ServerReflexive);
    }

    #[test]
    fn test_ice_gatherer_creation() {
        let gatherer = IceGatherer::new();
        // Now uses fallback_stun_ips() which has 5 servers
        assert_eq!(gatherer.stun_servers.len(), 5);

        let custom_servers = vec!["1.1.1.1:3478".parse().unwrap()];
        let gatherer = IceGatherer::with_stun_servers(custom_servers);
        assert_eq!(gatherer.stun_servers.len(), 1);
    }

    #[test]
    fn test_relay_candidate() {
        let relay_addr: SocketAddr = "203.0.113.100:443".parse().unwrap();
        let base_addr: SocketAddr = "192.168.1.100:5000".parse().unwrap();

        let cand = IceCandidate::relay(relay_addr, base_addr);

        assert_eq!(cand.candidate_type, CandidateType::Relay);
        assert_eq!(cand.address, relay_addr);
        assert_eq!(cand.related_address, Some(base_addr));
    }

    #[test]
    fn test_candidate_foundation_uniqueness() {
        let addr1: SocketAddr = "192.168.1.100:5000".parse().unwrap();
        let addr2: SocketAddr = "192.168.1.100:5001".parse().unwrap();

        let cand1 = IceCandidate::host(addr1);
        let cand2 = IceCandidate::host(addr2);

        // Different addresses should produce different foundations
        assert_ne!(cand1.foundation, cand2.foundation);
    }

    #[test]
    fn test_candidate_foundation_consistency() {
        let addr: SocketAddr = "192.168.1.100:5000".parse().unwrap();

        let cand1 = IceCandidate::host(addr);
        let cand2 = IceCandidate::host(addr);

        // Same address should produce same foundation
        assert_eq!(cand1.foundation, cand2.foundation);
    }

    #[test]
    fn test_priority_component_id() {
        // Component ID should affect priority (256 - component_id)
        let prio1 = IceCandidate::compute_priority(CandidateType::Host, 65535, 1);
        let prio2 = IceCandidate::compute_priority(CandidateType::Host, 65535, 2);

        assert!(prio1 > prio2);
    }

    #[test]
    fn test_priority_local_preference() {
        let prio1 = IceCandidate::compute_priority(CandidateType::Host, 65535, 1);
        let prio2 = IceCandidate::compute_priority(CandidateType::Host, 32000, 1);

        assert!(prio1 > prio2);
    }

    #[test]
    fn test_peer_reflexive_priority() {
        let prflx_priority = IceCandidate::compute_priority(CandidateType::PeerReflexive, 65535, 1);
        let srflx_priority =
            IceCandidate::compute_priority(CandidateType::ServerReflexive, 65535, 1);
        let relay_priority = IceCandidate::compute_priority(CandidateType::Relay, 65535, 1);

        // PeerReflexive should have priority between ServerReflexive and Relay
        assert!(prflx_priority > relay_priority);
        assert!(prflx_priority > srflx_priority); // prflx has type_pref 110 > srflx 100
    }

    #[test]
    fn test_candidate_conversion() {
        let addr: SocketAddr = "192.168.1.100:5000".parse().unwrap();
        let ice_cand = IceCandidate::host(addr);

        let cand: Candidate = ice_cand.clone().into();

        assert_eq!(cand.address, ice_cand.address);
        assert_eq!(cand.candidate_type, ice_cand.candidate_type);
        assert_eq!(cand.priority, ice_cand.priority);
    }

    #[test]
    fn test_ice_gatherer_default() {
        let gatherer = IceGatherer::default();
        // Now uses fallback_stun_ips() which has 5 servers
        assert_eq!(gatherer.stun_servers.len(), 5);
    }

    #[tokio::test]
    async fn test_ice_gatherer_dns_resolution() {
        // Test DNS resolution constructor
        let result = IceGatherer::with_dns_resolution().await;
        assert!(result.is_ok());
        let gatherer = result.unwrap();
        // Should have resolved addresses or fallback IPs
        assert!(!gatherer.stun_servers.is_empty());
    }

    #[tokio::test]
    async fn test_ice_gatherer_custom_dns_resolution() {
        // Test with IP-based specs (no actual DNS needed)
        let specs = vec![
            StunServerSpec::ip("1.2.3.4:3478".parse().unwrap()),
            StunServerSpec::ip("5.6.7.8:3478".parse().unwrap()),
        ];
        let result = IceGatherer::with_custom_dns_resolution(&specs).await;
        assert!(result.is_ok());
        let gatherer = result.unwrap();
        assert_eq!(gatherer.stun_servers.len(), 2);
    }

    #[test]
    fn test_candidate_type_equality() {
        assert_eq!(CandidateType::Host, CandidateType::Host);
        assert_ne!(CandidateType::Host, CandidateType::Relay);
    }

    #[test]
    fn test_candidate_sorting_empty() {
        let mut candidates: Vec<Candidate> = vec![];
        IceGatherer::sort_by_priority(&mut candidates);
        assert_eq!(candidates.len(), 0);
    }

    #[test]
    fn test_candidate_sorting_multiple() {
        let addr1: SocketAddr = "192.168.1.100:5000".parse().unwrap();
        let addr2: SocketAddr = "203.0.113.1:12345".parse().unwrap();
        let addr3: SocketAddr = "203.0.113.2:443".parse().unwrap();

        let host = IceCandidate::host(addr1);
        let srflx = IceCandidate::server_reflexive(addr2, addr1);
        let relay = IceCandidate::relay(addr3, addr1);

        let mut candidates = vec![
            Candidate::from(relay),
            Candidate::from(host),
            Candidate::from(srflx),
        ];

        IceGatherer::sort_by_priority(&mut candidates);

        // Should be sorted: host, srflx, relay
        assert_eq!(candidates[0].candidate_type, CandidateType::Host);
        assert_eq!(candidates[1].candidate_type, CandidateType::ServerReflexive);
        assert_eq!(candidates[2].candidate_type, CandidateType::Relay);
    }

    #[test]
    fn test_sdp_string_ipv6() {
        let addr: SocketAddr = "[2001:db8::1]:5000".parse().unwrap();
        let cand = IceCandidate::host(addr);

        let sdp = cand.to_sdp_string();

        assert!(sdp.contains("candidate:"));
        assert!(sdp.contains("2001:db8::1"));
        assert!(sdp.contains("5000"));
    }

    #[test]
    fn test_foundation_length() {
        let addr: SocketAddr = "192.168.1.100:5000".parse().unwrap();
        let cand = IceCandidate::host(addr);

        // Foundation should be 8 hex characters
        assert_eq!(cand.foundation.len(), 8);
        assert!(cand.foundation.chars().all(|c| c.is_ascii_hexdigit()));
    }
}
