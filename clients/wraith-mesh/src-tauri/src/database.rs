//! Database Module
//!
//! Provides SQLite-based persistence for network metrics, connection history,
//! and diagnostic results.

use crate::error::{MeshError, MeshResult};
use rusqlite::{params, Connection};
use std::path::Path;
use std::sync::Mutex;

/// Database wrapper for WRAITH Mesh
pub struct Database {
    conn: Mutex<Connection>,
}

impl Database {
    /// Open or create a database at the given path
    pub fn open(path: &Path) -> MeshResult<Self> {
        let conn = Connection::open(path)?;
        let db = Self {
            conn: Mutex::new(conn),
        };
        db.init_schema()?;
        Ok(db)
    }

    /// Initialize database schema
    fn init_schema(&self) -> MeshResult<()> {
        let conn = self.conn.lock().map_err(|e| MeshError::Database(e.to_string()))?;

        conn.execute_batch(
            r#"
            -- Connection history table
            CREATE TABLE IF NOT EXISTS connection_history (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                peer_id TEXT NOT NULL,
                peer_label TEXT,
                peer_type TEXT NOT NULL,
                connected_at INTEGER NOT NULL,
                disconnected_at INTEGER,
                avg_latency_ms REAL,
                avg_bandwidth_mbps REAL,
                packet_loss REAL
            );

            -- Network metrics time series
            CREATE TABLE IF NOT EXISTS network_metrics (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                timestamp INTEGER NOT NULL,
                connected_peers INTEGER NOT NULL,
                dht_nodes INTEGER NOT NULL,
                avg_latency_ms REAL,
                total_bandwidth_mbps REAL,
                packet_loss_rate REAL
            );

            -- DHT lookup traces
            CREATE TABLE IF NOT EXISTS dht_lookups (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                lookup_key TEXT NOT NULL,
                timestamp INTEGER NOT NULL,
                hop_count INTEGER NOT NULL,
                duration_ms INTEGER NOT NULL,
                success INTEGER NOT NULL
            );

            -- Diagnostic results
            CREATE TABLE IF NOT EXISTS diagnostics (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                diagnostic_type TEXT NOT NULL,
                peer_id TEXT,
                timestamp INTEGER NOT NULL,
                result_json TEXT NOT NULL
            );

            -- Create indices for faster queries
            CREATE INDEX IF NOT EXISTS idx_connection_history_peer ON connection_history(peer_id);
            CREATE INDEX IF NOT EXISTS idx_network_metrics_timestamp ON network_metrics(timestamp);
            CREATE INDEX IF NOT EXISTS idx_dht_lookups_timestamp ON dht_lookups(timestamp);
            CREATE INDEX IF NOT EXISTS idx_diagnostics_type ON diagnostics(diagnostic_type);
            "#,
        )?;

        Ok(())
    }

    /// Record a connection event
    pub fn record_connection(
        &self,
        peer_id: &str,
        peer_label: Option<&str>,
        peer_type: &str,
        connected_at: i64,
    ) -> MeshResult<i64> {
        let conn = self.conn.lock().map_err(|e| MeshError::Database(e.to_string()))?;

        conn.execute(
            "INSERT INTO connection_history (peer_id, peer_label, peer_type, connected_at)
             VALUES (?1, ?2, ?3, ?4)",
            params![peer_id, peer_label, peer_type, connected_at],
        )?;

        Ok(conn.last_insert_rowid())
    }

    /// Record a disconnection event
    pub fn record_disconnection(
        &self,
        peer_id: &str,
        disconnected_at: i64,
        avg_latency_ms: Option<f64>,
        avg_bandwidth_mbps: Option<f64>,
        packet_loss: Option<f64>,
    ) -> MeshResult<()> {
        let conn = self.conn.lock().map_err(|e| MeshError::Database(e.to_string()))?;

        conn.execute(
            "UPDATE connection_history
             SET disconnected_at = ?1, avg_latency_ms = ?2, avg_bandwidth_mbps = ?3, packet_loss = ?4
             WHERE peer_id = ?5 AND disconnected_at IS NULL",
            params![
                disconnected_at,
                avg_latency_ms,
                avg_bandwidth_mbps,
                packet_loss,
                peer_id
            ],
        )?;

        Ok(())
    }

    /// Record network metrics snapshot
    pub fn record_metrics(
        &self,
        timestamp: i64,
        connected_peers: i64,
        dht_nodes: i64,
        avg_latency_ms: Option<f64>,
        total_bandwidth_mbps: Option<f64>,
        packet_loss_rate: Option<f64>,
    ) -> MeshResult<()> {
        let conn = self.conn.lock().map_err(|e| MeshError::Database(e.to_string()))?;

        conn.execute(
            "INSERT INTO network_metrics (timestamp, connected_peers, dht_nodes, avg_latency_ms, total_bandwidth_mbps, packet_loss_rate)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                timestamp,
                connected_peers,
                dht_nodes,
                avg_latency_ms,
                total_bandwidth_mbps,
                packet_loss_rate
            ],
        )?;

        Ok(())
    }

    /// Get recent metrics (last N entries)
    pub fn get_recent_metrics(&self, limit: i64) -> MeshResult<Vec<NetworkMetricsRow>> {
        let conn = self.conn.lock().map_err(|e| MeshError::Database(e.to_string()))?;

        let mut stmt = conn.prepare(
            "SELECT timestamp, connected_peers, dht_nodes, avg_latency_ms, total_bandwidth_mbps, packet_loss_rate
             FROM network_metrics
             ORDER BY timestamp DESC
             LIMIT ?1",
        )?;

        let rows = stmt
            .query_map([limit], |row| {
                Ok(NetworkMetricsRow {
                    timestamp: row.get(0)?,
                    connected_peers: row.get(1)?,
                    dht_nodes: row.get(2)?,
                    avg_latency_ms: row.get(3)?,
                    total_bandwidth_mbps: row.get(4)?,
                    packet_loss_rate: row.get(5)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(rows)
    }

    /// Record DHT lookup trace
    pub fn record_dht_lookup(
        &self,
        lookup_key: &str,
        timestamp: i64,
        hop_count: i64,
        duration_ms: i64,
        success: bool,
    ) -> MeshResult<()> {
        let conn = self.conn.lock().map_err(|e| MeshError::Database(e.to_string()))?;

        conn.execute(
            "INSERT INTO dht_lookups (lookup_key, timestamp, hop_count, duration_ms, success)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![lookup_key, timestamp, hop_count, duration_ms, success as i64],
        )?;

        Ok(())
    }

    /// Record diagnostic result
    pub fn record_diagnostic(
        &self,
        diagnostic_type: &str,
        peer_id: Option<&str>,
        timestamp: i64,
        result_json: &str,
    ) -> MeshResult<()> {
        let conn = self.conn.lock().map_err(|e| MeshError::Database(e.to_string()))?;

        conn.execute(
            "INSERT INTO diagnostics (diagnostic_type, peer_id, timestamp, result_json)
             VALUES (?1, ?2, ?3, ?4)",
            params![diagnostic_type, peer_id, timestamp, result_json],
        )?;

        Ok(())
    }
}

/// Network metrics database row
#[derive(Debug, Clone)]
pub struct NetworkMetricsRow {
    pub timestamp: i64,
    pub connected_peers: i64,
    pub dht_nodes: i64,
    pub avg_latency_ms: Option<f64>,
    pub total_bandwidth_mbps: Option<f64>,
    pub packet_loss_rate: Option<f64>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_database_creation() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let _db = Database::open(&db_path).unwrap();
        assert!(db_path.exists());
    }

    #[test]
    fn test_record_connection() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let db = Database::open(&db_path).unwrap();

        let id = db
            .record_connection("peer123", Some("Test Peer"), "direct", 1234567890)
            .unwrap();
        assert!(id > 0);
    }

    #[test]
    fn test_record_metrics() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let db = Database::open(&db_path).unwrap();

        db.record_metrics(1234567890, 10, 50, Some(25.5), Some(100.0), Some(0.01))
            .unwrap();

        let metrics = db.get_recent_metrics(10).unwrap();
        assert_eq!(metrics.len(), 1);
        assert_eq!(metrics[0].connected_peers, 10);
    }
}
