//! Application State Management for WRAITH Recon
//!
//! Manages global state including the engagement manager, database, and WRAITH node.

use crate::active::ActiveRecon;
use crate::audit::AuditManager;
use crate::channels::ChannelManager;
use crate::database::Database;
use crate::error::{ReconError, Result};
use crate::killswitch::KillSwitchManager;
use crate::passive::PassiveRecon;
use crate::roe::RulesOfEngagement;
use crate::scope::ScopeManager;
use crate::timing::TimingManager;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use tokio::sync::Mutex;
use wraith_core::node::{Node, NodeConfig};

/// WRAITH node wrapper for thread-safe access
pub struct WraithNode {
    /// The actual WRAITH node
    node: Option<Node>,
    /// Whether the node is running
    running: bool,
}

impl WraithNode {
    /// Create a new uninitialized WRAITH node wrapper
    pub fn new() -> Self {
        Self {
            node: None,
            running: false,
        }
    }

    /// Initialize the WRAITH node with default configuration
    pub async fn initialize(&mut self) -> Result<()> {
        if self.node.is_some() {
            return Err(ReconError::WraithProtocol(
                "Node already initialized".into(),
            ));
        }

        let config = NodeConfig::default();
        let node = Node::new_with_config(config)
            .await
            .map_err(|e| ReconError::WraithProtocol(format!("Failed to create node: {}", e)))?;

        self.node = Some(node);
        Ok(())
    }

    /// Initialize the WRAITH node with custom configuration
    pub async fn initialize_with_config(&mut self, config: NodeConfig) -> Result<()> {
        if self.node.is_some() {
            return Err(ReconError::WraithProtocol(
                "Node already initialized".into(),
            ));
        }

        let node = Node::new_with_config(config)
            .await
            .map_err(|e| ReconError::WraithProtocol(format!("Failed to create node: {}", e)))?;

        self.node = Some(node);
        Ok(())
    }

    /// Start the WRAITH node
    pub async fn start(&mut self) -> Result<()> {
        let node = self
            .node
            .as_ref()
            .ok_or_else(|| ReconError::WraithProtocol("Node not initialized".into()))?;

        node.start()
            .await
            .map_err(|e| ReconError::WraithProtocol(format!("Failed to start node: {}", e)))?;

        self.running = true;
        Ok(())
    }

    /// Stop the WRAITH node
    pub async fn stop(&mut self) -> Result<()> {
        let node = self
            .node
            .as_ref()
            .ok_or_else(|| ReconError::WraithProtocol("Node not initialized".into()))?;

        node.stop()
            .await
            .map_err(|e| ReconError::WraithProtocol(format!("Failed to stop node: {}", e)))?;

        self.running = false;
        Ok(())
    }

    /// Check if the node is running
    pub fn is_running(&self) -> bool {
        self.running && self.node.as_ref().is_some_and(|n| n.is_running())
    }

    /// Get the node's peer ID (32-byte Ed25519 public key as hex string)
    pub fn peer_id(&self) -> Option<String> {
        self.node.as_ref().map(|n| hex::encode(n.node_id()))
    }

    /// Get the node's peer ID as raw bytes
    pub fn peer_id_bytes(&self) -> Option<[u8; 32]> {
        self.node.as_ref().map(|n| *n.node_id())
    }

    /// Get access to the underlying node for advanced operations
    pub fn node(&self) -> Option<&Node> {
        self.node.as_ref()
    }

    /// Get the number of active sessions
    pub fn active_route_count(&self) -> usize {
        self.node.as_ref().map_or(0, |n| n.active_route_count())
    }

    /// Establish a session with a peer
    pub async fn establish_session(&self, peer_id: &[u8; 32]) -> Result<[u8; 32]> {
        let node = self
            .node
            .as_ref()
            .ok_or_else(|| ReconError::WraithProtocol("Node not initialized".into()))?;

        let session_id = node.establish_session(peer_id).await.map_err(|e| {
            ReconError::WraithProtocol(format!("Failed to establish session: {}", e))
        })?;

        Ok(session_id)
    }

    /// Send data to a peer
    pub async fn send_data(&self, peer_id: &[u8; 32], data: &[u8]) -> Result<()> {
        let node = self
            .node
            .as_ref()
            .ok_or_else(|| ReconError::WraithProtocol("Node not initialized".into()))?;

        node.send_data(peer_id, data)
            .await
            .map_err(|e| ReconError::WraithProtocol(format!("Failed to send data: {}", e)))
    }

    /// Get the X25519 public key for key exchange
    pub fn x25519_public_key(&self) -> Option<[u8; 32]> {
        self.node.as_ref().map(|n| *n.x25519_public_key())
    }
}

impl Default for WraithNode {
    fn default() -> Self {
        Self::new()
    }
}

/// Engagement status
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum EngagementStatus {
    /// No engagement loaded
    NotLoaded,
    /// Engagement loaded but not started
    Ready,
    /// Engagement active
    Active,
    /// Engagement paused
    Paused,
    /// Engagement completed
    Completed,
    /// Engagement terminated (kill switch or error)
    Terminated,
}

impl std::fmt::Display for EngagementStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotLoaded => write!(f, "Not Loaded"),
            Self::Ready => write!(f, "Ready"),
            Self::Active => write!(f, "Active"),
            Self::Paused => write!(f, "Paused"),
            Self::Completed => write!(f, "Completed"),
            Self::Terminated => write!(f, "Terminated"),
        }
    }
}

/// Statistics tracker for reconnaissance operations
pub struct ReconStatistics {
    /// Total targets scanned
    targets_scanned: AtomicU64,
    /// Total ports discovered
    ports_discovered: AtomicU64,
    /// Total services identified
    services_identified: AtomicU64,
    /// Total bytes exfiltrated
    bytes_exfiltrated: AtomicU64,
    /// Total packets captured (passive)
    packets_captured: AtomicU64,
    /// Total scope violations blocked
    scope_violations: AtomicU64,
    /// Total audit entries
    audit_entries: AtomicU64,
    /// Total channel operations
    channel_operations: AtomicU64,
}

impl ReconStatistics {
    /// Create a new statistics tracker
    pub fn new() -> Self {
        Self {
            targets_scanned: AtomicU64::new(0),
            ports_discovered: AtomicU64::new(0),
            services_identified: AtomicU64::new(0),
            bytes_exfiltrated: AtomicU64::new(0),
            packets_captured: AtomicU64::new(0),
            scope_violations: AtomicU64::new(0),
            audit_entries: AtomicU64::new(0),
            channel_operations: AtomicU64::new(0),
        }
    }

    /// Record a target scan
    pub fn record_target_scanned(&self) {
        self.targets_scanned.fetch_add(1, Ordering::Relaxed);
    }

    /// Record discovered ports
    pub fn record_ports_discovered(&self, count: u64) {
        self.ports_discovered.fetch_add(count, Ordering::Relaxed);
    }

    /// Record identified services
    pub fn record_services_identified(&self, count: u64) {
        self.services_identified.fetch_add(count, Ordering::Relaxed);
    }

    /// Record bytes exfiltrated
    pub fn record_bytes_exfiltrated(&self, bytes: u64) {
        self.bytes_exfiltrated.fetch_add(bytes, Ordering::Relaxed);
    }

    /// Record packets captured
    pub fn record_packets_captured(&self, count: u64) {
        self.packets_captured.fetch_add(count, Ordering::Relaxed);
    }

    /// Record a scope violation
    pub fn record_scope_violation(&self) {
        self.scope_violations.fetch_add(1, Ordering::Relaxed);
    }

    /// Record an audit entry
    pub fn record_audit_entry(&self) {
        self.audit_entries.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a channel operation
    pub fn record_channel_operation(&self) {
        self.channel_operations.fetch_add(1, Ordering::Relaxed);
    }

    /// Get total targets scanned
    pub fn targets_scanned(&self) -> u64 {
        self.targets_scanned.load(Ordering::Relaxed)
    }

    /// Get total ports discovered
    pub fn ports_discovered(&self) -> u64 {
        self.ports_discovered.load(Ordering::Relaxed)
    }

    /// Get total services identified
    pub fn services_identified(&self) -> u64 {
        self.services_identified.load(Ordering::Relaxed)
    }

    /// Get total bytes exfiltrated
    pub fn bytes_exfiltrated(&self) -> u64 {
        self.bytes_exfiltrated.load(Ordering::Relaxed)
    }

    /// Get total packets captured
    pub fn packets_captured(&self) -> u64 {
        self.packets_captured.load(Ordering::Relaxed)
    }

    /// Get total scope violations
    pub fn scope_violations(&self) -> u64 {
        self.scope_violations.load(Ordering::Relaxed)
    }

    /// Get total audit entries
    pub fn audit_entries(&self) -> u64 {
        self.audit_entries.load(Ordering::Relaxed)
    }

    /// Get total channel operations
    pub fn channel_operations(&self) -> u64 {
        self.channel_operations.load(Ordering::Relaxed)
    }

    /// Get all statistics as a summary
    pub fn summary(&self) -> StatisticsSummary {
        StatisticsSummary {
            targets_scanned: self.targets_scanned(),
            ports_discovered: self.ports_discovered(),
            services_identified: self.services_identified(),
            bytes_exfiltrated: self.bytes_exfiltrated(),
            packets_captured: self.packets_captured(),
            scope_violations: self.scope_violations(),
            audit_entries: self.audit_entries(),
            channel_operations: self.channel_operations(),
        }
    }
}

impl Default for ReconStatistics {
    fn default() -> Self {
        Self::new()
    }
}

/// Statistics summary for serialization
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct StatisticsSummary {
    pub targets_scanned: u64,
    pub ports_discovered: u64,
    pub services_identified: u64,
    pub bytes_exfiltrated: u64,
    pub packets_captured: u64,
    pub scope_violations: u64,
    pub audit_entries: u64,
    pub channel_operations: u64,
}

/// Global application state for WRAITH Recon
pub struct AppState {
    /// Database connection
    pub db: Arc<parking_lot::Mutex<Database>>,

    /// Current Rules of Engagement
    pub roe: Mutex<Option<RulesOfEngagement>>,

    /// Scope manager for target validation
    pub scope: Arc<parking_lot::Mutex<ScopeManager>>,

    /// Timing manager for engagement windows
    pub timing: Arc<parking_lot::Mutex<TimingManager>>,

    /// Kill switch manager
    pub kill_switch: Arc<KillSwitchManager>,

    /// Audit manager for tamper-evident logging
    pub audit: Arc<AuditManager>,

    /// Channel manager for exfiltration
    pub channels: Mutex<Option<ChannelManager>>,

    /// Passive reconnaissance module
    pub passive_recon: Mutex<Option<PassiveRecon>>,

    /// Active reconnaissance module
    pub active_recon: Mutex<Option<ActiveRecon>>,

    /// WRAITH protocol node
    pub node: Arc<Mutex<WraithNode>>,

    /// Current engagement status
    pub status: Arc<parking_lot::Mutex<EngagementStatus>>,

    /// Current engagement ID
    pub engagement_id: Mutex<Option<String>>,

    /// Operator ID
    pub operator_id: Mutex<String>,

    /// Statistics tracker
    pub statistics: Arc<ReconStatistics>,

    /// Global killed flag (shared with all modules)
    pub killed: Arc<AtomicBool>,
}

impl AppState {
    /// Create new application state
    pub fn new(db: Database, operator_id: String) -> Self {
        let killed = Arc::new(AtomicBool::new(false));
        let scope = Arc::new(parking_lot::Mutex::new(ScopeManager::new()));
        let timing = Arc::new(parking_lot::Mutex::new(TimingManager::new()));
        let audit = Arc::new(AuditManager::new(operator_id.clone()));
        let kill_switch = Arc::new(KillSwitchManager::new());

        // Register kill switch callback to set killed flag
        let killed_clone = Arc::clone(&killed);
        kill_switch.on_shutdown(move || {
            killed_clone.store(true, Ordering::SeqCst);
        });

        Self {
            db: Arc::new(parking_lot::Mutex::new(db)),
            roe: Mutex::new(None),
            scope,
            timing,
            kill_switch,
            audit,
            channels: Mutex::new(None),
            passive_recon: Mutex::new(None),
            active_recon: Mutex::new(None),
            node: Arc::new(Mutex::new(WraithNode::new())),
            status: Arc::new(parking_lot::Mutex::new(EngagementStatus::NotLoaded)),
            engagement_id: Mutex::new(None),
            operator_id: Mutex::new(operator_id),
            statistics: Arc::new(ReconStatistics::new()),
            killed,
        }
    }

    /// Load Rules of Engagement
    pub async fn load_roe(&self, roe: RulesOfEngagement) -> Result<()> {
        // Verify RoE signature
        roe.verify_signature()?;

        // Validate RoE
        let validation = roe.validate();
        if !validation.valid {
            return Err(ReconError::InvalidRoE(validation.errors.join("; ")));
        }

        // Update scope manager
        {
            let mut scope = self.scope.lock();
            *scope = ScopeManager::from_roe(&roe)?;
        }

        // Update timing manager
        {
            let mut timing = self.timing.lock();
            timing.set_window(roe.start_time, roe.end_time)?;
        }

        // Update kill switch authorized operators
        for operator in &roe.authorized_operators {
            self.kill_switch.add_authorized_operator(operator.clone());
        }

        // Store in database
        {
            let db = self.db.lock();
            db.store_roe(&roe)?;
        }

        // Log the RoE load event
        let msg = format!("Loaded RoE: {} ({})", roe.title, roe.id);
        let entry = self
            .audit
            .info(crate::audit::AuditCategory::RulesOfEngagement, &msg);
        self.statistics.record_audit_entry();
        {
            let db = self.db.lock();
            db.store_audit_entry(&entry)?;
        }

        // Store RoE
        *self.roe.lock().await = Some(roe);
        *self.status.lock() = EngagementStatus::Ready;

        Ok(())
    }

    /// Start engagement
    pub async fn start_engagement(&self) -> Result<String> {
        // Check RoE is loaded
        let roe = self.roe.lock().await;
        let roe = roe.as_ref().ok_or(ReconError::RoENotLoaded)?;

        // Validate timing
        self.timing.lock().validate()?;

        // Validate kill switch
        self.kill_switch.validate()?;

        // Generate engagement ID
        let engagement_id = uuid::Uuid::new_v4().to_string();

        // Initialize channel manager
        {
            let mut channels = self.channels.lock().await;
            *channels = Some(ChannelManager::new(
                Arc::clone(&self.scope),
                Arc::clone(&self.audit),
                Arc::clone(&self.killed),
            ));
        }

        // Initialize passive recon
        {
            let mut passive = self.passive_recon.lock().await;
            *passive = Some(PassiveRecon::new(
                Arc::clone(&self.scope),
                Arc::clone(&self.audit),
                Arc::clone(&self.killed),
            ));
        }

        // Initialize active recon
        {
            let mut active = self.active_recon.lock().await;
            *active = Some(ActiveRecon::new(
                Arc::clone(&self.scope),
                Arc::clone(&self.audit),
                Arc::clone(&self.killed),
            ));
        }

        // Log engagement start
        let msg = format!("Engagement started: {} (RoE: {})", engagement_id, roe.id);
        let entry = self.audit.info(crate::audit::AuditCategory::System, &msg);
        self.statistics.record_audit_entry();
        {
            let db = self.db.lock();
            db.store_audit_entry(&entry)?;
        }

        // Update state
        *self.engagement_id.lock().await = Some(engagement_id.clone());
        *self.status.lock() = EngagementStatus::Active;

        Ok(engagement_id)
    }

    /// Stop engagement
    pub async fn stop_engagement(&self, reason: &str) -> Result<()> {
        // Log engagement stop
        let msg = format!("Engagement stopped: {}", reason);
        let entry = self.audit.info(crate::audit::AuditCategory::System, &msg);
        self.statistics.record_audit_entry();
        {
            let db = self.db.lock();
            db.store_audit_entry(&entry)?;
        }

        // Stop passive recon
        if let Some(passive) = self.passive_recon.lock().await.as_ref() {
            let _ = passive.stop_scan();
        }

        // Stop active recon
        if let Some(active) = self.active_recon.lock().await.as_ref() {
            let _ = active.stop_scan();
        }

        // Close all channels
        if let Some(channels) = self.channels.lock().await.as_ref() {
            channels.close_all()?;
        }

        // Update state
        *self.status.lock() = EngagementStatus::Completed;

        Ok(())
    }

    /// Activate kill switch
    pub async fn activate_kill_switch(&self, reason: &str) -> Result<()> {
        // Log kill switch activation
        let msg = format!("Kill switch activated: {}", reason);
        let entry = self
            .audit
            .emergency(crate::audit::AuditCategory::KillSwitch, &msg, reason);
        self.statistics.record_audit_entry();
        {
            let db = self.db.lock();
            db.store_audit_entry(&entry)?;
        }

        // Activate kill switch (sets killed flag via callback)
        let operator = self.operator_id.lock().await.clone();
        self.kill_switch.activate_manual(reason, &operator);

        // Stop all operations
        self.stop_engagement("Kill switch activated").await?;

        // Update state
        *self.status.lock() = EngagementStatus::Terminated;

        Ok(())
    }

    /// Check if operation is allowed
    pub fn check_operation_allowed(&self) -> Result<()> {
        // Check kill switch
        if self.killed.load(Ordering::SeqCst) {
            return Err(ReconError::KillSwitchActivated("Operation blocked".into()));
        }

        // Check timing
        self.timing.lock().validate()?;

        // Check status
        let status = *self.status.lock();
        if status != EngagementStatus::Active {
            return Err(ReconError::EngagementWindowViolation(format!(
                "Engagement not active (status: {})",
                status
            )));
        }

        Ok(())
    }

    /// Get current status
    pub fn get_status(&self) -> EngagementStatus {
        *self.status.lock()
    }

    /// Get statistics summary
    pub fn get_statistics(&self) -> StatisticsSummary {
        self.statistics.summary()
    }

    /// Get audit chain for verification
    pub fn get_audit_chain(
        &self,
        since_sequence: u64,
        limit: usize,
    ) -> Result<Vec<crate::audit::AuditEntry>> {
        let db = self.db.lock();
        db.get_audit_entries(since_sequence, limit)
    }

    /// Verify audit chain integrity
    pub fn verify_audit_chain(&self) -> Result<bool> {
        let result = self.audit.verify_chain();
        Ok(result.valid)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_recon_statistics() {
        let stats = ReconStatistics::new();

        stats.record_target_scanned();
        stats.record_target_scanned();
        assert_eq!(stats.targets_scanned(), 2);

        stats.record_ports_discovered(10);
        assert_eq!(stats.ports_discovered(), 10);

        stats.record_services_identified(5);
        assert_eq!(stats.services_identified(), 5);

        stats.record_bytes_exfiltrated(1024);
        assert_eq!(stats.bytes_exfiltrated(), 1024);

        stats.record_packets_captured(100);
        assert_eq!(stats.packets_captured(), 100);

        stats.record_scope_violation();
        assert_eq!(stats.scope_violations(), 1);

        stats.record_audit_entry();
        assert_eq!(stats.audit_entries(), 1);

        stats.record_channel_operation();
        assert_eq!(stats.channel_operations(), 1);
    }

    #[test]
    fn test_engagement_status_display() {
        assert_eq!(format!("{}", EngagementStatus::NotLoaded), "Not Loaded");
        assert_eq!(format!("{}", EngagementStatus::Active), "Active");
        assert_eq!(format!("{}", EngagementStatus::Terminated), "Terminated");
    }

    #[tokio::test]
    async fn test_app_state_creation() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let db = Database::open(&db_path).unwrap();

        let state = AppState::new(db, "test-operator".to_string());
        assert_eq!(state.get_status(), EngagementStatus::NotLoaded);
        assert!(!state.killed.load(Ordering::SeqCst));
    }

    #[test]
    fn test_wraith_node_default() {
        let node = WraithNode::new();
        assert!(!node.is_running());
        assert!(node.peer_id().is_none());
    }

    #[test]
    fn test_statistics_summary() {
        let stats = ReconStatistics::new();
        stats.record_target_scanned();
        stats.record_ports_discovered(5);

        let summary = stats.summary();
        assert_eq!(summary.targets_scanned, 1);
        assert_eq!(summary.ports_discovered, 5);
    }
}
