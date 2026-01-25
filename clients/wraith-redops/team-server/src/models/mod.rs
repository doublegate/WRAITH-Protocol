pub mod listener;
use chrono::{DateTime, Utc};
use ipnetwork::IpNetwork;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct Campaign {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub status: Option<String>,
    pub start_date: Option<DateTime<Utc>>,
    pub end_date: Option<DateTime<Utc>>,
    pub created_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct Implant {
    pub id: Uuid,
    pub campaign_id: Option<Uuid>,
    pub hostname: Option<String>,
    /// Internal IP address (INET type in database)
    pub internal_ip: Option<IpNetwork>,
    /// External IP address (INET type in database)
    pub external_ip: Option<IpNetwork>,
    pub os_type: Option<String>,
    pub os_version: Option<String>,
    pub architecture: Option<String>,
    pub username: Option<String>,
    pub domain: Option<String>,
    pub privileges: Option<String>,
    pub implant_version: Option<String>,
    pub first_seen: Option<DateTime<Utc>>,
    pub last_checkin: Option<DateTime<Utc>>,
    pub checkin_interval: Option<i32>,
    pub jitter_percent: Option<i32>,
    pub status: Option<String>,
    pub notes: Option<String>,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct Command {
    pub id: Uuid,
    pub implant_id: Option<Uuid>,
    pub operator_id: Option<Uuid>,
    pub command_type: String,
    pub payload: Option<Vec<u8>>,
    pub payload_encrypted: Option<bool>,
    pub priority: Option<i32>,
    pub status: Option<String>,
    pub created_at: Option<DateTime<Utc>>,
    pub sent_at: Option<DateTime<Utc>>,
    pub received_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub timeout_seconds: Option<i32>,
    pub retry_count: Option<i32>,
    pub max_retries: Option<i32>,
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct CommandResult {
    pub id: Uuid,
    pub command_id: Option<Uuid>,
    pub output: Option<Vec<u8>>,
    pub output_encrypted: Option<bool>,
    pub exit_code: Option<i32>,
    pub error_message: Option<String>,
    pub execution_time_ms: Option<i32>,
    pub received_at: Option<DateTime<Utc>>,
}

/// Operator model for team server authentication and authorization.
#[allow(dead_code)]
#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct Operator {
    pub id: Uuid,
    pub username: String,
    pub display_name: Option<String>,
    pub role: String,
    pub public_key: Vec<u8>,
    pub created_at: Option<DateTime<Utc>>,
    pub last_active: Option<DateTime<Utc>>,
    pub is_active: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct Artifact {
    pub id: Uuid,
    pub implant_id: Option<Uuid>,
    pub command_id: Option<Uuid>,
    pub filename: Option<String>,
    pub original_path: Option<String>,
    pub file_hash_sha256: Option<Vec<u8>>,
    pub file_hash_blake3: Option<Vec<u8>>,
    pub file_size: Option<i64>,
    pub mime_type: Option<String>,
    pub content: Option<Vec<u8>>,
    pub collected_at: Option<DateTime<Utc>>,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct Credential {
    pub id: Uuid,
    pub implant_id: Option<Uuid>,
    pub source: Option<String>,
    pub credential_type: Option<String>,
    pub domain: Option<String>,
    pub username: Option<String>,
    pub credential_data: Option<Vec<u8>>,
    pub collected_at: Option<DateTime<Utc>>,
    pub validated: Option<bool>,
    pub metadata: Option<serde_json::Value>,
}
