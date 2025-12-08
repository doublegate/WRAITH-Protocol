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

    // TODO: Implement actual stats retrieval from Node API
    // For now, return zeroed stats
    *stats_out = WraithConnectionStats {
        bytes_sent: 0,
        bytes_received: 0,
        packets_sent: 0,
        packets_received: 0,
        rtt_us: 0,
        loss_rate: 0.0,
    };

    WraithErrorCode::Success as c_int
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
}
