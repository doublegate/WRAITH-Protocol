//! DHT Propagation Tracking
//!
//! Monitors and reports content propagation status across the DHT network.
//! Provides real-time updates on replication and availability.

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{debug, info};

/// Propagation status for a piece of content
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PropagationState {
    /// Content is being uploaded
    Uploading,
    /// Content is being distributed to DHT nodes
    Propagating,
    /// Content has been confirmed on multiple nodes
    Confirmed,
    /// Propagation failed
    Failed,
}

impl PropagationState {
    pub fn as_str(&self) -> &'static str {
        match self {
            PropagationState::Uploading => "uploading",
            PropagationState::Propagating => "propagating",
            PropagationState::Confirmed => "confirmed",
            PropagationState::Failed => "failed",
        }
    }

    pub fn parse(s: &str) -> Self {
        match s {
            "uploading" => PropagationState::Uploading,
            "propagating" => PropagationState::Propagating,
            "confirmed" => PropagationState::Confirmed,
            "failed" => PropagationState::Failed,
            _ => PropagationState::Failed,
        }
    }
}

/// Detailed propagation status for a CID
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PropagationStatus {
    /// Content identifier
    pub cid: String,
    /// Current state
    pub state: PropagationState,
    /// Number of confirmed replicas
    pub replica_count: usize,
    /// Target replica count
    pub target_replicas: usize,
    /// Progress percentage (0-100)
    pub progress: u8,
    /// Estimated time to confirmation (seconds)
    pub eta_seconds: Option<u64>,
    /// Error message if failed
    pub error: Option<String>,
    /// Last update timestamp
    pub updated_at: i64,
    /// Start timestamp
    pub started_at: i64,
}

impl PropagationStatus {
    /// Create a new propagation status
    pub fn new(cid: String, target_replicas: usize) -> Self {
        let now = chrono::Utc::now().timestamp();
        Self {
            cid,
            state: PropagationState::Uploading,
            replica_count: 0,
            target_replicas,
            progress: 0,
            eta_seconds: None,
            error: None,
            updated_at: now,
            started_at: now,
        }
    }

    /// Check if propagation is complete
    pub fn is_complete(&self) -> bool {
        matches!(
            self.state,
            PropagationState::Confirmed | PropagationState::Failed
        )
    }

    /// Check if propagation succeeded
    pub fn is_success(&self) -> bool {
        self.state == PropagationState::Confirmed
    }
}

/// Propagation tracker for monitoring content distribution
pub struct PropagationTracker {
    statuses: Arc<RwLock<HashMap<String, PropagationStatus>>>,
    default_target_replicas: usize,
}

impl PropagationTracker {
    /// Create a new propagation tracker
    pub fn new(default_target_replicas: usize) -> Self {
        Self {
            statuses: Arc::new(RwLock::new(HashMap::new())),
            default_target_replicas,
        }
    }

    /// Start tracking propagation for a CID
    pub fn start(&self, cid: &str) -> PropagationStatus {
        let status = PropagationStatus::new(cid.to_string(), self.default_target_replicas);
        self.statuses
            .write()
            .insert(cid.to_string(), status.clone());
        info!("Started tracking propagation for: {}", cid);
        status
    }

    /// Update propagation state
    pub fn update_state(&self, cid: &str, state: PropagationState) -> Option<PropagationStatus> {
        let mut statuses = self.statuses.write();
        if let Some(status) = statuses.get_mut(cid) {
            status.state = state;
            status.updated_at = chrono::Utc::now().timestamp();
            debug!("Updated propagation state for {}: {:?}", cid, state);
            Some(status.clone())
        } else {
            None
        }
    }

    /// Update replica count
    pub fn update_replicas(&self, cid: &str, count: usize) -> Option<PropagationStatus> {
        let mut statuses = self.statuses.write();
        if let Some(status) = statuses.get_mut(cid) {
            status.replica_count = count;
            status.progress =
                ((count as f64 / status.target_replicas as f64) * 100.0).min(100.0) as u8;
            status.updated_at = chrono::Utc::now().timestamp();

            // Update state based on progress
            if count >= status.target_replicas {
                status.state = PropagationState::Confirmed;
                status.eta_seconds = None;
            } else if count > 0 {
                status.state = PropagationState::Propagating;
                // Estimate ETA based on current rate
                let elapsed = status.updated_at - status.started_at;
                if elapsed > 0 && count > 0 {
                    let rate = count as f64 / elapsed as f64;
                    let remaining = status.target_replicas - count;
                    status.eta_seconds = Some((remaining as f64 / rate) as u64);
                }
            }

            debug!(
                "Updated replica count for {}: {}/{}",
                cid, count, status.target_replicas
            );
            Some(status.clone())
        } else {
            None
        }
    }

    /// Mark propagation as failed
    pub fn mark_failed(&self, cid: &str, error: &str) -> Option<PropagationStatus> {
        let mut statuses = self.statuses.write();
        if let Some(status) = statuses.get_mut(cid) {
            status.state = PropagationState::Failed;
            status.error = Some(error.to_string());
            status.updated_at = chrono::Utc::now().timestamp();
            status.eta_seconds = None;
            info!("Propagation failed for {}: {}", cid, error);
            Some(status.clone())
        } else {
            None
        }
    }

    /// Get propagation status for a CID
    pub fn get(&self, cid: &str) -> Option<PropagationStatus> {
        self.statuses.read().get(cid).cloned()
    }

    /// Get all active (non-complete) propagations
    pub fn get_active(&self) -> Vec<PropagationStatus> {
        self.statuses
            .read()
            .values()
            .filter(|s| !s.is_complete())
            .cloned()
            .collect()
    }

    /// Get all propagation statuses
    pub fn get_all(&self) -> Vec<PropagationStatus> {
        self.statuses.read().values().cloned().collect()
    }

    /// Remove tracking for a CID
    pub fn remove(&self, cid: &str) -> Option<PropagationStatus> {
        self.statuses.write().remove(cid)
    }

    /// Clear all completed propagations
    pub fn clear_completed(&self) -> usize {
        let mut statuses = self.statuses.write();
        let to_remove: Vec<String> = statuses
            .iter()
            .filter(|(_, s)| s.is_complete())
            .map(|(k, _)| k.clone())
            .collect();

        let count = to_remove.len();
        for cid in to_remove {
            statuses.remove(&cid);
        }
        count
    }

    /// Simulate propagation progress (for development/testing)
    pub async fn simulate_propagation(&self, cid: &str) {
        self.start(cid);

        // Simulate upload phase
        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
        self.update_state(cid, PropagationState::Propagating);

        // Simulate replica discovery
        for i in 1..=self.default_target_replicas {
            tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;
            self.update_replicas(cid, i);
        }

        info!("Simulated propagation complete for: {}", cid);
    }
}

impl Default for PropagationTracker {
    fn default() -> Self {
        Self::new(3)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_propagation_status_new() {
        let status = PropagationStatus::new("test-cid".to_string(), 3);

        assert_eq!(status.cid, "test-cid");
        assert_eq!(status.state, PropagationState::Uploading);
        assert_eq!(status.replica_count, 0);
        assert_eq!(status.target_replicas, 3);
        assert_eq!(status.progress, 0);
    }

    #[test]
    fn test_tracker_start_and_get() {
        let tracker = PropagationTracker::new(3);

        let status = tracker.start("cid-1");
        assert_eq!(status.state, PropagationState::Uploading);

        let retrieved = tracker.get("cid-1").unwrap();
        assert_eq!(retrieved.cid, "cid-1");
    }

    #[test]
    fn test_tracker_update_replicas() {
        let tracker = PropagationTracker::new(3);
        tracker.start("cid-1");

        // Update to 1 replica
        let status = tracker.update_replicas("cid-1", 1).unwrap();
        assert_eq!(status.state, PropagationState::Propagating);
        assert_eq!(status.progress, 33);

        // Update to 3 replicas (target)
        let status = tracker.update_replicas("cid-1", 3).unwrap();
        assert_eq!(status.state, PropagationState::Confirmed);
        assert_eq!(status.progress, 100);
    }

    #[test]
    fn test_tracker_mark_failed() {
        let tracker = PropagationTracker::new(3);
        tracker.start("cid-1");

        let status = tracker.mark_failed("cid-1", "Network error").unwrap();
        assert_eq!(status.state, PropagationState::Failed);
        assert_eq!(status.error, Some("Network error".to_string()));
    }

    #[test]
    fn test_get_active() {
        let tracker = PropagationTracker::new(3);

        tracker.start("cid-1");
        tracker.start("cid-2");
        tracker.update_replicas("cid-2", 3); // Complete

        let active = tracker.get_active();
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].cid, "cid-1");
    }

    #[test]
    fn test_clear_completed() {
        let tracker = PropagationTracker::new(3);

        tracker.start("cid-1");
        tracker.start("cid-2");
        tracker.update_replicas("cid-1", 3); // Complete
        tracker.mark_failed("cid-2", "Error"); // Complete (failed)

        let cleared = tracker.clear_completed();
        assert_eq!(cleared, 2);
        assert!(tracker.get_all().is_empty());
    }

    #[tokio::test]
    async fn test_simulate_propagation() {
        let tracker = PropagationTracker::new(3);

        tracker.simulate_propagation("cid-1").await;

        let status = tracker.get("cid-1").unwrap();
        assert_eq!(status.state, PropagationState::Confirmed);
        assert_eq!(status.replica_count, 3);
    }
}
