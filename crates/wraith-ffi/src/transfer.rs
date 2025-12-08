//! Transfer API FFI

use std::os::raw::{c_char, c_int};
use std::path::PathBuf;

use crate::error::{WraithError, WraithErrorCode};
use crate::types::*;
use crate::{NodeHandle, WraithNode, WraithTransfer, ffi_try, from_c_string};

/// Send a file to a peer
///
/// # Safety
///
/// - `node` must be a valid node handle
/// - `peer_id` must be a valid pointer to a 32-byte peer ID
/// - `file_path` must be a valid null-terminated UTF-8 string
/// - `transfer_out` must be a valid pointer to receive the transfer handle
/// - `error_out` must be null or a valid pointer to receive error message
#[unsafe(no_mangle)]
pub unsafe extern "C" fn wraith_transfer_send_file(
    node: *mut WraithNode,
    peer_id: *const WraithNodeId,
    file_path: *const c_char,
    transfer_out: *mut *mut WraithTransfer,
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

    if transfer_out.is_null() {
        if !error_out.is_null() {
            *error_out = WraithError::invalid_argument("transfer_out is null").to_c_string();
        }
        return WraithErrorCode::InvalidArgument as c_int;
    }

    let path_str = ffi_try!(
        from_c_string(file_path).ok_or_else(|| WraithError::invalid_argument("file_path is null")),
        error_out
    );

    let peer_id_bytes = (*peer_id).bytes;
    let file_path_buf = PathBuf::from(path_str);

    let handle = &mut *(node as *mut NodeHandle);
    let node_clone = handle.node.clone();
    let runtime = handle.runtime.clone();

    let transfer_id = ffi_try!(
        runtime
            .block_on(async move { node_clone.send_file(file_path_buf, &peer_id_bytes).await })
            .map_err(WraithError::from),
        error_out
    );

    // Store transfer ID as handle
    let transfer_handle = Box::new(transfer_id);
    *transfer_out = Box::into_raw(transfer_handle) as *mut WraithTransfer;

    WraithErrorCode::Success as c_int
}

/// Wait for a file transfer to complete
///
/// This is a blocking call that waits until the transfer finishes.
///
/// # Safety
///
/// - `node` must be a valid node handle
/// - `transfer` must be a valid transfer handle
/// - `error_out` must be null or a valid pointer to receive error message
#[unsafe(no_mangle)]
pub unsafe extern "C" fn wraith_transfer_wait(
    node: *mut WraithNode,
    transfer: *const WraithTransfer,
    error_out: *mut *mut c_char,
) -> c_int {
    if node.is_null() {
        if !error_out.is_null() {
            *error_out = WraithError::invalid_argument("node is null").to_c_string();
        }
        return WraithErrorCode::InvalidArgument as c_int;
    }

    if transfer.is_null() {
        if !error_out.is_null() {
            *error_out = WraithError::invalid_argument("transfer is null").to_c_string();
        }
        return WraithErrorCode::InvalidArgument as c_int;
    }

    let transfer_id = *(transfer as *const [u8; 32]);
    let handle = &mut *(node as *mut NodeHandle);
    let node_clone = handle.node.clone();
    let runtime = handle.runtime.clone();

    ffi_try!(
        runtime
            .block_on(async move { node_clone.wait_for_transfer(transfer_id).await })
            .map_err(WraithError::from),
        error_out
    );

    WraithErrorCode::Success as c_int
}

/// Get transfer progress
///
/// # Safety
///
/// - `node` must be a valid node handle
/// - `transfer` must be a valid transfer handle
/// - `progress_out` must be a valid pointer to a WraithTransferProgress struct
/// - `error_out` must be null or a valid pointer to receive error message
#[unsafe(no_mangle)]
pub unsafe extern "C" fn wraith_transfer_get_progress(
    node: *const WraithNode,
    transfer: *const WraithTransfer,
    progress_out: *mut WraithTransferProgress,
    error_out: *mut *mut c_char,
) -> c_int {
    if node.is_null() {
        if !error_out.is_null() {
            *error_out = WraithError::invalid_argument("node is null").to_c_string();
        }
        return WraithErrorCode::InvalidArgument as c_int;
    }

    if transfer.is_null() {
        if !error_out.is_null() {
            *error_out = WraithError::invalid_argument("transfer is null").to_c_string();
        }
        return WraithErrorCode::InvalidArgument as c_int;
    }

    if progress_out.is_null() {
        if !error_out.is_null() {
            *error_out = WraithError::invalid_argument("progress_out is null").to_c_string();
        }
        return WraithErrorCode::InvalidArgument as c_int;
    }

    let transfer_id = *(transfer as *const [u8; 32]);
    let handle = &*(node as *const NodeHandle);
    let node_clone = handle.node.clone();
    let runtime = handle.runtime.clone();

    let progress_opt =
        runtime.block_on(async move { node_clone.get_transfer_progress(&transfer_id).await });

    match progress_opt {
        Some(progress) => {
            let is_complete = progress.is_complete();
            let pct = (progress.progress_percent / 100.0) as f32; // Convert from 0-100 to 0-1

            let eta_seconds = if let Some(eta) = progress.eta {
                eta.as_secs()
            } else {
                0
            };

            *progress_out = WraithTransferProgress {
                total_bytes: progress.bytes_total,
                transferred_bytes: progress.bytes_sent,
                progress: pct,
                eta_seconds,
                rate_bytes_per_sec: progress.speed_bytes_per_sec as u64,
                is_complete,
            };
        }
        None => {
            if !error_out.is_null() {
                *error_out = WraithError::transfer_not_found().to_c_string();
            }
            return WraithErrorCode::TransferNotFound as c_int;
        }
    }

    WraithErrorCode::Success as c_int
}

/// Free a transfer handle
///
/// # Safety
///
/// - `transfer` must be a valid transfer handle
/// - `transfer` must not be used after this call
/// - `transfer` must not be freed multiple times
#[unsafe(no_mangle)]
pub unsafe extern "C" fn wraith_transfer_free(transfer: *mut WraithTransfer) {
    if !transfer.is_null() {
        drop(Box::from_raw(transfer as *mut [u8; 32]));
    }
}

/// Get the number of active transfers
///
/// # Safety
///
/// - `node` must be a valid node handle
/// - `count_out` must be a valid pointer to receive the count
#[unsafe(no_mangle)]
pub unsafe extern "C" fn wraith_transfer_count(
    node: *const WraithNode,
    count_out: *mut u32,
) -> c_int {
    if node.is_null() || count_out.is_null() {
        return WraithErrorCode::InvalidArgument as c_int;
    }

    let handle = &*(node as *const NodeHandle);
    let node_clone = handle.node.clone();
    let runtime = handle.runtime.clone();

    let transfers = runtime.block_on(async move { node_clone.active_transfers().await });
    *count_out = transfers.len() as u32;

    WraithErrorCode::Success as c_int
}

/// Cancel an active transfer
///
/// Removes the transfer from the active transfers map and sends a STREAM_CLOSE
/// frame to the peer if the transfer has an active session.
///
/// # Safety
///
/// - `node` must be a valid node handle
/// - `transfer` must be a valid transfer handle
/// - `error_out` must be null or a valid pointer to receive error message
#[unsafe(no_mangle)]
pub unsafe extern "C" fn wraith_transfer_cancel(
    node: *mut WraithNode,
    transfer: *const WraithTransfer,
    error_out: *mut *mut c_char,
) -> c_int {
    if node.is_null() {
        if !error_out.is_null() {
            *error_out = WraithError::invalid_argument("node is null").to_c_string();
        }
        return WraithErrorCode::InvalidArgument as c_int;
    }

    if transfer.is_null() {
        if !error_out.is_null() {
            *error_out = WraithError::invalid_argument("transfer is null").to_c_string();
        }
        return WraithErrorCode::InvalidArgument as c_int;
    }

    let transfer_id = *(transfer as *const [u8; 32]);
    let handle = &mut *(node as *mut NodeHandle);
    let node_clone = handle.node.clone();
    let runtime = handle.runtime.clone();

    ffi_try!(
        runtime
            .block_on(async move { node_clone.cancel_transfer(&transfer_id).await })
            .map_err(WraithError::from),
        error_out
    );

    WraithErrorCode::Success as c_int
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ptr;

    #[test]
    fn test_transfer_count() {
        unsafe {
            let node = crate::node::wraith_node_new(ptr::null(), ptr::null_mut());
            let mut count: u32 = 0;

            let result = wraith_transfer_count(node, &mut count);
            assert_eq!(result, WraithErrorCode::Success as c_int);
            assert_eq!(count, 0);

            crate::node::wraith_node_free(node);
        }
    }
}
