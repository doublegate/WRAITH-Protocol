//! Tauri IPC Commands for WRAITH Recon
//!
//! This module provides all the IPC commands for the WRAITH Recon application,
//! including Rules of Engagement management, reconnaissance operations, channel
//! management, and audit logging.

use crate::active::{ActiveScanConfig, ProbeResult, ProbeType, ScanProgress};
use crate::audit::{AuditCategory, AuditEntry};
use crate::channels::{ChannelInfo, ChannelStats, ChannelType};
use crate::database::DatabaseStatistics;
use crate::killswitch::KillSwitchSignal;
use crate::passive::{NetworkAsset, PassiveScanConfig, ScanStatus};
use crate::roe::RulesOfEngagement;
use crate::scope::Target;
use crate::state::{AppState, EngagementStatus, StatisticsSummary};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::State;

// =============================================================================
// Response Types
// =============================================================================

/// Engagement status response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngagementStatusResponse {
    pub status: EngagementStatus,
    pub engagement_id: Option<String>,
    pub roe_id: Option<String>,
    pub roe_title: Option<String>,
    pub time_remaining_secs: Option<i64>,
    pub is_killed: bool,
}

/// Scope validation response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScopeValidationResponse {
    pub valid: bool,
    pub target: String,
    pub reason: Option<String>,
}

/// Node status response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeStatus {
    pub running: bool,
    pub peer_id: Option<String>,
    pub active_routes: usize,
}

/// Scan result response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanResult {
    pub scan_id: String,
    pub scan_type: String,
    pub targets_scanned: u32,
    pub assets_discovered: u32,
    pub ports_discovered: u32,
    pub services_identified: u32,
    pub duration_secs: f64,
}

/// Audit chain verification response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditVerificationResponse {
    pub valid: bool,
    pub entries_verified: u64,
    pub first_sequence: u64,
    pub last_sequence: u64,
    pub error: Option<String>,
}

// =============================================================================
// Rules of Engagement Commands
// =============================================================================

/// Load Rules of Engagement from JSON
#[tauri::command]
pub async fn load_roe(
    state: State<'_, Arc<AppState>>,
    roe_json: String,
) -> std::result::Result<String, String> {
    let roe: RulesOfEngagement =
        serde_json::from_str(&roe_json).map_err(|e| format!("Invalid RoE JSON: {}", e))?;

    state
        .load_roe(roe.clone())
        .await
        .map_err(|e| e.to_string())?;

    Ok(roe.id)
}

/// Load Rules of Engagement from file path
#[tauri::command]
pub async fn load_roe_file(
    state: State<'_, Arc<AppState>>,
    file_path: String,
) -> std::result::Result<String, String> {
    let contents = std::fs::read_to_string(&file_path)
        .map_err(|e| format!("Failed to read RoE file: {}", e))?;

    let roe: RulesOfEngagement =
        serde_json::from_str(&contents).map_err(|e| format!("Invalid RoE JSON: {}", e))?;

    state
        .load_roe(roe.clone())
        .await
        .map_err(|e| e.to_string())?;

    Ok(roe.id)
}

/// Get current RoE information
#[tauri::command]
pub async fn get_roe(
    state: State<'_, Arc<AppState>>,
) -> std::result::Result<Option<RulesOfEngagement>, String> {
    let roe = state.roe.lock().await;
    Ok(roe.clone())
}

/// Validate RoE signature
#[tauri::command]
pub async fn validate_roe(state: State<'_, Arc<AppState>>) -> std::result::Result<bool, String> {
    let roe = state.roe.lock().await;
    match &*roe {
        Some(roe) => {
            roe.verify_signature().map_err(|e| e.to_string())?;
            Ok(true)
        }
        None => Err("No RoE loaded".to_string()),
    }
}

// =============================================================================
// Engagement Commands
// =============================================================================

/// Start an engagement
#[tauri::command]
pub async fn start_engagement(
    state: State<'_, Arc<AppState>>,
) -> std::result::Result<String, String> {
    state.start_engagement().await.map_err(|e| e.to_string())
}

/// Stop the current engagement
#[tauri::command]
pub async fn stop_engagement(
    state: State<'_, Arc<AppState>>,
    reason: String,
) -> std::result::Result<(), String> {
    state
        .stop_engagement(&reason)
        .await
        .map_err(|e| e.to_string())
}

/// Pause the current engagement
#[tauri::command]
pub async fn pause_engagement(state: State<'_, Arc<AppState>>) -> std::result::Result<(), String> {
    // Log pause event
    let entry = state.audit.info(AuditCategory::System, "Engagement paused");
    state.statistics.record_audit_entry();
    {
        let db = state.db.lock();
        db.store_audit_entry(&entry).map_err(|e| e.to_string())?;
    }

    *state.status.lock() = EngagementStatus::Paused;
    Ok(())
}

/// Resume a paused engagement
#[tauri::command]
pub async fn resume_engagement(state: State<'_, Arc<AppState>>) -> std::result::Result<(), String> {
    // Check timing is still valid
    state.timing.lock().validate().map_err(|e| e.to_string())?;

    // Check kill switch
    state.kill_switch.validate().map_err(|e| e.to_string())?;

    // Log resume event
    let entry = state
        .audit
        .info(AuditCategory::System, "Engagement resumed");
    state.statistics.record_audit_entry();
    {
        let db = state.db.lock();
        db.store_audit_entry(&entry).map_err(|e| e.to_string())?;
    }

    *state.status.lock() = EngagementStatus::Active;
    Ok(())
}

/// Get current engagement status
#[tauri::command]
pub async fn get_engagement_status(
    state: State<'_, Arc<AppState>>,
) -> std::result::Result<EngagementStatusResponse, String> {
    let status = state.get_status();
    let engagement_id = state.engagement_id.lock().await.clone();
    let roe = state.roe.lock().await;

    let (roe_id, roe_title) = match &*roe {
        Some(roe) => (Some(roe.id.clone()), Some(roe.title.clone())),
        None => (None, None),
    };

    let time_remaining_secs = state
        .timing
        .lock()
        .time_remaining()
        .map(|d| d.num_seconds());

    let is_killed = state.killed.load(std::sync::atomic::Ordering::SeqCst);

    Ok(EngagementStatusResponse {
        status,
        engagement_id,
        roe_id,
        roe_title,
        time_remaining_secs,
        is_killed,
    })
}

// =============================================================================
// Scope Commands
// =============================================================================

/// Validate a target against the current scope
#[tauri::command]
pub async fn validate_target(
    state: State<'_, Arc<AppState>>,
    target: String,
) -> std::result::Result<ScopeValidationResponse, String> {
    let scope = state.scope.lock();

    let parsed_target: Target = target
        .parse()
        .map_err(|_| format!("Invalid target format: {}", target))?;

    match scope.validate(&parsed_target) {
        Ok(()) => Ok(ScopeValidationResponse {
            valid: true,
            target,
            reason: None,
        }),
        Err(e) => {
            state.statistics.record_scope_violation();
            Ok(ScopeValidationResponse {
                valid: false,
                target,
                reason: Some(e.to_string()),
            })
        }
    }
}

/// Add a custom target to scope
#[tauri::command]
pub async fn add_custom_target(
    state: State<'_, Arc<AppState>>,
    target: String,
) -> std::result::Result<(), String> {
    state.check_operation_allowed().map_err(|e| e.to_string())?;

    let parsed_target: Target = target
        .parse()
        .map_err(|_| format!("Invalid target format: {}", target))?;

    let mut scope = state.scope.lock();
    scope.add_custom_target(parsed_target);

    // Log scope change
    let msg = format!("Added custom target: {}", target);
    let entry = state.audit.info(AuditCategory::ScopeChange, &msg);
    state.statistics.record_audit_entry();
    {
        let db = state.db.lock();
        db.store_audit_entry(&entry).map_err(|e| e.to_string())?;
    }

    Ok(())
}

/// Get current scope summary
#[tauri::command]
pub async fn get_scope_summary(
    state: State<'_, Arc<AppState>>,
) -> std::result::Result<crate::scope::ScopeSummary, String> {
    let scope = state.scope.lock();
    Ok(scope.summary())
}

// =============================================================================
// Kill Switch Commands
// =============================================================================

/// Activate kill switch
#[tauri::command]
pub async fn activate_kill_switch(
    state: State<'_, Arc<AppState>>,
    reason: String,
) -> std::result::Result<(), String> {
    state
        .activate_kill_switch(&reason)
        .await
        .map_err(|e| e.to_string())
}

/// Process a signed kill switch signal
#[tauri::command]
pub async fn process_kill_switch_signal(
    state: State<'_, Arc<AppState>>,
    signal_json: String,
) -> std::result::Result<(), String> {
    let signal: KillSwitchSignal = serde_json::from_str(&signal_json)
        .map_err(|e| format!("Invalid kill switch signal: {}", e))?;

    // Verify and activate
    state
        .kill_switch
        .activate(signal.clone())
        .map_err(|e| e.to_string())?;

    // Log and stop engagement
    state
        .activate_kill_switch(&signal.reason)
        .await
        .map_err(|e| e.to_string())
}

/// Check if kill switch is active
#[tauri::command]
pub async fn is_kill_switch_active(
    state: State<'_, Arc<AppState>>,
) -> std::result::Result<bool, String> {
    Ok(state.killed.load(std::sync::atomic::Ordering::SeqCst))
}

// =============================================================================
// Passive Reconnaissance Commands
// =============================================================================

/// Start passive reconnaissance
#[tauri::command]
pub async fn start_passive_recon(
    state: State<'_, Arc<AppState>>,
    interface: Option<String>,
    capture_timeout_secs: Option<u64>,
) -> std::result::Result<String, String> {
    state.check_operation_allowed().map_err(|e| e.to_string())?;

    let config = PassiveScanConfig {
        interface: interface.unwrap_or_else(|| "any".to_string()),
        filter: None,
        max_duration_secs: capture_timeout_secs.unwrap_or(3600),
        max_packets: 1_000_000,
        os_fingerprinting: true,
        banner_grabbing: true,
    };

    let passive = state.passive_recon.lock().await;
    let passive = passive.as_ref().ok_or("Engagement not started")?;

    let scan_id = passive.start_scan(config).map_err(|e| e.to_string())?;

    // Log operation
    let msg = format!("Started passive reconnaissance: {}", scan_id);
    let entry = state.audit.info(AuditCategory::Reconnaissance, &msg);
    state.statistics.record_audit_entry();
    {
        let db = state.db.lock();
        db.store_audit_entry(&entry).map_err(|e| e.to_string())?;
    }

    Ok(scan_id)
}

/// Stop passive reconnaissance
#[tauri::command]
pub async fn stop_passive_recon(
    state: State<'_, Arc<AppState>>,
) -> std::result::Result<(), String> {
    let passive = state.passive_recon.lock().await;
    if let Some(passive) = passive.as_ref() {
        passive.stop_scan().map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Get passive recon statistics
#[tauri::command]
pub async fn get_passive_recon_stats(
    state: State<'_, Arc<AppState>>,
) -> std::result::Result<ScanStatus, String> {
    let passive = state.passive_recon.lock().await;
    let passive = passive.as_ref().ok_or("Passive recon not initialized")?;
    Ok(passive.status())
}

/// Get discovered assets from passive recon
#[tauri::command]
pub async fn get_discovered_assets(
    state: State<'_, Arc<AppState>>,
) -> std::result::Result<Vec<NetworkAsset>, String> {
    let passive = state.passive_recon.lock().await;
    let passive = passive.as_ref().ok_or("Passive recon not initialized")?;
    Ok(passive.get_assets())
}

// =============================================================================
// Active Reconnaissance Commands
// =============================================================================

/// Start active reconnaissance scan
#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub async fn start_active_scan(
    state: State<'_, Arc<AppState>>,
    targets: Vec<String>,
    port_range_start: u16,
    port_range_end: u16,
    probe_type: String,
    rate_limit_pps: Option<u32>,
    timeout_ms: Option<u64>,
) -> std::result::Result<String, String> {
    state.check_operation_allowed().map_err(|e| e.to_string())?;

    // Parse probe type
    let probe = match probe_type.to_lowercase().as_str() {
        "tcp_syn" | "syn" => ProbeType::TcpSyn,
        "tcp_connect" | "connect" => ProbeType::TcpConnect,
        "udp" => ProbeType::Udp,
        "icmp" | "ping" => ProbeType::IcmpEcho,
        "tcp_ack" | "ack" => ProbeType::TcpAck,
        "tcp_fin" | "fin" => ProbeType::TcpFin,
        _ => return Err(format!("Unknown probe type: {}", probe_type)),
    };

    // Build port list from range
    let ports: Vec<u16> = (port_range_start..=port_range_end).collect();

    let config = ActiveScanConfig {
        targets,
        ports,
        probe_type: probe,
        timeout_ms: timeout_ms.unwrap_or(3000),
        max_concurrent: rate_limit_pps.map(|r| r as usize).unwrap_or(1000),
        delay_ms: 0,
        jitter_ms: (0, 100),
        banner_grab: true,
        max_retries: 2,
    };

    let target_count = config.targets.len();

    let active = state.active_recon.lock().await;
    let active = active.as_ref().ok_or("Engagement not started")?;

    let scan_id = active.start_scan(config).map_err(|e| e.to_string())?;

    // Log operation with MITRE technique
    let msg = format!(
        "Started active scan: {} (targets: {})",
        scan_id, target_count
    );
    let entry = state.audit.log_recon(
        &msg,
        "multiple",
        "T1046",
        "Network Service Discovery",
        "Discovery",
    );
    state.statistics.record_audit_entry();
    {
        let db = state.db.lock();
        db.store_audit_entry(&entry).map_err(|e| e.to_string())?;
    }

    state.statistics.record_target_scanned();

    Ok(scan_id)
}

/// Stop active reconnaissance scan
#[tauri::command]
pub async fn stop_active_scan(state: State<'_, Arc<AppState>>) -> std::result::Result<(), String> {
    let active = state.active_recon.lock().await;
    if let Some(active) = active.as_ref() {
        active.stop_scan().map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Get active scan progress
#[tauri::command]
pub async fn get_active_scan_progress(
    state: State<'_, Arc<AppState>>,
) -> std::result::Result<ScanProgress, String> {
    let active = state.active_recon.lock().await;
    let active = active.as_ref().ok_or("Active recon not initialized")?;
    Ok(active.get_progress())
}

/// Get active scan results
#[tauri::command]
pub async fn get_active_scan_results(
    state: State<'_, Arc<AppState>>,
) -> std::result::Result<Vec<ProbeResult>, String> {
    let active = state.active_recon.lock().await;
    let active = active.as_ref().ok_or("Active recon not initialized")?;
    Ok(active.get_open_ports())
}

// =============================================================================
// Channel Commands
// =============================================================================

/// Open an exfiltration channel
#[tauri::command]
pub async fn open_channel(
    state: State<'_, Arc<AppState>>,
    channel_type: String,
    target: String,
    port: Option<u16>,
) -> std::result::Result<String, String> {
    state.check_operation_allowed().map_err(|e| e.to_string())?;

    // Parse channel type
    let ch_type = match channel_type.to_lowercase().as_str() {
        "udp" => ChannelType::Udp,
        "tcp" | "tcp_mimicry" => ChannelType::TcpMimicry,
        "https" | "https_encap" => ChannelType::HttpsEncap,
        "dns" | "dns_tunnel" => ChannelType::DnsTunnel,
        "icmp" => ChannelType::Icmp,
        "websocket" | "ws" => ChannelType::WebSocket,
        _ => return Err(format!("Unknown channel type: {}", channel_type)),
    };

    // Validate target format
    let _parsed_target: Target = target
        .parse()
        .map_err(|_| format!("Invalid target: {}", target))?;

    let channels = state.channels.lock().await;
    let channels = channels.as_ref().ok_or("Engagement not started")?;

    let channel_id = channels
        .open_channel(ch_type, &target, port)
        .map_err(|e| e.to_string())?;

    state.statistics.record_channel_operation();

    Ok(channel_id)
}

/// Send data through a channel
#[tauri::command]
pub async fn send_through_channel(
    state: State<'_, Arc<AppState>>,
    channel_id: String,
    data: Vec<u8>,
) -> std::result::Result<usize, String> {
    state.check_operation_allowed().map_err(|e| e.to_string())?;

    let channels = state.channels.lock().await;
    let channels = channels.as_ref().ok_or("Engagement not started")?;

    let bytes_sent = channels
        .send_test_data(&channel_id, &data)
        .map_err(|e| e.to_string())?;

    state.statistics.record_bytes_exfiltrated(bytes_sent as u64);
    state.statistics.record_channel_operation();

    Ok(bytes_sent)
}

/// Close a channel
#[tauri::command]
pub async fn close_channel(
    state: State<'_, Arc<AppState>>,
    channel_id: String,
) -> std::result::Result<(), String> {
    let channels = state.channels.lock().await;
    if let Some(channels) = channels.as_ref() {
        channels
            .close_channel(&channel_id)
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// List all open channels
#[tauri::command]
pub async fn list_channels(
    state: State<'_, Arc<AppState>>,
) -> std::result::Result<Vec<ChannelInfo>, String> {
    let channels = state.channels.lock().await;
    let channels = channels.as_ref().ok_or("Engagement not started")?;

    Ok(channels.get_all_channels())
}

/// Get channel statistics
#[tauri::command]
pub async fn get_channel_stats(
    state: State<'_, Arc<AppState>>,
    channel_id: String,
) -> std::result::Result<ChannelStats, String> {
    let channels = state.channels.lock().await;
    let channels = channels.as_ref().ok_or("Engagement not started")?;

    let info = channels
        .get_channel(&channel_id)
        .ok_or_else(|| format!("Channel not found: {}", channel_id))?;
    Ok(info.stats)
}

// =============================================================================
// Audit Commands
// =============================================================================

/// Get audit log entries
#[tauri::command]
pub async fn get_audit_entries(
    state: State<'_, Arc<AppState>>,
    since_sequence: u64,
    limit: usize,
) -> std::result::Result<Vec<AuditEntry>, String> {
    state
        .get_audit_chain(since_sequence, limit)
        .map_err(|e| e.to_string())
}

/// Verify audit chain integrity
#[tauri::command]
pub async fn verify_audit_chain(
    state: State<'_, Arc<AppState>>,
) -> std::result::Result<AuditVerificationResponse, String> {
    let entries = state.get_audit_chain(0, 10000).map_err(|e| e.to_string())?;

    if entries.is_empty() {
        return Ok(AuditVerificationResponse {
            valid: true,
            entries_verified: 0,
            first_sequence: 0,
            last_sequence: 0,
            error: None,
        });
    }

    let first_seq = entries.first().map(|e| e.sequence).unwrap_or(0);
    let last_seq = entries.last().map(|e| e.sequence).unwrap_or(0);

    match state.verify_audit_chain() {
        Ok(valid) => Ok(AuditVerificationResponse {
            valid,
            entries_verified: entries.len() as u64,
            first_sequence: first_seq,
            last_sequence: last_seq,
            error: if valid {
                None
            } else {
                Some("Chain verification failed".to_string())
            },
        }),
        Err(e) => Ok(AuditVerificationResponse {
            valid: false,
            entries_verified: entries.len() as u64,
            first_sequence: first_seq,
            last_sequence: last_seq,
            error: Some(e.to_string()),
        }),
    }
}

/// Export audit log
#[tauri::command]
pub async fn export_audit_log(
    state: State<'_, Arc<AppState>>,
    file_path: String,
) -> std::result::Result<u64, String> {
    let entries = state
        .get_audit_chain(0, 100000)
        .map_err(|e| e.to_string())?;
    let json = serde_json::to_string_pretty(&entries)
        .map_err(|e| format!("Failed to serialize audit log: {}", e))?;

    std::fs::write(&file_path, &json).map_err(|e| format!("Failed to write audit log: {}", e))?;

    Ok(entries.len() as u64)
}

/// Add a manual audit entry
#[tauri::command]
pub async fn add_audit_note(
    state: State<'_, Arc<AppState>>,
    summary: String,
    details: Option<String>,
) -> std::result::Result<String, String> {
    let mut entry = state.audit.info(AuditCategory::System, &summary);
    entry.details = details;

    state.statistics.record_audit_entry();
    {
        let db = state.db.lock();
        db.store_audit_entry(&entry).map_err(|e| e.to_string())?;
    }

    Ok(entry.id)
}

// =============================================================================
// Node Commands
// =============================================================================

/// Start the WRAITH node
#[tauri::command]
pub async fn start_node(state: State<'_, Arc<AppState>>) -> std::result::Result<(), String> {
    let mut node = state.node.lock().await;
    if node.node().is_none() {
        node.initialize().await.map_err(|e| e.to_string())?;
    }
    node.start().await.map_err(|e| e.to_string())
}

/// Stop the WRAITH node
#[tauri::command]
pub async fn stop_node(state: State<'_, Arc<AppState>>) -> std::result::Result<(), String> {
    let mut node = state.node.lock().await;
    node.stop().await.map_err(|e| e.to_string())
}

/// Get node status
#[tauri::command]
pub async fn get_node_status(
    state: State<'_, Arc<AppState>>,
) -> std::result::Result<NodeStatus, String> {
    let node = state.node.lock().await;
    Ok(NodeStatus {
        running: node.is_running(),
        peer_id: node.peer_id(),
        active_routes: node.active_route_count(),
    })
}

/// Get local peer ID
#[tauri::command]
pub async fn get_peer_id(
    state: State<'_, Arc<AppState>>,
) -> std::result::Result<Option<String>, String> {
    let node = state.node.lock().await;
    Ok(node.peer_id())
}

// =============================================================================
// Statistics Commands
// =============================================================================

/// Get runtime statistics
#[tauri::command]
pub async fn get_statistics(
    state: State<'_, Arc<AppState>>,
) -> std::result::Result<StatisticsSummary, String> {
    Ok(state.get_statistics())
}

/// Get database statistics
#[tauri::command]
pub async fn get_database_stats(
    state: State<'_, Arc<AppState>>,
) -> std::result::Result<DatabaseStatistics, String> {
    let db = state.db.lock();
    db.get_statistics().map_err(|e| e.to_string())
}

/// Get timing information
#[tauri::command]
pub async fn get_timing_info(
    state: State<'_, Arc<AppState>>,
) -> std::result::Result<crate::timing::TimingInfo, String> {
    let timing = state.timing.lock();
    Ok(timing.info())
}

// =============================================================================
// Operator Commands
// =============================================================================

/// Set operator ID
#[tauri::command]
pub async fn set_operator_id(
    state: State<'_, Arc<AppState>>,
    operator_id: String,
) -> std::result::Result<(), String> {
    *state.operator_id.lock().await = operator_id;
    Ok(())
}

/// Get operator ID
#[tauri::command]
pub async fn get_operator_id(
    state: State<'_, Arc<AppState>>,
) -> std::result::Result<String, String> {
    Ok(state.operator_id.lock().await.clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_engagement_status_response_serialization() {
        let response = EngagementStatusResponse {
            status: EngagementStatus::Active,
            engagement_id: Some("test-123".to_string()),
            roe_id: Some("roe-456".to_string()),
            roe_title: Some("Test Engagement".to_string()),
            time_remaining_secs: Some(3600),
            is_killed: false,
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("Active"));
        assert!(json.contains("test-123"));
    }

    #[test]
    fn test_scope_validation_response() {
        let response = ScopeValidationResponse {
            valid: true,
            target: "192.168.1.1".to_string(),
            reason: None,
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("192.168.1.1"));
        assert!(json.contains("true"));
    }

    #[test]
    fn test_node_status() {
        let status = NodeStatus {
            running: true,
            peer_id: Some("abc123".to_string()),
            active_routes: 5,
        };

        let json = serde_json::to_string(&status).unwrap();
        assert!(json.contains("running"));
        assert!(json.contains("abc123"));
    }
}
