use core::arch::asm;
use core::ffi::c_void;

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
pub const SYS_ACCEPT: usize = 43;
#[cfg(not(target_os = "windows"))]
pub const SYS_BIND: usize = 49;
#[cfg(not(target_os = "windows"))]
pub const SYS_LISTEN: usize = 50;
#[cfg(not(target_os = "windows"))]
pub const SYS_SENDTO: usize = 44; // sendto/write
#[cfg(not(target_os = "windows"))]
pub const SYS_RECVFROM: usize = 45;
#[cfg(not(target_os = "windows"))]
pub const SYS_FCNTL: usize = 72;
#[cfg(not(target_os = "windows"))]
pub const SYS_EXIT: usize = 60;
#[cfg(not(target_os = "windows"))]
pub const SYS_WAIT4: usize = 61;
#[cfg(not(target_os = "windows"))]
pub const SYS_PIPE: usize = 22;
#[cfg(not(target_os = "windows"))]
pub const SYS_DUP2: usize = 33;
#[cfg(not(target_os = "windows"))]
pub const SYS_FORK: usize = 57;
#[cfg(not(target_os = "windows"))]
pub const SYS_EXECVE: usize = 59;
#[cfg(not(target_os = "windows"))]
pub const SYS_UNAME: usize = 63;
#[cfg(not(target_os = "windows"))]
pub const SYS_SYSINFO: usize = 99;
#[cfg(not(target_os = "windows"))]
pub const SYS_GETUID: usize = 102;

#[cfg(not(target_os = "windows"))]
pub const SYS_PTRACE: usize = 101;
#[cfg(not(target_os = "windows"))]
pub const SYS_PROCESS_VM_WRITEV: usize = 310;
#[cfg(not(target_os = "windows"))]
pub const SYS_OPENAT: usize = 257;
#[cfg(not(target_os = "windows"))]
pub const SYS_CLOCK_GETTIME: usize = 228;

#[cfg(all(not(target_os = "windows"), target_arch = "x86_64"))]
pub const SYS_GETRANDOM: usize = 318;

#[cfg(all(not(target_os = "windows"), target_arch = "aarch64"))]
pub const SYS_GETRANDOM: usize = 278;

#[cfg(all(
    not(target_os = "windows"),
    not(any(target_arch = "x86_64", target_arch = "aarch64"))
))]
pub const SYS_GETRANDOM: usize = 0; // Fallback/Unsupported

#[cfg(not(target_os = "windows"))]
pub unsafe fn sys_getrandom(buf: *mut u8, len: usize, flags: u32) -> isize {
    if SYS_GETRANDOM == 0 {
        return -1;
    }
    syscall3(SYS_GETRANDOM, buf as usize, len, flags as usize) as isize
}

// Ptrace Constants
pub const PTRACE_TRACEME: usize = 0;
pub const PTRACE_PEEKTEXT: usize = 1;
pub const PTRACE_POKETEXT: usize = 4;
pub const PTRACE_CONT: usize = 7;
pub const PTRACE_KILL: usize = 8;
pub const PTRACE_ATTACH: usize = 16;
pub const PTRACE_DETACH: usize = 17;
pub const PTRACE_GETREGS: usize = 12;
pub const PTRACE_SETREGS: usize = 13;

#[cfg(not(target_os = "windows"))]
#[repr(C)]
pub struct user_regs_struct {
    pub r15: u64,
    pub r14: u64,
    pub r13: u64,
    pub r12: u64,
    pub rbp: u64,
    pub rbx: u64,
    pub r11: u64,
    pub r10: u64,
    pub r9: u64,
    pub r8: u64,
    pub rax: u64,
    pub rcx: u64,
    pub rdx: u64,
    pub rsi: u64,
    pub rdi: u64,
    pub orig_rax: u64,
    pub rip: u64,
    pub cs: u64,
    pub eflags: u64,
    pub rsp: u64,
    pub ss: u64,
    pub fs_base: u64,
    pub gs_base: u64,
    pub ds: u64,
    pub es: u64,
    pub fs: u64,
    pub gs: u64,
}

#[cfg(not(target_os = "windows"))]
#[repr(C)]
pub struct Sysinfo {
    pub uptime: i64,
    pub loads: [u64; 3],
    pub totalram: u64,
    pub freeram: u64,
    pub sharedram: u64,
    pub bufferram: u64,
    pub totalswap: u64,
    pub freeswap: u64,
    pub procs: u16,
    pub pad: u16,
    pub totalhigh: u64,
    pub freehigh: u64,
    pub mem_unit: u32,
    pub _f: [u8; 0],
}

#[cfg(not(target_os = "windows"))]
pub unsafe fn sys_sysinfo(info: *mut Sysinfo) -> isize {
    syscall1(SYS_SYSINFO, info as usize) as isize
}

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
pub unsafe fn syscall6(
    n: usize,
    a1: usize,
    a2: usize,
    a3: usize,
    a4: usize,
    a5: usize,
    a6: usize,
) -> usize {
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
pub unsafe fn sys_mmap(
    addr: usize,
    len: usize,
    prot: i32,
    flags: i32,
    fd: i32,
    offset: usize,
) -> usize {
    syscall6(
        SYS_MMAP,
        addr,
        len,
        prot as usize,
        flags as usize,
        fd as usize,
        offset,
    )
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
pub unsafe fn sys_bind(sockfd: usize, addr: *const u8, addrlen: u32) -> usize {
    syscall3(SYS_BIND, sockfd, addr as usize, addrlen as usize)
}

#[cfg(not(target_os = "windows"))]
pub unsafe fn sys_listen(sockfd: usize, backlog: i32) -> usize {
    syscall2(SYS_LISTEN, sockfd, backlog as usize)
}

#[cfg(not(target_os = "windows"))]
pub unsafe fn sys_accept(sockfd: usize, addr: *mut u8, addrlen: *mut u32) -> usize {
    syscall3(SYS_ACCEPT, sockfd, addr as usize, addrlen as usize)
}

#[cfg(not(target_os = "windows"))]
pub unsafe fn sys_fcntl(fd: usize, cmd: i32, arg: usize) -> usize {
    syscall3(SYS_FCNTL, fd, cmd as usize, arg)
}

#[cfg(not(target_os = "windows"))]
pub unsafe fn sys_connect(sockfd: usize, addr: *const u8, addrlen: u32) -> usize {
    syscall3(SYS_CONNECT, sockfd, addr as usize, addrlen as usize)
}

#[cfg(not(target_os = "windows"))]
pub unsafe fn sys_sendto(
    sockfd: usize,
    buf: *const u8,
    len: usize,
    flags: i32,
    dest_addr: *const u8,
    addrlen: u32,
) -> isize {
    syscall6(
        SYS_SENDTO,
        sockfd,
        buf as usize,
        len,
        flags as usize,
        dest_addr as usize,
        addrlen as usize,
    ) as isize
}

#[cfg(not(target_os = "windows"))]
pub unsafe fn sys_recvfrom(
    sockfd: usize,
    buf: *mut u8,
    len: usize,
    flags: i32,
    src_addr: *mut u8,
    addrlen: *mut u32,
) -> isize {
    syscall6(
        SYS_RECVFROM,
        sockfd,
        buf as usize,
        len,
        flags as usize,
        src_addr as usize,
        addrlen as usize,
    ) as isize
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
pub unsafe fn sys_open(path: *const u8, flags: i32, mode: i32) -> isize {
    syscall3(SYS_OPEN, path as usize, flags as usize, mode as usize) as isize
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
pub unsafe fn sys_clock_gettime(clock_id: i32, tp: *mut Timespec) -> i32 {
    syscall2(SYS_CLOCK_GETTIME, clock_id as usize, tp as usize) as i32
}

#[cfg(not(target_os = "windows"))]
pub unsafe fn sys_wait4(pid: i32, wstatus: *mut i32, options: i32, rusage: *mut c_void) -> isize {
    syscall4(
        SYS_WAIT4,
        pid as usize,
        wstatus as usize,
        options as usize,
        rusage as usize,
    ) as isize
}

#[cfg(not(target_os = "windows"))]
pub unsafe fn sys_fork() -> isize {
    syscall0(SYS_FORK) as isize
}

#[cfg(not(target_os = "windows"))]
pub unsafe fn sys_execve(
    filename: *const u8,
    argv: *const *const u8,
    envp: *const *const u8,
) -> isize {
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

#[cfg(not(target_os = "windows"))]
pub unsafe fn sys_uname(buf: *mut Utsname) -> isize {
    syscall1(SYS_UNAME, buf as usize) as isize
}

#[cfg(not(target_os = "windows"))]
pub unsafe fn sys_getuid() -> usize {
    syscall0(SYS_GETUID)
}

#[cfg(not(target_os = "windows"))]
pub unsafe fn sys_process_vm_writev(
    pid: i32,
    local_iov: *const Iovec,
    liovcnt: usize,
    remote_iov: *const Iovec,
    riovcnt: usize,
    flags: usize,
) -> isize {
    syscall6(
        SYS_PROCESS_VM_WRITEV,
        pid as usize,
        local_iov as usize,
        liovcnt,
        remote_iov as usize,
        riovcnt,
        flags,
    ) as isize
}

#[cfg(not(target_os = "windows"))]
pub unsafe fn sys_ptrace(request: usize, pid: i32, addr: usize, data: usize) -> isize {
    syscall4(101, request, pid as usize, addr, data) as isize
}

#[cfg(not(target_os = "windows"))]
#[inline(always)]
pub unsafe fn syscall4(n: usize, a1: usize, a2: usize, a3: usize, a4: usize) -> usize {
    let ret: usize;
    asm!(
        "syscall",
        inlateout("rax") n => ret,
        in("rdi") a1,
        in("rsi") a2,
        in("rdx") a3,
        in("r10") a4,
        out("rcx") _,
        out("r11") _,
        options(nostack, preserves_flags)
    );
    ret
}

#[repr(C)]
pub struct Iovec {
    pub iov_base: *mut c_void,
    pub iov_len: usize,
}

#[repr(C)]
pub struct Timespec {
    pub tv_sec: i64,
    pub tv_nsec: i64,
}

#[cfg(not(target_os = "windows"))]
#[repr(C)]
pub struct Utsname {
    pub sysname: [u8; 65],
    pub nodename: [u8; 65],
    pub release: [u8; 65],
    pub version: [u8; 65],
    pub machine: [u8; 65],
    pub domainname: [u8; 65],
}

#[repr(C)]
pub struct SockAddrIn {
    pub sin_family: u16,
    pub sin_port: u16,
    pub sin_addr: u32,
    pub sin_zero: [u8; 8],
}

#[repr(C)]
pub struct SockAddrIn6 {
    pub sin6_family: u16,
    pub sin6_port: u16,
    pub sin6_flowinfo: u32,
    pub sin6_addr: [u8; 16],
    pub sin6_scope_id: u32,
}

// -----------------------------------------------------------------------------
// Windows Implementation
// -----------------------------------------------------------------------------

#[cfg(target_os = "windows")]
use crate::utils::api_resolver::{hash_str, resolve_function};

#[cfg(target_os = "windows")]
static mut SSN_CACHE: [(u32, u16); 64] = [(0, 0); 64];
#[cfg(target_os = "windows")]
static mut SSN_CACHE_COUNT: usize = 0;

#[cfg(target_os = "windows")]
pub unsafe fn get_ssn(function_hash: u32) -> u16 {
    // 1. Check Cache
    for i in 0..SSN_CACHE_COUNT {
        if SSN_CACHE[i].0 == function_hash {
            return SSN_CACHE[i].1;
        }
    }

    let ntdll_hash = hash_str(b"ntdll.dll");
    let addr = resolve_function(ntdll_hash, function_hash);

    if addr.is_null() {
        return 0;
    }

    let mut ssn = 0;

    // 2. Try current function
    if let Some(s) = parse_syscall_stub(addr) {
        ssn = s;
    } else {
        // Halo's Gate: Check neighbors
        for i in 1..32 {
            // Check neighbor above
            if let Some(s) = parse_syscall_stub(addr.add(i * 32)) {
                ssn = s - i as u16;
                break;
            }
            // Check neighbor below
            if let Some(s) = parse_syscall_stub(addr.sub(i * 32)) {
                ssn = s + i as u16;
                break;
            }
        }
    }

    // 3. Update Cache
    if ssn != 0 && SSN_CACHE_COUNT < 64 {
        SSN_CACHE[SSN_CACHE_COUNT] = (function_hash, ssn);
        SSN_CACHE_COUNT += 1;
    }

    ssn
}

#[cfg(target_os = "windows")]
unsafe fn parse_syscall_stub(addr: *const ()) -> Option<u16> {
    let p = addr as *const u8;
    // Pattern:
    // mov r10, rcx; mov eax, <SSN>
    // Bytes: 4c 8b d1 b8 <SSN_LOW> <SSN_HIGH> 00 00
    // OR
    // mov eax, <SSN>; mov r10, rcx (Alternative stub)
    // b8 <SSN_LOW> <SSN_HIGH> 00 00 4c 8b d1

    if *p == 0x4c && *p.add(1) == 0x8b && *p.add(2) == 0xd1 && *p.add(3) == 0xb8 {
        let ssn_low = *p.add(4) as u16;
        let ssn_high = *p.add(5) as u16;
        return Some((ssn_high << 8) | ssn_low);
    } else if *p == 0xb8 && *p.add(5) == 0x4c && *p.add(6) == 0x8b && *p.add(7) == 0xd1 {
        let ssn_low = *p.add(1) as u16;
        let ssn_high = *p.add(2) as u16;
        return Some((ssn_high << 8) | ssn_low);
    }
    None
}

#[cfg(target_os = "windows")]
static mut SYSCALL_GADGET: Option<*const ()> = None;

#[cfg(target_os = "windows")]
unsafe fn get_syscall_gadget() -> Option<*const ()> {
    if let Some(addr) = SYSCALL_GADGET {
        return Some(addr);
    }

    let ntdll_hash = hash_str(b"ntdll.dll");

    // Multiple gadget sources for redundancy
    let targets = [
        hash_str(b"NtYieldExecution"),
        hash_str(b"NtTestAlert"),
        hash_str(b"NtOpenFile"),
    ];

    for &func_hash in &targets {
        let addr = resolve_function(ntdll_hash, func_hash) as *const u8;
        if addr.is_null() {
            continue;
        }

        // Scan for syscall; ret (0F 05 C3) within the function stub
        for i in 0..32 {
            if *addr.add(i) == 0x0F && *addr.add(i + 1) == 0x05 && *addr.add(i + 2) == 0xC3 {
                let gadget = addr.add(i) as *const ();
                SYSCALL_GADGET = Some(gadget);
                return Some(gadget);
            }
        }
    }

    // Egg hunt fallback: Scan ntdll .text section (simplified search)
    if let Some(ntdll_base) = crate::utils::api_resolver::get_module_base(ntdll_hash) {
        let p = ntdll_base as *const u8;
        // Search first 4KB for a gadget if function stubs failed
        for i in 0..4096 {
            if *p.add(i) == 0x0F && *p.add(i + 1) == 0x05 && *p.add(i + 2) == 0xC3 {
                let gadget = p.add(i) as *const ();
                SYSCALL_GADGET = Some(gadget);
                return Some(gadget);
            }
        }
    }

    None
}

#[cfg(target_os = "windows")]
#[inline(always)]
pub unsafe fn do_syscall(ssn: u16, w1: usize, w2: usize, w3: usize, w4: usize, w5: usize) -> usize {
    let ret: usize;
    if let Some(gadget) = get_syscall_gadget() {
        core::arch::asm!(
            "mov [rsp+0x28], {w5}", // 5th arg. 0x28 because of sub rsp below?
            // Actually, we should sub rsp first.
            "sub rsp, 0x30",        // Align + shadow + args
            "mov rax, {w5}",
            "mov [rsp+0x20], rax",  // 5th arg at offset 0x20
            "mov r10, rcx",
            "call {gadget}",
            "add rsp, 0x30",
            gadget = in(reg) gadget,
            w5 = in(reg) w5,
            in("eax") ssn as u32,
            in("rcx") w1,
            in("rdx") w2,
            in("r8")  w3,
            in("r9")  w4,
            lateout("rax") ret,
            out("r10") _,
            out("r11") _,
        );
    } else {
        core::arch::asm!(
            "sub rsp, 0x30",
            "mov rax, {w5}",
            "mov [rsp+0x20], rax",
            "mov r10, rcx",
            "syscall",
            "add rsp, 0x30",
            w5 = in(reg) w5,
            in("eax") ssn as u32,
            in("rcx") w1,
            in("rdx") w2,
            in("r8")  w3,
            in("r9")  w4,
            lateout("rax") ret,
            out("r10") _,
            out("r11") _,
        );
    }
    ret
}

#[cfg(target_os = "windows")]
pub unsafe fn sys_exit(code: i32) -> ! {
    let term_hash = hash_str(b"NtTerminateProcess");
    let ssn = get_ssn(term_hash);
    if ssn != 0 {
        // Handle -1 (Current Process)
        do_syscall(ssn, 0xFFFFFFFFFFFFFFFF, code as usize, 0, 0, 0);
    }

    loop {}
}
