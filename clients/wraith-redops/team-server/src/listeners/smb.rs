use crate::database::Database;
use crate::governance::GovernanceEngine;
use crate::services::protocol::ProtocolHandler;
use crate::services::session::SessionManager;
use crate::wraith::redops::Event;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::broadcast;
use wraith_crypto::noise::NoiseKeypair;

#[repr(C, packed)]
struct Smb2Header {
    protocol_id: [u8; 4], // \xFE SMB
    structure_size: u16,  // 64
    credit_charge: u16,
    status: u32,
    command: u16,
    credits: u16,
    flags: u32,
    next_command: u32,
    message_id: u64,
    process_id: u32,
    tree_id: u32,
    session_id: u64,
    signature: [u8; 16],
}

impl Smb2Header {
    fn new() -> Self {
        Self {
            protocol_id: [0xFE, b'S', b'M', b'B'],
            structure_size: 64,
            credit_charge: 0,
            status: 0,
            command: 0x09, // WRITE
            credits: 0,
            flags: 0,
            next_command: 0,
            message_id: 0,
            process_id: 0,
            tree_id: 0,
            session_id: 0,
            signature: [0u8; 16],
        }
    }
}

/// SMB listener simulating a named pipe over TCP (direct-hosted SMB on port 445).
/// This implementation handles full SMB2 protocol headers for WRAITH packets.
pub async fn start_smb_listener(
    db: Arc<Database>,
    port: u16,
    event_tx: broadcast::Sender<Event>,
    governance: Arc<GovernanceEngine>,
    static_key: NoiseKeypair,
    session_manager: Arc<SessionManager>,
) {
    let addr = format!("0.0.0.0:{}", port);
    tracing::info!("SMB Named Pipe Listener starting on {}", addr);

    let listener = match TcpListener::bind(&addr).await {
        Ok(l) => l,
        Err(e) => {
            tracing::error!("Failed to bind SMB socket: {}", e);
            return;
        }
    };

    let protocol = ProtocolHandler::new(db, session_manager, Arc::new(static_key), event_tx);

    loop {
        match listener.accept().await {
            Ok((mut socket, src)) => {
                if !governance.validate_action(src.ip()) {
                    continue;
                }
                tracing::info!("New SMB/WRAITH connection from {}", src);

                let protocol = protocol.clone();
                tokio::spawn(async move {
                    let mut buf = [0u8; 8192];
                    let mut pending_data = Vec::new();
                    loop {
                        // 1. Read NetBIOS Header (4 bytes)
                        let mut netbios_header = [0u8; 4];
                        if let Err(_) = socket.read_exact(&mut netbios_header).await {
                            break;
                        }
                        
                        let len = u32::from_be_bytes([0, netbios_header[1], netbios_header[2], netbios_header[3]]) as usize;
                        if len > buf.len() {
                            tracing::error!("SMB packet too large: {}", len);
                            break;
                        }

                        // 2. Read SMB2 Packet
                        if let Err(_) = socket.read_exact(&mut buf[..len]).await {
                            break;
                        }
                        
                        let header_buf = &buf[..64];
                        if header_buf.len() < 64 { break; }

                        // Verify magic
                        if &header_buf[0..4] != &[0xFE, b'S', b'M', b'B'] {
                            tracing::error!("Invalid SMB2 magic from {}", src);
                            break;
                        }

                        let command = u16::from_le_bytes([header_buf[12], header_buf[13]]);
                        let msg_id = u64::from_le_bytes(header_buf[24..32].try_into().unwrap());
                        let proc_id = u32::from_le_bytes(header_buf[32..36].try_into().unwrap());
                        let session_id = u64::from_le_bytes(header_buf[40..48].try_into().unwrap());

                        match command {
                            0x0000 => { // Negotiate
                                // Construct Negotiate Response
                                let mut resp = Vec::new();
                                let mut h = Smb2Header::new();
                                h.command = 0x0000;
                                h.flags = 0x00000001; // Response
                                h.message_id = msg_id;
                                h.process_id = proc_id;
                                h.credits = 1; // Credits granted
                                let h_bytes: [u8; 64] = unsafe { core::mem::transmute(h) };
                                resp.extend_from_slice(&h_bytes);
                                
                                // Body (Simplified 2.0.2)
                                // StructureSize(2) + SecurityMode(2) + DialectRevision(2) + Reserved(2) + Guid(16) + Caps(4) + Max(4*3) + Time(8*2) + SecurityBuffer(4)
                                let body = [
                                    65, 0, // StructureSize
                                    1, 0, // SecurityMode
                                    0x02, 0x02, // Dialect
                                    0, 0, // Reserved
                                    0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0, // Guid
                                    0, 0, 0, 0, // Caps
                                    0, 0, 0, 1, // MaxTrans
                                    0, 0, 0, 1, // MaxRead
                                    0, 0, 0, 1, // MaxWrite
                                    0,0,0,0,0,0,0,0, // SystemTime
                                    0,0,0,0,0,0,0,0, // BootTime
                                    0,0, 0,0 // SecurityBuffer Offset/Len
                                ];
                                resp.extend_from_slice(&body);
                                send_netbios(&mut socket, &resp).await;
                            },
                            0x0001 => { // Session Setup
                                // Accept any setup
                                let mut resp = Vec::new();
                                let mut h = Smb2Header::new();
                                h.command = 0x0001;
                                h.flags = 0x00000001;
                                h.message_id = msg_id;
                                h.process_id = proc_id;
                                h.session_id = 0x10000001; // Assign session ID
                                let h_bytes: [u8; 64] = unsafe { core::mem::transmute(h) };
                                resp.extend_from_slice(&h_bytes);
                                
                                let body = [9, 0, 0, 0]; // StructureSize 9, Flags 0
                                resp.extend_from_slice(&body);
                                send_netbios(&mut socket, &resp).await;
                            },
                            0x0003 => { // Tree Connect
                                let mut resp = Vec::new();
                                let mut h = Smb2Header::new();
                                h.command = 0x0003;
                                h.flags = 0x00000001;
                                h.message_id = msg_id;
                                h.process_id = proc_id;
                                h.session_id = session_id;
                                h.tree_id = 0x20000001; // Assign tree ID
                                let h_bytes: [u8; 64] = unsafe { core::mem::transmute(h) };
                                resp.extend_from_slice(&h_bytes);
                                
                                let body = [16, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]; // StructureSize 16
                                resp.extend_from_slice(&body);
                                send_netbios(&mut socket, &resp).await;
                            },
                            0x0009 => { // Write (Data from Implant)
                                // Parse Write Request Body
                                // Header(64) + Body. Body starts at 64.
                                if len < 64 + 48 { continue; } // Min body size
                                let body = &buf[64..];
                                // Offset is at 16 (u16), Length is at 20 (u32)
                                let offset = u16::from_le_bytes([body[16], body[17]]) as usize;
                                let length = u32::from_le_bytes([body[20], body[21], body[22], body[23]]) as usize;
                                
                                // Offset is from header start.
                                // Typically offset >= 64 + 48.
                                if offset + length <= len {
                                    let payload = &buf[offset..offset+length];
                                    
                                    // Handle WRAITH packet
                                    if let Some(response_data) = protocol.handle_packet(payload, src.to_string()).await {
                                        // Buffer response for subsequent READ
                                        pending_data.extend_from_slice(&response_data);

                                        // WRITE Response:
                                        let mut resp = Vec::new();
                                        let mut h = Smb2Header::new();
                                        h.command = 0x0009;
                                        h.flags = 0x00000001;
                                        h.message_id = msg_id;
                                        h.process_id = proc_id;
                                        h.session_id = session_id;
                                        let h_bytes: [u8; 64] = unsafe { core::mem::transmute(h) };
                                        resp.extend_from_slice(&h_bytes);
                                        
                                        // Write Response Body (17 bytes)
                                        let mut body = vec![17, 0, 0, 0];
                                        body.extend_from_slice(&(length as u32).to_le_bytes()); // Count written
                                        body.extend_from_slice(&[0; 9]);
                                        resp.extend_from_slice(&body);
                                        send_netbios(&mut socket, &resp).await;
                                    }
                                }
                            },
                            0x0008 => { // Read
                                // Parse Read Request (Length is at offset 4 in body)
                                if len < 64 + 4 { continue; }
                                let req_len = u32::from_le_bytes([buf[68], buf[69], buf[70], buf[71]]) as usize;
                                
                                let take_len = std::cmp::min(req_len, pending_data.len());
                                let data_to_send: Vec<u8> = pending_data.drain(0..take_len).collect();
                                
                                let mut resp = Vec::new();
                                let mut h = Smb2Header::new();
                                h.command = 0x0008;
                                h.flags = 0x00000001;
                                h.message_id = msg_id;
                                h.process_id = proc_id;
                                h.session_id = session_id;
                                let h_bytes: [u8; 64] = unsafe { core::mem::transmute(h) };
                                resp.extend_from_slice(&h_bytes);
                                
                                // Read Response Body (17 bytes + Data)
                                // DataOffset(1), Reserved(1), DataLength(4), DataRemaining(4), Reserved(4)
                                let data_offset = 80u8; // 64 header + 16 body
                                let mut body = vec![data_offset, 0];
                                body.extend_from_slice(&(data_to_send.len() as u32).to_le_bytes());
                                body.extend_from_slice(&(pending_data.len() as u32).to_le_bytes()); // Remaining
                                body.extend_from_slice(&[0; 4]);
                                
                                resp.extend_from_slice(&body);
                                resp.extend_from_slice(&data_to_send);
                                
                                send_netbios(&mut socket, &resp).await;
                            },
                            _ => {}
                        }
                    }
                });
            }
            Err(e) => tracing::error!("SMB Accept error: {}", e),
        }
    }
}

async fn send_netbios(socket: &mut tokio::net::TcpStream, data: &[u8]) {
    let len = data.len() as u32;
    let mut header = [0u8; 4];
    header[3] = (len & 0xFF) as u8;
    header[2] = ((len >> 8) & 0xFF) as u8;
    header[1] = ((len >> 16) & 0xFF) as u8;
    // header[0] is 0 (Session Message)
    
    let _ = socket.write_all(&header).await;
    let _ = socket.write_all(data).await;
}