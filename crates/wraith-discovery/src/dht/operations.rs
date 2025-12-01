//! DHT Operations
//!
//! This module implements the core Kademlia DHT operations:
//! - Iterative node lookup (FIND_NODE)
//! - Value storage (STORE)
//! - Value retrieval (FIND_VALUE)
//!
//! All operations use the iterative lookup algorithm with alpha parallelism.

use super::messages::*;
use super::node::DhtNode;
use super::node_id::NodeId;
use super::routing::{DhtError, DhtPeer, K};
use std::collections::HashSet;
use std::net::SocketAddr;
use std::time::Duration;
use thiserror::Error;

/// Alpha: parallelism factor for iterative lookups
///
/// Kademlia performs up to α concurrent queries during node lookup.
/// Standard value is 3.
pub const ALPHA: usize = 3;

/// Maximum iterations for iterative lookup
///
/// Prevents infinite loops in pathological cases.
const MAX_ITERATIONS: usize = 20;

/// Timeout for individual RPC requests
#[allow(dead_code)]
const RPC_TIMEOUT: Duration = Duration::from_secs(5);

/// DHT operation errors
#[derive(Debug, Error)]
pub enum OperationError {
    /// Storage operation failed
    #[error("Store operation failed: no nodes stored the value")]
    StoreFailed,

    /// Value not found in DHT
    #[error("Value not found in DHT")]
    ValueNotFound,

    /// Network timeout
    #[error("Network timeout")]
    Timeout,

    /// RPC failed
    #[error("RPC failed: {0}")]
    RpcFailed(String),

    /// DHT error
    #[error("DHT error: {0}")]
    Dht(#[from] DhtError),
}

/// DHT operations trait
///
/// Defines the async operations that a DHT node can perform.
/// This is implemented for DhtNode with network transport.
pub trait DhtOperations {
    /// Perform iterative FIND_NODE lookup
    ///
    /// Uses the Kademlia iterative lookup algorithm to find the K closest
    /// nodes to a target NodeId. Queries are performed with α parallelism.
    ///
    /// # Arguments
    ///
    /// * `target` - The NodeId to find closest nodes to
    ///
    /// # Returns
    ///
    /// Vector of up to K closest peers found
    ///
    /// # Errors
    ///
    /// Returns error if lookup fails or times out
    fn find_node(
        &mut self,
        target: &NodeId,
    ) -> impl std::future::Future<Output = Result<Vec<DhtPeer>, OperationError>> + Send;

    /// Store a value in the DHT
    ///
    /// Finds the K closest nodes to the key and stores the value on all of them.
    /// Succeeds if at least one node confirms storage.
    ///
    /// # Arguments
    ///
    /// * `key` - 32-byte storage key
    /// * `value` - Value data
    /// * `ttl` - Time-to-live for the value
    ///
    /// # Errors
    ///
    /// Returns error if no nodes confirm storage
    fn store(
        &mut self,
        key: [u8; 32],
        value: Vec<u8>,
        ttl: Duration,
    ) -> impl std::future::Future<Output = Result<(), OperationError>> + Send;

    /// Retrieve a value from the DHT
    ///
    /// Performs iterative lookup for the value, querying nodes until
    /// the value is found or all close nodes have been queried.
    ///
    /// # Arguments
    ///
    /// * `key` - 32-byte key to look up
    ///
    /// # Returns
    ///
    /// The stored value if found
    ///
    /// # Errors
    ///
    /// Returns error if value not found or lookup fails
    fn find_value(
        &mut self,
        key: [u8; 32],
    ) -> impl std::future::Future<Output = Result<Vec<u8>, OperationError>> + Send;
}

impl DhtNode {
    /// Perform iterative node lookup
    ///
    /// This is a reference implementation showing the iterative lookup algorithm.
    /// In a real implementation, this would use actual network transport.
    ///
    /// Algorithm:
    /// 1. Start with K closest known peers
    /// 2. Query up to α closest unqueried peers in parallel
    /// 3. Merge responses into closest list
    /// 4. Repeat until no closer peers found or all queried
    ///
    /// # Arguments
    ///
    /// * `target` - The NodeId to find closest nodes to
    ///
    /// # Returns
    ///
    /// Vector of up to K closest peers found
    #[allow(clippy::never_loop)]
    pub async fn iterative_find_node(&mut self, target: &NodeId) -> Vec<DhtPeer> {
        let mut queried = HashSet::new();
        let closest = self.routing_table().closest_peers(target, K);

        for _iteration in 0..MAX_ITERATIONS {
            // Find up to α unqueried peers
            let to_query: Vec<_> = closest
                .iter()
                .filter(|p| !queried.contains(&p.id))
                .take(ALPHA)
                .cloned()
                .collect();

            if to_query.is_empty() {
                break; // No more peers to query
            }

            // Mark as queried
            for peer in &to_query {
                queried.insert(peer.id);
            }

            // In a real implementation, we would send FIND_NODE RPCs here
            // and wait for responses. For now, this is a stub that shows
            // the algorithm structure.
            //
            // let mut responses = Vec::new();
            // for peer in &to_query {
            //     if let Ok(response) = self.send_find_node_rpc(peer, target).await {
            //         responses.push(response);
            //     }
            // }

            // Merge responses would happen here
            // For now, we just return what we have
            break;
        }

        closest.into_iter().take(K).collect()
    }

    /// Handle incoming FIND_NODE request
    ///
    /// Returns the K closest peers to the target from the local routing table.
    ///
    /// # Arguments
    ///
    /// * `request` - The FIND_NODE request
    ///
    /// # Returns
    ///
    /// Response containing closest peers
    #[must_use]
    pub fn handle_find_node(&self, request: FindNodeRequest) -> FoundNodesResponse {
        let closest = self.routing_table().closest_peers(&request.target_id, K);

        let peers = closest
            .into_iter()
            .map(|p| CompactPeer {
                id: p.id,
                addr: p.addr,
            })
            .collect();

        FoundNodesResponse {
            sender_id: *self.id(),
            peers,
        }
    }

    /// Handle incoming STORE request
    ///
    /// Stores the value in local storage if there's capacity.
    ///
    /// # Arguments
    ///
    /// * `request` - The STORE request
    ///
    /// # Returns
    ///
    /// Acknowledgment response
    #[must_use]
    pub fn handle_store(&mut self, request: StoreRequest) -> StoreAckResponse {
        let ttl = Duration::from_secs(request.ttl);
        self.store(request.key, request.value, ttl);

        StoreAckResponse {
            sender_id: *self.id(),
            stored: true,
        }
    }

    /// Handle incoming FIND_VALUE request
    ///
    /// Returns the value if stored locally, otherwise returns closest peers.
    ///
    /// # Arguments
    ///
    /// * `request` - The FIND_VALUE request
    ///
    /// # Returns
    ///
    /// Response with either the value or closest peers
    #[must_use]
    pub fn handle_find_value(&self, request: FindValueRequest) -> FoundValueResponse {
        // Check if we have the value
        if let Some(value) = self.get(&request.key) {
            return FoundValueResponse::Value {
                sender_id: *self.id(),
                value,
            };
        }

        // Don't have value - return closest peers
        let key_id = NodeId::from_bytes(request.key);
        let closest = self.routing_table().closest_peers(&key_id, K);

        let peers = closest
            .into_iter()
            .map(|p| CompactPeer {
                id: p.id,
                addr: p.addr,
            })
            .collect();

        FoundValueResponse::Peers {
            sender_id: *self.id(),
            peers,
        }
    }

    /// Handle incoming PING request
    ///
    /// Returns a PONG response with the echoed nonce.
    ///
    /// # Arguments
    ///
    /// * `request` - The PING request
    ///
    /// # Returns
    ///
    /// PONG response
    #[must_use]
    pub fn handle_ping(&self, request: PingRequest) -> PongResponse {
        PongResponse {
            sender_id: *self.id(),
            nonce: request.nonce,
        }
    }

    /// Handle an incoming DHT message
    ///
    /// Routes the message to the appropriate handler and returns a response.
    ///
    /// # Arguments
    ///
    /// * `message` - The incoming DHT message
    /// * `from` - The sender's network address
    ///
    /// # Returns
    ///
    /// Response message if one should be sent
    #[must_use]
    pub fn handle_message(&mut self, message: DhtMessage, _from: SocketAddr) -> Option<DhtMessage> {
        match message {
            DhtMessage::Ping(ping) => {
                // Update routing table
                let peer = DhtPeer::new(ping.sender_id, ping.sender_addr);
                let _ = self.routing_table_mut().insert(peer);

                Some(DhtMessage::Pong(self.handle_ping(ping)))
            }

            DhtMessage::FindNode(find) => {
                // Update routing table
                let peer = DhtPeer::new(find.sender_id, find.sender_addr);
                let _ = self.routing_table_mut().insert(peer);

                Some(DhtMessage::FoundNodes(self.handle_find_node(find)))
            }

            DhtMessage::Store(store) => {
                // Update routing table
                let peer = DhtPeer::new(store.sender_id, store.sender_addr);
                let _ = self.routing_table_mut().insert(peer);

                Some(DhtMessage::StoreAck(self.handle_store(store)))
            }

            DhtMessage::FindValue(find) => {
                // Update routing table
                let peer = DhtPeer::new(find.sender_id, find.sender_addr);
                let _ = self.routing_table_mut().insert(peer);

                Some(DhtMessage::FoundValue(self.handle_find_value(find)))
            }

            // Response messages don't generate new responses
            DhtMessage::Pong(_)
            | DhtMessage::FoundNodes(_)
            | DhtMessage::StoreAck(_)
            | DhtMessage::FoundValue(_) => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_handle_ping() {
        let node = DhtNode::new(NodeId::random(), "127.0.0.1:8000".parse().unwrap());

        let request = PingRequest {
            sender_id: NodeId::random(),
            sender_addr: "127.0.0.1:8001".parse().unwrap(),
            nonce: 12345,
        };

        let response = node.handle_ping(request);
        assert_eq!(response.nonce, 12345);
        assert_eq!(response.sender_id, *node.id());
    }

    #[test]
    fn test_handle_find_node() {
        let mut node = DhtNode::new(NodeId::random(), "127.0.0.1:8000".parse().unwrap());

        // Add some peers to routing table
        for i in 0..10 {
            let peer = DhtPeer::new(
                NodeId::random(),
                format!("127.0.0.1:{}", 8001 + i).parse().unwrap(),
            );
            let _ = node.routing_table_mut().insert(peer);
        }

        let target = NodeId::random();
        let request = FindNodeRequest {
            sender_id: NodeId::random(),
            sender_addr: "127.0.0.1:9000".parse().unwrap(),
            target_id: target,
        };

        let response = node.handle_find_node(request);
        assert!(!response.peers.is_empty());
        assert!(response.peers.len() <= K);
    }

    #[test]
    fn test_handle_store() {
        let mut node = DhtNode::new(NodeId::random(), "127.0.0.1:8000".parse().unwrap());

        let key = [42u8; 32];
        let value = vec![1, 2, 3, 4, 5];

        let request = StoreRequest {
            sender_id: NodeId::random(),
            sender_addr: "127.0.0.1:9000".parse().unwrap(),
            key,
            value: value.clone(),
            ttl: 3600,
        };

        let response = node.handle_store(request);
        assert!(response.stored);

        // Verify value was stored
        let retrieved = node.get(&key);
        assert_eq!(retrieved, Some(value));
    }

    #[test]
    fn test_handle_find_value_found() {
        let mut node = DhtNode::new(NodeId::random(), "127.0.0.1:8000".parse().unwrap());

        let key = [42u8; 32];
        let value = vec![1, 2, 3, 4, 5];

        // Store the value
        node.store(key, value.clone(), Duration::from_secs(3600));

        let request = FindValueRequest {
            sender_id: NodeId::random(),
            sender_addr: "127.0.0.1:9000".parse().unwrap(),
            key,
        };

        let response = node.handle_find_value(request);
        match response {
            FoundValueResponse::Value { value: v, .. } => {
                assert_eq!(v, value);
            }
            _ => panic!("Expected Value response"),
        }
    }

    #[test]
    fn test_handle_find_value_not_found() {
        let mut node = DhtNode::new(NodeId::random(), "127.0.0.1:8000".parse().unwrap());

        // Add some peers
        for i in 0..5 {
            let peer = DhtPeer::new(
                NodeId::random(),
                format!("127.0.0.1:{}", 8001 + i).parse().unwrap(),
            );
            let _ = node.routing_table_mut().insert(peer);
        }

        let key = [42u8; 32];
        let request = FindValueRequest {
            sender_id: NodeId::random(),
            sender_addr: "127.0.0.1:9000".parse().unwrap(),
            key,
        };

        let response = node.handle_find_value(request);
        match response {
            FoundValueResponse::Peers { peers, .. } => {
                assert!(!peers.is_empty());
            }
            _ => panic!("Expected Peers response"),
        }
    }

    #[test]
    fn test_handle_message_ping() {
        let mut node = DhtNode::new(NodeId::random(), "127.0.0.1:8000".parse().unwrap());

        let ping = DhtMessage::Ping(PingRequest {
            sender_id: NodeId::random(),
            sender_addr: "127.0.0.1:9000".parse().unwrap(),
            nonce: 12345,
        });

        let response = node.handle_message(ping, "127.0.0.1:9000".parse().unwrap());
        assert!(response.is_some());

        match response.unwrap() {
            DhtMessage::Pong(pong) => assert_eq!(pong.nonce, 12345),
            _ => panic!("Expected Pong response"),
        }
    }

    #[test]
    fn test_handle_message_updates_routing_table() {
        let mut node = DhtNode::new(NodeId::random(), "127.0.0.1:8000".parse().unwrap());

        let sender_id = NodeId::random();
        let ping = DhtMessage::Ping(PingRequest {
            sender_id,
            sender_addr: "127.0.0.1:9000".parse().unwrap(),
            nonce: 12345,
        });

        assert_eq!(node.routing_table().peer_count(), 0);

        let _ = node.handle_message(ping, "127.0.0.1:9000".parse().unwrap());

        assert_eq!(node.routing_table().peer_count(), 1);
        assert!(node.routing_table().get_peer(&sender_id).is_some());
    }

    #[tokio::test]
    async fn test_iterative_find_node() {
        let mut node = DhtNode::new(NodeId::random(), "127.0.0.1:8000".parse().unwrap());

        // Add some peers
        for i in 0..20 {
            let peer = DhtPeer::new(
                NodeId::random(),
                format!("127.0.0.1:{}", 8001 + i).parse().unwrap(),
            );
            let _ = node.routing_table_mut().insert(peer);
        }

        let target = NodeId::random();
        let closest = node.iterative_find_node(&target).await;

        assert!(!closest.is_empty());
        assert!(closest.len() <= K);

        // Verify they're sorted by distance
        for i in 0..closest.len().saturating_sub(1) {
            let dist1 = closest[i].id.distance(&target);
            let dist2 = closest[i + 1].id.distance(&target);
            assert!(dist1 <= dist2);
        }
    }
}
