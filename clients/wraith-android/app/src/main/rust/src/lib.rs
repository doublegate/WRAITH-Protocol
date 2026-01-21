// WRAITH Android JNI Bindings
//
// This library provides JNI bindings for the WRAITH protocol to enable
// Android applications to use WRAITH for secure file transfer and communication.

#![allow(unsafe_op_in_unsafe_fn)]

use jni::JNIEnv;
use jni::objects::{JClass, JString};
use jni::sys::{jlong, jstring};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use tokio::runtime::Runtime;
use wraith_core::node::{Node, NodeConfig};

mod discovery;
mod error;
#[cfg(test)]
mod integration_tests;
mod keystore;
mod push;
mod types;

use error::Error;

// Re-export keystore types for external use
pub use keystore::{KeyInfo, KeystoreError, KeystoreResult, SecureKeyStorage};

// Re-export push notification types for external use
pub use push::{PushAction, PushError, PushPayload, PushPlatform, PushSettings, PushToken};

/// Global Tokio runtime for async operations
static RUNTIME: Mutex<Option<Arc<Runtime>>> = Mutex::new(None);

/// Global node instance
static NODE: Mutex<Option<Arc<Node>>> = Mutex::new(None);

/// Global counter for active transfers
static ACTIVE_TRANSFERS: AtomicUsize = AtomicUsize::new(0);

/// Transfer ID to peer ID mapping for tracking active transfers
static TRANSFER_MAP: RwLock<Option<HashMap<[u8; 32], TransferState>>> = RwLock::new(None);

/// Transfer state for tracking progress
/// Note: Fields are used for tracking and will be accessed in future progress reporting
#[derive(Clone)]
#[allow(dead_code)]
struct TransferState {
    peer_id: [u8; 32],
    file_path: String,
    total_bytes: u64,
    bytes_transferred: u64,
    is_complete: bool,
    is_cancelled: bool,
}

/// Initialize transfer map if not already done
fn init_transfer_map() {
    if let Ok(mut map) = TRANSFER_MAP.write() {
        if map.is_none() {
            *map = Some(HashMap::new());
        }
    }
}

/// Increment active transfer count
fn increment_transfers() {
    ACTIVE_TRANSFERS.fetch_add(1, Ordering::SeqCst);
}

/// Decrement active transfer count
fn decrement_transfers() {
    let current = ACTIVE_TRANSFERS.load(Ordering::SeqCst);
    if current > 0 {
        ACTIVE_TRANSFERS.fetch_sub(1, Ordering::SeqCst);
    }
}

/// Get current active transfer count
fn get_active_transfers() -> usize {
    ACTIVE_TRANSFERS.load(Ordering::SeqCst)
}

/// Get or create the runtime
fn get_runtime() -> Result<Arc<Runtime>, Error> {
    let mut rt_lock = RUNTIME
        .lock()
        .map_err(|e| Error::Other(format!("Lock poisoned: {}", e)))?;

    if rt_lock.is_none() {
        let rt =
            Runtime::new().map_err(|e| Error::Other(format!("Failed to create runtime: {}", e)))?;
        *rt_lock = Some(Arc::new(rt));
    }

    rt_lock
        .clone()
        .ok_or_else(|| Error::Other("Runtime not available".to_string()))
}

/// Helper to convert JString to Rust String safely
fn jstring_to_string(env: &mut JNIEnv, s: &JString) -> Result<String, Error> {
    env.get_string(s).map(|s| s.into()).map_err(Error::Jni)
}

/// Helper to create JSON error response
fn error_response(message: &str) -> String {
    serde_json::json!({
        "error": true,
        "message": message
    })
    .to_string()
}

/// Initialize the WRAITH node
///
/// # Safety
/// This function is called from Java via JNI. It expects:
/// - `listen_addr`: String - The address to listen on (e.g., "0.0.0.0:0")
///
/// Returns: jlong representing the node handle, or -1 on error
#[unsafe(no_mangle)]
pub extern "C" fn Java_com_wraith_android_WraithNative_initNode(
    mut env: JNIEnv,
    _class: JClass,
    listen_addr: JString,
) -> jlong {
    // Initialize Android logging
    android_logger::init_once(
        android_logger::Config::default()
            .with_max_level(log::LevelFilter::Info)
            .with_tag("wraith-android"),
    );

    // Initialize transfer map
    init_transfer_map();

    // Get runtime
    let rt = match get_runtime() {
        Ok(rt) => rt,
        Err(e) => {
            log::error!("Failed to get runtime: {}", e);
            return -1;
        }
    };

    // Parse listen address
    let listen_addr_str: String = match jstring_to_string(&mut env, &listen_addr) {
        Ok(s) => s,
        Err(e) => {
            log::error!("Failed to parse listen_addr: {}", e);
            return -1;
        }
    };

    let listen_addr_parsed: std::net::SocketAddr = match listen_addr_str.parse() {
        Ok(addr) => addr,
        Err(e) => {
            log::error!("Failed to parse listen address: {}", e);
            return -1;
        }
    };

    // Create node with custom configuration
    let config = NodeConfig {
        listen_addr: listen_addr_parsed,
        ..NodeConfig::default()
    };

    let node = match rt.block_on(async { Node::new_with_config(config).await }) {
        Ok(n) => Arc::new(n),
        Err(e) => {
            log::error!("Failed to create WRAITH node: {}", e);
            return -1;
        }
    };

    // Start the node
    if let Err(e) = rt.block_on(node.start()) {
        log::error!("Failed to start node: {}", e);
        return -1;
    }

    // Store node globally
    {
        if let Ok(mut node_lock) = NODE.lock() {
            *node_lock = Some(node.clone());
        } else {
            log::error!("Failed to acquire node lock");
            return -1;
        }
    }

    // Return node handle (use Arc pointer as handle)
    Arc::into_raw(node) as jlong
}

/// Shut down the WRAITH node
///
/// # Safety
/// This function is called from Java via JNI. It expects:
/// - `handle`: jlong - The node handle returned from initNode
#[unsafe(no_mangle)]
pub extern "C" fn Java_com_wraith_android_WraithNative_shutdownNode(
    _env: JNIEnv,
    _class: JClass,
    handle: jlong,
) {
    if handle == 0 || handle == -1 {
        return;
    }

    let node = unsafe { Arc::from_raw(handle as *const Node) };

    if let Ok(rt) = get_runtime() {
        rt.block_on(async {
            if let Err(e) = node.stop().await {
                log::error!("Error during node shutdown: {}", e);
            }
        });
    }

    // Clear global node
    if let Ok(mut node_lock) = NODE.lock() {
        *node_lock = None;
    }

    // Clear transfer map
    if let Ok(mut map) = TRANSFER_MAP.write() {
        *map = None;
    }
}

/// Establish a session with a remote peer
///
/// # Safety
/// This function is called from Java via JNI. It expects:
/// - `handle`: jlong - The node handle
/// - `peer_id`: String - The peer ID to connect to (hex encoded)
/// - `peer_addr`: String - The peer's address (IP:port)
///
/// Returns: jstring (JSON) containing session info, or null on error
#[unsafe(no_mangle)]
pub extern "C" fn Java_com_wraith_android_WraithNative_establishSession(
    mut env: JNIEnv,
    _class: JClass,
    handle: jlong,
    peer_id: JString,
    peer_addr: JString,
) -> jstring {
    if handle == 0 || handle == -1 {
        return std::ptr::null_mut();
    }

    let node = unsafe { &*(handle as *const Node) };

    let peer_id_str: String = match jstring_to_string(&mut env, &peer_id) {
        Ok(s) => s,
        Err(e) => {
            log::error!("Failed to parse peer_id: {}", e);
            return std::ptr::null_mut();
        }
    };

    let peer_addr_str: String = match jstring_to_string(&mut env, &peer_addr) {
        Ok(s) => s,
        Err(e) => {
            log::error!("Failed to parse peer_addr: {}", e);
            return std::ptr::null_mut();
        }
    };

    let rt = match get_runtime() {
        Ok(rt) => rt,
        Err(e) => {
            log::error!("Failed to get runtime: {}", e);
            return std::ptr::null_mut();
        }
    };

    let session_info = match rt.block_on(async {
        // Convert peer_id string to PeerId type
        let peer_id_bytes = hex::decode(&peer_id_str)
            .map_err(|e| Error::Other(format!("Invalid peer ID hex: {}", e)))?;
        let peer_id_array: [u8; 32] = peer_id_bytes
            .try_into()
            .map_err(|_| Error::Other("Peer ID must be 32 bytes".to_string()))?;

        // Parse peer address
        let peer_socket_addr: std::net::SocketAddr = peer_addr_str
            .parse()
            .map_err(|e| Error::Other(format!("Invalid peer address: {}", e)))?;

        // Establish session with address
        let session_id = node
            .establish_session_with_addr(&peer_id_array, peer_socket_addr)
            .await
            .map_err(|e| Error::Protocol(e.to_string()))?;

        // Build response JSON
        let info = serde_json::json!({
            "sessionId": hex::encode(session_id),
            "peerId": hex::encode(peer_id_array),
            "peerAddr": peer_addr_str,
            "connected": true,
        });

        Ok::<_, Error>(info.to_string())
    }) {
        Ok(info) => info,
        Err(e) => {
            log::error!("Failed to establish session: {}", e);
            return std::ptr::null_mut();
        }
    };

    match env.new_string(session_info) {
        Ok(s) => s.into_raw(),
        Err(e) => {
            log::error!("Failed to create Java string: {}", e);
            std::ptr::null_mut()
        }
    }
}

/// Close a session with a peer
///
/// # Safety
/// This function is called from Java via JNI.
#[unsafe(no_mangle)]
pub extern "C" fn Java_com_wraith_android_WraithNative_closeSession(
    mut env: JNIEnv,
    _class: JClass,
    handle: jlong,
    peer_id: JString,
) -> jstring {
    if handle == 0 || handle == -1 {
        return std::ptr::null_mut();
    }

    let node = unsafe { &*(handle as *const Node) };

    let peer_id_str: String = match jstring_to_string(&mut env, &peer_id) {
        Ok(s) => s,
        Err(e) => {
            log::error!("Failed to parse peer_id: {}", e);
            return std::ptr::null_mut();
        }
    };

    let rt = match get_runtime() {
        Ok(rt) => rt,
        Err(e) => {
            log::error!("Failed to get runtime: {}", e);
            return std::ptr::null_mut();
        }
    };

    let result = rt.block_on(async {
        let peer_id_bytes = hex::decode(&peer_id_str)
            .map_err(|e| Error::Other(format!("Invalid peer ID hex: {}", e)))?;
        let peer_id_array: [u8; 32] = peer_id_bytes
            .try_into()
            .map_err(|_| Error::Other("Peer ID must be 32 bytes".to_string()))?;

        node.close_session(&peer_id_array)
            .await
            .map_err(|e| Error::Protocol(e.to_string()))?;

        let info = serde_json::json!({
            "success": true,
            "peerId": hex::encode(peer_id_array),
        });

        Ok::<_, Error>(info.to_string())
    });

    match result {
        Ok(info) => match env.new_string(info) {
            Ok(s) => s.into_raw(),
            Err(_) => std::ptr::null_mut(),
        },
        Err(e) => {
            log::error!("Failed to close session: {}", e);
            match env.new_string(error_response(&e.to_string())) {
                Ok(s) => s.into_raw(),
                Err(_) => std::ptr::null_mut(),
            }
        }
    }
}

/// Send a file to a peer
///
/// # Safety
/// This function is called from Java via JNI. It expects:
/// - `handle`: jlong - The node handle
/// - `peer_id`: String - The peer ID (hex encoded)
/// - `file_path`: String - Path to the file to send
///
/// Returns: jstring (JSON) containing transfer info, or null on error
#[unsafe(no_mangle)]
pub extern "C" fn Java_com_wraith_android_WraithNative_sendFile(
    mut env: JNIEnv,
    _class: JClass,
    handle: jlong,
    peer_id: JString,
    file_path: JString,
) -> jstring {
    if handle == 0 || handle == -1 {
        return std::ptr::null_mut();
    }

    let node = unsafe { &*(handle as *const Node) };

    let peer_id_str: String = match jstring_to_string(&mut env, &peer_id) {
        Ok(s) => s,
        Err(_) => return std::ptr::null_mut(),
    };

    let file_path_str: String = match jstring_to_string(&mut env, &file_path) {
        Ok(s) => s,
        Err(_) => return std::ptr::null_mut(),
    };

    let rt = match get_runtime() {
        Ok(rt) => rt,
        Err(e) => {
            log::error!("Failed to get runtime: {}", e);
            return std::ptr::null_mut();
        }
    };

    // Track transfer start
    increment_transfers();

    let transfer_info = match rt.block_on(async {
        let peer_id_bytes = hex::decode(&peer_id_str)
            .map_err(|e| Error::Other(format!("Invalid peer ID: {}", e)))?;
        let peer_id_array: [u8; 32] = peer_id_bytes
            .try_into()
            .map_err(|_| Error::Other("Invalid peer ID length".to_string()))?;

        let path = PathBuf::from(&file_path_str);

        // Get file size
        let file_size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);

        // Send file using Node API
        let transfer_id = node
            .send_file(path.clone(), &peer_id_array)
            .await
            .map_err(|e| Error::Protocol(e.to_string()))?;

        // Track the transfer
        if let Ok(mut map) = TRANSFER_MAP.write() {
            if let Some(ref mut m) = *map {
                m.insert(
                    transfer_id,
                    TransferState {
                        peer_id: peer_id_array,
                        file_path: file_path_str.clone(),
                        total_bytes: file_size,
                        bytes_transferred: 0,
                        is_complete: false,
                        is_cancelled: false,
                    },
                );
            }
        }

        let info = serde_json::json!({
            "transferId": hex::encode(transfer_id),
            "peerId": hex::encode(peer_id_array),
            "filePath": file_path_str,
            "fileSize": file_size,
            "status": "sending",
        });

        Ok::<_, Error>(info.to_string())
    }) {
        Ok(info) => info,
        Err(e) => {
            log::error!("Failed to send file: {}", e);
            decrement_transfers(); // Decrement on failure
            return std::ptr::null_mut();
        }
    };

    match env.new_string(transfer_info) {
        Ok(s) => s.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Get transfer progress
///
/// # Safety
/// This function is called from Java via JNI.
#[unsafe(no_mangle)]
pub extern "C" fn Java_com_wraith_android_WraithNative_getTransferProgress(
    mut env: JNIEnv,
    _class: JClass,
    handle: jlong,
    transfer_id: JString,
) -> jstring {
    if handle == 0 || handle == -1 {
        return std::ptr::null_mut();
    }

    let node = unsafe { &*(handle as *const Node) };

    let transfer_id_str: String = match jstring_to_string(&mut env, &transfer_id) {
        Ok(s) => s,
        Err(_) => return std::ptr::null_mut(),
    };

    let rt = match get_runtime() {
        Ok(rt) => rt,
        Err(e) => {
            log::error!("Failed to get runtime: {}", e);
            return std::ptr::null_mut();
        }
    };

    let progress_info = rt.block_on(async {
        let transfer_id_bytes = hex::decode(&transfer_id_str)
            .map_err(|e| Error::Other(format!("Invalid transfer ID: {}", e)))?;
        let transfer_id_array: [u8; 32] = transfer_id_bytes
            .try_into()
            .map_err(|_| Error::Other("Invalid transfer ID length".to_string()))?;

        // Get progress from node
        if let Some(progress) = node.get_transfer_progress(&transfer_id_array).await {
            let info = serde_json::json!({
                "transferId": transfer_id_str,
                "totalBytes": progress.bytes_total,
                "bytesTransferred": progress.bytes_sent,
                "progress": progress.progress_percent,
                "speedBytesPerSec": progress.speed_bytes_per_sec,
                "etaSeconds": progress.eta.map(|d| d.as_secs()).unwrap_or(0),
                "isComplete": progress.is_complete(),
            });
            Ok(info.to_string())
        } else {
            Err(Error::Other("Transfer not found".to_string()))
        }
    });

    match progress_info {
        Ok(info) => match env.new_string(info) {
            Ok(s) => s.into_raw(),
            Err(_) => std::ptr::null_mut(),
        },
        Err(e) => {
            log::error!("Failed to get transfer progress: {}", e);
            std::ptr::null_mut()
        }
    }
}

/// Cancel an active transfer
///
/// # Safety
/// This function is called from Java via JNI.
#[unsafe(no_mangle)]
pub extern "C" fn Java_com_wraith_android_WraithNative_cancelTransfer(
    mut env: JNIEnv,
    _class: JClass,
    handle: jlong,
    transfer_id: JString,
) -> jstring {
    if handle == 0 || handle == -1 {
        return std::ptr::null_mut();
    }

    let node = unsafe { &*(handle as *const Node) };

    let transfer_id_str: String = match jstring_to_string(&mut env, &transfer_id) {
        Ok(s) => s,
        Err(_) => return std::ptr::null_mut(),
    };

    let rt = match get_runtime() {
        Ok(rt) => rt,
        Err(e) => {
            log::error!("Failed to get runtime: {}", e);
            return std::ptr::null_mut();
        }
    };

    let result = rt.block_on(async {
        let transfer_id_bytes = hex::decode(&transfer_id_str)
            .map_err(|e| Error::Other(format!("Invalid transfer ID: {}", e)))?;
        let transfer_id_array: [u8; 32] = transfer_id_bytes
            .try_into()
            .map_err(|_| Error::Other("Invalid transfer ID length".to_string()))?;

        // Cancel the transfer
        node.cancel_transfer(&transfer_id_array)
            .await
            .map_err(|e| Error::Protocol(e.to_string()))?;

        // Update tracking
        if let Ok(mut map) = TRANSFER_MAP.write() {
            if let Some(ref mut m) = *map {
                if let Some(state) = m.get_mut(&transfer_id_array) {
                    state.is_cancelled = true;
                }
            }
        }

        decrement_transfers();

        let info = serde_json::json!({
            "success": true,
            "transferId": transfer_id_str,
            "status": "cancelled",
        });

        Ok::<_, Error>(info.to_string())
    });

    match result {
        Ok(info) => match env.new_string(info) {
            Ok(s) => s.into_raw(),
            Err(_) => std::ptr::null_mut(),
        },
        Err(e) => {
            log::error!("Failed to cancel transfer: {}", e);
            match env.new_string(error_response(&e.to_string())) {
                Ok(s) => s.into_raw(),
                Err(_) => std::ptr::null_mut(),
            }
        }
    }
}

/// Get session statistics
///
/// # Safety
/// This function is called from Java via JNI.
#[unsafe(no_mangle)]
pub extern "C" fn Java_com_wraith_android_WraithNative_getSessionStats(
    mut env: JNIEnv,
    _class: JClass,
    handle: jlong,
    peer_id: JString,
) -> jstring {
    if handle == 0 || handle == -1 {
        return std::ptr::null_mut();
    }

    let node = unsafe { &*(handle as *const Node) };

    let peer_id_str: String = match jstring_to_string(&mut env, &peer_id) {
        Ok(s) => s,
        Err(_) => return std::ptr::null_mut(),
    };

    let peer_id_bytes = match hex::decode(&peer_id_str) {
        Ok(bytes) => bytes,
        Err(_) => return std::ptr::null_mut(),
    };

    let peer_id_array: [u8; 32] = match peer_id_bytes.try_into() {
        Ok(arr) => arr,
        Err(_) => return std::ptr::null_mut(),
    };

    let stats = node.get_connection_stats(&peer_id_array);

    let stats_json = match stats {
        Some(s) => serde_json::json!({
            "peerId": peer_id_str,
            "bytesSent": s.bytes_sent,
            "bytesReceived": s.bytes_received,
            "packetsSent": s.packets_sent,
            "packetsReceived": s.packets_received,
            "rttUs": s.rtt_us.unwrap_or(0),
            "lossRate": s.loss_rate,
        }),
        None => serde_json::json!({
            "error": "Session not found",
            "peerId": peer_id_str,
        }),
    };

    match env.new_string(stats_json.to_string()) {
        Ok(s) => s.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Get node status
///
/// # Safety
/// This function is called from Java via JNI.
///
/// Returns: jstring (JSON) containing node status
#[unsafe(no_mangle)]
pub extern "C" fn Java_com_wraith_android_WraithNative_getNodeStatus(
    env: JNIEnv,
    _class: JClass,
    handle: jlong,
) -> jstring {
    if handle == 0 || handle == -1 {
        return std::ptr::null_mut();
    }

    let node = unsafe { &*(handle as *const Node) };

    let status = serde_json::json!({
        "running": node.is_running(),
        "localPeerId": hex::encode(node.node_id()),
        "sessionCount": node.active_route_count(),
        "activeTransfers": get_active_transfers(),
    });

    // JNIEnv uses interior mutability for string operations
    #[allow(unused_mut)]
    let mut env = env;
    match env.new_string(status.to_string()) {
        Ok(s) => s.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Get the local peer ID
///
/// # Safety
/// This function is called from Java via JNI.
#[unsafe(no_mangle)]
pub extern "C" fn Java_com_wraith_android_WraithNative_getLocalPeerId(
    env: JNIEnv,
    _class: JClass,
    handle: jlong,
) -> jstring {
    if handle == 0 || handle == -1 {
        return std::ptr::null_mut();
    }

    let node = unsafe { &*(handle as *const Node) };
    let peer_id = hex::encode(node.node_id());

    // JNIEnv uses interior mutability for string operations
    #[allow(unused_mut)]
    let mut env = env;
    match env.new_string(peer_id) {
        Ok(s) => s.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}

// ============================================================================
// Discovery JNI Bindings
// ============================================================================

/// Initialize the discovery service
///
/// # Safety
/// This function is called from Java via JNI. It expects:
/// - `node_id`: String - The node ID (hex encoded)
/// - `listen_addr`: String - The address to listen on
/// - `bootstrap_nodes`: String - Comma-separated list of bootstrap nodes
/// - `stun_servers`: String - Comma-separated list of STUN servers
///
/// Returns: jstring (JSON) containing initialization result
#[unsafe(no_mangle)]
pub extern "C" fn Java_com_wraith_android_WraithNative_initDiscovery(
    mut env: JNIEnv,
    _class: JClass,
    node_id: JString,
    listen_addr: JString,
    bootstrap_nodes: JString,
    stun_servers: JString,
) -> jstring {
    let node_id_str = match jstring_to_string(&mut env, &node_id) {
        Ok(s) => s,
        Err(e) => {
            log::error!("Failed to parse node_id: {}", e);
            return std::ptr::null_mut();
        }
    };

    let listen_addr_str = match jstring_to_string(&mut env, &listen_addr) {
        Ok(s) => s,
        Err(e) => {
            log::error!("Failed to parse listen_addr: {}", e);
            return std::ptr::null_mut();
        }
    };

    let bootstrap_str = jstring_to_string(&mut env, &bootstrap_nodes).unwrap_or_default();
    let stun_str = jstring_to_string(&mut env, &stun_servers).unwrap_or_default();

    let rt = match get_runtime() {
        Ok(rt) => rt,
        Err(e) => {
            log::error!("Failed to get runtime: {}", e);
            return std::ptr::null_mut();
        }
    };

    let result = rt.block_on(async {
        use wraith_discovery::dht::NodeId;

        // Parse node ID
        let node_id_bytes = hex::decode(&node_id_str)
            .map_err(|e| Error::Other(format!("Invalid node ID: {}", e)))?;
        let node_id_array: [u8; 32] = node_id_bytes
            .try_into()
            .map_err(|_| Error::Other("Node ID must be 32 bytes".to_string()))?;
        let node_id = NodeId::from_bytes(node_id_array);

        // Parse listen address
        let listen_addr: std::net::SocketAddr = listen_addr_str
            .parse()
            .map_err(|e| Error::Other(format!("Invalid listen address: {}", e)))?;

        // Parse bootstrap nodes
        let bootstrap_nodes: Vec<std::net::SocketAddr> = bootstrap_str
            .split(',')
            .filter(|s| !s.is_empty())
            .filter_map(|s| s.trim().parse().ok())
            .collect();

        // Parse STUN servers
        let stun_servers: Vec<std::net::SocketAddr> = stun_str
            .split(',')
            .filter(|s| !s.is_empty())
            .filter_map(|s| s.trim().parse().ok())
            .collect();

        let config = discovery::MobileDiscoveryConfig {
            node_id,
            listen_addr,
            bootstrap_nodes,
            stun_servers,
        };

        discovery::init_discovery(config).await?;

        let info = serde_json::json!({
            "success": true,
            "message": "Discovery initialized",
        });

        Ok::<_, Error>(info.to_string())
    });

    match result {
        Ok(info) => match env.new_string(info) {
            Ok(s) => s.into_raw(),
            Err(_) => std::ptr::null_mut(),
        },
        Err(e) => {
            log::error!("Failed to initialize discovery: {}", e);
            match env.new_string(error_response(&e.to_string())) {
                Ok(s) => s.into_raw(),
                Err(_) => std::ptr::null_mut(),
            }
        }
    }
}

/// Shutdown the discovery service
///
/// # Safety
/// This function is called from Java via JNI.
#[unsafe(no_mangle)]
pub extern "C" fn Java_com_wraith_android_WraithNative_shutdownDiscovery(
    env: JNIEnv,
    _class: JClass,
) -> jstring {
    let rt = match get_runtime() {
        Ok(rt) => rt,
        Err(e) => {
            log::error!("Failed to get runtime: {}", e);
            return std::ptr::null_mut();
        }
    };

    let result = rt.block_on(async { discovery::shutdown_discovery().await });

    #[allow(unused_mut)]
    let mut env = env;
    match result {
        Ok(()) => {
            let info = serde_json::json!({
                "success": true,
                "message": "Discovery shutdown",
            });
            match env.new_string(info.to_string()) {
                Ok(s) => s.into_raw(),
                Err(_) => std::ptr::null_mut(),
            }
        }
        Err(e) => {
            log::error!("Failed to shutdown discovery: {}", e);
            match env.new_string(error_response(&e.to_string())) {
                Ok(s) => s.into_raw(),
                Err(_) => std::ptr::null_mut(),
            }
        }
    }
}

/// Discover a peer by ID
///
/// # Safety
/// This function is called from Java via JNI.
/// - `peer_id`: String - The peer ID to discover (hex encoded)
///
/// Returns: jstring (JSON) containing peer information
#[unsafe(no_mangle)]
pub extern "C" fn Java_com_wraith_android_WraithNative_discoverPeer(
    mut env: JNIEnv,
    _class: JClass,
    peer_id: JString,
) -> jstring {
    let peer_id_str = match jstring_to_string(&mut env, &peer_id) {
        Ok(s) => s,
        Err(e) => {
            log::error!("Failed to parse peer_id: {}", e);
            return std::ptr::null_mut();
        }
    };

    let rt = match get_runtime() {
        Ok(rt) => rt,
        Err(e) => {
            log::error!("Failed to get runtime: {}", e);
            return std::ptr::null_mut();
        }
    };

    let result = rt.block_on(async { discovery::discover_peer(&peer_id_str).await });

    match result {
        Ok(peer_info) => {
            let info = serde_json::json!({
                "success": true,
                "peerId": peer_info.peer_id,
                "address": peer_info.address,
                "connectionType": peer_info.connection_type,
                "lastSeen": peer_info.last_seen,
            });
            match env.new_string(info.to_string()) {
                Ok(s) => s.into_raw(),
                Err(_) => std::ptr::null_mut(),
            }
        }
        Err(e) => {
            log::error!("Failed to discover peer: {}", e);
            match env.new_string(error_response(&e.to_string())) {
                Ok(s) => s.into_raw(),
                Err(_) => std::ptr::null_mut(),
            }
        }
    }
}

/// Detect NAT type and get external address
///
/// # Safety
/// This function is called from Java via JNI.
///
/// Returns: jstring (JSON) containing NAT information
#[unsafe(no_mangle)]
pub extern "C" fn Java_com_wraith_android_WraithNative_detectNat(
    env: JNIEnv,
    _class: JClass,
) -> jstring {
    let rt = match get_runtime() {
        Ok(rt) => rt,
        Err(e) => {
            log::error!("Failed to get runtime: {}", e);
            return std::ptr::null_mut();
        }
    };

    let result = rt.block_on(async { discovery::detect_nat().await });

    #[allow(unused_mut)]
    let mut env = env;
    match result {
        Ok(nat_info) => {
            let info = serde_json::json!({
                "success": true,
                "natType": nat_info.nat_type,
                "externalIp": nat_info.external_ip,
                "externalPort": nat_info.external_port,
                "holePunchCapable": nat_info.hole_punch_capable,
            });
            match env.new_string(info.to_string()) {
                Ok(s) => s.into_raw(),
                Err(_) => std::ptr::null_mut(),
            }
        }
        Err(e) => {
            log::error!("Failed to detect NAT: {}", e);
            match env.new_string(error_response(&e.to_string())) {
                Ok(s) => s.into_raw(),
                Err(_) => std::ptr::null_mut(),
            }
        }
    }
}

/// Notify that the network has changed
///
/// # Safety
/// This function is called from Java via JNI.
/// - `network_type`: String - The network type ("wifi", "cellular", "unknown")
#[unsafe(no_mangle)]
pub extern "C" fn Java_com_wraith_android_WraithNative_onNetworkChanged(
    mut env: JNIEnv,
    _class: JClass,
    network_type: JString,
) {
    let network_str = match jstring_to_string(&mut env, &network_type) {
        Ok(s) => s,
        Err(_) => return,
    };

    let network = match network_str.to_lowercase().as_str() {
        "wifi" => discovery::MobileNetworkType::Wifi,
        "cellular" => discovery::MobileNetworkType::Cellular,
        _ => discovery::MobileNetworkType::Unknown,
    };

    if let Ok(rt) = get_runtime() {
        rt.block_on(async {
            discovery::on_network_changed(network).await;
        });
    }
}

/// Notify that the app has been backgrounded
///
/// # Safety
/// This function is called from Java via JNI.
#[unsafe(no_mangle)]
pub extern "C" fn Java_com_wraith_android_WraithNative_onAppBackgrounded(
    _env: JNIEnv,
    _class: JClass,
) {
    if let Ok(rt) = get_runtime() {
        rt.block_on(async {
            discovery::on_app_backgrounded().await;
        });
    }
}

/// Notify that the app has been foregrounded
///
/// # Safety
/// This function is called from Java via JNI.
#[unsafe(no_mangle)]
pub extern "C" fn Java_com_wraith_android_WraithNative_onAppForegrounded(
    _env: JNIEnv,
    _class: JClass,
) {
    if let Ok(rt) = get_runtime() {
        rt.block_on(async {
            discovery::on_app_foregrounded().await;
        });
    }
}

/// Get the current discovery state
///
/// # Safety
/// This function is called from Java via JNI.
///
/// Returns: jstring containing the state ("running", "stopped", etc.)
#[unsafe(no_mangle)]
pub extern "C" fn Java_com_wraith_android_WraithNative_getDiscoveryState(
    env: JNIEnv,
    _class: JClass,
) -> jstring {
    let rt = match get_runtime() {
        Ok(rt) => rt,
        Err(_) => return std::ptr::null_mut(),
    };

    let state = rt.block_on(async { discovery::get_discovery_state().await });

    #[allow(unused_mut)]
    let mut env = env;
    match env.new_string(state) {
        Ok(s) => s.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transfer_tracking() {
        init_transfer_map();

        // Reset counter for test isolation
        ACTIVE_TRANSFERS.store(0, Ordering::SeqCst);

        assert_eq!(get_active_transfers(), 0);
        increment_transfers();
        assert_eq!(get_active_transfers(), 1);
        increment_transfers();
        assert_eq!(get_active_transfers(), 2);
        decrement_transfers();
        assert_eq!(get_active_transfers(), 1);
        decrement_transfers();
        assert_eq!(get_active_transfers(), 0);
        // Should not underflow
        decrement_transfers();
        assert_eq!(get_active_transfers(), 0);
    }

    #[test]
    fn test_runtime_creation() {
        let rt = get_runtime();
        assert!(rt.is_ok());

        // Second call should return the same runtime
        let rt2 = get_runtime();
        assert!(rt2.is_ok());
    }

    #[test]
    fn test_error_response() {
        let response = error_response("test error");
        assert!(response.contains("error"));
        assert!(response.contains("test error"));
    }

    #[test]
    fn test_transfer_map_initialization() {
        init_transfer_map();

        // Verify map is initialized
        let map = TRANSFER_MAP.read().expect("Should read transfer map");
        assert!(map.is_some());
    }

    #[test]
    fn test_transfer_state_creation() {
        let state = TransferState {
            peer_id: [1u8; 32],
            file_path: "/test/file.txt".to_string(),
            total_bytes: 1000,
            bytes_transferred: 500,
            is_complete: false,
            is_cancelled: false,
        };

        assert_eq!(state.total_bytes, 1000);
        assert_eq!(state.bytes_transferred, 500);
        assert!(!state.is_complete);
        assert!(!state.is_cancelled);
    }

    #[test]
    fn test_peer_id_hex_encoding() {
        let peer_id: [u8; 32] = [
            0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e,
            0x0f, 0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0x1a, 0x1b, 0x1c,
            0x1d, 0x1e, 0x1f, 0x20,
        ];

        let encoded = hex::encode(peer_id);
        assert_eq!(encoded.len(), 64);

        let decoded = hex::decode(&encoded).expect("Should decode");
        assert_eq!(decoded, peer_id);
    }

    #[test]
    fn test_invalid_peer_id_hex() {
        let invalid_hex = "not_valid_hex";
        let result = hex::decode(invalid_hex);
        assert!(result.is_err());
    }

    #[test]
    fn test_peer_id_wrong_length() {
        let short_hex = "0102030405"; // Only 5 bytes
        let decoded = hex::decode(short_hex).expect("Should decode hex");
        let result: Result<[u8; 32], _> = decoded.try_into();
        assert!(result.is_err());
    }

    #[test]
    fn test_socket_address_parsing() {
        let valid_addr = "127.0.0.1:8420";
        let parsed: Result<std::net::SocketAddr, _> = valid_addr.parse();
        assert!(parsed.is_ok());

        let invalid_addr = "not_an_address";
        let parsed: Result<std::net::SocketAddr, _> = invalid_addr.parse();
        assert!(parsed.is_err());
    }

    #[test]
    fn test_concurrent_atomic_operations() {
        use std::sync::atomic::AtomicUsize;
        use std::thread;

        // Test that AtomicUsize operations are thread-safe using an isolated counter
        // We don't use the global ACTIVE_TRANSFERS here to avoid race conditions
        // with other tests running in parallel
        let test_counter = std::sync::Arc::new(AtomicUsize::new(0));
        let mut handles = vec![];

        for _ in 0..10 {
            let counter_clone = test_counter.clone();
            handles.push(thread::spawn(move || {
                counter_clone.fetch_add(1, Ordering::SeqCst);
            }));
        }

        for handle in handles {
            handle.join().unwrap();
        }

        // Verify all 10 increments happened atomically
        assert_eq!(test_counter.load(Ordering::SeqCst), 10);
    }
}
