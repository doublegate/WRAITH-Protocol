// WRAITH Android JNI Bindings
//
// This library provides JNI bindings for the WRAITH protocol to enable
// Android applications to use WRAITH for secure file transfer and communication.

use jni::objects::{JClass, JObject, JString};
use jni::sys::{jbyteArray, jint, jlong, jstring};
use jni::JNIEnv;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use tokio::runtime::Runtime;
use wraith_core::node::{Node, NodeConfig};

mod error;
mod types;

use error::{Error, Result};
use types::*;

/// Global Tokio runtime for async operations
static RUNTIME: Mutex<Option<Runtime>> = Mutex::new(None);

/// Global node instance
static NODE: Mutex<Option<Arc<Node>>> = Mutex::new(None);

/// Global counter for active transfers
static ACTIVE_TRANSFERS: AtomicUsize = AtomicUsize::new(0);

/// Increment active transfer count
fn increment_transfers() {
    ACTIVE_TRANSFERS.fetch_add(1, Ordering::SeqCst);
}

/// Decrement active transfer count
fn decrement_transfers() {
    ACTIVE_TRANSFERS.fetch_sub(1, Ordering::SeqCst);
}

/// Get current active transfer count
fn get_active_transfers() -> usize {
    ACTIVE_TRANSFERS.load(Ordering::SeqCst)
}

/// Initialize the WRAITH node
///
/// # Safety
/// This function is called from Java via JNI. It expects:
/// - `listen_addr`: String - The address to listen on (e.g., "0.0.0.0:0")
/// - `config_json`: String - JSON configuration for the node
///
/// Returns: jlong representing the node handle, or -1 on error
#[no_mangle]
pub unsafe extern "C" fn Java_com_wraith_android_WraithNative_initNode(
    mut env: JNIEnv,
    _class: JClass,
    listen_addr: JString,
    config_json: JString,
) -> jlong {
    android_logger::init_once(
        android_logger::Config::default()
            .with_max_level(log::LevelFilter::Info)
            .with_tag("wraith-android"),
    );

    // Initialize runtime if not already done
    {
        let mut rt_lock = match RUNTIME.lock() {
            Ok(guard) => guard,
            Err(e) => {
                log::error!("Failed to acquire runtime lock: {}", e);
                return -1;
            }
        };
        if rt_lock.is_none() {
            match Runtime::new() {
                Ok(rt) => *rt_lock = Some(rt),
                Err(e) => {
                    log::error!("Failed to create Tokio runtime: {}", e);
                    return -1;
                }
            }
        }
    }

    // Parse parameters
    let listen_addr: String = match env.get_string(&listen_addr) {
        Ok(s) => s.into(),
        Err(e) => {
            log::error!("Failed to parse listen_addr: {}", e);
            return -1;
        }
    };

    let config_json: String = match env.get_string(&config_json) {
        Ok(s) => s.into(),
        Err(e) => {
            log::error!("Failed to parse config_json: {}", e);
            return -1;
        }
    };

    let config: NodeConfig = match serde_json::from_str(&config_json) {
        Ok(c) => c,
        Err(e) => {
            log::error!("Failed to parse NodeConfig: {}", e);
            return -1;
        }
    };

    // Create node
    let rt = match RUNTIME.lock() {
        Ok(guard) => guard,
        Err(e) => {
            log::error!("Failed to acquire runtime lock: {}", e);
            return -1;
        }
    };
    let rt = match rt.as_ref() {
        Some(rt) => rt,
        None => {
            log::error!("Runtime not initialized");
            return -1;
        }
    };

    let node = match rt.block_on(async { Node::new(config).await }) {
        Ok(n) => Arc::new(n),
        Err(e) => {
            log::error!("Failed to create WRAITH node: {}", e);
            return -1;
        }
    };

    // Start listening
    let listen_addr_parsed = match listen_addr.parse() {
        Ok(addr) => addr,
        Err(e) => {
            log::error!("Failed to parse listen address: {}", e);
            return -1;
        }
    };

    if let Err(e) = rt.block_on(node.start_listening(listen_addr_parsed)) {
        log::error!("Failed to start listening: {}", e);
        return -1;
    }

    // Store node globally
    {
        let mut node_lock = match NODE.lock() {
            Ok(guard) => guard,
            Err(e) => {
                log::error!("Failed to acquire node lock: {}", e);
                return -1;
            }
        };
        *node_lock = Some(node.clone());
    }

    // Return node handle (use Arc pointer as handle)
    Arc::into_raw(node) as jlong
}

/// Shut down the WRAITH node
///
/// # Safety
/// This function is called from Java via JNI. It expects:
/// - `handle`: jlong - The node handle returned from initNode
#[no_mangle]
pub unsafe extern "C" fn Java_com_wraith_android_WraithNative_shutdownNode(
    _env: JNIEnv,
    _class: JClass,
    handle: jlong,
) {
    if handle == 0 {
        return;
    }

    let node = Arc::from_raw(handle as *const Node);

    if let Ok(rt) = RUNTIME.lock() {
        if let Some(rt) = rt.as_ref() {
            rt.block_on(async {
                if let Err(e) = node.stop().await {
                    log::error!("Error during node shutdown: {}", e);
                }
            });
        }
    }

    // Clear global node
    if let Ok(mut node_lock) = NODE.lock() {
        *node_lock = None;
    }
}

/// Establish a session with a remote peer
///
/// # Safety
/// This function is called from Java via JNI. It expects:
/// - `handle`: jlong - The node handle
/// - `peer_id`: String - The peer ID to connect to
///
/// Returns: jstring (JSON) containing session info, or null on error
#[no_mangle]
pub unsafe extern "C" fn Java_com_wraith_android_WraithNative_establishSession(
    mut env: JNIEnv,
    _class: JClass,
    handle: jlong,
    peer_id: JString,
) -> jstring {
    if handle == 0 {
        return std::ptr::null_mut();
    }

    let node = &*(handle as *const Node);

    let peer_id: String = match env.get_string(&peer_id) {
        Ok(s) => s.into(),
        Err(e) => {
            log::error!("Failed to parse peer_id: {}", e);
            return std::ptr::null_mut();
        }
    };

    let rt = match RUNTIME.lock() {
        Ok(guard) => guard,
        Err(e) => {
            log::error!("Failed to acquire runtime lock: {}", e);
            return std::ptr::null_mut();
        }
    };
    let rt = match rt.as_ref() {
        Some(rt) => rt,
        None => {
            log::error!("Runtime not initialized");
            return std::ptr::null_mut();
        }
    };

    let session_info = match rt.block_on(async {
        // Convert peer_id string to PeerId type
        let peer_id_bytes = hex::decode(&peer_id)
            .map_err(|e| anyhow::anyhow!("Invalid peer ID hex: {}", e))?;
        let peer_id_array: [u8; 32] = peer_id_bytes.try_into()
            .map_err(|_| anyhow::anyhow!("Peer ID must be 32 bytes"))?;

        // Establish session
        let session = node.establish_session(peer_id_array).await?;

        // Build response JSON
        let info = serde_json::json!({
            "sessionId": hex::encode(session.id()),
            "peerId": hex::encode(peer_id_array),
            "connected": true,
        });

        Ok::<_, anyhow::Error>(info.to_string())
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

/// Send a file to a peer
///
/// # Safety
/// This function is called from Java via JNI. It expects:
/// - `handle`: jlong - The node handle
/// - `peer_id`: String - The peer ID
/// - `file_path`: String - Path to the file to send
///
/// Returns: jstring (JSON) containing transfer info, or null on error
#[no_mangle]
pub unsafe extern "C" fn Java_com_wraith_android_WraithNative_sendFile(
    mut env: JNIEnv,
    _class: JClass,
    handle: jlong,
    peer_id: JString,
    file_path: JString,
) -> jstring {
    if handle == 0 {
        return std::ptr::null_mut();
    }

    let node = &*(handle as *const Node);

    let peer_id: String = match env.get_string(&peer_id) {
        Ok(s) => s.into(),
        Err(_) => return std::ptr::null_mut(),
    };

    let file_path: String = match env.get_string(&file_path) {
        Ok(s) => s.into(),
        Err(_) => return std::ptr::null_mut(),
    };

    let rt = match RUNTIME.lock() {
        Ok(guard) => guard,
        Err(e) => {
            log::error!("Failed to acquire runtime lock: {}", e);
            return std::ptr::null_mut();
        }
    };
    let rt = match rt.as_ref() {
        Some(rt) => rt,
        None => {
            log::error!("Runtime not initialized");
            return std::ptr::null_mut();
        }
    };

    // Track transfer start
    increment_transfers();

    let transfer_info = match rt.block_on(async {
        use std::path::Path;
        use wraith_files::{FileTransfer, TransferConfig};

        let peer_id_bytes = hex::decode(&peer_id)?;
        let peer_id_array: [u8; 32] = peer_id_bytes.try_into()
            .map_err(|_| anyhow::anyhow!("Invalid peer ID length"))?;

        let config = TransferConfig::default();
        let transfer = FileTransfer::new(config);

        let transfer_id = transfer.send_file(
            node,
            peer_id_array,
            Path::new(&file_path),
        ).await?;

        let info = serde_json::json!({
            "transferId": hex::encode(transfer_id),
            "peerId": hex::encode(peer_id_array),
            "filePath": file_path,
            "status": "sending",
        });

        Ok::<_, anyhow::Error>(info.to_string())
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

/// Get node status
///
/// # Safety
/// This function is called from Java via JNI.
///
/// Returns: jstring (JSON) containing node status
#[no_mangle]
pub unsafe extern "C" fn Java_com_wraith_android_WraithNative_getNodeStatus(
    mut env: JNIEnv,
    _class: JClass,
    handle: jlong,
) -> jstring {
    if handle == 0 {
        return std::ptr::null_mut();
    }

    let node = &*(handle as *const Node);

    let status = serde_json::json!({
        "running": node.is_running(),
        "localPeerId": hex::encode(node.node_id()),
        "sessionCount": node.active_route_count(),
        "activeTransfers": get_active_transfers(),
    });

    match env.new_string(status.to_string()) {
        Ok(s) => s.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}
