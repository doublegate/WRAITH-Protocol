//! Error handling for FFI boundary

use std::ffi::CString;
use std::os::raw::c_char;

/// FFI error codes
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WraithErrorCode {
    /// Operation succeeded
    Success = 0,
    /// Invalid argument provided
    InvalidArgument = 1,
    /// Node not initialized
    NotInitialized = 2,
    /// Node already initialized
    AlreadyInitialized = 3,
    /// Session not found
    SessionNotFound = 4,
    /// Transfer not found
    TransferNotFound = 5,
    /// I/O error
    IoError = 6,
    /// Cryptographic error
    CryptoError = 7,
    /// Transport error
    TransportError = 8,
    /// Discovery error
    DiscoveryError = 9,
    /// Timeout
    Timeout = 10,
    /// Out of memory
    OutOfMemory = 11,
    /// Invalid state
    InvalidState = 12,
    /// Internal error
    InternalError = 99,
}

impl From<i32> for WraithErrorCode {
    fn from(code: i32) -> Self {
        match code {
            0 => WraithErrorCode::Success,
            1 => WraithErrorCode::InvalidArgument,
            2 => WraithErrorCode::NotInitialized,
            3 => WraithErrorCode::AlreadyInitialized,
            4 => WraithErrorCode::SessionNotFound,
            5 => WraithErrorCode::TransferNotFound,
            6 => WraithErrorCode::IoError,
            7 => WraithErrorCode::CryptoError,
            8 => WraithErrorCode::TransportError,
            9 => WraithErrorCode::DiscoveryError,
            10 => WraithErrorCode::Timeout,
            11 => WraithErrorCode::OutOfMemory,
            12 => WraithErrorCode::InvalidState,
            _ => WraithErrorCode::InternalError,
        }
    }
}

/// Error type for FFI operations
#[derive(Debug)]
pub struct WraithError {
    pub code: WraithErrorCode,
    pub message: String,
}

impl WraithError {
    pub fn new(code: WraithErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }

    pub fn invalid_argument(message: impl Into<String>) -> Self {
        Self::new(WraithErrorCode::InvalidArgument, message)
    }

    pub fn not_initialized() -> Self {
        Self::new(WraithErrorCode::NotInitialized, "Node not initialized")
    }

    pub fn session_not_found() -> Self {
        Self::new(WraithErrorCode::SessionNotFound, "Session not found")
    }

    pub fn transfer_not_found() -> Self {
        Self::new(WraithErrorCode::TransferNotFound, "Transfer not found")
    }

    pub fn internal_error(message: impl Into<String>) -> Self {
        Self::new(WraithErrorCode::InternalError, message)
    }

    /// Convert error to C-compatible error string
    pub fn to_c_string(&self) -> *mut c_char {
        CString::new(self.message.clone())
            .unwrap_or_else(|_| {
                // Safety: ASCII string without null bytes is guaranteed valid
                unsafe { CString::from_vec_unchecked(b"Invalid error message".to_vec()) }
            })
            .into_raw()
    }
}

impl From<wraith_core::node::NodeError> for WraithError {
    fn from(err: wraith_core::node::NodeError) -> Self {
        use wraith_core::node::NodeError;

        match &err {
            NodeError::TransportInit(_) => {
                Self::new(WraithErrorCode::TransportError, err.to_string())
            }
            NodeError::Transport(_) => Self::new(WraithErrorCode::TransportError, err.to_string()),
            NodeError::Crypto(_) => Self::new(WraithErrorCode::CryptoError, err.to_string()),
            NodeError::Handshake(_) => Self::new(WraithErrorCode::CryptoError, err.to_string()),
            NodeError::SessionEstablishment(_) => {
                Self::new(WraithErrorCode::InternalError, err.to_string())
            }
            NodeError::SessionNotFound(_) => Self::session_not_found(),
            NodeError::SessionMigration(_) => {
                Self::new(WraithErrorCode::InternalError, err.to_string())
            }
            NodeError::Transfer(_) => Self::new(WraithErrorCode::InternalError, err.to_string()),
            NodeError::TransferNotFound(_) => Self::transfer_not_found(),
            NodeError::HashMismatch => Self::new(WraithErrorCode::CryptoError, err.to_string()),
            NodeError::Io(_) => Self::new(WraithErrorCode::IoError, err.to_string()),
            NodeError::Discovery(_) => Self::new(WraithErrorCode::DiscoveryError, err.to_string()),
            NodeError::NatTraversal(_) => {
                Self::new(WraithErrorCode::DiscoveryError, err.to_string())
            }
            NodeError::PeerNotFound(_) => {
                Self::new(WraithErrorCode::SessionNotFound, err.to_string())
            }
            NodeError::Migration(_) => Self::new(WraithErrorCode::InternalError, err.to_string()),
            NodeError::Obfuscation(_) => Self::new(WraithErrorCode::InternalError, err.to_string()),
            NodeError::InvalidConfig(_) => {
                Self::new(WraithErrorCode::InvalidArgument, err.to_string())
            }
            NodeError::InvalidState(_) => Self::new(WraithErrorCode::InvalidState, err.to_string()),
            NodeError::Timeout(_) => Self::new(WraithErrorCode::Timeout, err.to_string()),
            NodeError::TaskJoin(_) => Self::new(WraithErrorCode::InternalError, err.to_string()),
            NodeError::Channel(_) => Self::new(WraithErrorCode::InternalError, err.to_string()),
            NodeError::Serialization(_) => {
                Self::new(WraithErrorCode::InternalError, err.to_string())
            }
            NodeError::Other(_) => Self::new(WraithErrorCode::InternalError, err.to_string()),
        }
    }
}

impl From<std::io::Error> for WraithError {
    fn from(err: std::io::Error) -> Self {
        Self::new(WraithErrorCode::IoError, err.to_string())
    }
}

/// Helper macro for FFI error handling (for functions returning c_int error codes)
///
/// Writes error message to `error_out` if provided and returns error code.
#[macro_export]
macro_rules! ffi_try {
    ($result:expr, $error_out:expr) => {
        match $result {
            Ok(value) => value,
            Err(err) => {
                let wraith_err: $crate::error::WraithError = err.into();
                if !$error_out.is_null() {
                    unsafe {
                        *$error_out = wraith_err.to_c_string();
                    }
                }
                return wraith_err.code as i32;
            }
        }
    };
}

/// Helper macro for FFI error handling (for functions returning pointers)
///
/// Writes error message to `error_out` if provided and returns null pointer.
#[macro_export]
macro_rules! ffi_try_ptr {
    ($result:expr, $error_out:expr) => {
        match $result {
            Ok(value) => value,
            Err(err) => {
                let wraith_err: $crate::error::WraithError = err.into();
                if !$error_out.is_null() {
                    unsafe {
                        *$error_out = wraith_err.to_c_string();
                    }
                }
                return std::ptr::null_mut();
            }
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_code_conversion() {
        assert_eq!(WraithErrorCode::from(0), WraithErrorCode::Success);
        assert_eq!(WraithErrorCode::from(1), WraithErrorCode::InvalidArgument);
        assert_eq!(WraithErrorCode::from(99), WraithErrorCode::InternalError);
        assert_eq!(WraithErrorCode::from(999), WraithErrorCode::InternalError);
    }

    #[test]
    fn test_error_creation() {
        let err = WraithError::invalid_argument("test message");
        assert_eq!(err.code, WraithErrorCode::InvalidArgument);
        assert_eq!(err.message, "test message");
    }

    #[test]
    fn test_error_to_c_string() {
        let err = WraithError::internal_error("test error");
        let c_str = err.to_c_string();
        assert!(!c_str.is_null());

        unsafe {
            use std::ffi::CStr;
            let message = CStr::from_ptr(c_str).to_str().unwrap();
            assert_eq!(message, "test error");
            crate::wraith_free_string(c_str);
        }
    }
}
