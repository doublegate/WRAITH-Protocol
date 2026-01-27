use alloc::vec::Vec;

#[cfg(not(target_os = "windows"))]
use crate::utils::syscalls::*;
#[cfg(target_os = "windows")]
use crate::utils::api_resolver::{hash_str, resolve_function};
#[cfg(target_os = "windows")]
use crate::utils::windows_definitions::*;
use crate::utils::syscalls::SockAddrIn;

pub enum SocksState {
    Greeting,
    Auth,
    Request,
    Forwarding,
    Error,
}

pub struct SocksProxy {
    state: SocksState,
    #[cfg(not(target_os = "windows"))]
    fd: Option<usize>,
    #[cfg(target_os = "windows")]
    fd: Option<HANDLE>,
}

impl SocksProxy {
    pub fn new() -> Self {
        Self { 
            state: SocksState::Greeting,
            fd: None,
        }
    }

    /// Processes incoming data from the SOCKS client and returns the response bytes.
    /// Manages the internal state machine for the SOCKS handshake and relaying.
    pub fn process(&mut self, data: &[u8]) -> Vec<u8> {
        match self.state {
            SocksState::Greeting => self.handle_greeting(data),
            SocksState::Auth => self.handle_auth(data),
            SocksState::Request => self.handle_request(data),
            SocksState::Forwarding => self.handle_forwarding(data),
            SocksState::Error => Vec::new(),
        }
    }

    fn handle_forwarding(&mut self, data: &[u8]) -> Vec<u8> {
        if let Some(fd) = self.fd {
            #[cfg(not(target_os = "windows"))]
            unsafe {
                if !data.is_empty() {
                    sys_write(fd, data.as_ptr(), data.len());
                }
                let mut buf = [0u8; 4096];
                let n = sys_read(fd, buf.as_mut_ptr(), 4096);
                if n > 0 {
                    return buf[..n].to_vec();
                }
            }

            #[cfg(target_os = "windows")]
            unsafe {
                let ws2_32 = hash_str(b"ws2_32.dll");
                let send = resolve_function(ws2_32, hash_str(b"send"));
                let recv = resolve_function(ws2_32, hash_str(b"recv"));

                if !send.is_null() && !recv.is_null() {
                    type FnSend = unsafe extern "system" fn(HANDLE, *const u8, i32, i32) -> i32;
                    type FnRecv = unsafe extern "system" fn(HANDLE, *mut u8, i32, i32) -> i32;

                    if !data.is_empty() {
                        core::mem::transmute::<_, FnSend>(send)(fd, data.as_ptr(), data.len() as i32, 0);
                    }

                    let mut buf = [0u8; 4096];
                    let n = core::mem::transmute::<_, FnRecv>(recv)(fd, buf.as_mut_ptr(), 4096, 0);
                    if n > 0 {
                        return buf[..n as usize].to_vec();
                    }
                }
            }
        }
        Vec::new()
    }

    fn handle_greeting(&mut self, data: &[u8]) -> Vec<u8> {
        if data.len() < 2 {
            return Vec::new();
        }

        match data[0] {
            0x05 => { // SOCKS5
                let n_methods = data[1] as usize;
                if data.len() < 2 + n_methods {
                    return Vec::new();
                }
                
                // Select 0x00 (No Authentication Required)
                let methods = &data[2..2 + n_methods];
                if methods.contains(&0x00) {
                    self.state = SocksState::Request;
                    Vec::from([0x05, 0x00])
                } else {
                    // 0xFF = No acceptable methods
                    self.state = SocksState::Error;
                    Vec::from([0x05, 0xFF])
                }
            },
            0x04 => { // SOCKS4
                // SOCKS4 doesn't have a greeting, it goes straight to Request
                self.state = SocksState::Request;
                self.handle_request(data)
            },
            _ => {
                self.state = SocksState::Error;
                Vec::new()
            }
        }
    }

    pub fn handle_auth(&self, _data: &[u8]) -> Vec<u8> {
        // Currently we only support 'No Auth'
        Vec::new()
    }

    pub fn handle_request(&mut self, data: &[u8]) -> Vec<u8> {
        if data.is_empty() { return Vec::new(); }

        match data[0] {
            0x05 => { // SOCKS5 Request
                if data.len() < 7 { return Vec::new(); }
                let cmd = data[1];
                if cmd != 0x01 { // CONNECT
                    return Vec::from([0x05, 0x07, 0x00, 0x01, 0,0,0,0, 0,0]);
                }

                let atyp = data[3];
                let mut ip_bytes = [0u8; 4];
                let mut port_bytes = [0u8; 2];

                match atyp {
                    0x01 => { // IPv4
                        if data.len() < 10 { return Vec::new(); }
                        ip_bytes.copy_from_slice(&data[4..8]);
                        port_bytes.copy_from_slice(&data[8..10]);
                    },
                    _ => {
                        // Simplified: Only support IPv4 for now
                        return Vec::from([0x05, 0x08, 0x00, 0x01, 0,0,0,0, 0,0]);
                    }
                }

                let port = u16::from_be_bytes(port_bytes);
                if self.tcp_connect(ip_bytes, port).is_ok() {
                    self.state = SocksState::Forwarding;
                    Vec::from([0x05, 0x00, 0x00, 0x01, 0,0,0,0, 0,0])
                } else {
                    self.state = SocksState::Error;
                    Vec::from([0x05, 0x04, 0x00, 0x01, 0,0,0,0, 0,0]) // Host unreachable
                }
            },
            0x04 => { // SOCKS4 Request
                if data.len() < 8 { return Vec::new(); }
                let cmd = data[1];
                if cmd != 0x01 {
                    return Vec::from([0x00, 0x5B, 0,0, 0,0,0,0]); // Request rejected
                }
                
                let mut port_bytes = [0u8; 2];
                let mut ip_bytes = [0u8; 4];
                port_bytes.copy_from_slice(&data[2..4]);
                ip_bytes.copy_from_slice(&data[4..8]);

                let port = u16::from_be_bytes(port_bytes);
                if self.tcp_connect(ip_bytes, port).is_ok() {
                    self.state = SocksState::Forwarding;
                    Vec::from([0x00, 0x5A, 0,0, 0,0,0,0]) // Request granted
                } else {
                    self.state = SocksState::Error;
                    Vec::from([0x00, 0x5B, 0,0, 0,0,0,0])
                }
            },
            _ => {
                self.state = SocksState::Error;
                Vec::new()
            }
        }
    }

    fn tcp_connect(&mut self, ip: [u8; 4], port: u16) -> Result<(), ()> {
        #[cfg(not(target_os = "windows"))]
        unsafe {
            let sock = sys_socket(2, 1, 0); // AF_INET, SOCK_STREAM
            if (sock as isize) < 0 { return Err(()); }

            let addr = SockAddrIn {
                sin_family: 2,
                sin_port: port.to_be(),
                sin_addr: u32::from_ne_bytes(ip),
                sin_zero: [0; 8],
            };

            if sys_connect(sock, &addr as *const _ as *const u8, 16) == 0 {
                self.fd = Some(sock);
                Ok(())
            } else {
                sys_close(sock);
                Err(())
            }
        }

        #[cfg(target_os = "windows")]
        unsafe {
            let ws2_32 = hash_str(b"ws2_32.dll");
            let socket_fn = resolve_function(ws2_32, hash_str(b"socket"));
            let connect_fn = resolve_function(ws2_32, hash_str(b"connect"));
            let wsa_startup = resolve_function(ws2_32, hash_str(b"WSAStartup"));

            if socket_fn.is_null() || connect_fn.is_null() || wsa_startup.is_null() {
                return Err(());
            }

            type FnWSAStartup = unsafe extern "system" fn(u16, *mut u8) -> i32;
            type FnSocket = unsafe extern "system" fn(i32, i32, i32) -> HANDLE;
            type FnConnect = unsafe extern "system" fn(HANDLE, *const u8, i32) -> i32;

            // Initialize Winsock if needed (simplified: 2.2)
            let mut wsa_data = [0u8; 512];
            core::mem::transmute::<_, FnWSAStartup>(wsa_startup)(0x0202, wsa_data.as_mut_ptr());

            let sock = core::mem::transmute::<_, FnSocket>(socket_fn)(2, 1, 0);
            if sock == (-1isize as HANDLE) { return Err(()); }

            let addr = SockAddrIn {
                sin_family: 2,
                sin_port: port.to_be(),
                sin_addr: u32::from_ne_bytes(ip),
                sin_zero: [0; 8],
            };

            if core::mem::transmute::<_, FnConnect>(connect_fn)(sock, &addr as *const _ as *const u8, 16) == 0 {
                self.fd = Some(sock);
                Ok(())
            } else {
                let closesocket = resolve_function(ws2_32, hash_str(b"closesocket"));
                if !closesocket.is_null() {
                    type FnCloseSocket = unsafe extern "system" fn(HANDLE) -> i32;
                    core::mem::transmute::<_, FnCloseSocket>(closesocket)(sock);
                }
                Err(())
            }
        }
    }
}

impl Drop for SocksProxy {
    fn drop(&mut self) {
        if let Some(fd) = self.fd {
            #[cfg(not(target_os = "windows"))]
            unsafe { sys_close(fd); }

            #[cfg(target_os = "windows")]
            unsafe {
                let ws2_32 = hash_str(b"ws2_32.dll");
                let closesocket = resolve_function(ws2_32, hash_str(b"closesocket"));
                if !closesocket.is_null() {
                    type FnCloseSocket = unsafe extern "system" fn(HANDLE) -> i32;
                    core::mem::transmute::<_, FnCloseSocket>(closesocket)(fd);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_socks5_greeting() {
        let mut proxy = SocksProxy::new();
        let greeting = [0x05, 0x01, 0x00];
        let response = proxy.process(&greeting);
        assert_eq!(response, [0x05, 0x00]);
    }

    #[test]
    fn test_socks5_connect_ipv4() {
        // This test will fail at runtime on non-Windows/no-root because sys_socket might fail
        // but we can check the state transition if we mock tcp_connect or just let it fail.
        let mut proxy = SocksProxy::new();
        proxy.process(&[0x05, 0x01, 0x00]); // Handshake
        
        let request = [0x05, 0x01, 0x00, 0x01, 127, 0, 0, 1, 0, 80];
        let _response = proxy.process(&request);
        // We don't assert success here because real connection might fail in test env
    }
}
