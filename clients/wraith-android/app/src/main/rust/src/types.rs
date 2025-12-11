// Type definitions for WRAITH Android

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionInfo {
    pub session_id: String,
    pub peer_id: String,
    pub connected: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferInfo {
    pub transfer_id: String,
    pub peer_id: String,
    pub file_path: String,
    pub file_size: u64,
    pub bytes_transferred: u64,
    pub status: TransferStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TransferStatus {
    Pending,
    Sending,
    Receiving,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeStatus {
    pub running: bool,
    pub local_peer_id: String,
    pub session_count: usize,
    pub active_transfers: usize,
}
