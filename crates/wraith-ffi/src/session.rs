//! Session API FFI

use std::os::raw::{c_char, c_int};

use crate::error::{WraithError, WraithErrorCode};
use crate::types::*;
use crate::{NodeHandle, WraithNode, WraithSession, ffi_try};

/// Establish a new session with a peer
///
/// # Safety
///
/// - `node` must be a valid node handle
/// - `peer_id` must be a valid pointer to a WraithNodeId struct (32-byte peer ID)
/// - `session_out` must be a valid pointer to receive the session handle
/// - `error_out` must be null or a valid pointer to receive error message
#[unsafe(no_mangle)]
pub unsafe extern "C" fn wraith_session_establish(
    node: *mut WraithNode,
    peer_id: *const WraithNodeId,
    session_out: *mut *mut WraithSession,
    error_out: *mut *mut c_char,
) -> c_int {
    if node.is_null() {
        if !error_out.is_null() {
            *error_out = WraithError::invalid_argument("node is null").to_c_string();
        }
        return WraithErrorCode::InvalidArgument as c_int;
    }

    if peer_id.is_null() {
        if !error_out.is_null() {
            *error_out = WraithError::invalid_argument("peer_id is null").to_c_string();
        }
        return WraithErrorCode::InvalidArgument as c_int;
    }

    if session_out.is_null() {
        if !error_out.is_null() {
            *error_out = WraithError::invalid_argument("session_out is null").to_c_string();
        }
        return WraithErrorCode::InvalidArgument as c_int;
    }

    // SAFETY: peer_id null check performed above (line 31).
    // Caller must ensure peer_id points to valid, properly aligned WraithNodeId.
    // This is documented in the function's safety contract.
    let peer_id_bytes = (*peer_id).bytes;
    let handle = &mut *(node as *mut NodeHandle);
    let node_clone = handle.node.clone();
    let runtime = handle.runtime.clone();

    let _session_id = ffi_try!(
        runtime
            .block_on(async move { node_clone.establish_session(&peer_id_bytes).await })
            .map_err(WraithError::from),
        error_out
    );

    // Store peer_id in handle (needed for close_session which takes peer_id)
    let session_handle = Box::new(peer_id_bytes);
    *session_out = Box::into_raw(session_handle) as *mut WraithSession;

    WraithErrorCode::Success as c_int
}

/// Close an active session
///
/// # Safety
///
/// - `node` must be a valid node handle
/// - `session` must be a valid session handle returned by `wraith_session_establish()`
/// - `error_out` must be null or a valid pointer to receive error message
/// - `session` must not be used after this call
#[unsafe(no_mangle)]
pub unsafe extern "C" fn wraith_session_close(
    node: *mut WraithNode,
    session: *mut WraithSession,
    error_out: *mut *mut c_char,
) -> c_int {
    if node.is_null() {
        if !error_out.is_null() {
            *error_out = WraithError::invalid_argument("node is null").to_c_string();
        }
        return WraithErrorCode::InvalidArgument as c_int;
    }

    if session.is_null() {
        if !error_out.is_null() {
            *error_out = WraithError::invalid_argument("session is null").to_c_string();
        }
        return WraithErrorCode::InvalidArgument as c_int;
    }

    // Extract peer_id from session handle (session stores the peer_id)
    let peer_id_bytes = *(session as *mut [u8; 32]);
    drop(Box::from_raw(session as *mut [u8; 32]));

    let handle = &mut *(node as *mut NodeHandle);
    let node_clone = handle.node.clone();
    let runtime = handle.runtime.clone();

    ffi_try!(
        runtime
            .block_on(async move { node_clone.close_session(&peer_id_bytes).await })
            .map_err(WraithError::from),
        error_out
    );

    WraithErrorCode::Success as c_int
}

/// Get connection statistics for a session
///
/// # Safety
///
/// - `node` must be a valid node handle
/// - `session` must be a valid session handle
/// - `stats_out` must be a valid pointer to a WraithConnectionStats struct
/// - `error_out` must be null or a valid pointer to receive error message
#[unsafe(no_mangle)]
pub unsafe extern "C" fn wraith_session_get_stats(
    node: *const WraithNode,
    session: *const WraithSession,
    stats_out: *mut WraithConnectionStats,
    error_out: *mut *mut c_char,
) -> c_int {
    if node.is_null() {
        if !error_out.is_null() {
            *error_out = WraithError::invalid_argument("node is null").to_c_string();
        }
        return WraithErrorCode::InvalidArgument as c_int;
    }

    if session.is_null() {
        if !error_out.is_null() {
            *error_out = WraithError::invalid_argument("session is null").to_c_string();
        }
        return WraithErrorCode::InvalidArgument as c_int;
    }

    if stats_out.is_null() {
        if !error_out.is_null() {
            *error_out = WraithError::invalid_argument("stats_out is null").to_c_string();
        }
        return WraithErrorCode::InvalidArgument as c_int;
    }

    // Get peer_id from session handle
    let peer_id_bytes = *(session as *const [u8; 32]);

    let handle = &*(node as *const NodeHandle);
    let node_clone = handle.node.clone();

    // Get connection stats from Node API
    if let Some(stats) = node_clone.get_connection_stats(&peer_id_bytes) {
        *stats_out = WraithConnectionStats {
            bytes_sent: stats.bytes_sent,
            bytes_received: stats.bytes_received,
            packets_sent: stats.packets_sent,
            packets_received: stats.packets_received,
            rtt_us: stats.rtt_us.unwrap_or(0),
            loss_rate: stats.loss_rate as f32,
        };

        WraithErrorCode::Success as c_int
    } else {
        if !error_out.is_null() {
            *error_out = WraithError::session_not_found().to_c_string();
        }
        WraithErrorCode::SessionNotFound as c_int
    }
}

/// Get the number of active sessions
///
/// # Safety
///
/// - `node` must be a valid node handle
/// - `count_out` must be a valid pointer to receive the count
#[unsafe(no_mangle)]
pub unsafe extern "C" fn wraith_session_count(
    node: *const WraithNode,
    count_out: *mut u32,
) -> c_int {
    if node.is_null() || count_out.is_null() {
        return WraithErrorCode::InvalidArgument as c_int;
    }

    let handle = &*(node as *const NodeHandle);
    let node_clone = handle.node.clone();
    let runtime = handle.runtime.clone();

    let sessions = runtime.block_on(async move { node_clone.active_sessions().await });
    *count_out = sessions.len() as u32;

    WraithErrorCode::Success as c_int
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CStr;
    use std::ptr;

    #[test]
    fn test_session_count() {
        unsafe {
            let node = crate::node::wraith_node_new(ptr::null(), ptr::null_mut());
            let mut count: u32 = 0;

            let result = wraith_session_count(node, &mut count);
            assert_eq!(result, WraithErrorCode::Success as c_int);
            assert_eq!(count, 0);

            crate::node::wraith_node_free(node);
        }
    }

    #[test]
    fn test_session_count_null_node() {
        unsafe {
            let mut count: u32 = 0;
            let result = wraith_session_count(ptr::null(), &mut count);
            assert_eq!(result, WraithErrorCode::InvalidArgument as c_int);
        }
    }

    #[test]
    fn test_session_count_null_count_out() {
        unsafe {
            let node = crate::node::wraith_node_new(ptr::null(), ptr::null_mut());
            let result = wraith_session_count(node, ptr::null_mut());
            assert_eq!(result, WraithErrorCode::InvalidArgument as c_int);
            crate::node::wraith_node_free(node);
        }
    }

    #[test]
    fn test_session_establish_null_node() {
        unsafe {
            let peer_id = WraithNodeId { bytes: [1u8; 32] };
            let mut session_ptr: *mut WraithSession = ptr::null_mut();
            let mut error_ptr: *mut c_char = ptr::null_mut();

            let result = wraith_session_establish(
                ptr::null_mut(),
                &peer_id,
                &mut session_ptr,
                &mut error_ptr,
            );

            assert_eq!(result, WraithErrorCode::InvalidArgument as c_int);
            assert!(!error_ptr.is_null());

            let error_msg = CStr::from_ptr(error_ptr).to_str().unwrap();
            assert!(error_msg.contains("node is null"));
            crate::wraith_free_string(error_ptr);
        }
    }

    #[test]
    fn test_session_establish_null_peer_id() {
        unsafe {
            let node = crate::node::wraith_node_new(ptr::null(), ptr::null_mut());
            let mut session_ptr: *mut WraithSession = ptr::null_mut();
            let mut error_ptr: *mut c_char = ptr::null_mut();

            let result =
                wraith_session_establish(node, ptr::null(), &mut session_ptr, &mut error_ptr);

            assert_eq!(result, WraithErrorCode::InvalidArgument as c_int);
            assert!(!error_ptr.is_null());

            let error_msg = CStr::from_ptr(error_ptr).to_str().unwrap();
            assert!(error_msg.contains("peer_id is null"));
            crate::wraith_free_string(error_ptr);

            crate::node::wraith_node_free(node);
        }
    }

    #[test]
    fn test_session_establish_null_session_out() {
        unsafe {
            let node = crate::node::wraith_node_new(ptr::null(), ptr::null_mut());
            let peer_id = WraithNodeId { bytes: [1u8; 32] };
            let mut error_ptr: *mut c_char = ptr::null_mut();

            let result = wraith_session_establish(node, &peer_id, ptr::null_mut(), &mut error_ptr);

            assert_eq!(result, WraithErrorCode::InvalidArgument as c_int);
            assert!(!error_ptr.is_null());

            let error_msg = CStr::from_ptr(error_ptr).to_str().unwrap();
            assert!(error_msg.contains("session_out is null"));
            crate::wraith_free_string(error_ptr);

            crate::node::wraith_node_free(node);
        }
    }

    #[test]
    fn test_session_close_null_node() {
        unsafe {
            let peer_id = [1u8; 32];
            let session = Box::into_raw(Box::new(peer_id)) as *mut WraithSession;
            let mut error_ptr: *mut c_char = ptr::null_mut();

            let result = wraith_session_close(ptr::null_mut(), session, &mut error_ptr);

            assert_eq!(result, WraithErrorCode::InvalidArgument as c_int);
            assert!(!error_ptr.is_null());

            let error_msg = CStr::from_ptr(error_ptr).to_str().unwrap();
            assert!(error_msg.contains("node is null"));
            crate::wraith_free_string(error_ptr);

            // Clean up session handle
            drop(Box::from_raw(session as *mut [u8; 32]));
        }
    }

    #[test]
    fn test_session_close_null_session() {
        unsafe {
            let node = crate::node::wraith_node_new(ptr::null(), ptr::null_mut());
            let mut error_ptr: *mut c_char = ptr::null_mut();

            let result = wraith_session_close(node, ptr::null_mut(), &mut error_ptr);

            assert_eq!(result, WraithErrorCode::InvalidArgument as c_int);
            assert!(!error_ptr.is_null());

            let error_msg = CStr::from_ptr(error_ptr).to_str().unwrap();
            assert!(error_msg.contains("session is null"));
            crate::wraith_free_string(error_ptr);

            crate::node::wraith_node_free(node);
        }
    }

    #[test]
    fn test_session_get_stats_null_node() {
        unsafe {
            let peer_id = [1u8; 32];
            let session = Box::into_raw(Box::new(peer_id)) as *mut WraithSession;
            let mut stats = WraithConnectionStats {
                bytes_sent: 0,
                bytes_received: 0,
                packets_sent: 0,
                packets_received: 0,
                rtt_us: 0,
                loss_rate: 0.0,
            };
            let mut error_ptr: *mut c_char = ptr::null_mut();

            let result = wraith_session_get_stats(ptr::null(), session, &mut stats, &mut error_ptr);

            assert_eq!(result, WraithErrorCode::InvalidArgument as c_int);
            assert!(!error_ptr.is_null());

            let error_msg = CStr::from_ptr(error_ptr).to_str().unwrap();
            assert!(error_msg.contains("node is null"));
            crate::wraith_free_string(error_ptr);

            // Clean up session handle
            drop(Box::from_raw(session as *mut [u8; 32]));
        }
    }

    #[test]
    fn test_session_get_stats_null_session() {
        unsafe {
            let node = crate::node::wraith_node_new(ptr::null(), ptr::null_mut());
            let mut stats = WraithConnectionStats {
                bytes_sent: 0,
                bytes_received: 0,
                packets_sent: 0,
                packets_received: 0,
                rtt_us: 0,
                loss_rate: 0.0,
            };
            let mut error_ptr: *mut c_char = ptr::null_mut();

            let result = wraith_session_get_stats(node, ptr::null(), &mut stats, &mut error_ptr);

            assert_eq!(result, WraithErrorCode::InvalidArgument as c_int);
            assert!(!error_ptr.is_null());

            let error_msg = CStr::from_ptr(error_ptr).to_str().unwrap();
            assert!(error_msg.contains("session is null"));
            crate::wraith_free_string(error_ptr);

            crate::node::wraith_node_free(node);
        }
    }

    #[test]
    fn test_session_get_stats_null_stats_out() {
        unsafe {
            let node = crate::node::wraith_node_new(ptr::null(), ptr::null_mut());
            let peer_id = [1u8; 32];
            let session = Box::into_raw(Box::new(peer_id)) as *mut WraithSession;
            let mut error_ptr: *mut c_char = ptr::null_mut();

            let result = wraith_session_get_stats(node, session, ptr::null_mut(), &mut error_ptr);

            assert_eq!(result, WraithErrorCode::InvalidArgument as c_int);
            assert!(!error_ptr.is_null());

            let error_msg = CStr::from_ptr(error_ptr).to_str().unwrap();
            assert!(error_msg.contains("stats_out is null"));
            crate::wraith_free_string(error_ptr);

            // Clean up
            drop(Box::from_raw(session as *mut [u8; 32]));
            crate::node::wraith_node_free(node);
        }
    }

    #[test]
    fn test_session_get_stats_session_not_found() {
        unsafe {
            let node = crate::node::wraith_node_new(ptr::null(), ptr::null_mut());
            let peer_id = [1u8; 32];
            let session = Box::into_raw(Box::new(peer_id)) as *mut WraithSession;
            let mut stats = WraithConnectionStats {
                bytes_sent: 0,
                bytes_received: 0,
                packets_sent: 0,
                packets_received: 0,
                rtt_us: 0,
                loss_rate: 0.0,
            };
            let mut error_ptr: *mut c_char = ptr::null_mut();

            let result = wraith_session_get_stats(node, session, &mut stats, &mut error_ptr);

            // Should return SessionNotFound since session doesn't exist
            assert_eq!(result, WraithErrorCode::SessionNotFound as c_int);
            assert!(!error_ptr.is_null());

            let error_msg = CStr::from_ptr(error_ptr).to_str().unwrap();
            assert!(error_msg.contains("Session not found"));
            crate::wraith_free_string(error_ptr);

            // Clean up
            drop(Box::from_raw(session as *mut [u8; 32]));
            crate::node::wraith_node_free(node);
        }
    }
}
