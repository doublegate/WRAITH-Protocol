//! Active Reconnaissance Module
//!
//! This module provides active probing capabilities for network reconnaissance.
//! All operations are subject to scope validation and timing constraints.
//!
//! ## MITRE ATT&CK Mapping
//! - T1046: Network Service Discovery
//! - T1018: Remote System Discovery

use crate::audit::{AuditCategory, AuditLevel, AuditManager, MitreReference};
use crate::error::{ReconError, Result};
use crate::scope::ScopeManager;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

/// Probe type enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProbeType {
    /// TCP SYN probe
    TcpSyn,
    /// TCP Connect probe
    TcpConnect,
    /// UDP probe
    Udp,
    /// ICMP Echo (ping)
    IcmpEcho,
    /// ICMP Timestamp
    IcmpTimestamp,
    /// TCP ACK probe
    TcpAck,
    /// TCP FIN probe
    TcpFin,
}

impl ProbeType {
    /// Get MITRE ATT&CK technique for this probe type
    pub fn mitre_technique(&self) -> MitreReference {
        MitreReference {
            technique_id: "T1046".to_string(),
            technique_name: "Network Service Discovery".to_string(),
            tactic: "Discovery".to_string(),
        }
    }
}

/// Probe result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProbeResult {
    /// Target address
    pub target: String,
    /// Target port
    pub port: u16,
    /// Probe type used
    pub probe_type: ProbeType,
    /// Whether the port is open
    pub open: bool,
    /// Response time in milliseconds
    pub response_time_ms: Option<u64>,
    /// Service detected (if any)
    pub service: Option<String>,
    /// Banner captured (if any)
    pub banner: Option<String>,
    /// Probe timestamp
    pub timestamp: DateTime<Utc>,
    /// Error message (if any)
    pub error: Option<String>,
}

/// Scan configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActiveScanConfig {
    /// Target hosts (IPs or CIDRs)
    pub targets: Vec<String>,
    /// Ports to scan
    pub ports: Vec<u16>,
    /// Probe type
    pub probe_type: ProbeType,
    /// Timeout per probe (milliseconds)
    pub timeout_ms: u64,
    /// Maximum concurrent probes
    pub max_concurrent: usize,
    /// Delay between probes (milliseconds)
    pub delay_ms: u64,
    /// Jitter range (milliseconds)
    pub jitter_ms: (u64, u64),
    /// Enable banner grabbing
    pub banner_grab: bool,
    /// Maximum retries per probe
    pub max_retries: u32,
}

impl Default for ActiveScanConfig {
    fn default() -> Self {
        Self {
            targets: Vec::new(),
            ports: vec![22, 80, 443, 8080],
            probe_type: ProbeType::TcpSyn,
            timeout_ms: 3000,
            max_concurrent: 100,
            delay_ms: 10,
            jitter_ms: (0, 50),
            banner_grab: true,
            max_retries: 1,
        }
    }
}

/// Scan progress information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanProgress {
    /// Total probes to send
    pub total_probes: u64,
    /// Probes completed
    pub probes_completed: u64,
    /// Open ports found
    pub open_ports_found: u64,
    /// Closed ports found
    pub closed_ports_found: u64,
    /// Filtered/no response
    pub filtered_ports: u64,
    /// Errors encountered
    pub errors: u64,
    /// Progress percentage
    pub progress_percent: f64,
    /// Estimated time remaining (seconds)
    pub eta_seconds: Option<u64>,
    /// Current scan rate (probes/second)
    pub scan_rate: f64,
}

/// Scan status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ActiveScanStatus {
    /// Not started
    Idle,
    /// Initializing
    Initializing,
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

/// Active scan result summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActiveScanResult {
    /// Scan identifier
    pub scan_id: String,
    /// Scan status
    pub status: ActiveScanStatus,
    /// Configuration used
    pub config: ActiveScanConfig,
    /// Start time
    pub start_time: DateTime<Utc>,
    /// End time (if completed)
    pub end_time: Option<DateTime<Utc>>,
    /// Duration in seconds
    pub duration_secs: u64,
    /// Progress information
    pub progress: ScanProgress,
    /// Individual probe results
    pub results: Vec<ProbeResult>,
}

/// Active reconnaissance manager
pub struct ActiveRecon {
    /// Scope manager reference
    scope: Arc<parking_lot::Mutex<ScopeManager>>,
    /// Audit manager reference
    audit: Arc<AuditManager>,
    /// Kill switch flag
    killed: Arc<AtomicBool>,
    /// Current scan status
    status: parking_lot::Mutex<ActiveScanStatus>,
    /// Current scan ID
    scan_id: parking_lot::Mutex<Option<String>>,
    /// Current configuration
    config: parking_lot::Mutex<Option<ActiveScanConfig>>,
    /// Probe results
    results: parking_lot::Mutex<HashMap<String, ProbeResult>>,
    /// Progress counters
    total_probes: AtomicU64,
    probes_completed: AtomicU64,
    open_ports: AtomicU64,
    closed_ports: AtomicU64,
    filtered_ports: AtomicU64,
    errors: AtomicU64,
    /// Start time
    start_time: parking_lot::Mutex<Option<DateTime<Utc>>>,
}

impl ActiveRecon {
    /// Create a new active reconnaissance manager
    pub fn new(
        scope: Arc<parking_lot::Mutex<ScopeManager>>,
        audit: Arc<AuditManager>,
        killed: Arc<AtomicBool>,
    ) -> Self {
        Self {
            scope,
            audit,
            killed,
            status: parking_lot::Mutex::new(ActiveScanStatus::Idle),
            scan_id: parking_lot::Mutex::new(None),
            config: parking_lot::Mutex::new(None),
            results: parking_lot::Mutex::new(HashMap::new()),
            total_probes: AtomicU64::new(0),
            probes_completed: AtomicU64::new(0),
            open_ports: AtomicU64::new(0),
            closed_ports: AtomicU64::new(0),
            filtered_ports: AtomicU64::new(0),
            errors: AtomicU64::new(0),
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

    /// Validate all targets are in scope
    fn validate_targets(&self, targets: &[String]) -> Result<()> {
        let scope = self.scope.lock();
        for target in targets {
            let result = scope.validate_str(target);
            if !result.in_scope {
                return Err(ReconError::TargetOutOfScope {
                    target: target.clone(),
                });
            }
        }
        Ok(())
    }

    /// Start an active scan
    pub fn start_scan(&self, config: ActiveScanConfig) -> Result<String> {
        self.check_kill_switch()?;

        // Validate all targets are in scope
        self.validate_targets(&config.targets)?;

        let mut status = self.status.lock();
        if matches!(*status, ActiveScanStatus::Running) {
            return Err(ReconError::InvalidState("Scan already running".to_string()));
        }

        let scan_id = uuid::Uuid::new_v4().to_string();

        // Calculate total probes
        let total = (config.targets.len() * config.ports.len()) as u64;

        // Log the scan start
        self.audit.log(
            AuditLevel::Info,
            AuditCategory::Reconnaissance,
            "Started active network scan",
            Some(&format!(
                "Targets: {}, Ports: {}, Type: {:?}",
                config.targets.len(),
                config.ports.len(),
                config.probe_type
            )),
            Some(&config.targets.join(", ")),
            Some(config.probe_type.mitre_technique()),
        );

        // Reset counters
        self.total_probes.store(total, Ordering::Relaxed);
        self.probes_completed.store(0, Ordering::Relaxed);
        self.open_ports.store(0, Ordering::Relaxed);
        self.closed_ports.store(0, Ordering::Relaxed);
        self.filtered_ports.store(0, Ordering::Relaxed);
        self.errors.store(0, Ordering::Relaxed);

        *self.config.lock() = Some(config);
        *status = ActiveScanStatus::Running;
        *self.scan_id.lock() = Some(scan_id.clone());
        *self.start_time.lock() = Some(Utc::now());
        self.results.lock().clear();

        Ok(scan_id)
    }

    /// Stop the current scan
    pub fn stop_scan(&self) -> Result<ActiveScanResult> {
        let mut status = self.status.lock();
        if !matches!(
            *status,
            ActiveScanStatus::Running | ActiveScanStatus::Paused
        ) {
            return Err(ReconError::InvalidState("No scan running".to_string()));
        }

        *status = ActiveScanStatus::Stopped;

        let scan_id = self.scan_id.lock().clone().unwrap_or_default();

        self.audit.info(
            AuditCategory::Reconnaissance,
            &format!("Stopped active scan {}", scan_id),
        );

        self.get_scan_result()
    }

    /// Pause the current scan
    pub fn pause_scan(&self) -> Result<()> {
        let mut status = self.status.lock();
        if !matches!(*status, ActiveScanStatus::Running) {
            return Err(ReconError::InvalidState("No scan running".to_string()));
        }

        *status = ActiveScanStatus::Paused;

        self.audit
            .info(AuditCategory::Reconnaissance, "Paused active scan");
        Ok(())
    }

    /// Resume a paused scan
    pub fn resume_scan(&self) -> Result<()> {
        self.check_kill_switch()?;

        let mut status = self.status.lock();
        if !matches!(*status, ActiveScanStatus::Paused) {
            return Err(ReconError::InvalidState("No paused scan".to_string()));
        }

        *status = ActiveScanStatus::Running;

        self.audit
            .info(AuditCategory::Reconnaissance, "Resumed active scan");
        Ok(())
    }

    /// Probe a single target:port
    pub fn probe(&self, target: &str, port: u16) -> Result<ProbeResult> {
        self.check_kill_switch()?;

        // Validate target is in scope
        {
            let scope = self.scope.lock();
            let result = scope.validate_str(target);
            if !result.in_scope {
                return Err(ReconError::TargetOutOfScope {
                    target: target.to_string(),
                });
            }
        }

        let config = self.config.lock();
        let probe_type = config
            .as_ref()
            .map(|c| c.probe_type)
            .unwrap_or(ProbeType::TcpConnect);

        let start = std::time::Instant::now();

        // TODO: Implement actual probing based on probe type
        // For now, simulate a probe
        let open = simulate_probe(target, port, probe_type);
        let response_time = start.elapsed().as_millis() as u64;

        let result = ProbeResult {
            target: target.to_string(),
            port,
            probe_type,
            open,
            response_time_ms: if open { Some(response_time) } else { None },
            service: if open { identify_service(port) } else { None },
            banner: None,
            timestamp: Utc::now(),
            error: None,
        };

        // Update counters
        self.probes_completed.fetch_add(1, Ordering::Relaxed);
        if open {
            self.open_ports.fetch_add(1, Ordering::Relaxed);
        } else {
            self.closed_ports.fetch_add(1, Ordering::Relaxed);
        }

        // Store result
        let key = format!("{}:{}", target, port);
        self.results.lock().insert(key, result.clone());

        // Log significant findings
        if open {
            self.audit.log(
                AuditLevel::Info,
                AuditCategory::Reconnaissance,
                &format!(
                    "Open port discovered: {}:{} ({})",
                    target,
                    port,
                    result.service.as_deref().unwrap_or("unknown")
                ),
                None,
                Some(target),
                Some(probe_type.mitre_technique()),
            );
        }

        Ok(result)
    }

    /// Get current progress
    pub fn get_progress(&self) -> ScanProgress {
        let total = self.total_probes.load(Ordering::Relaxed);
        let completed = self.probes_completed.load(Ordering::Relaxed);
        let progress_percent = if total > 0 {
            (completed as f64 / total as f64) * 100.0
        } else {
            0.0
        };

        let elapsed = self
            .start_time
            .lock()
            .map(|t| (Utc::now() - t).num_seconds() as f64)
            .unwrap_or(0.0);

        let scan_rate = if elapsed > 0.0 {
            completed as f64 / elapsed
        } else {
            0.0
        };

        let remaining = total.saturating_sub(completed);
        let eta = if scan_rate > 0.0 {
            Some((remaining as f64 / scan_rate) as u64)
        } else {
            None
        };

        ScanProgress {
            total_probes: total,
            probes_completed: completed,
            open_ports_found: self.open_ports.load(Ordering::Relaxed),
            closed_ports_found: self.closed_ports.load(Ordering::Relaxed),
            filtered_ports: self.filtered_ports.load(Ordering::Relaxed),
            errors: self.errors.load(Ordering::Relaxed),
            progress_percent,
            eta_seconds: eta,
            scan_rate,
        }
    }

    /// Get scan result
    pub fn get_scan_result(&self) -> Result<ActiveScanResult> {
        let status = self.status.lock().clone();
        let scan_id = self.scan_id.lock().clone().unwrap_or_default();
        let config = self.config.lock().clone().unwrap_or_default();
        let start_time = self.start_time.lock().unwrap_or(Utc::now());
        let end_time = if matches!(
            status,
            ActiveScanStatus::Completed | ActiveScanStatus::Stopped | ActiveScanStatus::Error(_)
        ) {
            Some(Utc::now())
        } else {
            None
        };

        let duration = (Utc::now() - start_time).num_seconds() as u64;

        Ok(ActiveScanResult {
            scan_id,
            status,
            config,
            start_time,
            end_time,
            duration_secs: duration,
            progress: self.get_progress(),
            results: self.results.lock().values().cloned().collect(),
        })
    }

    /// Get current status
    pub fn status(&self) -> ActiveScanStatus {
        self.status.lock().clone()
    }

    /// Get open ports found
    pub fn get_open_ports(&self) -> Vec<ProbeResult> {
        self.results
            .lock()
            .values()
            .filter(|r| r.open)
            .cloned()
            .collect()
    }
}

/// Simulate a probe (placeholder for actual implementation)
fn simulate_probe(target: &str, port: u16, _probe_type: ProbeType) -> bool {
    // In a real implementation, this would perform actual network probing
    // For testing, simulate some open ports
    let common_open_ports = [22, 80, 443, 8080, 3389, 3306, 5432];
    let hash = target.as_bytes().iter().map(|&b| b as u64).sum::<u64>() + port as u64;
    common_open_ports.contains(&port) && hash.is_multiple_of(3)
}

/// Identify common services by port number
fn identify_service(port: u16) -> Option<String> {
    match port {
        21 => Some("FTP".to_string()),
        22 => Some("SSH".to_string()),
        23 => Some("Telnet".to_string()),
        25 => Some("SMTP".to_string()),
        53 => Some("DNS".to_string()),
        80 => Some("HTTP".to_string()),
        110 => Some("POP3".to_string()),
        143 => Some("IMAP".to_string()),
        443 => Some("HTTPS".to_string()),
        445 => Some("SMB".to_string()),
        1433 => Some("MSSQL".to_string()),
        3306 => Some("MySQL".to_string()),
        3389 => Some("RDP".to_string()),
        5432 => Some("PostgreSQL".to_string()),
        5900 => Some("VNC".to_string()),
        6379 => Some("Redis".to_string()),
        8080 => Some("HTTP Proxy".to_string()),
        27017 => Some("MongoDB".to_string()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_recon() -> ActiveRecon {
        let mut scope = ScopeManager::new();
        scope.add_cidr("192.168.1.0/24").unwrap();

        let audit = Arc::new(AuditManager::new("test-operator".to_string()));
        let killed = Arc::new(AtomicBool::new(false));

        ActiveRecon::new(Arc::new(parking_lot::Mutex::new(scope)), audit, killed)
    }

    #[test]
    fn test_active_recon_creation() {
        let recon = create_test_recon();
        assert!(matches!(recon.status(), ActiveScanStatus::Idle));
    }

    #[test]
    fn test_start_scan() {
        let recon = create_test_recon();
        let config = ActiveScanConfig {
            targets: vec!["192.168.1.1".to_string()],
            ports: vec![22, 80, 443],
            ..Default::default()
        };

        let scan_id = recon.start_scan(config).unwrap();
        assert!(!scan_id.is_empty());
        assert!(matches!(recon.status(), ActiveScanStatus::Running));
    }

    #[test]
    fn test_out_of_scope_rejected() {
        let recon = create_test_recon();
        let config = ActiveScanConfig {
            targets: vec!["10.0.0.1".to_string()], // Out of scope
            ports: vec![22],
            ..Default::default()
        };

        let result = recon.start_scan(config);
        assert!(result.is_err());
    }

    #[test]
    fn test_probe() {
        let recon = create_test_recon();
        let config = ActiveScanConfig {
            targets: vec!["192.168.1.1".to_string()],
            ports: vec![22],
            ..Default::default()
        };
        recon.start_scan(config).unwrap();

        let result = recon.probe("192.168.1.1", 22).unwrap();
        assert_eq!(result.target, "192.168.1.1");
        assert_eq!(result.port, 22);
    }

    #[test]
    fn test_probe_out_of_scope() {
        let recon = create_test_recon();
        let config = ActiveScanConfig {
            targets: vec!["192.168.1.1".to_string()],
            ports: vec![22],
            ..Default::default()
        };
        recon.start_scan(config).unwrap();

        let result = recon.probe("10.0.0.1", 22);
        assert!(result.is_err());
    }

    #[test]
    fn test_progress_tracking() {
        let recon = create_test_recon();
        let config = ActiveScanConfig {
            targets: vec!["192.168.1.1".to_string(), "192.168.1.2".to_string()],
            ports: vec![22, 80],
            ..Default::default()
        };
        recon.start_scan(config).unwrap();

        let progress = recon.get_progress();
        assert_eq!(progress.total_probes, 4);
        assert_eq!(progress.probes_completed, 0);

        // Probe some targets
        let _ = recon.probe("192.168.1.1", 22);
        let _ = recon.probe("192.168.1.1", 80);

        let progress = recon.get_progress();
        assert_eq!(progress.probes_completed, 2);
    }

    #[test]
    fn test_pause_resume() {
        let recon = create_test_recon();
        let config = ActiveScanConfig {
            targets: vec!["192.168.1.1".to_string()],
            ports: vec![22],
            ..Default::default()
        };
        recon.start_scan(config).unwrap();

        recon.pause_scan().unwrap();
        assert!(matches!(recon.status(), ActiveScanStatus::Paused));

        recon.resume_scan().unwrap();
        assert!(matches!(recon.status(), ActiveScanStatus::Running));
    }

    #[test]
    fn test_kill_switch() {
        let mut scope = ScopeManager::new();
        scope.add_cidr("192.168.1.0/24").unwrap();

        let audit = Arc::new(AuditManager::new("test-operator".to_string()));
        let killed = Arc::new(AtomicBool::new(false));

        let recon = ActiveRecon::new(
            Arc::new(parking_lot::Mutex::new(scope)),
            audit,
            killed.clone(),
        );

        let config = ActiveScanConfig {
            targets: vec!["192.168.1.1".to_string()],
            ports: vec![22],
            ..Default::default()
        };
        recon.start_scan(config).unwrap();

        // Activate kill switch
        killed.store(true, Ordering::SeqCst);

        // Probing should fail
        let result = recon.probe("192.168.1.1", 22);
        assert!(result.is_err());
    }

    #[test]
    fn test_service_identification() {
        assert_eq!(identify_service(22), Some("SSH".to_string()));
        assert_eq!(identify_service(80), Some("HTTP".to_string()));
        assert_eq!(identify_service(443), Some("HTTPS".to_string()));
        assert_eq!(identify_service(12345), None);
    }
}
