//! Configuration FFI

use std::os::raw::{c_char, c_int};

use crate::error::{WraithError, WraithErrorCode};
use crate::types::*;
use crate::{WraithConfig, ffi_try, from_c_string};

use wraith_core::node::NodeConfig;

/// Internal representation of WraithConfig
pub(crate) struct ConfigHandle {
    pub(crate) config: NodeConfig,
}

/// Create a new default configuration
///
/// # Safety
///
/// - `error_out` must be null or a valid pointer to receive error message
#[unsafe(no_mangle)]
pub unsafe extern "C" fn wraith_config_new(_error_out: *mut *mut c_char) -> *mut WraithConfig {
    let config = NodeConfig::default();
    let handle = Box::new(ConfigHandle { config });
    Box::into_raw(handle) as *mut WraithConfig
}

/// Free a configuration handle
///
/// # Safety
///
/// - `config` must be a valid pointer returned by `wraith_config_new()`
/// - `config` must not be used after this call
/// - `config` must not be freed multiple times
#[unsafe(no_mangle)]
pub unsafe extern "C" fn wraith_config_free(config: *mut WraithConfig) {
    if !config.is_null() {
        drop(Box::from_raw(config as *mut ConfigHandle));
    }
}

/// Set the bind address for the node
///
/// # Safety
///
/// - `config` must be a valid configuration handle
/// - `address` must be a valid null-terminated UTF-8 string
/// - `error_out` must be null or a valid pointer to receive error message
#[unsafe(no_mangle)]
pub unsafe extern "C" fn wraith_config_set_bind_address(
    config: *mut WraithConfig,
    address: *const c_char,
    error_out: *mut *mut c_char,
) -> c_int {
    if config.is_null() {
        if !error_out.is_null() {
            *error_out = WraithError::invalid_argument("config is null").to_c_string();
        }
        return WraithErrorCode::InvalidArgument as c_int;
    }

    let address_str = ffi_try!(
        from_c_string(address).ok_or_else(|| WraithError::invalid_argument("address is null")),
        error_out
    );

    let handle = &mut *(config as *mut ConfigHandle);
    let addr = ffi_try!(
        address_str
            .parse()
            .map_err(|_| WraithError::invalid_argument("invalid address format")),
        error_out
    );

    handle.config.listen_addr = addr;
    WraithErrorCode::Success as c_int
}

/// Set obfuscation padding mode
///
/// # Safety
///
/// - `config` must be a valid configuration handle
/// - `error_out` must be null or a valid pointer to receive error message
#[unsafe(no_mangle)]
pub unsafe extern "C" fn wraith_config_set_padding_mode(
    config: *mut WraithConfig,
    mode: WraithPaddingMode,
    error_out: *mut *mut c_char,
) -> c_int {
    if config.is_null() {
        if !error_out.is_null() {
            *error_out = WraithError::invalid_argument("config is null").to_c_string();
        }
        return WraithErrorCode::InvalidArgument as c_int;
    }

    let handle = &mut *(config as *mut ConfigHandle);
    handle.config.obfuscation.padding_mode = mode.into();
    WraithErrorCode::Success as c_int
}

/// Set obfuscation timing mode
///
/// # Safety
///
/// - `config` must be a valid configuration handle
/// - `error_out` must be null or a valid pointer to receive error message
#[unsafe(no_mangle)]
pub unsafe extern "C" fn wraith_config_set_timing_mode(
    config: *mut WraithConfig,
    mode: WraithTimingMode,
    error_out: *mut *mut c_char,
) -> c_int {
    if config.is_null() {
        if !error_out.is_null() {
            *error_out = WraithError::invalid_argument("config is null").to_c_string();
        }
        return WraithErrorCode::InvalidArgument as c_int;
    }

    let handle = &mut *(config as *mut ConfigHandle);
    handle.config.obfuscation.timing_mode = mode.into();
    WraithErrorCode::Success as c_int
}

/// Set protocol mimicry mode
///
/// # Safety
///
/// - `config` must be a valid configuration handle
/// - `error_out` must be null or a valid pointer to receive error message
#[unsafe(no_mangle)]
pub unsafe extern "C" fn wraith_config_set_mimicry_mode(
    config: *mut WraithConfig,
    mode: WraithMimicryMode,
    error_out: *mut *mut c_char,
) -> c_int {
    if config.is_null() {
        if !error_out.is_null() {
            *error_out = WraithError::invalid_argument("config is null").to_c_string();
        }
        return WraithErrorCode::InvalidArgument as c_int;
    }

    let handle = &mut *(config as *mut ConfigHandle);
    handle.config.obfuscation.mimicry_mode = mode.into();
    WraithErrorCode::Success as c_int
}

/// Enable AF_XDP kernel bypass (requires root privileges)
///
/// # Safety
///
/// - `config` must be a valid configuration handle
/// - `error_out` must be null or a valid pointer to receive error message
#[unsafe(no_mangle)]
pub unsafe extern "C" fn wraith_config_enable_af_xdp(
    config: *mut WraithConfig,
    enabled: bool,
    error_out: *mut *mut c_char,
) -> c_int {
    if config.is_null() {
        if !error_out.is_null() {
            *error_out = WraithError::invalid_argument("config is null").to_c_string();
        }
        return WraithErrorCode::InvalidArgument as c_int;
    }

    let handle = &mut *(config as *mut ConfigHandle);
    handle.config.transport.enable_xdp = enabled;
    WraithErrorCode::Success as c_int
}

/// Enable io_uring for file I/O (Linux only)
///
/// # Safety
///
/// - `config` must be a valid configuration handle
/// - `error_out` must be null or a valid pointer to receive error message
#[unsafe(no_mangle)]
pub unsafe extern "C" fn wraith_config_enable_io_uring(
    config: *mut WraithConfig,
    enabled: bool,
    error_out: *mut *mut c_char,
) -> c_int {
    if config.is_null() {
        if !error_out.is_null() {
            *error_out = WraithError::invalid_argument("config is null").to_c_string();
        }
        return WraithErrorCode::InvalidArgument as c_int;
    }

    let handle = &mut *(config as *mut ConfigHandle);
    handle.config.transport.enable_io_uring = enabled;
    WraithErrorCode::Success as c_int
}

/// Set the number of worker threads
///
/// # Safety
///
/// - `config` must be a valid configuration handle
/// - `error_out` must be null or a valid pointer to receive error message
#[unsafe(no_mangle)]
pub unsafe extern "C" fn wraith_config_set_worker_threads(
    config: *mut WraithConfig,
    num_threads: u32,
    error_out: *mut *mut c_char,
) -> c_int {
    if config.is_null() {
        if !error_out.is_null() {
            *error_out = WraithError::invalid_argument("config is null").to_c_string();
        }
        return WraithErrorCode::InvalidArgument as c_int;
    }

    if num_threads == 0 {
        if !error_out.is_null() {
            *error_out = WraithError::invalid_argument("num_threads must be > 0").to_c_string();
        }
        return WraithErrorCode::InvalidArgument as c_int;
    }

    let handle = &mut *(config as *mut ConfigHandle);
    handle.config.transport.worker_threads = Some(num_threads as usize);
    WraithErrorCode::Success as c_int
}

/// Set the download directory for received files
///
/// # Safety
///
/// - `config` must be a valid configuration handle
/// - `path` must be a valid null-terminated UTF-8 string
/// - `error_out` must be null or a valid pointer to receive error message
#[unsafe(no_mangle)]
pub unsafe extern "C" fn wraith_config_set_download_dir(
    config: *mut WraithConfig,
    path: *const c_char,
    error_out: *mut *mut c_char,
) -> c_int {
    if config.is_null() {
        if !error_out.is_null() {
            *error_out = WraithError::invalid_argument("config is null").to_c_string();
        }
        return WraithErrorCode::InvalidArgument as c_int;
    }

    let path_str = ffi_try!(
        from_c_string(path).ok_or_else(|| WraithError::invalid_argument("path is null")),
        error_out
    );

    let handle = &mut *(config as *mut ConfigHandle);
    handle.config.transfer.download_dir = std::path::PathBuf::from(path_str);
    WraithErrorCode::Success as c_int
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;
    use std::ptr;

    #[test]
    fn test_config_new_free() {
        unsafe {
            let config = wraith_config_new(ptr::null_mut());
            assert!(!config.is_null());
            wraith_config_free(config);
        }
    }

    #[test]
    fn test_config_set_bind_address() {
        unsafe {
            let config = wraith_config_new(ptr::null_mut());
            let addr = CString::new("127.0.0.1:8080").unwrap();
            let result = wraith_config_set_bind_address(config, addr.as_ptr(), ptr::null_mut());
            assert_eq!(result, WraithErrorCode::Success as c_int);
            wraith_config_free(config);
        }
    }

    #[test]
    fn test_config_set_padding_mode() {
        unsafe {
            let config = wraith_config_new(ptr::null_mut());
            let result = wraith_config_set_padding_mode(
                config,
                WraithPaddingMode::PowerOfTwo,
                ptr::null_mut(),
            );
            assert_eq!(result, WraithErrorCode::Success as c_int);
            wraith_config_free(config);
        }
    }
}
