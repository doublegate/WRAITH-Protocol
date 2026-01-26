use alloc::string::String;
use alloc::format;

#[cfg(target_os = "windows")]
use crate::utils::api_resolver::{hash_str, resolve_function};
#[cfg(target_os = "windows")]
use crate::utils::windows_definitions::*;

pub struct Discovery;

#[cfg(target_os = "windows")]
#[repr(C)]
#[allow(non_snake_case)]
struct SYSTEM_INFO {
    wProcessorArchitecture: u16,
    wReserved: u16,
    dwPageSize: u32,
    lpMinimumApplicationAddress: PVOID,
    lpMaximumApplicationAddress: PVOID,
    dwActiveProcessorMask: usize,
    dwNumberOfProcessors: u32,
    dwProcessorType: u32,
    dwAllocationGranularity: u32,
    wProcessorLevel: u16,
    wProcessorRevision: u16,
}

impl Discovery {
    pub fn sys_info(&self) -> String {
        #[cfg(target_os = "windows")]
        unsafe {
            let kernel32 = hash_str(b"kernel32.dll");
            let get_sys_info = resolve_function(kernel32, hash_str(b"GetSystemInfo"));
            
            if get_sys_info.is_null() { return String::from("Failed to resolve GetSystemInfo"); }

            type FnGetSystemInfo = unsafe extern "system" fn(*mut SYSTEM_INFO);
            
            let mut si: SYSTEM_INFO = core::mem::zeroed();
            core::mem::transmute::<_, FnGetSystemInfo>(get_sys_info)(&mut si);

            format!("Processors: {}\nArch: {}\nPageSize: {}", 
                si.dwNumberOfProcessors, 
                si.wProcessorArchitecture,
                si.dwPageSize
            )
        }
        #[cfg(not(target_os = "windows"))]
        {
            String::from("Linux System Info (Stub)")
        }
    }

    pub fn net_scan(&self, target: &str) -> String {
        format!("Scanning {}", target)
    }

    pub fn get_hostname(&self) -> String {
        #[cfg(target_os = "windows")]
        unsafe {
            let kernel32 = hash_str(b"kernel32.dll");
            let get_comp_name = resolve_function(kernel32, hash_str(b"GetComputerNameA"));
            if get_comp_name.is_null() { return String::from("unknown"); }

            type FnGetComputerNameA = unsafe extern "system" fn(*mut u8, *mut u32) -> i32;
            let mut buf = [0u8; 256];
            let mut len = 256;
            if core::mem::transmute::<_, FnGetComputerNameA>(get_comp_name)(buf.as_mut_ptr(), &mut len) != 0 {
                let s = core::slice::from_raw_parts(buf.as_ptr(), len as usize);
                return String::from_utf8_lossy(s).into_owned();
            }
            String::from("unknown")
        }
        #[cfg(not(target_os = "windows"))]
        {
            String::from("linux-target")
        }
    }

    pub fn get_username(&self) -> String {
        #[cfg(target_os = "windows")]
        unsafe {
            let advapi32 = hash_str(b"advapi32.dll");
            let get_user_name = resolve_function(advapi32, hash_str(b"GetUserNameA"));
            
            // Fallback if advapi32 not loaded? We can try LoadLibrary but let's assume standard session.
            // If failed, return "unknown"
            if get_user_name.is_null() { return String::from("unknown"); }

            type FnGetUserNameA = unsafe extern "system" fn(*mut u8, *mut u32) -> i32;
            let mut buf = [0u8; 256];
            let mut len = 256;
            if core::mem::transmute::<_, FnGetUserNameA>(get_user_name)(buf.as_mut_ptr(), &mut len) != 0 {
                // GetUserName includes null terminator in len sometimes? MDN says:
                // "If the function succeeds, the return value is a nonzero value... buffer receives the null-terminated string... lpnSize points to the size of the copied string, including the terminating null character."
                // So len includes null. We subtract 1.
                let s_len = if len > 0 { len - 1 } else { 0 };
                let s = core::slice::from_raw_parts(buf.as_ptr(), s_len as usize);
                return String::from_utf8_lossy(s).into_owned();
            }
            String::from("unknown")
        }
        #[cfg(not(target_os = "windows"))]
        {
            String::from("root")
        }
    }
}
