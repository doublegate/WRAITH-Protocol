//! Tauri IPC commands for WRAITH Transfer

use std::path::PathBuf;

use tauri::State;
use tracing::info;

use wraith_core::node::{Node, NodeConfig};

use crate::error::AppError;
use crate::state::AppState;
use crate::{AppResult, NodeStatus, SessionInfo, TransferInfo};

/// Get the current node status
#[tauri::command]
pub async fn get_node_status(state: State<'_, AppState>) -> AppResult<NodeStatus> {
    let running = state.is_node_running().await;
    let node_id = state.get_node_id_hex().await;
    let active_sessions = state.active_session_count().await;
    let active_transfers = state.active_transfer_count().await;

    Ok(NodeStatus {
        running,
        node_id,
        active_sessions,
        active_transfers,
    })
}

/// Start the WRAITH node
#[tauri::command]
pub async fn start_node(state: State<'_, AppState>) -> AppResult<NodeStatus> {
    info!("Starting WRAITH node");

    // Check if already running
    if state.is_node_running().await {
        return Ok(NodeStatus {
            running: true,
            node_id: state.get_node_id_hex().await,
            active_sessions: state.active_session_count().await,
            active_transfers: state.active_transfer_count().await,
        });
    }

    // Create and start new node
    let config = NodeConfig::default();
    let node = Node::new_with_config(config)
        .await
        .map_err(|e| AppError::Node(format!("Failed to create node: {}", e)))?;

    node.start()
        .await
        .map_err(|e| AppError::Node(format!("Failed to start node: {}", e)))?;

    let node_id = hex::encode(node.node_id());
    info!("Node started with ID: {}", node_id);

    // Store node in state
    {
        let mut node_lock = state.node.write().await;
        *node_lock = Some(node);
    }

    Ok(NodeStatus {
        running: true,
        node_id: Some(node_id),
        active_sessions: 0,
        active_transfers: 0,
    })
}

/// Stop the WRAITH node
#[tauri::command]
pub async fn stop_node(state: State<'_, AppState>) -> AppResult<()> {
    info!("Stopping WRAITH node");

    let mut node_lock = state.node.write().await;
    if let Some(node) = node_lock.take() {
        node.stop()
            .await
            .map_err(|e| AppError::Node(format!("Failed to stop node: {}", e)))?;
        info!("Node stopped");
    }

    Ok(())
}

/// Get the node ID as a hex string
#[tauri::command]
pub async fn get_node_id(state: State<'_, AppState>) -> AppResult<Option<String>> {
    Ok(state.get_node_id_hex().await)
}

/// Get all active sessions
#[tauri::command]
pub async fn get_sessions(state: State<'_, AppState>) -> AppResult<Vec<SessionInfo>> {
    let node = state.node.read().await;
    let Some(node) = node.as_ref() else {
        return Err(AppError::NodeNotRunning);
    };

    let sessions = node.active_sessions().await;
    let mut result = Vec::with_capacity(sessions.len());

    for peer_id in sessions {
        // Get connection stats
        let (bytes_sent, bytes_received) = if let Some(stats) = node.get_connection_stats(&peer_id)
        {
            (stats.bytes_sent, stats.bytes_received)
        } else {
            (0, 0)
        };

        // Get established_at timestamp (seconds since epoch)
        let established_at = node
            .get_session_established_at(&peer_id)
            .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|duration| duration.as_secs())
            .unwrap_or(0);

        result.push(SessionInfo {
            peer_id: hex::encode(peer_id),
            established_at,
            bytes_sent,
            bytes_received,
        });
    }

    Ok(result)
}

/// Close a session with a peer
#[tauri::command]
pub async fn close_session(state: State<'_, AppState>, peer_id: String) -> AppResult<()> {
    let node = state.node.read().await;
    let Some(node) = node.as_ref() else {
        return Err(AppError::NodeNotRunning);
    };

    let peer_bytes = hex::decode(&peer_id)
        .map_err(|e| AppError::InvalidPeerId(format!("Invalid hex: {}", e)))?;

    if peer_bytes.len() != 32 {
        return Err(AppError::InvalidPeerId("Peer ID must be 32 bytes".into()));
    }

    let mut peer_id_arr = [0u8; 32];
    peer_id_arr.copy_from_slice(&peer_bytes);

    node.close_session(&peer_id_arr)
        .await
        .map_err(|e| AppError::Session(e.to_string()))?;

    info!("Closed session with peer: {}", peer_id);
    Ok(())
}

/// Send a file to a peer
#[tauri::command]
pub async fn send_file(
    state: State<'_, AppState>,
    peer_id: String,
    file_path: String,
) -> AppResult<String> {
    let node = state.node.read().await;
    let Some(node) = node.as_ref() else {
        return Err(AppError::NodeNotRunning);
    };

    let peer_bytes = hex::decode(&peer_id)
        .map_err(|e| AppError::InvalidPeerId(format!("Invalid hex: {}", e)))?;

    if peer_bytes.len() != 32 {
        return Err(AppError::InvalidPeerId("Peer ID must be 32 bytes".into()));
    }

    let mut peer_id_arr = [0u8; 32];
    peer_id_arr.copy_from_slice(&peer_bytes);

    let path = PathBuf::from(&file_path);
    if !path.exists() {
        return Err(AppError::FileNotFound(file_path));
    }

    let transfer_id = node
        .send_file(path, &peer_id_arr)
        .await
        .map_err(|e| AppError::Transfer(e.to_string()))?;

    let transfer_id_hex = hex::encode(transfer_id);
    info!(
        "Started file transfer: {} to peer {}",
        transfer_id_hex, peer_id
    );

    // Track transfer in state
    {
        let file_name = PathBuf::from(&file_path)
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "unknown".to_string());

        let mut transfers = state.transfers.write().await;
        transfers.insert(
            transfer_id_hex.clone(),
            TransferInfo {
                id: transfer_id_hex.clone(),
                peer_id: peer_id.clone(),
                file_name,
                total_bytes: 0,
                transferred_bytes: 0,
                progress: 0.0,
                status: "initializing".to_string(),
                direction: "upload".to_string(),
            },
        );
    }

    Ok(transfer_id_hex)
}

/// Get all active transfers
#[tauri::command]
pub async fn get_transfers(state: State<'_, AppState>) -> AppResult<Vec<TransferInfo>> {
    let node = state.node.read().await;
    let Some(node) = node.as_ref() else {
        return Err(AppError::NodeNotRunning);
    };

    let transfer_ids = node.active_transfers().await;
    let mut result = Vec::with_capacity(transfer_ids.len());

    // Get stored transfer info and update with current progress
    let transfers = state.transfers.read().await;

    for transfer_id in transfer_ids {
        let transfer_id_hex = hex::encode(transfer_id);

        if let Some(progress) = node.get_transfer_progress(&transfer_id).await {
            let pct = if progress.bytes_total > 0 {
                (progress.bytes_sent as f32) / (progress.bytes_total as f32)
            } else {
                0.0
            };

            let is_complete =
                progress.bytes_sent >= progress.bytes_total && progress.bytes_total > 0;
            let status = if is_complete {
                "completed".to_string()
            } else {
                "in_progress".to_string()
            };

            // Get stored info or create default
            let stored = transfers.get(&transfer_id_hex);
            result.push(TransferInfo {
                id: transfer_id_hex,
                peer_id: stored.map(|s| s.peer_id.clone()).unwrap_or_default(),
                file_name: stored.map(|s| s.file_name.clone()).unwrap_or_default(),
                total_bytes: progress.bytes_total,
                transferred_bytes: progress.bytes_sent,
                progress: pct,
                status,
                direction: stored
                    .map(|s| s.direction.clone())
                    .unwrap_or_else(|| "upload".to_string()),
            });
        }
    }

    Ok(result)
}

/// Get progress for a specific transfer
#[tauri::command]
pub async fn get_transfer_progress(
    state: State<'_, AppState>,
    transfer_id: String,
) -> AppResult<Option<TransferInfo>> {
    let node = state.node.read().await;
    let Some(node) = node.as_ref() else {
        return Err(AppError::NodeNotRunning);
    };

    let transfer_bytes =
        hex::decode(&transfer_id).map_err(|_| AppError::Transfer("Invalid transfer ID".into()))?;

    if transfer_bytes.len() != 32 {
        return Err(AppError::Transfer("Transfer ID must be 32 bytes".into()));
    }

    let mut transfer_id_arr = [0u8; 32];
    transfer_id_arr.copy_from_slice(&transfer_bytes);

    if let Some(progress) = node.get_transfer_progress(&transfer_id_arr).await {
        let pct = if progress.bytes_total > 0 {
            (progress.bytes_sent as f32) / (progress.bytes_total as f32)
        } else {
            0.0
        };

        let is_complete = progress.bytes_sent >= progress.bytes_total && progress.bytes_total > 0;
        let status = if is_complete {
            "completed".to_string()
        } else {
            "in_progress".to_string()
        };

        let transfers = state.transfers.read().await;
        let stored = transfers.get(&transfer_id);

        Ok(Some(TransferInfo {
            id: transfer_id,
            peer_id: stored.map(|s| s.peer_id.clone()).unwrap_or_default(),
            file_name: stored.map(|s| s.file_name.clone()).unwrap_or_default(),
            total_bytes: progress.bytes_total,
            transferred_bytes: progress.bytes_sent,
            progress: pct,
            status,
            direction: stored
                .map(|s| s.direction.clone())
                .unwrap_or_else(|| "upload".to_string()),
        }))
    } else {
        Ok(None)
    }
}

/// Cancel an active transfer
#[tauri::command]
pub async fn cancel_transfer(state: State<'_, AppState>, transfer_id: String) -> AppResult<()> {
    let node = state.node.read().await;
    let Some(node) = node.as_ref() else {
        return Err(AppError::NodeNotRunning);
    };

    let transfer_bytes =
        hex::decode(&transfer_id).map_err(|_| AppError::Transfer("Invalid transfer ID".into()))?;

    if transfer_bytes.len() != 32 {
        return Err(AppError::Transfer("Transfer ID must be 32 bytes".into()));
    }

    let mut transfer_id_arr = [0u8; 32];
    transfer_id_arr.copy_from_slice(&transfer_bytes);

    // Call actual cancellation via Node API
    node.cancel_transfer(&transfer_id_arr)
        .await
        .map_err(|e| AppError::Transfer(e.to_string()))?;

    // Remove from tracked transfers
    {
        let mut transfers = state.transfers.write().await;
        transfers.remove(&transfer_id);
    }

    info!("Cancelled transfer: {}", transfer_id);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::sync::Arc;
    use tokio::sync::RwLock;

    /// Create a mock AppState for testing
    fn mock_app_state() -> AppState {
        AppState {
            node: Arc::new(RwLock::new(None)),
            transfers: Arc::new(RwLock::new(HashMap::new())),
            download_dir: Arc::new(RwLock::new(None)),
        }
    }

    #[tokio::test]
    async fn test_app_state_node_not_running() {
        let state = mock_app_state();
        let running = state.is_node_running().await;
        assert!(!running, "Node should not be running");
    }

    #[tokio::test]
    async fn test_app_state_node_id_none() {
        let state = mock_app_state();
        let node_id = state.get_node_id_hex().await;
        assert!(node_id.is_none(), "Node ID should be None");
    }

    #[tokio::test]
    async fn test_app_state_session_count() {
        let state = mock_app_state();
        let count = state.active_session_count().await;
        assert_eq!(count, 0, "Active sessions should be 0");
    }

    #[tokio::test]
    async fn test_app_state_transfer_count() {
        let state = mock_app_state();
        let count = state.active_transfer_count().await;
        assert_eq!(count, 0, "Active transfers should be 0");
    }

    #[tokio::test]
    async fn test_app_state_transfers_empty() {
        let state = mock_app_state();
        let transfers = state.transfers.read().await;
        assert!(transfers.is_empty(), "Transfers map should be empty");
    }

    #[tokio::test]
    async fn test_app_state_download_dir_none() {
        let state = mock_app_state();
        let download_dir = state.download_dir.read().await;
        assert!(download_dir.is_none(), "Download directory should be None");
    }
}
