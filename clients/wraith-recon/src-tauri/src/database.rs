//! Database Module for WRAITH Recon
//!
//! Provides persistent storage for engagements, audit logs, and scan results.

use crate::audit::AuditEntry;
use crate::error::Result;
use crate::roe::RulesOfEngagement;
use rusqlite::{Connection, OptionalExtension, params};
use std::path::Path;

/// Database manager for WRAITH Recon
pub struct Database {
    conn: Connection,
}

impl Database {
    /// Open or create a database at the given path
    pub fn open(path: &Path) -> Result<Self> {
        let conn = Connection::open(path)?;

        // Enable WAL mode for better concurrency
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")?;

        let db = Self { conn };
        db.initialize_schema()?;
        Ok(db)
    }

    /// Initialize database schema
    fn initialize_schema(&self) -> Result<()> {
        self.conn.execute_batch(
            r#"
            -- Rules of Engagement table
            CREATE TABLE IF NOT EXISTS roe (
                id TEXT PRIMARY KEY,
                version TEXT NOT NULL,
                organization TEXT NOT NULL,
                title TEXT NOT NULL,
                description TEXT,
                start_time TEXT NOT NULL,
                end_time TEXT NOT NULL,
                authorized_cidrs TEXT NOT NULL,
                authorized_domains TEXT NOT NULL,
                excluded_targets TEXT NOT NULL,
                authorized_techniques TEXT NOT NULL,
                prohibited_techniques TEXT NOT NULL,
                signer_public_key TEXT NOT NULL,
                signature TEXT NOT NULL,
                created_at TEXT NOT NULL,
                loaded_at TEXT NOT NULL
            );

            -- Engagements table
            CREATE TABLE IF NOT EXISTS engagements (
                id TEXT PRIMARY KEY,
                roe_id TEXT NOT NULL REFERENCES roe(id),
                status TEXT NOT NULL,
                started_at TEXT NOT NULL,
                ended_at TEXT,
                operator_id TEXT NOT NULL,
                notes TEXT
            );

            -- Audit log table
            CREATE TABLE IF NOT EXISTS audit_log (
                id TEXT PRIMARY KEY,
                sequence INTEGER NOT NULL,
                timestamp TEXT NOT NULL,
                level TEXT NOT NULL,
                category TEXT NOT NULL,
                summary TEXT NOT NULL,
                details TEXT,
                target TEXT,
                operator_id TEXT NOT NULL,
                mitre_technique_id TEXT,
                mitre_technique_name TEXT,
                mitre_tactic TEXT,
                previous_hash TEXT NOT NULL,
                entry_hash TEXT NOT NULL,
                signature TEXT NOT NULL,
                signer_public_key TEXT NOT NULL
            );

            -- Scan results table
            CREATE TABLE IF NOT EXISTS scan_results (
                id TEXT PRIMARY KEY,
                engagement_id TEXT NOT NULL REFERENCES engagements(id),
                scan_type TEXT NOT NULL,
                status TEXT NOT NULL,
                config TEXT NOT NULL,
                started_at TEXT NOT NULL,
                ended_at TEXT,
                packets_captured INTEGER DEFAULT 0,
                bytes_captured INTEGER DEFAULT 0,
                assets_discovered INTEGER DEFAULT 0
            );

            -- Discovered assets table
            CREATE TABLE IF NOT EXISTS discovered_assets (
                id TEXT PRIMARY KEY,
                engagement_id TEXT NOT NULL REFERENCES engagements(id),
                scan_id TEXT REFERENCES scan_results(id),
                ip TEXT NOT NULL,
                mac TEXT,
                hostname TEXT,
                first_seen TEXT NOT NULL,
                last_seen TEXT NOT NULL,
                traffic_volume INTEGER DEFAULT 0,
                os_fingerprint TEXT,
                protocols TEXT
            );

            -- Discovered ports table
            CREATE TABLE IF NOT EXISTS discovered_ports (
                id TEXT PRIMARY KEY,
                asset_id TEXT NOT NULL REFERENCES discovered_assets(id),
                port INTEGER NOT NULL,
                protocol TEXT NOT NULL,
                service TEXT,
                banner TEXT,
                first_seen TEXT NOT NULL,
                last_seen TEXT NOT NULL
            );

            -- Channel statistics table
            CREATE TABLE IF NOT EXISTS channel_stats (
                id TEXT PRIMARY KEY,
                engagement_id TEXT NOT NULL REFERENCES engagements(id),
                channel_type TEXT NOT NULL,
                target TEXT NOT NULL,
                port INTEGER,
                bytes_sent INTEGER DEFAULT 0,
                bytes_received INTEGER DEFAULT 0,
                packets_sent INTEGER DEFAULT 0,
                packets_received INTEGER DEFAULT 0,
                errors INTEGER DEFAULT 0,
                opened_at TEXT NOT NULL,
                closed_at TEXT
            );

            -- Create indexes
            CREATE INDEX IF NOT EXISTS idx_audit_timestamp ON audit_log(timestamp);
            CREATE INDEX IF NOT EXISTS idx_audit_category ON audit_log(category);
            CREATE INDEX IF NOT EXISTS idx_audit_sequence ON audit_log(sequence);
            CREATE INDEX IF NOT EXISTS idx_assets_engagement ON discovered_assets(engagement_id);
            CREATE INDEX IF NOT EXISTS idx_assets_ip ON discovered_assets(ip);
            CREATE INDEX IF NOT EXISTS idx_ports_asset ON discovered_ports(asset_id);
            "#,
        )?;

        Ok(())
    }

    /// Store an RoE document
    pub fn store_roe(&self, roe: &RulesOfEngagement) -> Result<()> {
        self.conn.execute(
            r#"INSERT OR REPLACE INTO roe (
                id, version, organization, title, description,
                start_time, end_time, authorized_cidrs, authorized_domains,
                excluded_targets, authorized_techniques, prohibited_techniques,
                signer_public_key, signature, created_at, loaded_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)"#,
            params![
                roe.id,
                roe.version,
                roe.organization,
                roe.title,
                roe.description,
                roe.start_time.to_rfc3339(),
                roe.end_time.to_rfc3339(),
                serde_json::to_string(&roe.authorized_cidrs)?,
                serde_json::to_string(&roe.authorized_domains)?,
                serde_json::to_string(&roe.excluded_targets)?,
                serde_json::to_string(&roe.authorized_techniques)?,
                serde_json::to_string(&roe.prohibited_techniques)?,
                roe.signer_public_key,
                roe.signature,
                roe.created_at.to_rfc3339(),
                chrono::Utc::now().to_rfc3339(),
            ],
        )?;

        Ok(())
    }

    /// Get an RoE document by ID
    pub fn get_roe(&self, roe_id: &str) -> Result<Option<RulesOfEngagement>> {
        let result: Option<RulesOfEngagement> = self
            .conn
            .query_row("SELECT * FROM roe WHERE id = ?1", params![roe_id], |row| {
                Ok(RulesOfEngagement {
                    id: row.get(0)?,
                    version: row.get(1)?,
                    organization: row.get(2)?,
                    title: row.get(3)?,
                    description: row.get(4)?,
                    authorized_operators: Vec::new(),
                    client_name: String::new(),
                    start_time: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(5)?)
                        .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?
                        .with_timezone(&chrono::Utc),
                    end_time: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(6)?)
                        .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?
                        .with_timezone(&chrono::Utc),
                    authorized_cidrs: serde_json::from_str(&row.get::<_, String>(7)?)
                        .unwrap_or_default(),
                    authorized_domains: serde_json::from_str(&row.get::<_, String>(8)?)
                        .unwrap_or_default(),
                    excluded_targets: serde_json::from_str(&row.get::<_, String>(9)?)
                        .unwrap_or_default(),
                    authorized_techniques: serde_json::from_str(&row.get::<_, String>(10)?)
                        .unwrap_or_default(),
                    prohibited_techniques: serde_json::from_str(&row.get::<_, String>(11)?)
                        .unwrap_or_default(),
                    max_exfil_rate: None,
                    max_exfil_total: None,
                    emergency_contacts: Vec::new(),
                    constraints: Vec::new(),
                    signer_public_key: row.get(12)?,
                    signature: row.get(13)?,
                    created_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(14)?)
                        .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?
                        .with_timezone(&chrono::Utc),
                })
            })
            .optional()?;

        Ok(result)
    }

    /// Store an audit entry
    pub fn store_audit_entry(&self, entry: &AuditEntry) -> Result<()> {
        self.conn.execute(
            r#"INSERT INTO audit_log (
                id, sequence, timestamp, level, category, summary, details, target,
                operator_id, mitre_technique_id, mitre_technique_name, mitre_tactic,
                previous_hash, entry_hash, signature, signer_public_key
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)"#,
            params![
                entry.id,
                entry.sequence as i64,
                entry.timestamp.to_rfc3339(),
                format!("{:?}", entry.level),
                format!("{:?}", entry.category),
                entry.summary,
                entry.details,
                entry.target,
                entry.operator_id,
                entry.mitre_ref.as_ref().map(|m| &m.technique_id),
                entry.mitre_ref.as_ref().map(|m| &m.technique_name),
                entry.mitre_ref.as_ref().map(|m| &m.tactic),
                entry.previous_hash,
                entry.entry_hash,
                entry.signature,
                entry.signer_public_key,
            ],
        )?;

        Ok(())
    }

    /// Get audit entries since a given sequence number
    pub fn get_audit_entries(&self, since_sequence: u64, limit: usize) -> Result<Vec<AuditEntry>> {
        let mut stmt = self.conn.prepare(
            "SELECT * FROM audit_log WHERE sequence >= ?1 ORDER BY sequence ASC LIMIT ?2",
        )?;

        let entries = stmt
            .query_map(params![since_sequence as i64, limit as i64], |row| {
                Ok(AuditEntry {
                    sequence: row.get::<_, i64>(1)? as u64,
                    id: row.get(0)?,
                    timestamp: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(2)?)
                        .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?
                        .with_timezone(&chrono::Utc),
                    level: parse_audit_level(&row.get::<_, String>(3)?),
                    category: parse_audit_category(&row.get::<_, String>(4)?),
                    summary: row.get(5)?,
                    details: row.get(6)?,
                    target: row.get(7)?,
                    operator_id: row.get(8)?,
                    mitre_ref: {
                        let technique_id: Option<String> = row.get(9)?;
                        let technique_name: Option<String> = row.get(10)?;
                        let tactic: Option<String> = row.get(11)?;
                        match (technique_id, technique_name, tactic) {
                            (Some(tid), Some(tname), Some(tac)) => {
                                Some(crate::audit::MitreReference {
                                    technique_id: tid,
                                    technique_name: tname,
                                    tactic: tac,
                                })
                            }
                            _ => None,
                        }
                    },
                    previous_hash: row.get(12)?,
                    entry_hash: row.get(13)?,
                    signature: row.get(14)?,
                    signer_public_key: row.get(15)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(entries)
    }

    /// Get statistics
    pub fn get_statistics(&self) -> Result<DatabaseStatistics> {
        let audit_count: i64 =
            self.conn
                .query_row("SELECT COUNT(*) FROM audit_log", [], |row| row.get(0))?;
        let asset_count: i64 =
            self.conn
                .query_row("SELECT COUNT(*) FROM discovered_assets", [], |row| {
                    row.get(0)
                })?;
        let engagement_count: i64 =
            self.conn
                .query_row("SELECT COUNT(*) FROM engagements", [], |row| row.get(0))?;

        Ok(DatabaseStatistics {
            audit_entries: audit_count as u64,
            discovered_assets: asset_count as u64,
            engagements: engagement_count as u64,
        })
    }
}

/// Database statistics
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DatabaseStatistics {
    pub audit_entries: u64,
    pub discovered_assets: u64,
    pub engagements: u64,
}

fn parse_audit_level(s: &str) -> crate::audit::AuditLevel {
    match s {
        "Info" => crate::audit::AuditLevel::Info,
        "Warning" => crate::audit::AuditLevel::Warning,
        "Error" => crate::audit::AuditLevel::Error,
        "Critical" => crate::audit::AuditLevel::Critical,
        "Emergency" => crate::audit::AuditLevel::Emergency,
        _ => crate::audit::AuditLevel::Info,
    }
}

fn parse_audit_category(s: &str) -> crate::audit::AuditCategory {
    match s {
        "System" => crate::audit::AuditCategory::System,
        "RulesOfEngagement" => crate::audit::AuditCategory::RulesOfEngagement,
        "ScopeChange" => crate::audit::AuditCategory::ScopeChange,
        "TimingEvent" => crate::audit::AuditCategory::TimingEvent,
        "KillSwitch" => crate::audit::AuditCategory::KillSwitch,
        "Reconnaissance" => crate::audit::AuditCategory::Reconnaissance,
        "Channel" => crate::audit::AuditCategory::Channel,
        "DataTransfer" => crate::audit::AuditCategory::DataTransfer,
        "Configuration" => crate::audit::AuditCategory::Configuration,
        "Authentication" => crate::audit::AuditCategory::Authentication,
        "AuditSystem" => crate::audit::AuditCategory::AuditSystem,
        _ => crate::audit::AuditCategory::System,
    }
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
    fn test_store_audit_entry() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let db = Database::open(&db_path).unwrap();

        let manager = crate::audit::AuditManager::new("test".to_string());
        let entry = manager.info(crate::audit::AuditCategory::System, "Test entry");

        db.store_audit_entry(&entry).unwrap();

        let entries = db.get_audit_entries(0, 10).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].summary, "Test entry");
    }

    #[test]
    fn test_statistics() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let db = Database::open(&db_path).unwrap();

        let stats = db.get_statistics().unwrap();
        assert_eq!(stats.audit_entries, 0);
    }
}
