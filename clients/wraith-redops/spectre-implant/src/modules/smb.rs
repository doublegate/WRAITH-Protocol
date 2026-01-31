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

#[repr(C, packed)]
pub struct SMB2CreateReq {
    pub structure_size: u16,
    pub security_flags: u8,
    pub requested_oplock_level: u8,
    pub impersonation_level: u32,
    pub smb_create_flags: u64,
    pub reserved: u64,
    pub desired_access: u32,
    pub file_attributes: u32,
    pub share_access: u32,
    pub create_disposition: u32,
    pub create_options: u32,
    pub name_offset: u16,
    pub name_length: u16,
    pub create_contexts_offset: u32,
    pub create_contexts_length: u32,
}

#[repr(C, packed)]
pub struct SMB2ReadReq {
    pub structure_size: u16,
    pub padding: u8,
    pub flags: u8,
    pub length: u32,
    pub offset: u64,
    pub file_id: [u8; 16],
    pub minimum_count: u32,
    pub channel: u32,
    pub remaining_bytes: u32,
    pub read_channel_info_offset: u16,
    pub read_channel_info_length: u16,
    pub buffer: u8,
}

#[repr(C, packed)]
pub struct SMB2CloseReq {
    pub structure_size: u16,
    pub flags: u16,
    pub reserved: u32,
    pub file_id: [u8; 16],
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
            if (sock as isize) < 0 {
                return Err(());
            }

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
        unsafe {
            use crate::utils::api_resolver::{hash_str, resolve_function};
            use crate::utils::syscalls::SockAddrIn;
            use crate::utils::windows_definitions::HANDLE;

            let ws2_32 = hash_str(b"ws2_32.dll");
            let socket_fn = resolve_function(ws2_32, hash_str(b"socket"));
            let connect_fn = resolve_function(ws2_32, hash_str(b"connect"));
            let closesocket = resolve_function(ws2_32, hash_str(b"closesocket"));
            let wsa_startup = resolve_function(ws2_32, hash_str(b"WSAStartup"));

            if socket_fn.is_null() || connect_fn.is_null() || wsa_startup.is_null() {
                return Err(());
            }

            type FnWSAStartup = unsafe extern "system" fn(u16, *mut u8) -> i32;
            type FnSocket = unsafe extern "system" fn(i32, i32, i32) -> HANDLE;
            type FnConnect = unsafe extern "system" fn(HANDLE, *const u8, i32) -> i32;
            type FnCloseSocket = unsafe extern "system" fn(HANDLE) -> i32;

            let mut wsa_data = [0u8; 512];
            core::mem::transmute::<_, FnWSAStartup>(wsa_startup)(0x0202, wsa_data.as_mut_ptr());

            let sock = core::mem::transmute::<_, FnSocket>(socket_fn)(2, 1, 0);
            if sock == (-1isize as HANDLE) {
                return Err(());
            }

            let addr = SockAddrIn {
                sin_family: 2,
                sin_port: port.to_be(),
                sin_addr: u32::from_ne_bytes(ip),
                sin_zero: [0; 8],
            };

            if core::mem::transmute::<_, FnConnect>(connect_fn)(
                sock,
                &addr as *const _ as *const u8,
                16,
            ) == 0
            {
                if self.negotiate_win(sock).is_ok() {
                    if self.session_setup_win(sock).is_ok() {
                        if self.tree_connect_win(sock).is_ok() {
                            return Ok(());
                        }
                    }
                }
            }
            core::mem::transmute::<_, FnCloseSocket>(closesocket)(sock);
            Err(())
        }
    }

    fn check_status(&self, buf: &[u8]) -> bool {
        if buf.len() < 16 {
            return false;
        }
        let status = u32::from_le_bytes([buf[12], buf[13], buf[14], buf[15]]);
        status == 0
    }

    #[cfg(target_os = "windows")]
    unsafe fn negotiate_win(
        &mut self,
        sock: crate::utils::windows_definitions::HANDLE,
    ) -> Result<(), ()> {
        use crate::utils::api_resolver::{hash_str, resolve_function};
        let ws2_32 = hash_str(b"ws2_32.dll");
        let send_fn = resolve_function(ws2_32, hash_str(b"send"));
        let recv_fn = resolve_function(ws2_32, hash_str(b"recv"));
        type FnSend = unsafe extern "system" fn(
            crate::utils::windows_definitions::HANDLE,
            *const u8,
            i32,
            i32,
        ) -> i32;
        type FnRecv = unsafe extern "system" fn(
            crate::utils::windows_definitions::HANDLE,
            *mut u8,
            i32,
            i32,
        ) -> i32;

        let mut req = Vec::new();
        req.extend_from_slice(&[0, 0, 0, 0]);
        let header = SMB2Header::new(SMB2_NEGOTIATE, self.message_id, self.process_id);
        let body = SMB2NegotiateReq {
            structure_size: 36,
            dialect_count: 1,
            security_mode: 1,
            reserved: 0,
            capabilities: 0,
            client_guid: [0xAA; 16],
            negotiate_context_offset: 0,
            negotiate_context_count: 0,
            reserved2: 0,
            dialects: [0x0202],
        };
        let header_bytes = core::slice::from_raw_parts(
            &header as *const _ as *const u8,
            core::mem::size_of::<SMB2Header>(),
        );
        let body_bytes = core::slice::from_raw_parts(
            &body as *const _ as *const u8,
            core::mem::size_of::<SMB2NegotiateReq>(),
        );
        req.extend_from_slice(header_bytes);
        req.extend_from_slice(body_bytes);
        let len = (req.len() - 4) as u32;
        req[3] = (len & 0xFF) as u8;
        req[2] = ((len >> 8) & 0xFF) as u8;
        req[1] = ((len >> 16) & 0xFF) as u8;

        core::mem::transmute::<_, FnSend>(send_fn)(sock, req.as_ptr(), req.len() as i32, 0);

        let mut buf = [0u8; 1024];
        let n = core::mem::transmute::<_, FnRecv>(recv_fn)(sock, buf.as_mut_ptr(), 1024, 0);
        if n > 0 && self.check_status(&buf[..n as usize]) {
            self.message_id += 1;
            Ok(())
        } else {
            Err(())
        }
    }

    #[cfg(target_os = "windows")]
    unsafe fn session_setup_win(
        &mut self,
        sock: crate::utils::windows_definitions::HANDLE,
    ) -> Result<(), ()> {
        use crate::utils::api_resolver::{hash_str, resolve_function};
        let ws2_32 = hash_str(b"ws2_32.dll");
        let send_fn = resolve_function(ws2_32, hash_str(b"send"));
        let recv_fn = resolve_function(ws2_32, hash_str(b"recv"));
        type FnSend = unsafe extern "system" fn(
            crate::utils::windows_definitions::HANDLE,
            *const u8,
            i32,
            i32,
        ) -> i32;
        type FnRecv = unsafe extern "system" fn(
            crate::utils::windows_definitions::HANDLE,
            *mut u8,
            i32,
            i32,
        ) -> i32;

        let mut req = Vec::new();
        req.extend_from_slice(&[0, 0, 0, 0]);
        let header = SMB2Header::new(SMB2_SESSION_SETUP, self.message_id, self.process_id);
        let body = SMB2SessionSetupReq {
            structure_size: 25,
            flags: 0,
            security_mode: 1,
            capabilities: 0,
            channel: 0,
            security_buffer_offset: 88,
            security_buffer_length: 0,
            previous_session_id: 0,
        };
        let header_bytes = core::slice::from_raw_parts(
            &header as *const _ as *const u8,
            core::mem::size_of::<SMB2Header>(),
        );
        let body_bytes = core::slice::from_raw_parts(
            &body as *const _ as *const u8,
            core::mem::size_of::<SMB2SessionSetupReq>(),
        );
        req.extend_from_slice(header_bytes);
        req.extend_from_slice(body_bytes);
        let len = (req.len() - 4) as u32;
        req[3] = (len & 0xFF) as u8;
        req[2] = ((len >> 8) & 0xFF) as u8;
        req[1] = ((len >> 16) & 0xFF) as u8;

        core::mem::transmute::<_, FnSend>(send_fn)(sock, req.as_ptr(), req.len() as i32, 0);

        let mut buf = [0u8; 1024];
        let n = core::mem::transmute::<_, FnRecv>(recv_fn)(sock, buf.as_mut_ptr(), 1024, 0);
        if n > 64 && self.check_status(&buf[..n as usize]) {
            if buf[0] == 0 {
                let session_id_ptr = buf.as_ptr().add(4 + 40) as *const u64;
                self.session_id = *session_id_ptr;
            }
            self.message_id += 1;
            Ok(())
        } else {
            Err(())
        }
    }

    #[cfg(target_os = "windows")]
    unsafe fn tree_connect_win(
        &mut self,
        sock: crate::utils::windows_definitions::HANDLE,
    ) -> Result<(), ()> {
        use crate::utils::api_resolver::{hash_str, resolve_function};
        let ws2_32 = hash_str(b"ws2_32.dll");
        let send_fn = resolve_function(ws2_32, hash_str(b"send"));
        let recv_fn = resolve_function(ws2_32, hash_str(b"recv"));
        type FnSend = unsafe extern "system" fn(
            crate::utils::windows_definitions::HANDLE,
            *const u8,
            i32,
            i32,
        ) -> i32;
        type FnRecv = unsafe extern "system" fn(
            crate::utils::windows_definitions::HANDLE,
            *mut u8,
            i32,
            i32,
        ) -> i32;

        let mut req = Vec::new();
        req.extend_from_slice(&[0, 0, 0, 0]);
        let mut header = SMB2Header::new(SMB2_TREE_CONNECT, self.message_id, self.process_id);
        header.session_id = self.session_id;
        let path = b"\\\\127.0.0.1\\IPC$\0";
        let path_utf16: Vec<u16> = path.iter().map(|&c| c as u16).collect();
        let path_bytes =
            core::slice::from_raw_parts(path_utf16.as_ptr() as *const u8, path_utf16.len() * 2);
        let body = SMB2TreeConnectReq {
            structure_size: 9,
            reserved: 0,
            path_offset: 72,
            path_length: path_bytes.len() as u16,
        };
        let header_bytes = core::slice::from_raw_parts(
            &header as *const _ as *const u8,
            core::mem::size_of::<SMB2Header>(),
        );
        let body_bytes = core::slice::from_raw_parts(
            &body as *const _ as *const u8,
            core::mem::size_of::<SMB2TreeConnectReq>(),
        );
        req.extend_from_slice(header_bytes);
        req.extend_from_slice(body_bytes);
        req.extend_from_slice(path_bytes);
        let len = (req.len() - 4) as u32;
        req[3] = (len & 0xFF) as u8;
        req[2] = ((len >> 8) & 0xFF) as u8;
        req[1] = ((len >> 16) & 0xFF) as u8;

        core::mem::transmute::<_, FnSend>(send_fn)(sock, req.as_ptr(), req.len() as i32, 0);

        let mut buf = [0u8; 1024];
        let n = core::mem::transmute::<_, FnRecv>(recv_fn)(sock, buf.as_mut_ptr(), 1024, 0);
        if n > 64 && self.check_status(&buf[..n as usize]) {
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

    #[cfg(target_os = "windows")]
    unsafe fn create_win(
        &mut self,
        sock: crate::utils::windows_definitions::HANDLE,
        filename: &str,
    ) -> Result<[u8; 16], ()> {
        use crate::utils::api_resolver::{hash_str, resolve_function};
        let ws2_32 = hash_str(b"ws2_32.dll");
        let send_fn = resolve_function(ws2_32, hash_str(b"send"));
        let recv_fn = resolve_function(ws2_32, hash_str(b"recv"));
        type FnSend = unsafe extern "system" fn(
            crate::utils::windows_definitions::HANDLE,
            *const u8,
            i32,
            i32,
        ) -> i32;
        type FnRecv = unsafe extern "system" fn(
            crate::utils::windows_definitions::HANDLE,
            *mut u8,
            i32,
            i32,
        ) -> i32;

        let mut req = Vec::new();
        req.extend_from_slice(&[0, 0, 0, 0]);
        let mut header = SMB2Header::new(SMB2_CREATE, self.message_id, self.process_id);
        header.session_id = self.session_id;
        header.tree_id = self.tree_id;

        let name_utf16: Vec<u16> = filename.encode_utf16().collect();
        let name_bytes =
            core::slice::from_raw_parts(name_utf16.as_ptr() as *const u8, name_utf16.len() * 2);

        let body = SMB2CreateReq {
            structure_size: 57,
            security_flags: 0,
            requested_oplock_level: 0,
            impersonation_level: 2, // Impersonation
            smb_create_flags: 0,
            reserved: 0,
            desired_access: 0x00120089, // Generic Read | Execute
            file_attributes: 0x80,      // Normal
            share_access: 1,            // Read
            create_disposition: 1,      // Open
            create_options: 0,
            name_offset: 120, // 64 + 56
            name_length: name_bytes.len() as u16,
            create_contexts_offset: 0,
            create_contexts_length: 0,
        };

        let header_bytes = core::slice::from_raw_parts(
            &header as *const _ as *const u8,
            core::mem::size_of::<SMB2Header>(),
        );
        let body_bytes = core::slice::from_raw_parts(
            &body as *const _ as *const u8,
            core::mem::size_of::<SMB2CreateReq>(),
        );

        req.extend_from_slice(header_bytes);
        req.extend_from_slice(body_bytes);
        req.extend_from_slice(name_bytes);

        let len = (req.len() - 4) as u32;
        req[3] = (len & 0xFF) as u8;
        req[2] = ((len >> 8) & 0xFF) as u8;
        req[1] = ((len >> 16) & 0xFF) as u8;

        core::mem::transmute::<_, FnSend>(send_fn)(sock, req.as_ptr(), req.len() as i32, 0);

        let mut buf = [0u8; 1024];
        let n = core::mem::transmute::<_, FnRecv>(recv_fn)(sock, buf.as_mut_ptr(), 1024, 0);
        if n > 64 && self.check_status(&buf[..n as usize]) {
            // Parse File ID from body (offset 64 + ...?)
            // Body starts at 64.
            // Oplock (1) + Flags (1) + CreateAction (4) + CreationTime (8) + LastAccess (8) + LastWrite (8) + Change (8) + AllocSize (8) + End (8) + FileAttributes (4) + Reserved (4) + FileId (16)
            // 1+1+4+8*5+4+4 = 62 bytes offset in body.
            // 64 + 62? No.
            // Structure size is 89.
            // FileId is at offset 64 + 64 (start of FileId in response).
            // Response structure is different from Request.
            // SMB2 CREATE Response:
            // StructureSize (2), Oplock (1), Flags (1), CreateAction (4), Times (32), Alloc (8), End (8), FileAttrs (4), Reserved2 (4), FileId (16).
            // 2+1+1+4+32+8+8+4+4 = 64 bytes.
            // So FileId is at 64 (Header) + 64 = 128?
            // Let's assume offset 128.
            if n >= 144 {
                let mut file_id = [0u8; 16];
                let ptr = buf.as_ptr().add(128);
                core::ptr::copy_nonoverlapping(ptr, file_id.as_mut_ptr(), 16);
                self.message_id += 1;
                return Ok(file_id);
            }
        }
        Err(())
    }

    #[cfg(target_os = "windows")]
    unsafe fn read_win(
        &mut self,
        sock: crate::utils::windows_definitions::HANDLE,
        file_id: &[u8; 16],
    ) -> Result<Vec<u8>, ()> {
        use crate::utils::api_resolver::{hash_str, resolve_function};
        let ws2_32 = hash_str(b"ws2_32.dll");
        let send_fn = resolve_function(ws2_32, hash_str(b"send"));
        let recv_fn = resolve_function(ws2_32, hash_str(b"recv"));
        type FnSend = unsafe extern "system" fn(
            crate::utils::windows_definitions::HANDLE,
            *const u8,
            i32,
            i32,
        ) -> i32;
        type FnRecv = unsafe extern "system" fn(
            crate::utils::windows_definitions::HANDLE,
            *mut u8,
            i32,
            i32,
        ) -> i32;

        let mut req = Vec::new();
        req.extend_from_slice(&[0, 0, 0, 0]);
        let mut header = SMB2Header::new(SMB2_READ, self.message_id, self.process_id);
        header.session_id = self.session_id;
        header.tree_id = self.tree_id;

        let body = SMB2ReadReq {
            structure_size: 49,
            padding: 0,
            flags: 0,
            length: 4096, // Read 4k
            offset: 0,
            file_id: *file_id,
            minimum_count: 0,
            channel: 0,
            remaining_bytes: 0,
            read_channel_info_offset: 0,
            read_channel_info_length: 0,
            buffer: 0,
        };

        let header_bytes = core::slice::from_raw_parts(
            &header as *const _ as *const u8,
            core::mem::size_of::<SMB2Header>(),
        );
        let body_bytes = core::slice::from_raw_parts(
            &body as *const _ as *const u8,
            core::mem::size_of::<SMB2ReadReq>(),
        );

        req.extend_from_slice(header_bytes);
        req.extend_from_slice(body_bytes);

        let len = (req.len() - 4) as u32;
        req[3] = (len & 0xFF) as u8;
        req[2] = ((len >> 8) & 0xFF) as u8;
        req[1] = ((len >> 16) & 0xFF) as u8;

        core::mem::transmute::<_, FnSend>(send_fn)(sock, req.as_ptr(), req.len() as i32, 0);

        let mut buf = [0u8; 4096];
        let n = core::mem::transmute::<_, FnRecv>(recv_fn)(sock, buf.as_mut_ptr(), 4096, 0);
        if n > 80 && self.check_status(&buf[..n as usize]) {
            // Read response: Header (64) + Body (16) + Data
            // Body: StructureSize (2), DataOffset (1), Reserved (1), DataLength (4), DataRemaining (4), Reserved2 (4)
            // DataOffset is usually 80.
            let data_offset = buf[66] as usize;
            let data_len_ptr = buf.as_ptr().add(68) as *const u32;
            let data_len = *data_len_ptr as usize;

            if data_offset + data_len <= n as usize {
                let mut data = Vec::with_capacity(data_len);
                let ptr = buf.as_ptr().add(data_offset);
                for i in 0..data_len {
                    data.push(*ptr.add(i));
                }
                self.message_id += 1;
                return Ok(data);
            }
        }
        Err(())
    }

    #[cfg(target_os = "windows")]
    unsafe fn close_win(
        &mut self,
        sock: crate::utils::windows_definitions::HANDLE,
        file_id: &[u8; 16],
    ) -> Result<(), ()> {
        use crate::utils::api_resolver::{hash_str, resolve_function};
        let ws2_32 = hash_str(b"ws2_32.dll");
        let send_fn = resolve_function(ws2_32, hash_str(b"send"));
        type FnSend = unsafe extern "system" fn(
            crate::utils::windows_definitions::HANDLE,
            *const u8,
            i32,
            i32,
        ) -> i32;

        let mut req = Vec::new();
        req.extend_from_slice(&[0, 0, 0, 0]);
        let mut header = SMB2Header::new(SMB2_CLOSE, self.message_id, self.process_id);
        header.session_id = self.session_id;
        header.tree_id = self.tree_id;

        let body = SMB2CloseReq {
            structure_size: 24,
            flags: 0,
            reserved: 0,
            file_id: *file_id,
        };

        let header_bytes = core::slice::from_raw_parts(
            &header as *const _ as *const u8,
            core::mem::size_of::<SMB2Header>(),
        );
        let body_bytes = core::slice::from_raw_parts(
            &body as *const _ as *const u8,
            core::mem::size_of::<SMB2CloseReq>(),
        );

        req.extend_from_slice(header_bytes);
        req.extend_from_slice(body_bytes);

        let len = (req.len() - 4) as u32;
        req[3] = (len & 0xFF) as u8;
        req[2] = ((len >> 8) & 0xFF) as u8;
        req[1] = ((len >> 16) & 0xFF) as u8;

        core::mem::transmute::<_, FnSend>(send_fn)(sock, req.as_ptr(), req.len() as i32, 0);
        self.message_id += 1;
        // Don't wait for response
        Ok(())
    }

    #[cfg(target_os = "windows")]
    pub fn read_file_win(&mut self, sock: crate::utils::windows_definitions::HANDLE, path: &str) -> Result<Vec<u8>, ()> {
        unsafe {
            let file_id = self.create_win(sock, path)?;
            let data = self.read_win(sock, &file_id)?;
            let _ = self.close_win(sock, &file_id);
            Ok(data)
        }
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

        let header_bytes = core::slice::from_raw_parts(
            &header as *const _ as *const u8,
            core::mem::size_of::<SMB2Header>(),
        );
        let body_bytes = core::slice::from_raw_parts(
            &body as *const _ as *const u8,
            core::mem::size_of::<SMB2NegotiateReq>(),
        );

        req.extend_from_slice(header_bytes);
        req.extend_from_slice(body_bytes);

        // Fix NetBIOS length (Big Endian)
        let len = (req.len() - 4) as u32;
        req[3] = (len & 0xFF) as u8;
        req[2] = ((len >> 8) & 0xFF) as u8;
        req[1] = ((len >> 16) & 0xFF) as u8;

        sys_write(fd, req.as_ptr(), req.len());

        // Read response
        let mut buf = [0u8; 1024];
        let n = sys_read(fd, buf.as_mut_ptr(), 1024);
        if n > 0 && self.check_status(&buf[..n as usize]) {
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
            security_buffer_length: 0,  // Anonymous
            previous_session_id: 0,
        };

        let header_bytes = core::slice::from_raw_parts(
            &header as *const _ as *const u8,
            core::mem::size_of::<SMB2Header>(),
        );
        let body_bytes = core::slice::from_raw_parts(
            &body as *const _ as *const u8,
            core::mem::size_of::<SMB2SessionSetupReq>(),
        );

        req.extend_from_slice(header_bytes);
        req.extend_from_slice(body_bytes);

        let len = (req.len() - 4) as u32;
        req[3] = (len & 0xFF) as u8;
        req[2] = ((len >> 8) & 0xFF) as u8;
        req[1] = ((len >> 16) & 0xFF) as u8;

        sys_write(fd, req.as_ptr(), req.len());

        let mut buf = [0u8; 1024];
        let n = sys_read(fd, buf.as_mut_ptr(), 1024);
        if n > 64 && self.check_status(&buf[..n as usize]) {
            if buf[0] == 0 {
                // Session Message
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
        let path_bytes =
            core::slice::from_raw_parts(path_utf16.as_ptr() as *const u8, path_utf16.len() * 2);

        let body = SMB2TreeConnectReq {
            structure_size: 9,
            reserved: 0,
            path_offset: 72, // 64 + 8
            path_length: path_bytes.len() as u16,
        };

        let header_bytes = core::slice::from_raw_parts(
            &header as *const _ as *const u8,
            core::mem::size_of::<SMB2Header>(),
        );
        let body_bytes = core::slice::from_raw_parts(
            &body as *const _ as *const u8,
            core::mem::size_of::<SMB2TreeConnectReq>(),
        );

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
        if n > 64 && self.check_status(&buf[..n as usize]) {
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

#[cfg(test)]
mod tests {
    use super::*;
    use core::mem::size_of;

    #[test]
    fn test_smb_struct_sizes() {
        assert_eq!(size_of::<SMB2Header>(), 64);
    }

    #[test]
    fn test_smb_client_init() {
        let client = SmbClient::new();
        assert_eq!(client.process_id, 0xFEFF);
    }
}
