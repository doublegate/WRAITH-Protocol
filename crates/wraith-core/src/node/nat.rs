//! NAT traversal integration for Node API
//!
//! Implements ICE-lite style NAT hole punching and relay fallback.

use crate::node::discovery::{NatType, PeerInfo};
use crate::node::session::PeerConnection;
use crate::node::{Node, NodeError};
use std::net::SocketAddr;
use std::time::Duration;

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
        tracing::debug!("Detecting NAT type via STUN");

        // TODO: Integrate with wraith-discovery::StunClient
        // For now, return a placeholder:
        //
        // let stun_servers = &self.inner.config.discovery.stun_servers;
        // if stun_servers.is_empty() {
        //     return Err(NodeError::NatTraversal("No STUN servers configured".to_string()));
        // }
        //
        // let stun_client = StunClient::new(stun_servers);
        // stun_client
        //     .detect_nat_type()
        //     .await
        //     .map_err(|e| NodeError::NatTraversal(e.to_string()))

        // Placeholder: assume no NAT for development
        Ok(NatType::None)
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
    async fn direct_connect(&self, peer: &PeerInfo) -> Result<PeerConnection, NodeError> {
        tracing::debug!("Direct connecting to peer {:?}", peer.peer_id);

        // Try each advertised address
        for addr in &peer.addresses {
            tracing::trace!("Trying address: {}", addr);

            // TODO: Integrate with wraith-transport
            // For now, create a mock connection:
            //
            // match self.transport.connect(*addr).await {
            //     Ok(conn) => {
            //         // Perform handshake
            //         let session = self.establish_session_with_transport(peer, conn).await?;
            //         return Ok(session);
            //     }
            //     Err(e) => {
            //         tracing::trace!("Failed to connect to {}: {}", addr, e);
            //         continue;
            //     }
            // }
        }

        Err(NodeError::NatTraversal(
            "All direct connection attempts failed".to_string(),
        ))
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

        Err(NodeError::NatTraversal(
            "All candidate pairs failed".to_string(),
        ))
    }

    /// Connect via relay server
    async fn connect_via_relay(&self, peer: &PeerInfo) -> Result<PeerConnection, NodeError> {
        tracing::debug!("Connecting via relay to peer {:?}", peer.peer_id);

        // TODO: Integrate with wraith-discovery::RelayManager
        // For now, return error:
        //
        // let relay = self.discovery_manager
        //     .find_relay()
        //     .await
        //     .map_err(|e| NodeError::Discovery(e.to_string()))?;
        //
        // let conn = self.transport
        //     .connect_via_relay(&relay, peer)
        //     .await
        //     .map_err(|e| NodeError::Transport(e.to_string()))?;
        //
        // Ok(PeerConnection::new_relayed(conn, peer.peer_id))

        Err(NodeError::NatTraversal(
            "Relay not yet implemented".to_string(),
        ))
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
                foundation: format!("host-{}", addr),
            });
        }

        // 2. Server reflexive candidates (STUN)
        // TODO: Integrate with STUN client
        // if let Ok(reflexive_addr) = self.get_reflexive_address().await {
        //     candidates.push(IceCandidate {
        //         address: reflexive_addr,
        //         candidate_type: CandidateType::ServerReflexive,
        //         priority: 100,
        //         foundation: format!("srflx-{}", reflexive_addr),
        //     });
        // }

        // 3. Relayed candidates (TURN)
        // TODO: Integrate with relay manager
        // if self.inner.config.discovery.enable_relay {
        //     if let Ok(relayed_addr) = self.get_relayed_address().await {
        //         candidates.push(IceCandidate {
        //             address: relayed_addr,
        //             candidate_type: CandidateType::Relayed,
        //             priority: 0,
        //             foundation: format!("relay-{}", relayed_addr),
        //         });
        //     }
        // }

        Ok(candidates)
    }

    /// Exchange ICE candidates with peer
    ///
    /// In a real implementation, this would use a signaling channel or relay.
    async fn exchange_candidates(
        &self,
        peer: &PeerInfo,
        _local_candidates: &[IceCandidate],
    ) -> Result<Vec<IceCandidate>, NodeError> {
        // TODO: Implement candidate exchange via signaling
        // For now, convert peer addresses to candidates
        let remote_candidates: Vec<IceCandidate> = peer
            .addresses
            .iter()
            .map(|addr| IceCandidate {
                address: *addr,
                candidate_type: CandidateType::Host,
                priority: 126,
                foundation: format!("host-{}", addr),
            })
            .collect();

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
    async fn try_connect_candidate(
        &self,
        _local: &IceCandidate,
        _remote: &IceCandidate,
    ) -> Result<PeerConnection, NodeError> {
        // TODO: Implement actual connection attempt
        // For now, return error
        Err(NodeError::NatTraversal(
            "Candidate connection not implemented".to_string(),
        ))
    }

    /// Send simultaneous packets to punch hole
    ///
    /// Both peers send packets to each other's reflexive addresses
    /// to create temporary NAT bindings.
    #[allow(dead_code)]
    async fn send_hole_punch_packets(
        &self,
        local_addr: SocketAddr,
        remote_addr: SocketAddr,
    ) -> Result<(), NodeError> {
        tracing::trace!(
            "Sending hole punch packets {} -> {}",
            local_addr,
            remote_addr
        );

        // TODO: Integrate with transport layer
        // Send multiple packets to increase chance of success
        for _i in 0..3 {
            // self.transport.send_raw(local_addr, remote_addr, &[_i as u8; 8]).await?;
            tokio::time::sleep(Duration::from_millis(10)).await;
        }

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
        let node = Node::new_random().await.unwrap();
        let result = node.detect_nat_type().await;

        assert!(result.is_ok());
        // Placeholder returns None
        assert_eq!(result.unwrap(), NatType::None);
    }

    #[tokio::test]
    async fn test_gather_ice_candidates() {
        let node = Node::new_random().await.unwrap();
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
        let node = Node::new_random().await.unwrap();

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
        let node = Node::new_random().await.unwrap();

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
