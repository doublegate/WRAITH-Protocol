//! NAT traversal integration for Node API
//!
//! Implements ICE-lite style NAT hole punching and relay fallback.

use crate::node::discovery::{NatType, PeerInfo};
use crate::node::session::PeerConnection;
use crate::node::{Node, NodeError};
use std::net::SocketAddr;
use std::time::Duration;
use wraith_transport::transport::Transport;

/// ICE candidate for NAT traversal
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IceCandidate {
    /// Candidate address
    pub address: SocketAddr,

    /// Candidate type
    pub candidate_type: CandidateType,

    /// Priority (higher = more preferred)
    pub priority: u32,

    /// Foundation (for pairing candidates)
    pub foundation: String,
}

/// ICE candidate types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CandidateType {
    /// Host candidate (local interface)
    Host,

    /// Server reflexive candidate (from STUN)
    ServerReflexive,

    /// Relayed candidate (from TURN/relay)
    Relayed,
}

impl Node {
    /// Detect local NAT type using STUN
    ///
    /// Performs STUN queries to determine the NAT type.
    ///
    /// # Errors
    ///
    /// Returns error if STUN queries fail or no STUN servers configured.
    pub async fn detect_nat_type(&self) -> Result<NatType, NodeError> {
        tracing::debug!("Detecting NAT type via discovery manager");

        // Get discovery manager
        let discovery = {
            let guard = self.inner.discovery.lock().await;
            guard
                .as_ref()
                .ok_or(NodeError::Discovery(std::borrow::Cow::Borrowed(
                    "Discovery not initialized",
                )))?
                .clone()
        };

        // Query NAT type from discovery manager
        match discovery.nat_type().await {
            Some(discovery_nat_type) => {
                // Convert from wraith-discovery NatType to wraith-core NatType
                let nat_type = NatType::from(discovery_nat_type);
                tracing::info!("Detected NAT type: {:?}", nat_type);
                Ok(nat_type)
            }
            None => {
                // NAT detection not run or failed
                tracing::warn!("NAT type not detected, assuming None");
                Ok(NatType::None)
            }
        }
    }

    /// Attempt NAT traversal to connect to peer
    ///
    /// Uses ICE-lite to establish a connection through NAT.
    /// Strategy depends on both local and remote NAT types:
    /// - No NAT or Full Cone: Direct connection
    /// - Restricted/Port-restricted: Hole punching
    /// - Symmetric NAT: Relay fallback
    ///
    /// # Arguments
    ///
    /// * `peer` - Peer information including NAT type
    ///
    /// # Errors
    ///
    /// Returns error if all connection attempts fail.
    pub async fn traverse_nat(&self, peer: &PeerInfo) -> Result<PeerConnection, NodeError> {
        tracing::info!(
            "Attempting NAT traversal to peer {:?} (NAT: {:?})",
            peer.peer_id,
            peer.nat_type
        );

        let local_nat = self.detect_nat_type().await?;
        let remote_nat = peer.nat_type;

        // Categorize NAT types for easier decision making
        let can_direct_connect = matches!(
            (local_nat, remote_nat),
            (NatType::None, _)
                | (_, NatType::None)
                | (NatType::FullCone, _)
                | (_, NatType::FullCone)
        );

        let both_symmetric = matches!(
            (local_nat, remote_nat),
            (NatType::Symmetric, NatType::Symmetric)
        );

        if can_direct_connect {
            tracing::debug!("Attempting direct connection");
            self.direct_connect(peer).await
        } else if both_symmetric {
            tracing::debug!("Both symmetric NAT, using relay");
            self.connect_via_relay(peer).await
        } else {
            // One or both sides have restricted NAT (RestrictedCone/PortRestricted)
            // or one side is symmetric with the other being restricted
            tracing::debug!("Attempting hole punching with relay fallback");
            match self.hole_punch(peer).await {
                Ok(conn) => Ok(conn),
                Err(e) => {
                    tracing::warn!("Hole punching failed ({}), falling back to relay", e);
                    self.connect_via_relay(peer).await
                }
            }
        }
    }

    /// Attempt direct connection to peer
    ///
    /// Tries each advertised peer address in sequence until one succeeds.
    /// Returns the established session ID and peer connection on success.
    async fn direct_connect(&self, peer: &PeerInfo) -> Result<PeerConnection, NodeError> {
        tracing::debug!("Direct connecting to peer {:?}", peer.peer_id);

        // Try each advertised address
        let mut last_error = None;
        for addr in &peer.addresses {
            tracing::trace!("Trying direct connection to address: {}", addr);

            // Attempt to establish session with this address
            match self.establish_session_with_addr(&peer.peer_id, *addr).await {
                Ok(session_id) => {
                    // Session established successfully, retrieve the connection from sessions map
                    // The connection is stored as Arc<PeerConnection> in the DashMap
                    if let Some(conn_arc) = self.inner.sessions.get(&peer.peer_id) {
                        tracing::info!(
                            "Direct connection established to peer {:?} at {} (session: {})",
                            peer.peer_id,
                            addr,
                            hex::encode(&session_id[..8])
                        );

                        // Clone the PeerConnection (shares Arc references)
                        return Ok((**conn_arc).clone());
                    } else {
                        last_error = Some(NodeError::SessionNotFound(peer.peer_id));
                        tracing::warn!(
                            "Session established but not found in sessions map for peer {:?}",
                            peer.peer_id
                        );
                        continue;
                    }
                }
                Err(e) => {
                    tracing::trace!("Failed to connect to {}: {}", addr, e);
                    last_error = Some(e);
                    continue;
                }
            }
        }

        Err(last_error.unwrap_or_else(|| {
            NodeError::NatTraversal("All direct connection attempts failed".into())
        }))
    }

    /// Perform ICE-lite hole punching
    async fn hole_punch(&self, peer: &PeerInfo) -> Result<PeerConnection, NodeError> {
        tracing::debug!("Starting hole punch for peer {:?}", peer.peer_id);

        // 1. Gather local ICE candidates
        let local_candidates = self.gather_ice_candidates().await?;

        // 2. Exchange candidates with peer (via signaling/relay)
        let remote_candidates = self.exchange_candidates(peer, &local_candidates).await?;

        // 3. Try candidates in priority order
        let candidate_pairs = self.prioritize_candidates(&local_candidates, &remote_candidates);

        for (local, remote) in candidate_pairs {
            tracing::trace!(
                "Trying candidate pair: {:?} -> {:?}",
                local.address,
                remote.address
            );

            match self.try_connect_candidate(&local, &remote).await {
                Ok(conn) => {
                    tracing::info!("Hole punch successful via {:?}", local.address);
                    return Ok(conn);
                }
                Err(e) => {
                    tracing::trace!("Candidate pair failed: {}", e);
                    continue;
                }
            }
        }

        Err(NodeError::NatTraversal(std::borrow::Cow::Borrowed(
            "All candidate pairs failed",
        )))
    }

    /// Connect via relay server
    ///
    /// Uses the discovery manager to establish a relay path, then performs
    /// a Noise_XX handshake over the relay connection to establish a secure session.
    async fn connect_via_relay(&self, peer: &PeerInfo) -> Result<PeerConnection, NodeError> {
        tracing::debug!("Connecting via relay to peer {:?}", peer.peer_id);

        // Get discovery manager
        let discovery = {
            let guard = self.inner.discovery.lock().await;
            guard
                .as_ref()
                .ok_or(NodeError::Discovery(std::borrow::Cow::Borrowed(
                    "Discovery not initialized",
                )))?
                .clone()
        };

        // Convert peer_id to NodeId for relay connection
        let peer_node_id = wraith_discovery::dht::NodeId::from_bytes(peer.peer_id);

        // Use discovery manager to establish relay path
        // The discovery manager handles the relay connection establishment
        match discovery.connect_to_peer(peer_node_id).await {
            Ok(conn_info) => {
                tracing::info!(
                    "Discovery manager established {} connection to peer {:?} via relay {}",
                    conn_info.connection_type,
                    peer.peer_id,
                    conn_info.addr
                );

                // Now establish a protocol-level session over the relay connection
                // The relay address is used as the peer address for the handshake
                // The actual peer_id is already known from the PeerInfo
                match self
                    .establish_session_with_addr(&peer.peer_id, conn_info.addr)
                    .await
                {
                    Ok(session_id) => {
                        // Session established successfully over relay
                        if let Some(conn_arc) = self.inner.sessions.get(&peer.peer_id) {
                            tracing::info!(
                                "Relay session established to peer {:?} via {} (session: {})",
                                peer.peer_id,
                                conn_info.addr,
                                hex::encode(&session_id[..8])
                            );

                            // Clone the PeerConnection (shares Arc references)
                            Ok((**conn_arc).clone())
                        } else {
                            Err(NodeError::SessionNotFound(peer.peer_id))
                        }
                    }
                    Err(e) => {
                        tracing::warn!(
                            "Failed to establish session over relay to peer {:?}: {}",
                            peer.peer_id,
                            e
                        );
                        Err(NodeError::NatTraversal(
                            format!("Relay handshake failed: {e}").into(),
                        ))
                    }
                }
            }
            Err(e) => {
                tracing::warn!("Relay connection failed: {}", e);
                Err(NodeError::NatTraversal(
                    format!("Relay connection failed: {e}").into(),
                ))
            }
        }
    }

    /// Gather ICE candidates from local interfaces
    async fn gather_ice_candidates(&self) -> Result<Vec<IceCandidate>, NodeError> {
        let mut candidates = Vec::new();

        // 1. Host candidates (local interfaces)
        for addr in self.local_addresses() {
            candidates.push(IceCandidate {
                address: addr,
                candidate_type: CandidateType::Host,
                priority: 126, // Type preference for host
                foundation: format!("host-{addr}"),
            });
        }

        // 2. Server reflexive candidates (STUN) - Integrate with discovery manager
        let discovery = {
            let guard = self.inner.discovery.lock().await;
            guard.as_ref().cloned()
        };

        if let Some(ref disc) = discovery {
            // Get NAT type which triggers STUN detection
            if let Some(nat_type) = disc.nat_type().await {
                tracing::debug!("Detected NAT type: {:?}", nat_type);
                // NAT type detected means STUN was successful
                // In a real implementation, we would get the actual reflexive address
                // from the STUN response. For now, this confirms STUN is working.
            }
        }

        // 3. Relayed candidates (TURN) - Integrate with relay manager
        if discovery.is_some() && self.inner.config.discovery.enable_relay {
            // The discovery manager handles relay connections
            // Relayed addresses are established when connect_via_relay() is called
            // For candidate gathering, we just note that relay is available
            tracing::debug!("Relay is enabled and available for fallback");
        }

        tracing::debug!("Gathered {} ICE candidates", candidates.len());

        Ok(candidates)
    }

    /// Exchange ICE candidates with peer
    ///
    /// In a full implementation, this would use a signaling channel (e.g., via relay server
    /// or out-of-band mechanism) to exchange ICE candidates with the peer. The signaling
    /// protocol would:
    ///
    /// 1. Send local candidates to peer via signaling channel
    /// 2. Receive remote candidates from peer via signaling channel
    /// 3. Wait for candidate exchange to complete with timeout
    ///
    /// For now, this implementation uses the peer's known addresses from discovery
    /// and converts them to ICE candidates. A full signaling implementation would
    /// be added in a future sprint.
    ///
    /// # Arguments
    ///
    /// * `peer` - Peer information including known addresses
    /// * `local_candidates` - Local ICE candidates to send to peer (currently unused)
    ///
    /// # Future Implementation
    ///
    /// A complete implementation would:
    /// - Use DHT STORE/GET for public signaling (with encryption)
    /// - Use relay server for private signaling
    /// - Support SDP-like candidate description format
    /// - Handle candidate gathering and exchange in parallel
    /// - Implement candidate filtering and priority calculation
    async fn exchange_candidates(
        &self,
        peer: &PeerInfo,
        local_candidates: &[IceCandidate],
    ) -> Result<Vec<IceCandidate>, NodeError> {
        tracing::debug!(
            "Exchanging ICE candidates with peer {:?} ({} local candidates)",
            peer.peer_id,
            local_candidates.len()
        );

        // In a future implementation, this would:
        // 1. Serialize local_candidates to a wire format
        // 2. Send them to the peer via signaling (DHT STORE or relay message)
        // 3. Wait for peer's candidates via signaling (DHT GET or relay response)
        // 4. Deserialize and return peer's candidates
        //
        // For now, use the peer's known addresses from discovery as candidates
        let remote_candidates: Vec<IceCandidate> = peer
            .addresses
            .iter()
            .enumerate()
            .map(|(idx, addr)| {
                // Assign priority based on address type
                let priority = if addr.is_ipv4() { 126 } else { 100 };

                IceCandidate {
                    address: *addr,
                    candidate_type: CandidateType::Host,
                    priority,
                    foundation: format!("host-{idx}-{addr}"),
                }
            })
            .collect();

        tracing::debug!(
            "Received {} remote candidates from peer {:?} (from discovery)",
            remote_candidates.len(),
            peer.peer_id
        );

        // NOTE: Full signaling-based candidate exchange is deferred.
        //
        // Current implementation uses discovery-based candidate retrieval, which works
        // for most NAT scenarios (cone NAT, restricted cone NAT) where peer addresses
        // are available through DHT discovery.
        //
        // Full ICE signaling would be needed for:
        // - Symmetric NAT traversal (where port mapping changes per destination)
        // - TURN relay fallback when direct connectivity fails
        // - Multi-homed hosts with complex routing
        //
        // Future implementation requirements:
        // 1. Define signaling message types:
        //    - CANDIDATE_OFFER: Local candidates with SDP-like attributes
        //    - CANDIDATE_ANSWER: Response with remote candidates
        //    - CANDIDATE_ERROR: Signaling failure notification
        // 2. Signaling transport options:
        //    - DHT STORE/GET with encrypted payloads (peer_id as key)
        //    - Relay-mediated message forwarding (for symmetric NAT)
        // 3. ICE-style candidate processing:
        //    - Concurrent gathering (STUN/TURN probes) with timeout
        //    - Trickle ICE for progressive candidate discovery
        //    - Candidate priority negotiation (local > srflx > relay)
        // 4. Security considerations:
        //    - Encrypt signaling with peer's public key
        //    - Validate candidate addresses against spoofing
        //
        // See: RFC 8445 (ICE), RFC 8838 (Trickle ICE)
        // See: to-dos/technical-debt/TECH-DEBT-v1.5.0.md TM-001

        Ok(remote_candidates)
    }

    /// Prioritize candidate pairs
    ///
    /// Returns pairs sorted by preference (highest priority first).
    fn prioritize_candidates(
        &self,
        local: &[IceCandidate],
        remote: &[IceCandidate],
    ) -> Vec<(IceCandidate, IceCandidate)> {
        let mut pairs = Vec::new();

        // Generate all possible pairs
        for l in local {
            for r in remote {
                pairs.push((l.clone(), r.clone()));
            }
        }

        // Sort by combined priority (higher first)
        pairs.sort_by_key(|(l, r)| std::cmp::Reverse(l.priority + r.priority));

        pairs
    }

    /// Try to connect using a specific candidate pair
    ///
    /// Attempts to establish a connection using the specified local and remote ICE candidates.
    /// This includes sending hole-punch packets and attempting the Noise handshake.
    async fn try_connect_candidate(
        &self,
        local: &IceCandidate,
        remote: &IceCandidate,
    ) -> Result<PeerConnection, NodeError> {
        tracing::trace!(
            "Attempting connection with candidate pair: {:?} ({:?}) -> {:?} ({:?})",
            local.address,
            local.candidate_type,
            remote.address,
            remote.candidate_type
        );

        // For hole punching scenarios (restricted NAT), send simultaneous packets
        // to create NAT bindings on both sides
        if matches!(
            (local.candidate_type, remote.candidate_type),
            (CandidateType::Host, CandidateType::Host)
                | (
                    CandidateType::ServerReflexive,
                    CandidateType::ServerReflexive
                )
                | (CandidateType::ServerReflexive, CandidateType::Host)
                | (CandidateType::Host, CandidateType::ServerReflexive)
        ) {
            // Send hole punch packets to create NAT bindings
            if let Err(e) = self
                .send_hole_punch_packets(local.address, remote.address)
                .await
            {
                tracing::debug!(
                    "Hole punch packets failed for {:?} -> {:?}: {}",
                    local.address,
                    remote.address,
                    e
                );
                // Don't fail immediately - handshake might still succeed
            }

            // Brief delay to allow NAT bindings to stabilize
            tokio::time::sleep(Duration::from_millis(50)).await;
        }

        // Attempt to establish session using the remote candidate's address
        // We derive a temporary peer ID from the remote address for the connection attempt
        // The real peer ID will be discovered during the Noise handshake
        let temp_peer_id = {
            let addr_bytes = remote.address.to_string();
            let hash = blake3::hash(addr_bytes.as_bytes());
            *hash.as_bytes()
        };

        match self
            .establish_session_with_addr(&temp_peer_id, remote.address)
            .await
        {
            Ok(session_id) => {
                // Session established successfully
                // Retrieve the actual peer ID that was discovered during handshake
                // Find the connection by session_id in the sessions map
                if let Some(entry) = self
                    .inner
                    .sessions
                    .iter()
                    .find(|e| e.value().session_id == session_id)
                {
                    let conn_arc = entry.value();
                    tracing::info!(
                        "Candidate connection successful: {:?} -> {:?} (session: {})",
                        local.address,
                        remote.address,
                        hex::encode(&session_id[..8])
                    );

                    // Clone the PeerConnection (shares Arc references)
                    Ok((**conn_arc).clone())
                } else {
                    Err(NodeError::NatTraversal(
                        "Session established but connection not found".into(),
                    ))
                }
            }
            Err(e) => {
                tracing::trace!(
                    "Candidate connection failed {:?} -> {:?}: {}",
                    local.address,
                    remote.address,
                    e
                );
                Err(NodeError::NatTraversal(
                    format!("Candidate connection failed: {e}").into(),
                ))
            }
        }
    }

    /// Send simultaneous packets to punch hole
    ///
    /// Both peers send packets to each other's reflexive addresses
    /// to create temporary NAT bindings. Sends multiple packets with
    /// small delays to increase the likelihood of successful traversal.
    ///
    /// # Arguments
    ///
    /// * `_local_addr` - Local address to send from (currently unused - transport binds to configured address)
    /// * `remote_addr` - Remote address to send to
    async fn send_hole_punch_packets(
        &self,
        _local_addr: SocketAddr,
        remote_addr: SocketAddr,
    ) -> Result<(), NodeError> {
        tracing::trace!(
            "Sending hole punch packets to {} (creating NAT binding)",
            remote_addr
        );

        // Get transport layer
        let transport = self.get_transport().await?;

        // Send multiple small packets to increase chance of creating NAT binding
        // The packet content is minimal - just a marker to identify hole punch packets
        // Real handshake will follow if the binding is successful
        for i in 0..5 {
            // Send a small identification packet
            // Format: [0xFF, 0xFE, sequence_number, padding...]
            let packet = vec![0xFFu8, 0xFEu8, i, 0x00, 0x00, 0x00, 0x00, 0x00];

            match transport.send_to(&packet, remote_addr).await {
                Ok(sent) => {
                    tracing::trace!(
                        "Sent hole punch packet #{} ({} bytes) to {}",
                        i,
                        sent,
                        remote_addr
                    );
                }
                Err(e) => {
                    tracing::debug!(
                        "Failed to send hole punch packet #{} to {}: {}",
                        i,
                        remote_addr,
                        e
                    );
                    // Continue anyway - some packets may get through
                }
            }

            // Small delay between packets to space them out
            // This helps with different NAT timeout characteristics
            if i < 4 {
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
        }

        tracing::debug!("Completed sending 5 hole punch packets to {}", remote_addr);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ice_candidate_creation() {
        let candidate = IceCandidate {
            address: "192.168.1.100:8420".parse().unwrap(),
            candidate_type: CandidateType::Host,
            priority: 126,
            foundation: "host-192.168.1.100:8420".to_string(),
        };

        assert_eq!(candidate.candidate_type, CandidateType::Host);
        assert_eq!(candidate.priority, 126);
    }

    #[test]
    fn test_candidate_type_equality() {
        assert_eq!(CandidateType::Host, CandidateType::Host);
        assert_ne!(CandidateType::Host, CandidateType::ServerReflexive);
        assert_ne!(CandidateType::ServerReflexive, CandidateType::Relayed);
    }

    #[tokio::test]
    async fn test_detect_nat_type() {
        let node = Node::new_random_with_port(0).await.unwrap();
        node.start().await.unwrap();

        let result = node.detect_nat_type().await;

        assert!(result.is_ok());
        // Should return None when NAT detection hasn't run or failed
        assert_eq!(result.unwrap(), NatType::None);

        node.stop().await.unwrap();
    }

    #[tokio::test]
    async fn test_gather_ice_candidates() {
        let node = Node::new_random_with_port(0).await.unwrap();
        let result = node.gather_ice_candidates().await;

        assert!(result.is_ok());
        let candidates = result.unwrap();

        // Should at least have host candidate
        assert!(!candidates.is_empty());
        assert!(
            candidates
                .iter()
                .any(|c| c.candidate_type == CandidateType::Host)
        );
    }

    #[tokio::test]
    async fn test_prioritize_candidates() {
        let node = Node::new_random_with_port(0).await.unwrap();

        let local = vec![
            IceCandidate {
                address: "192.168.1.100:8420".parse().unwrap(),
                candidate_type: CandidateType::Host,
                priority: 126,
                foundation: "host-1".to_string(),
            },
            IceCandidate {
                address: "203.0.113.10:8420".parse().unwrap(),
                candidate_type: CandidateType::ServerReflexive,
                priority: 100,
                foundation: "srflx-1".to_string(),
            },
        ];

        let remote = vec![IceCandidate {
            address: "198.51.100.20:8420".parse().unwrap(),
            candidate_type: CandidateType::Host,
            priority: 126,
            foundation: "host-2".to_string(),
        }];

        let pairs = node.prioritize_candidates(&local, &remote);

        assert_eq!(pairs.len(), 2); // 2 local * 1 remote
        // First pair should have highest combined priority
        assert_eq!(pairs[0].0.candidate_type, CandidateType::Host);
    }

    #[tokio::test]
    async fn test_exchange_candidates() {
        let node = Node::new_random_with_port(0).await.unwrap();

        let peer = PeerInfo {
            peer_id: [42u8; 32],
            addresses: vec!["192.168.1.200:8420".parse().unwrap()],
            nat_type: NatType::None,
            capabilities: crate::node::discovery::NodeCapabilities::default(),
            last_seen: std::time::SystemTime::now(),
        };

        let local = vec![IceCandidate {
            address: "192.168.1.100:8420".parse().unwrap(),
            candidate_type: CandidateType::Host,
            priority: 126,
            foundation: "host-1".to_string(),
        }];

        let result = node.exchange_candidates(&peer, &local).await;
        assert!(result.is_ok());

        let remote = result.unwrap();
        assert_eq!(remote.len(), 1);
        assert_eq!(remote[0].address, peer.addresses[0]);
    }
}
