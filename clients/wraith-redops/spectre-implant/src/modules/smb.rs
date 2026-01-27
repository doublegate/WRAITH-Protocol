use alloc::vec::Vec;

#[repr(C, packed)]
pub struct SMB2Header {
    pub protocol_id: [u8; 4],
    pub structure_size: u16,
    pub credit_charge: u16,
    pub status: u32,
    pub command: u16,
    pub credit_request: u16,
    pub flags: u32,
    pub next_command: u32,
    pub message_id: u64,
    pub process_id: u32,
    pub tree_id: u32,
    pub session_id: u64,
    pub signature: [u8; 16],
}

impl SMB2Header {
    pub fn new(command: u16, message_id: u64, process_id: u32) -> Self {
        Self {
            protocol_id: [0xFE, b'S', b'M', b'B'],
            structure_size: 64,
            credit_charge: 0,
            status: 0,
            command,
            credit_request: 127,
            flags: 0,
            next_command: 0,
            message_id,
            process_id,
            tree_id: 0,
            session_id: 0,
            signature: [0; 16],
        }
    }
}

// Commands
pub const SMB2_NEGOTIATE: u16 = 0x0000;
pub const SMB2_SESSION_SETUP: u16 = 0x0001;
pub const SMB2_TREE_CONNECT: u16 = 0x0003;
pub const SMB2_CREATE: u16 = 0x0005;
pub const SMB2_CLOSE: u16 = 0x0006;
pub const SMB2_READ: u16 = 0x0008;
pub const SMB2_WRITE: u16 = 0x0009;

#[repr(C, packed)]
pub struct SMB2NegotiateReq {
    pub structure_size: u16,
    pub dialect_count: u16,
    pub security_mode: u16,
    pub reserved: u16,
    pub capabilities: u32,
    pub client_guid: [u8; 16],
    pub negotiate_context_offset: u32,
    pub negotiate_context_count: u16,
    pub reserved2: u16,
    pub dialects: [u16; 1], // Simplified: Just 0x0202 (SMB 2.0.2)
}

#[repr(C, packed)]
pub struct SMB2SessionSetupReq {
    pub structure_size: u16,
    pub flags: u8,
    pub security_mode: u8,
    pub capabilities: u32,
    pub channel: u32,
    pub security_buffer_offset: u16,
    pub security_buffer_length: u16,
    pub previous_session_id: u64,
}

#[repr(C, packed)]
pub struct SMB2TreeConnectReq {
    pub structure_size: u16,
    pub reserved: u16,
    pub path_offset: u16,
    pub path_length: u16,
}

pub struct SmbClient {
    pub message_id: u64,
    pub process_id: u32,
    pub session_id: u64,
    pub tree_id: u32,
}

impl SmbClient {
    pub fn new() -> Self {
        Self {
            message_id: 0,
            process_id: 0xFEFF, // Arbitrary
            session_id: 0,
            tree_id: 0,
        }
    }
    
    // Serialization helpers would go here
    
    pub fn connect(&mut self, ip: [u8; 4], port: u16) -> Result<(), ()> {
        #[cfg(not(target_os = "windows"))]
        unsafe {
            use crate::utils::syscalls::*;
            let sock = sys_socket(2, 1, 0);
            if (sock as isize) < 0 { return Err(()); }

            let addr = SockAddrIn {
                sin_family: 2,
                sin_port: port.to_be(),
                sin_addr: u32::from_ne_bytes(ip),
                sin_zero: [0; 8],
            };

            if sys_connect(sock, &addr as *const _ as *const u8, 16) == 0 {
                if self.negotiate(sock).is_ok() {
                    if self.session_setup(sock).is_ok() {
                        if self.tree_connect(sock).is_ok() {
                            return Ok(());
                        }
                    }
                }
            }
            sys_close(sock);
            Err(())
        }
        
        #[cfg(target_os = "windows")]
        Err(()) // TODO: Windows socket impl (similar to socks.rs)
    }

    #[cfg(not(target_os = "windows"))]
    unsafe fn negotiate(&mut self, fd: usize) -> Result<(), ()> {
        use crate::utils::syscalls::*;
        
        let mut req = Vec::new();
        // NetBIOS Header (4 bytes)
        req.extend_from_slice(&[0, 0, 0, 0]);
        
        let header = SMB2Header::new(SMB2_NEGOTIATE, self.message_id, self.process_id);
        let body = SMB2NegotiateReq {
            structure_size: 36,
            dialect_count: 1,
            security_mode: 1, // Signing enabled
            reserved: 0,
            capabilities: 0,
            client_guid: [0xAA; 16], // Random GUID
            negotiate_context_offset: 0,
            negotiate_context_count: 0,
            reserved2: 0,
            dialects: [0x0202],
        };

        let header_bytes = core::slice::from_raw_parts(&header as *const _ as *const u8, core::mem::size_of::<SMB2Header>());
        let body_bytes = core::slice::from_raw_parts(&body as *const _ as *const u8, core::mem::size_of::<SMB2NegotiateReq>());
        
        req.extend_from_slice(header_bytes);
        req.extend_from_slice(body_bytes);
        
        // Fix NetBIOS length (Big Endian)
        let len = (req.len() - 4) as u32;
        req[3] = (len & 0xFF) as u8;
        req[2] = ((len >> 8) & 0xFF) as u8;
        req[1] = ((len >> 16) & 0xFF) as u8;

        sys_write(fd, req.as_ptr(), req.len());

        // Read response (Simplified: Just skip it and assume success for now)
        let mut buf = [0u8; 1024];
        let n = sys_read(fd, buf.as_mut_ptr(), 1024);
        if n > 0 {
            self.message_id += 1;
            Ok(())
        } else {
            Err(())
        }
    }

    #[cfg(not(target_os = "windows"))]
    unsafe fn session_setup(&mut self, fd: usize) -> Result<(), ()> {
        use crate::utils::syscalls::*;
        
        let mut req = Vec::new();
        req.extend_from_slice(&[0, 0, 0, 0]); // NetBIOS
        
        let header = SMB2Header::new(SMB2_SESSION_SETUP, self.message_id, self.process_id);
        let body = SMB2SessionSetupReq {
            structure_size: 25,
            flags: 0,
            security_mode: 1,
            capabilities: 0,
            channel: 0,
            security_buffer_offset: 88, // 64 + 24
            security_buffer_length: 0, // Anonymous
            previous_session_id: 0,
        };

        let header_bytes = core::slice::from_raw_parts(&header as *const _ as *const u8, core::mem::size_of::<SMB2Header>());
        let body_bytes = core::slice::from_raw_parts(&body as *const _ as *const u8, core::mem::size_of::<SMB2SessionSetupReq>());
        
        req.extend_from_slice(header_bytes);
        req.extend_from_slice(body_bytes);
        
        let len = (req.len() - 4) as u32;
        req[3] = (len & 0xFF) as u8;
        req[2] = ((len >> 8) & 0xFF) as u8;
        req[1] = ((len >> 16) & 0xFF) as u8;

        sys_write(fd, req.as_ptr(), req.len());

        let mut buf = [0u8; 1024];
        let n = sys_read(fd, buf.as_mut_ptr(), 1024);
        if n > 64 {
            // Parse Session ID from response header (offset 40)
            // NetBIOS (4) + Header (64). SessionId is at offset 4+40 = 44.
            // But we read into buf starting at 0.
            // Wait, NetBIOS header is 4 bytes.
            // If we assume response has NetBIOS header.
            if buf[0] == 0 { // Session Message
                let session_id_ptr = buf.as_ptr().add(4 + 40) as *const u64;
                self.session_id = *session_id_ptr;
            }
            self.message_id += 1;
            Ok(())
        } else {
            Err(())
        }
    }

    #[cfg(not(target_os = "windows"))]
    unsafe fn tree_connect(&mut self, fd: usize) -> Result<(), ()> {
        use crate::utils::syscalls::*;
        
        let mut req = Vec::new();
        req.extend_from_slice(&[0, 0, 0, 0]);
        
        let mut header = SMB2Header::new(SMB2_TREE_CONNECT, self.message_id, self.process_id);
        header.session_id = self.session_id;
        
        let path = b"\\\\127.0.0.1\\IPC$\0";
        let path_utf16: Vec<u16> = path.iter().map(|&c| c as u16).collect();
        let path_bytes = core::slice::from_raw_parts(path_utf16.as_ptr() as *const u8, path_utf16.len() * 2);

        let body = SMB2TreeConnectReq {
            structure_size: 9,
            reserved: 0,
            path_offset: 72, // 64 + 8
            path_length: path_bytes.len() as u16,
        };

        let header_bytes = core::slice::from_raw_parts(&header as *const _ as *const u8, core::mem::size_of::<SMB2Header>());
        let body_bytes = core::slice::from_raw_parts(&body as *const _ as *const u8, core::mem::size_of::<SMB2TreeConnectReq>());
        
        req.extend_from_slice(header_bytes);
        req.extend_from_slice(body_bytes);
        req.extend_from_slice(path_bytes);
        
        let len = (req.len() - 4) as u32;
        req[3] = (len & 0xFF) as u8;
        req[2] = ((len >> 8) & 0xFF) as u8;
        req[1] = ((len >> 16) & 0xFF) as u8;

        sys_write(fd, req.as_ptr(), req.len());

        let mut buf = [0u8; 1024];
        let n = sys_read(fd, buf.as_mut_ptr(), 1024);
        if n > 64 {
            if buf[0] == 0 {
                let tree_id_ptr = buf.as_ptr().add(4 + 36) as *const u32;
                self.tree_id = *tree_id_ptr;
            }
            self.message_id += 1;
            Ok(())
        } else {
            Err(())
        }
    }
}
