//! # wraith-ffi - C-compatible FFI bindings
//!
//! This crate provides a stable C ABI for the WRAITH Protocol Node API,
//! enabling integration with Tauri, Electron, Python, Go, and other ecosystems.
//!
//! ## Safety
//!
//! All FFI functions are marked `unsafe` because they cross the FFI boundary.
//! Callers must ensure:
//! - Valid pointers (non-null, properly aligned, pointing to initialized data)
//! - Proper memory ownership (don't double-free, don't use-after-free)
//! - String encoding (UTF-8 for all strings)
//! - Thread safety (some operations require exclusive access)
//!
//! ## Memory Management
//!
//! - Rust owns all opaque handle memory (Node, Session, Transfer)
//! - Caller owns string buffers returned via `*mut c_char`
//! - All `*_new()` functions allocate, corresponding `*_free()` functions deallocate
//! - Error strings must be freed with `wraith_free_string()`
//!
//! ## Error Handling
//!
//! Most functions return an error code (0 = success, non-zero = error).
//! Error details are written to an optional `error_out` parameter.

// FFI code inherently requires unsafe operations within unsafe functions
#![allow(unsafe_op_in_unsafe_fn)]

use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int};
use std::sync::Arc;

use tokio::runtime::Runtime;
use wraith_core::node::Node;

pub mod config;
pub mod error;
pub mod node;
pub mod session;
pub mod transfer;
pub mod types;

// Re-export for convenience
pub use error::{WraithError, WraithErrorCode};
pub use types::*;

/// Opaque handle to a WRAITH node
#[repr(C)]
pub struct WraithNode {
    _private: [u8; 0],
}

/// Opaque handle to a peer session
#[repr(C)]
pub struct WraithSession {
    _private: [u8; 0],
}

/// Opaque handle to a file transfer
#[repr(C)]
pub struct WraithTransfer {
    _private: [u8; 0],
}

/// Opaque handle to node configuration
#[repr(C)]
pub struct WraithConfig {
    _private: [u8; 0],
}

/// Internal representation of WraithNode
pub(crate) struct NodeHandle {
    pub(crate) node: Node,
    pub(crate) runtime: Arc<Runtime>,
}

/// Initialize the WRAITH FFI library
///
/// Must be called before any other FFI functions.
/// Returns 0 on success, non-zero on error.
///
/// # Safety
///
/// Safe to call multiple times (idempotent).
#[unsafe(no_mangle)]
pub extern "C" fn wraith_init() -> c_int {
    // Initialize tracing subscriber for logging
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .try_init();

    0 // Success
}

/// Get the version string of the WRAITH library
///
/// Returns a pointer to a static null-terminated string.
/// Caller must NOT free this pointer.
///
/// # Safety
///
/// The returned pointer is valid for the lifetime of the process.
#[unsafe(no_mangle)]
pub extern "C" fn wraith_version() -> *const c_char {
    const VERSION: &str = concat!(env!("CARGO_PKG_VERSION"), "\0");
    VERSION.as_ptr() as *const c_char
}

/// Free a string returned by WRAITH FFI functions
///
/// # Safety
///
/// - `s` must be a valid pointer returned by a WRAITH FFI function
/// - `s` must not be used after this call
/// - `s` must not be freed multiple times
#[unsafe(no_mangle)]
pub unsafe extern "C" fn wraith_free_string(s: *mut c_char) {
    if !s.is_null() {
        drop(CString::from_raw(s));
    }
}

/// Helper to convert Rust String to C string
#[allow(dead_code)]
pub(crate) fn to_c_string(s: String) -> *mut c_char {
    CString::new(s)
        .unwrap_or_else(|_| CString::new("Invalid UTF-8").unwrap())
        .into_raw()
}

/// Helper to convert C string to Rust String
///
/// # Safety
///
/// - `s` must be a valid null-terminated UTF-8 string
pub(crate) unsafe fn from_c_string(s: *const c_char) -> Option<String> {
    if s.is_null() {
        None
    } else {
        CStr::from_ptr(s).to_str().ok().map(|s| s.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_init() {
        assert_eq!(wraith_init(), 0);
    }

    #[test]
    fn test_version() {
        let version_ptr = wraith_version();
        assert!(!version_ptr.is_null());

        unsafe {
            let version_cstr = CStr::from_ptr(version_ptr);
            let version_str = version_cstr.to_str().unwrap();
            assert!(version_str.starts_with(env!("CARGO_PKG_VERSION")));
        }
    }

    #[test]
    fn test_string_conversion() {
        let rust_str = "test string".to_string();
        let c_str = to_c_string(rust_str.clone());

        unsafe {
            let converted = from_c_string(c_str);
            assert_eq!(converted, Some(rust_str));
            wraith_free_string(c_str);
        }
    }
}
