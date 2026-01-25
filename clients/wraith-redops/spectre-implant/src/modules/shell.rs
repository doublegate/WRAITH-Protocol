use alloc::vec::Vec;

#[cfg(not(target_os = "windows"))]
use crate::utils::syscalls::{sys_fork, sys_execve, sys_pipe, sys_dup2, sys_read, sys_close, sys_exit};

pub struct Shell;

impl Shell {
    pub fn exec(&self, cmd: &str) -> Vec<u8> {
        #[cfg(not(target_os = "windows"))]
        unsafe {
            let mut pipefd = [0i32; 2];
            if sys_pipe(pipefd.as_mut_ptr()) < 0 {
                return Vec::from(b"Pipe failed");
            }

            let pid = sys_fork();
            if pid < 0 {
                return Vec::from(b"Fork failed");
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
                
                let argv = [sh.as_ptr(), arg1.as_ptr(), cmd_c.as_ptr(), core::ptr::null()];
                let envp = [core::ptr::null()];
                
                sys_execve(sh.as_ptr(), argv.as_ptr(), envp.as_ptr());
                sys_exit(1);
            } else {
                // Parent
                sys_close(pipefd[1] as usize);
                let mut output = Vec::new();
                let mut buf = [0u8; 1024];
                loop {
                    let n = sys_read(pipefd[0] as usize, buf.as_mut_ptr(), 1024);
                    if n as isize <= 0 { break; }
                    output.extend_from_slice(&buf[..n]);
                }
                sys_close(pipefd[0] as usize);
                output
            }
        }

        #[cfg(target_os = "windows")]
        {
            Vec::from(b"Windows shell not implemented")
        }
    }
}
