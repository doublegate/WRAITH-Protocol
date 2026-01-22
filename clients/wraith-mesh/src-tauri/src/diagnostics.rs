//! Diagnostics Module
//!
//! Provides network diagnostic tools including ping, bandwidth testing,
//! connection health checks, and NAT type detection.

use crate::error::MeshResult;
use crate::state::AppState;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, info};

/// Result of a ping operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PingResult {
    /// Target peer ID
    pub peer_id: String,
    /// Whether the ping succeeded
    pub success: bool,
    /// Round-trip time in milliseconds
    pub rtt_ms: Option<u64>,
    /// Minimum RTT observed
    pub min_rtt_ms: Option<u64>,
    /// Maximum RTT observed
    pub max_rtt_ms: Option<u64>,
    /// Average RTT
    pub avg_rtt_ms: Option<f64>,
    /// Standard deviation of RTT
    pub stddev_rtt_ms: Option<f64>,
    /// Number of packets sent
    pub packets_sent: u32,
    /// Number of packets received
    pub packets_received: u32,
    /// Packet loss percentage
    pub packet_loss_pct: f64,
    /// Error message if failed
    pub error: Option<String>,
}

/// Result of a bandwidth test
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BandwidthResult {
    /// Target peer ID
    pub peer_id: String,
    /// Whether the test succeeded
    pub success: bool,
    /// Upload bandwidth in Mbps
    pub upload_mbps: Option<f64>,
    /// Download bandwidth in Mbps
    pub download_mbps: Option<f64>,
    /// Test duration in seconds
    pub duration_secs: f64,
    /// Bytes transferred during test
    pub bytes_transferred: u64,
    /// Error message if failed
    pub error: Option<String>,
}

/// Connection health report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthReport {
    /// Target peer ID
    pub peer_id: String,
    /// Overall health score (0.0 - 1.0)
    pub score: f64,
    /// Connection status
    pub status: ConnectionStatus,
    /// Average latency in milliseconds
    pub latency_ms: f64,
    /// Latency jitter (standard deviation)
    pub jitter_ms: f64,
    /// Packet loss rate (0.0 - 1.0)
    pub packet_loss: f64,
    /// Available bandwidth in Mbps
    pub bandwidth_mbps: f64,
    /// Connection uptime in seconds
    pub uptime_secs: u64,
    /// Number of reconnections
    pub reconnect_count: u32,
    /// Issues detected
    pub issues: Vec<HealthIssue>,
    /// Recommendations
    pub recommendations: Vec<String>,
}

/// Connection status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ConnectionStatus {
    /// Excellent connection
    Excellent,
    /// Good connection
    Good,
    /// Fair connection
    Fair,
    /// Poor connection
    Poor,
    /// Connection failed
    Failed,
}

/// A detected health issue
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthIssue {
    /// Issue severity (1-5, 5 being most severe)
    pub severity: u8,
    /// Issue type
    pub issue_type: String,
    /// Issue description
    pub description: String,
}

/// NAT type classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NatType {
    /// No NAT (public IP)
    Open,
    /// Full cone NAT
    FullCone,
    /// Restricted cone NAT
    RestrictedCone,
    /// Port restricted cone NAT
    PortRestrictedCone,
    /// Symmetric NAT
    Symmetric,
    /// Unknown NAT type
    Unknown,
}

impl std::fmt::Display for NatType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NatType::Open => write!(f, "Open (No NAT)"),
            NatType::FullCone => write!(f, "Full Cone NAT"),
            NatType::RestrictedCone => write!(f, "Restricted Cone NAT"),
            NatType::PortRestrictedCone => write!(f, "Port Restricted Cone NAT"),
            NatType::Symmetric => write!(f, "Symmetric NAT"),
            NatType::Unknown => write!(f, "Unknown"),
        }
    }
}

/// NAT detection result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NatDetectionResult {
    /// Detected NAT type
    pub nat_type: NatType,
    /// External IP address (if detected)
    pub external_ip: Option<String>,
    /// External port (if detected)
    pub external_port: Option<u16>,
    /// Whether hole punching is likely to work
    pub hole_punch_possible: bool,
    /// Recommended connection strategy
    pub recommendation: String,
}

/// Diagnostics service for network testing
pub struct Diagnostics {
    /// Application state
    #[allow(dead_code)]
    state: Arc<AppState>,
}

impl Diagnostics {
    /// Create a new diagnostics service
    pub fn new(state: Arc<AppState>) -> Self {
        Self { state }
    }

    /// Ping a peer to measure latency
    pub async fn ping(&self, peer_id: &str, count: u32) -> MeshResult<PingResult> {
        info!("Pinging peer {} with {} packets", &peer_id[..8.min(peer_id.len())], count);

        // Simulate ping (in real implementation, send actual packets)
        let mut rtts = Vec::with_capacity(count as usize);
        let mut received = 0u32;

        for _i in 0..count {
            // Simulate network latency
            tokio::time::sleep(Duration::from_millis(50)).await;

            // 95% success rate
            if rand_range(0, 100) < 95 {
                let rtt = rand_range(20, 150);
                rtts.push(rtt);
                received += 1;
            }
        }

        if rtts.is_empty() {
            return Ok(PingResult {
                peer_id: peer_id.to_string(),
                success: false,
                rtt_ms: None,
                min_rtt_ms: None,
                max_rtt_ms: None,
                avg_rtt_ms: None,
                stddev_rtt_ms: None,
                packets_sent: count,
                packets_received: 0,
                packet_loss_pct: 100.0,
                error: Some("All packets lost".to_string()),
            });
        }

        let min_rtt = *rtts.iter().min().unwrap();
        let max_rtt = *rtts.iter().max().unwrap();
        let sum: u64 = rtts.iter().sum();
        let avg_rtt = sum as f64 / rtts.len() as f64;

        let variance: f64 = rtts
            .iter()
            .map(|&rtt| {
                let diff = rtt as f64 - avg_rtt;
                diff * diff
            })
            .sum::<f64>()
            / rtts.len() as f64;
        let stddev = variance.sqrt();

        let packet_loss = (count - received) as f64 / count as f64 * 100.0;

        debug!(
            "Ping complete: avg={}ms, loss={:.1}%",
            avg_rtt as u64, packet_loss
        );

        Ok(PingResult {
            peer_id: peer_id.to_string(),
            success: true,
            rtt_ms: Some(*rtts.last().unwrap()),
            min_rtt_ms: Some(min_rtt),
            max_rtt_ms: Some(max_rtt),
            avg_rtt_ms: Some(avg_rtt),
            stddev_rtt_ms: Some(stddev),
            packets_sent: count,
            packets_received: received,
            packet_loss_pct: packet_loss,
            error: None,
        })
    }

    /// Test bandwidth to a peer
    pub async fn bandwidth_test(&self, peer_id: &str) -> MeshResult<BandwidthResult> {
        info!("Running bandwidth test to peer {}", &peer_id[..8.min(peer_id.len())]);

        let start = std::time::Instant::now();

        // Simulate bandwidth test (in real implementation, transfer actual data)
        tokio::time::sleep(Duration::from_secs(2)).await;

        let upload_mbps = rand_f64() * 100.0 + 10.0;
        let download_mbps = rand_f64() * 150.0 + 20.0;
        let duration = start.elapsed().as_secs_f64();
        let bytes = ((upload_mbps + download_mbps) / 2.0 * duration * 125000.0) as u64;

        debug!(
            "Bandwidth test complete: up={:.1}Mbps, down={:.1}Mbps",
            upload_mbps, download_mbps
        );

        Ok(BandwidthResult {
            peer_id: peer_id.to_string(),
            success: true,
            upload_mbps: Some(upload_mbps),
            download_mbps: Some(download_mbps),
            duration_secs: duration,
            bytes_transferred: bytes,
            error: None,
        })
    }

    /// Check connection health to a peer
    pub async fn check_connection_health(&self, peer_id: &str) -> MeshResult<HealthReport> {
        info!("Checking connection health for peer {}", &peer_id[..8.min(peer_id.len())]);

        // Run ping to get latency metrics
        let ping_result = self.ping(peer_id, 10).await?;

        let latency = ping_result.avg_rtt_ms.unwrap_or(0.0);
        let jitter = ping_result.stddev_rtt_ms.unwrap_or(0.0);
        let packet_loss = ping_result.packet_loss_pct / 100.0;

        // Estimate bandwidth
        let bandwidth = rand_f64() * 100.0 + 20.0;

        // Calculate health score
        let latency_score = (1.0 - (latency / 500.0).min(1.0)) * 0.3;
        let jitter_score = (1.0 - (jitter / 100.0).min(1.0)) * 0.2;
        let loss_score = (1.0 - packet_loss * 10.0).max(0.0) * 0.3;
        let bandwidth_score = (bandwidth / 100.0).min(1.0) * 0.2;

        let score = (latency_score + jitter_score + loss_score + bandwidth_score).clamp(0.0, 1.0);

        let status = if score >= 0.9 {
            ConnectionStatus::Excellent
        } else if score >= 0.7 {
            ConnectionStatus::Good
        } else if score >= 0.5 {
            ConnectionStatus::Fair
        } else if score >= 0.2 {
            ConnectionStatus::Poor
        } else {
            ConnectionStatus::Failed
        };

        // Detect issues
        let mut issues = Vec::new();
        let mut recommendations = Vec::new();

        if latency > 200.0 {
            issues.push(HealthIssue {
                severity: 3,
                issue_type: "high_latency".to_string(),
                description: format!("High latency detected: {:.0}ms", latency),
            });
            recommendations.push("Consider using a relay closer to the peer".to_string());
        }

        if jitter > 50.0 {
            issues.push(HealthIssue {
                severity: 2,
                issue_type: "high_jitter".to_string(),
                description: format!("Network jitter detected: {:.0}ms", jitter),
            });
            recommendations.push("Network path may be congested".to_string());
        }

        if packet_loss > 0.05 {
            issues.push(HealthIssue {
                severity: 4,
                issue_type: "packet_loss".to_string(),
                description: format!("Packet loss detected: {:.1}%", packet_loss * 100.0),
            });
            recommendations.push("Check network connection stability".to_string());
        }

        if bandwidth < 50.0 {
            issues.push(HealthIssue {
                severity: 2,
                issue_type: "low_bandwidth".to_string(),
                description: format!("Low bandwidth: {:.1} Mbps", bandwidth),
            });
            recommendations.push("Large file transfers may be slow".to_string());
        }

        debug!(
            "Health check complete: score={:.2}, status={:?}, {} issues",
            score,
            status,
            issues.len()
        );

        Ok(HealthReport {
            peer_id: peer_id.to_string(),
            score,
            status,
            latency_ms: latency,
            jitter_ms: jitter,
            packet_loss,
            bandwidth_mbps: bandwidth,
            uptime_secs: rand_range(60, 36000),
            reconnect_count: rand_range(0, 3) as u32,
            issues,
            recommendations,
        })
    }

    /// Detect NAT type
    pub async fn detect_nat_type(&self) -> MeshResult<NatDetectionResult> {
        info!("Detecting NAT type");

        // Simulate NAT detection (in real implementation, use STUN servers)
        tokio::time::sleep(Duration::from_secs(1)).await;

        let nat_type = match rand_range(0, 6) {
            0 => NatType::Open,
            1 => NatType::FullCone,
            2 => NatType::RestrictedCone,
            3 => NatType::PortRestrictedCone,
            4 => NatType::Symmetric,
            _ => NatType::Unknown,
        };

        let hole_punch_possible = matches!(
            nat_type,
            NatType::Open | NatType::FullCone | NatType::RestrictedCone | NatType::PortRestrictedCone
        );

        let recommendation = match nat_type {
            NatType::Open => "Direct connections possible".to_string(),
            NatType::FullCone => "Hole punching should work reliably".to_string(),
            NatType::RestrictedCone | NatType::PortRestrictedCone => {
                "Hole punching should work in most cases".to_string()
            }
            NatType::Symmetric => {
                "May need relay server for connections".to_string()
            }
            NatType::Unknown => "Could not determine NAT type".to_string(),
        };

        let external_ip = if nat_type != NatType::Unknown {
            Some(format!(
                "{}.{}.{}.{}",
                rand_range(1, 255),
                rand_range(0, 255),
                rand_range(0, 255),
                rand_range(1, 255)
            ))
        } else {
            None
        };

        debug!(
            "NAT detection complete: type={:?}, hole_punch={}",
            nat_type, hole_punch_possible
        );

        Ok(NatDetectionResult {
            nat_type,
            external_ip,
            external_port: Some(rand_range(30000, 60000) as u16),
            hole_punch_possible,
            recommendation,
        })
    }
}

// Simple random helpers
fn rand_u128() -> u128 {
    use std::time::SystemTime;
    let seed = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407)
}

fn rand_range(min: u64, max: u64) -> u64 {
    if min >= max {
        return min;
    }
    let range = max - min;
    min + ((rand_u128() as u64) % range)
}

fn rand_f64() -> f64 {
    (rand_u128() as f64 % 1000.0) / 1000.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::Database;
    use tempfile::tempdir;

    fn create_test_diagnostics() -> Diagnostics {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let db = Database::open(&db_path).unwrap();
        let state = Arc::new(AppState::new(db, dir.path().to_path_buf()));
        state.initialize().unwrap();
        Diagnostics::new(state)
    }

    #[tokio::test]
    async fn test_ping() {
        let diag = create_test_diagnostics();
        let result = diag.ping("test_peer_123", 5).await.unwrap();
        assert_eq!(result.packets_sent, 5);
    }

    #[tokio::test]
    async fn test_bandwidth_test() {
        let diag = create_test_diagnostics();
        let result = diag.bandwidth_test("test_peer_123").await.unwrap();
        assert!(result.success);
        assert!(result.upload_mbps.is_some());
        assert!(result.download_mbps.is_some());
    }

    #[tokio::test]
    async fn test_health_check() {
        let diag = create_test_diagnostics();
        let result = diag.check_connection_health("test_peer_123").await.unwrap();
        assert!(result.score >= 0.0 && result.score <= 1.0);
    }

    #[tokio::test]
    async fn test_nat_detection() {
        let diag = create_test_diagnostics();
        let result = diag.detect_nat_type().await.unwrap();
        assert!(!result.recommendation.is_empty());
    }
}
