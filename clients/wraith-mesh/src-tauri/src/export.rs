//! Export Module
//!
//! Provides data export functionality for network topology,
//! metrics history, and diagnostic results.

use crate::error::MeshResult;
use crate::network_monitor::NetworkSnapshot;
use serde::{Deserialize, Serialize};
use tracing::info;

/// Export format options
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ExportFormat {
    /// JSON format
    Json,
    /// CSV format
    Csv,
}

/// Export network topology data
pub fn export_network_snapshot(snapshot: &NetworkSnapshot, format: ExportFormat) -> MeshResult<String> {
    info!("Exporting network snapshot as {:?}", format);

    match format {
        ExportFormat::Json => export_json(snapshot),
        ExportFormat::Csv => export_csv(snapshot),
    }
}

/// Export as JSON
fn export_json(snapshot: &NetworkSnapshot) -> MeshResult<String> {
    let json = serde_json::to_string_pretty(snapshot)?;
    Ok(json)
}

/// Export as CSV
fn export_csv(snapshot: &NetworkSnapshot) -> MeshResult<String> {
    let mut csv = String::new();

    // Nodes section
    csv.push_str("# NODES\n");
    csv.push_str("id,label,type,connected_at,last_seen\n");

    for node in &snapshot.nodes {
        csv.push_str(&format!(
            "{},{},{},{},{}\n",
            node.id,
            node.label,
            format!("{:?}", node.peer_type).to_lowercase(),
            node.connected_at,
            node.last_seen
        ));
    }

    csv.push('\n');

    // Links section
    csv.push_str("# LINKS\n");
    csv.push_str("source,target,latency_ms,bandwidth_mbps,packet_loss,strength\n");

    for link in &snapshot.links {
        csv.push_str(&format!(
            "{},{},{},{:.2},{:.4},{:.2}\n",
            link.source,
            link.target,
            link.latency_ms,
            link.bandwidth_mbps,
            link.packet_loss,
            link.strength
        ));
    }

    csv.push('\n');

    // DHT stats section
    csv.push_str("# DHT_STATS\n");
    csv.push_str("total_nodes,routing_table_size,stored_keys,lookup_count_1h,avg_lookup_latency_ms\n");
    csv.push_str(&format!(
        "{},{},{},{},{:.2}\n",
        snapshot.dht_stats.total_nodes,
        snapshot.dht_stats.routing_table_size,
        snapshot.dht_stats.stored_keys,
        snapshot.dht_stats.lookup_count_1h,
        snapshot.dht_stats.avg_lookup_latency_ms
    ));

    Ok(csv)
}

/// Export metrics history as CSV
pub fn export_metrics_history(
    history: &[crate::network_monitor::MetricsEntry],
) -> MeshResult<String> {
    let mut csv = String::new();

    csv.push_str("timestamp,peer_count,avg_latency_ms,total_bandwidth_mbps,packet_loss_rate\n");

    for entry in history {
        csv.push_str(&format!(
            "{},{},{:.2},{:.2},{:.4}\n",
            entry.timestamp,
            entry.peer_count,
            entry.avg_latency_ms,
            entry.total_bandwidth_mbps,
            entry.packet_loss_rate
        ));
    }

    Ok(csv)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::network_monitor::{DhtStats, LinkInfo, PeerInfo, PeerType};

    fn create_test_snapshot() -> NetworkSnapshot {
        NetworkSnapshot {
            timestamp: 1234567890,
            nodes: vec![
                PeerInfo {
                    id: "peer1".to_string(),
                    label: "Me".to_string(),
                    peer_type: PeerType::SelfNode,
                    connected_at: 1234567000,
                    last_seen: 1234567890,
                    location: None,
                },
                PeerInfo {
                    id: "peer2".to_string(),
                    label: "Direct-1".to_string(),
                    peer_type: PeerType::Direct,
                    connected_at: 1234567100,
                    last_seen: 1234567880,
                    location: None,
                },
            ],
            links: vec![LinkInfo {
                source: "peer1".to_string(),
                target: "peer2".to_string(),
                latency_ms: 50,
                bandwidth_mbps: 100.5,
                packet_loss: 0.01,
                strength: 0.9,
            }],
            dht_stats: DhtStats {
                total_nodes: 100,
                routing_table_size: 20,
                stored_keys: 15,
                lookup_count_1h: 50,
                avg_lookup_latency_ms: 150.0,
            },
            health_score: 0.95,
        }
    }

    #[test]
    fn test_export_json() {
        let snapshot = create_test_snapshot();
        let json = export_network_snapshot(&snapshot, ExportFormat::Json).unwrap();

        assert!(json.contains("\"timestamp\": 1234567890"));
        assert!(json.contains("\"id\": \"peer1\""));
        assert!(json.contains("\"total_nodes\": 100"));
    }

    #[test]
    fn test_export_csv() {
        let snapshot = create_test_snapshot();
        let csv = export_network_snapshot(&snapshot, ExportFormat::Csv).unwrap();

        assert!(csv.contains("# NODES"));
        assert!(csv.contains("# LINKS"));
        assert!(csv.contains("# DHT_STATS"));
        assert!(csv.contains("peer1,Me,selfnode"));
    }
}
