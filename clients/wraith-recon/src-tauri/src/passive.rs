//! Passive Reconnaissance Module
//!
//! This module provides passive traffic analysis capabilities that require no
//! active probing of targets. All operations are zero-touch from the target's
//! perspective.
//!
//! ## MITRE ATT&CK Mapping
//! - T1040: Network Sniffing
//! - T1046: Network Service Discovery (passive variant)

use crate::audit::{AuditCategory, AuditLevel, AuditManager, MitreReference};
use crate::error::{ReconError, Result};
use crate::scope::ScopeManager;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

/// Discovered network asset
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkAsset {
    /// IP address
    pub ip: String,
    /// MAC address (if discovered)
    pub mac: Option<String>,
    /// Hostname (if discovered)
    pub hostname: Option<String>,
    /// Open ports (discovered passively)
    pub ports: Vec<DiscoveredPort>,
    /// First seen timestamp
    pub first_seen: DateTime<Utc>,
    /// Last seen timestamp
    pub last_seen: DateTime<Utc>,
    /// Traffic volume (bytes observed)
    pub traffic_volume: u64,
    /// Connection count
    pub connection_count: u64,
    /// Operating system fingerprint (if detected)
    pub os_fingerprint: Option<String>,
    /// Protocols observed
    pub protocols: Vec<String>,
}

/// Discovered port information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveredPort {
    /// Port number
    pub port: u16,
    /// Protocol (TCP/UDP)
    pub protocol: String,
    /// Service name (if identified)
    pub service: Option<String>,
    /// Banner (if captured)
    pub banner: Option<String>,
    /// First seen
    pub first_seen: DateTime<Utc>,
    /// Last seen
    pub last_seen: DateTime<Utc>,
}

/// Traffic pattern analysis result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrafficPattern {
    /// Source IP
    pub source: String,
    /// Destination IP
    pub destination: String,
    /// Protocol
    pub protocol: String,
    /// Port (if applicable)
    pub port: Option<u16>,
    /// Packet count
    pub packet_count: u64,
    /// Byte count
    pub byte_count: u64,
    /// First observed
    pub first_observed: DateTime<Utc>,
    /// Last observed
    pub last_observed: DateTime<Utc>,
    /// Average packet size
    pub avg_packet_size: f64,
    /// Pattern classification
    pub classification: String,
}

/// Passive scan configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PassiveScanConfig {
    /// Interface to capture on
    pub interface: String,
    /// Capture filter (BPF syntax)
    pub filter: Option<String>,
    /// Maximum duration (seconds)
    pub max_duration_secs: u64,
    /// Maximum packets to capture
    pub max_packets: u64,
    /// Enable OS fingerprinting
    pub os_fingerprinting: bool,
    /// Enable banner grabbing (passive)
    pub banner_grabbing: bool,
}

impl Default for PassiveScanConfig {
    fn default() -> Self {
        Self {
            interface: String::new(),
            filter: None,
            max_duration_secs: 3600, // 1 hour
            max_packets: 1_000_000,
            os_fingerprinting: true,
            banner_grabbing: true,
        }
    }
}

/// Passive scan status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ScanStatus {
    /// Not started
    Idle,
    /// Currently running
    Running,
    /// Paused
    Paused,
    /// Completed
    Completed,
    /// Stopped by user
    Stopped,
    /// Error occurred
    Error(String),
}

/// Passive scan result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PassiveScanResult {
    /// Scan identifier
    pub scan_id: String,
    /// Scan status
    pub status: ScanStatus,
    /// Start time
    pub start_time: DateTime<Utc>,
    /// End time (if completed)
    pub end_time: Option<DateTime<Utc>>,
    /// Duration in seconds
    pub duration_secs: u64,
    /// Packets captured
    pub packets_captured: u64,
    /// Bytes captured
    pub bytes_captured: u64,
    /// Assets discovered
    pub assets_discovered: usize,
    /// Patterns identified
    pub patterns_identified: usize,
}

/// Passive reconnaissance manager
pub struct PassiveRecon {
    /// Scope manager reference
    scope: Arc<parking_lot::Mutex<ScopeManager>>,
    /// Audit manager reference
    audit: Arc<AuditManager>,
    /// Kill switch flag
    killed: Arc<AtomicBool>,
    /// Current scan configuration
    config: parking_lot::Mutex<Option<PassiveScanConfig>>,
    /// Current scan status
    status: parking_lot::Mutex<ScanStatus>,
    /// Current scan ID
    scan_id: parking_lot::Mutex<Option<String>>,
    /// Discovered assets
    assets: parking_lot::Mutex<HashMap<String, NetworkAsset>>,
    /// Traffic patterns
    patterns: parking_lot::Mutex<Vec<TrafficPattern>>,
    /// Packet counter
    packets_captured: AtomicU64,
    /// Byte counter
    bytes_captured: AtomicU64,
    /// Start time
    start_time: parking_lot::Mutex<Option<DateTime<Utc>>>,
}

impl PassiveRecon {
    /// Create a new passive reconnaissance manager
    pub fn new(
        scope: Arc<parking_lot::Mutex<ScopeManager>>,
        audit: Arc<AuditManager>,
        killed: Arc<AtomicBool>,
    ) -> Self {
        Self {
            scope,
            audit,
            killed,
            config: parking_lot::Mutex::new(None),
            status: parking_lot::Mutex::new(ScanStatus::Idle),
            scan_id: parking_lot::Mutex::new(None),
            assets: parking_lot::Mutex::new(HashMap::new()),
            patterns: parking_lot::Mutex::new(Vec::new()),
            packets_captured: AtomicU64::new(0),
            bytes_captured: AtomicU64::new(0),
            start_time: parking_lot::Mutex::new(None),
        }
    }

    /// Check if operations are allowed
    fn check_kill_switch(&self) -> Result<()> {
        if self.killed.load(Ordering::SeqCst) {
            Err(ReconError::KillSwitchActivated(
                "Operations halted".to_string(),
            ))
        } else {
            Ok(())
        }
    }

    /// Start a passive scan
    pub fn start_scan(&self, config: PassiveScanConfig) -> Result<String> {
        self.check_kill_switch()?;

        let mut status = self.status.lock();
        if matches!(*status, ScanStatus::Running) {
            return Err(ReconError::InvalidState("Scan already running".to_string()));
        }

        let scan_id = uuid::Uuid::new_v4().to_string();

        // Log the scan start
        self.audit.log(
            AuditLevel::Info,
            AuditCategory::Reconnaissance,
            "Started passive network scan",
            Some(&format!(
                "Interface: {}, Duration: {}s",
                config.interface, config.max_duration_secs
            )),
            None,
            Some(MitreReference {
                technique_id: "T1040".to_string(),
                technique_name: "Network Sniffing".to_string(),
                tactic: "Credential Access".to_string(),
            }),
        );

        *self.config.lock() = Some(config);
        *status = ScanStatus::Running;
        *self.scan_id.lock() = Some(scan_id.clone());
        *self.start_time.lock() = Some(Utc::now());

        // Reset counters
        self.packets_captured.store(0, Ordering::Relaxed);
        self.bytes_captured.store(0, Ordering::Relaxed);
        self.assets.lock().clear();
        self.patterns.lock().clear();

        Ok(scan_id)
    }

    /// Stop the current scan
    pub fn stop_scan(&self) -> Result<PassiveScanResult> {
        let mut status = self.status.lock();
        if !matches!(*status, ScanStatus::Running | ScanStatus::Paused) {
            return Err(ReconError::InvalidState("No scan running".to_string()));
        }

        *status = ScanStatus::Stopped;

        let scan_id = self.scan_id.lock().clone().unwrap_or_default();

        self.audit.info(
            AuditCategory::Reconnaissance,
            &format!("Stopped passive scan {}", scan_id),
        );

        self.get_scan_result()
    }

    /// Pause the current scan
    pub fn pause_scan(&self) -> Result<()> {
        let mut status = self.status.lock();
        if !matches!(*status, ScanStatus::Running) {
            return Err(ReconError::InvalidState("No scan running".to_string()));
        }

        *status = ScanStatus::Paused;

        self.audit
            .info(AuditCategory::Reconnaissance, "Paused passive scan");
        Ok(())
    }

    /// Resume a paused scan
    pub fn resume_scan(&self) -> Result<()> {
        self.check_kill_switch()?;

        let mut status = self.status.lock();
        if !matches!(*status, ScanStatus::Paused) {
            return Err(ReconError::InvalidState("No paused scan".to_string()));
        }

        *status = ScanStatus::Running;

        self.audit
            .info(AuditCategory::Reconnaissance, "Resumed passive scan");
        Ok(())
    }

    /// Process a captured packet (called from packet capture loop)
    pub fn process_packet(
        &self,
        source_ip: &str,
        dest_ip: &str,
        source_port: u16,
        dest_port: u16,
        protocol: &str,
        size: u64,
    ) -> Result<()> {
        self.check_kill_switch()?;

        // Verify at least one endpoint is in scope
        let scope = self.scope.lock();
        let source_in_scope = scope.validate_str(source_ip).in_scope;
        let dest_in_scope = scope.validate_str(dest_ip).in_scope;

        if !source_in_scope && !dest_in_scope {
            // Packet is entirely out of scope, skip
            return Ok(());
        }

        drop(scope);

        // Update counters
        self.packets_captured.fetch_add(1, Ordering::Relaxed);
        self.bytes_captured.fetch_add(size, Ordering::Relaxed);

        // Update assets
        let now = Utc::now();
        let mut assets = self.assets.lock();

        // Process source IP
        if source_in_scope {
            let asset = assets
                .entry(source_ip.to_string())
                .or_insert_with(|| NetworkAsset {
                    ip: source_ip.to_string(),
                    mac: None,
                    hostname: None,
                    ports: Vec::new(),
                    first_seen: now,
                    last_seen: now,
                    traffic_volume: 0,
                    connection_count: 0,
                    os_fingerprint: None,
                    protocols: Vec::new(),
                });
            asset.last_seen = now;
            asset.traffic_volume += size;
            asset.connection_count += 1;

            // Add protocol if not already present
            if !asset.protocols.contains(&protocol.to_string()) {
                asset.protocols.push(protocol.to_string());
            }

            // Add source port if not already tracked
            if source_port > 0 && !asset.ports.iter().any(|p| p.port == source_port) {
                asset.ports.push(DiscoveredPort {
                    port: source_port,
                    protocol: protocol.to_string(),
                    service: identify_service(source_port),
                    banner: None,
                    first_seen: now,
                    last_seen: now,
                });
            }
        }

        // Process destination IP
        if dest_in_scope {
            let asset = assets
                .entry(dest_ip.to_string())
                .or_insert_with(|| NetworkAsset {
                    ip: dest_ip.to_string(),
                    mac: None,
                    hostname: None,
                    ports: Vec::new(),
                    first_seen: now,
                    last_seen: now,
                    traffic_volume: 0,
                    connection_count: 0,
                    os_fingerprint: None,
                    protocols: Vec::new(),
                });
            asset.last_seen = now;
            asset.traffic_volume += size;

            // Add protocol if not already present
            if !asset.protocols.contains(&protocol.to_string()) {
                asset.protocols.push(protocol.to_string());
            }

            // Add destination port as a service port
            if dest_port > 0 {
                if !asset.ports.iter().any(|p| p.port == dest_port) {
                    asset.ports.push(DiscoveredPort {
                        port: dest_port,
                        protocol: protocol.to_string(),
                        service: identify_service(dest_port),
                        banner: None,
                        first_seen: now,
                        last_seen: now,
                    });
                } else {
                    // Update last seen
                    if let Some(port) = asset.ports.iter_mut().find(|p| p.port == dest_port) {
                        port.last_seen = now;
                    }
                }
            }
        }

        Ok(())
    }

    /// Get current scan result
    pub fn get_scan_result(&self) -> Result<PassiveScanResult> {
        let status = self.status.lock().clone();
        let scan_id = self.scan_id.lock().clone().unwrap_or_default();
        let start_time = self.start_time.lock().unwrap_or(Utc::now());
        let end_time = if matches!(
            status,
            ScanStatus::Completed | ScanStatus::Stopped | ScanStatus::Error(_)
        ) {
            Some(Utc::now())
        } else {
            None
        };

        let duration = (Utc::now() - start_time).num_seconds() as u64;

        Ok(PassiveScanResult {
            scan_id,
            status,
            start_time,
            end_time,
            duration_secs: duration,
            packets_captured: self.packets_captured.load(Ordering::Relaxed),
            bytes_captured: self.bytes_captured.load(Ordering::Relaxed),
            assets_discovered: self.assets.lock().len(),
            patterns_identified: self.patterns.lock().len(),
        })
    }

    /// Get discovered assets
    pub fn get_assets(&self) -> Vec<NetworkAsset> {
        self.assets.lock().values().cloned().collect()
    }

    /// Get traffic patterns
    pub fn get_patterns(&self) -> Vec<TrafficPattern> {
        self.patterns.lock().clone()
    }

    /// Get current status
    pub fn status(&self) -> ScanStatus {
        self.status.lock().clone()
    }
}

/// Identify common services by port number
fn identify_service(port: u16) -> Option<String> {
    match port {
        20 => Some("FTP Data".to_string()),
        21 => Some("FTP".to_string()),
        22 => Some("SSH".to_string()),
        23 => Some("Telnet".to_string()),
        25 => Some("SMTP".to_string()),
        53 => Some("DNS".to_string()),
        67 | 68 => Some("DHCP".to_string()),
        69 => Some("TFTP".to_string()),
        80 => Some("HTTP".to_string()),
        110 => Some("POP3".to_string()),
        119 => Some("NNTP".to_string()),
        123 => Some("NTP".to_string()),
        135 => Some("MSRPC".to_string()),
        137..=139 => Some("NetBIOS".to_string()),
        143 => Some("IMAP".to_string()),
        161 | 162 => Some("SNMP".to_string()),
        389 => Some("LDAP".to_string()),
        443 => Some("HTTPS".to_string()),
        445 => Some("SMB".to_string()),
        465 => Some("SMTPS".to_string()),
        514 => Some("Syslog".to_string()),
        515 => Some("LPD".to_string()),
        587 => Some("Submission".to_string()),
        636 => Some("LDAPS".to_string()),
        993 => Some("IMAPS".to_string()),
        995 => Some("POP3S".to_string()),
        1433 => Some("MSSQL".to_string()),
        1521 => Some("Oracle".to_string()),
        3306 => Some("MySQL".to_string()),
        3389 => Some("RDP".to_string()),
        5432 => Some("PostgreSQL".to_string()),
        5900..=5999 => Some("VNC".to_string()),
        6379 => Some("Redis".to_string()),
        8080 => Some("HTTP Proxy".to_string()),
        8443 => Some("HTTPS Alt".to_string()),
        9200 => Some("Elasticsearch".to_string()),
        27017 => Some("MongoDB".to_string()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_recon() -> PassiveRecon {
        let mut scope = ScopeManager::new();
        scope.add_cidr("192.168.1.0/24").unwrap();

        let audit = Arc::new(AuditManager::new("test-operator".to_string()));
        let killed = Arc::new(AtomicBool::new(false));

        PassiveRecon::new(Arc::new(parking_lot::Mutex::new(scope)), audit, killed)
    }

    #[test]
    fn test_passive_recon_creation() {
        let recon = create_test_recon();
        assert!(matches!(recon.status(), ScanStatus::Idle));
    }

    #[test]
    fn test_start_scan() {
        let recon = create_test_recon();
        let config = PassiveScanConfig {
            interface: "eth0".to_string(),
            ..Default::default()
        };

        let scan_id = recon.start_scan(config).unwrap();
        assert!(!scan_id.is_empty());
        assert!(matches!(recon.status(), ScanStatus::Running));
    }

    #[test]
    fn test_pause_resume_scan() {
        let recon = create_test_recon();
        let config = PassiveScanConfig::default();

        recon.start_scan(config).unwrap();
        recon.pause_scan().unwrap();
        assert!(matches!(recon.status(), ScanStatus::Paused));

        recon.resume_scan().unwrap();
        assert!(matches!(recon.status(), ScanStatus::Running));
    }

    #[test]
    fn test_process_packet() {
        let recon = create_test_recon();
        let config = PassiveScanConfig::default();
        recon.start_scan(config).unwrap();

        // Process some packets
        recon
            .process_packet("192.168.1.100", "192.168.1.1", 54321, 80, "TCP", 100)
            .unwrap();
        recon
            .process_packet("192.168.1.100", "192.168.1.1", 54322, 443, "TCP", 200)
            .unwrap();

        let assets = recon.get_assets();
        assert!(!assets.is_empty());

        // Find the asset
        let asset = assets.iter().find(|a| a.ip == "192.168.1.100").unwrap();
        assert!(asset.traffic_volume > 0);
    }

    #[test]
    fn test_out_of_scope_packet_filtered() {
        let recon = create_test_recon();
        let config = PassiveScanConfig::default();
        recon.start_scan(config).unwrap();

        // Process packet entirely out of scope
        recon
            .process_packet("10.0.0.1", "10.0.0.2", 54321, 80, "TCP", 100)
            .unwrap();

        let assets = recon.get_assets();
        assert!(assets.is_empty());
    }

    #[test]
    fn test_service_identification() {
        assert_eq!(identify_service(22), Some("SSH".to_string()));
        assert_eq!(identify_service(80), Some("HTTP".to_string()));
        assert_eq!(identify_service(443), Some("HTTPS".to_string()));
        assert_eq!(identify_service(3389), Some("RDP".to_string()));
        assert_eq!(identify_service(12345), None);
    }

    #[test]
    fn test_scan_result() {
        let recon = create_test_recon();
        let config = PassiveScanConfig::default();
        recon.start_scan(config).unwrap();

        recon
            .process_packet("192.168.1.100", "192.168.1.1", 54321, 80, "TCP", 100)
            .unwrap();

        let result = recon.get_scan_result().unwrap();
        assert!(matches!(result.status, ScanStatus::Running));
        assert_eq!(result.packets_captured, 1);
        assert_eq!(result.bytes_captured, 100);
        assert_eq!(result.assets_discovered, 2); // Both source and dest
    }

    #[test]
    fn test_kill_switch_stops_scan() {
        let mut scope = ScopeManager::new();
        scope.add_cidr("192.168.1.0/24").unwrap();

        let audit = Arc::new(AuditManager::new("test-operator".to_string()));
        let killed = Arc::new(AtomicBool::new(false));

        let recon = PassiveRecon::new(
            Arc::new(parking_lot::Mutex::new(scope)),
            audit,
            killed.clone(),
        );

        let config = PassiveScanConfig::default();
        recon.start_scan(config).unwrap();

        // Activate kill switch
        killed.store(true, Ordering::SeqCst);

        // Processing should fail
        let result = recon.process_packet("192.168.1.100", "192.168.1.1", 54321, 80, "TCP", 100);
        assert!(result.is_err());
    }
}
