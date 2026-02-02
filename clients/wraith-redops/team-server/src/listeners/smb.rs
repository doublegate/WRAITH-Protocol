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

#[derive(Clone, Copy)]
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

    fn to_bytes(self) -> [u8; 64] {
        let mut buf = [0u8; 64];
        buf[0..4].copy_from_slice(&self.protocol_id);
        buf[4..6].copy_from_slice(&self.structure_size.to_le_bytes());
        buf[6..8].copy_from_slice(&self.credit_charge.to_le_bytes());
        buf[8..12].copy_from_slice(&self.status.to_le_bytes());
        buf[12..14].copy_from_slice(&self.command.to_le_bytes());
        buf[14..16].copy_from_slice(&self.credits.to_le_bytes());
        buf[16..20].copy_from_slice(&self.flags.to_le_bytes());
        buf[20..24].copy_from_slice(&self.next_command.to_le_bytes());
        buf[24..32].copy_from_slice(&self.message_id.to_le_bytes());
        buf[32..36].copy_from_slice(&self.process_id.to_le_bytes());
        buf[36..40].copy_from_slice(&self.tree_id.to_le_bytes());
        buf[40..48].copy_from_slice(&self.session_id.to_le_bytes());
        buf[48..64].copy_from_slice(&self.signature);
        buf
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
                        if socket.read_exact(&mut netbios_header).await.is_err() {
                            break;
                        }

                        let len = u32::from_be_bytes([
                            0,
                            netbios_header[1],
                            netbios_header[2],
                            netbios_header[3],
                        ]) as usize;
                        if len > buf.len() {
                            tracing::error!("SMB packet too large: {}", len);
                            break;
                        }

                        // 2. Read SMB2 Packet
                        if socket.read_exact(&mut buf[..len]).await.is_err() {
                            break;
                        }

                        let header_buf = &buf[..64];
                        if header_buf.len() < 64 {
                            break;
                        }

                        // Verify magic
                        if header_buf[0..4] != [0xFE, b'S', b'M', b'B'] {
                            tracing::error!("Invalid SMB2 magic from {}", src);
                            break;
                        }

                        let command = u16::from_le_bytes([header_buf[12], header_buf[13]]);
                        let msg_id =
                            u64::from_le_bytes(header_buf[24..32].try_into().unwrap_or_default());
                        let proc_id =
                            u32::from_le_bytes(header_buf[32..36].try_into().unwrap_or_default());
                        let session_id =
                            u64::from_le_bytes(header_buf[40..48].try_into().unwrap_or_default());

                        match command {
                            0x0000 => {
                                // Negotiate
                                // Construct Negotiate Response
                                let mut resp = Vec::new();
                                let mut h = Smb2Header::new();
                                h.command = 0x0000;
                                h.flags = 0x00000001; // Response
                                h.message_id = msg_id;
                                h.process_id = proc_id;
                                h.credits = 1; // Credits granted
                                let h_bytes = h.to_bytes();
                                resp.extend_from_slice(&h_bytes);

                                // Body (Simplified 2.0.2)
                                // StructureSize(2) + SecurityMode(2) + DialectRevision(2) + Reserved(2) + Guid(16) + Caps(4) + Max(4*3) + Time(8*2) + SecurityBuffer(4)
                                let body = [
                                    65, 0, // StructureSize
                                    1, 0, // SecurityMode
                                    0x02, 0x02, // Dialect
                                    0, 0, // Reserved
                                    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, // Guid
                                    0, 0, 0, 0, // Caps
                                    0, 0, 0, 1, // MaxTrans
                                    0, 0, 0, 1, // MaxRead
                                    0, 0, 0, 1, // MaxWrite
                                    0, 0, 0, 0, 0, 0, 0, 0, // SystemTime
                                    0, 0, 0, 0, 0, 0, 0, 0, // BootTime
                                    0, 0, 0, 0, // SecurityBuffer Offset/Len
                                ];
                                resp.extend_from_slice(&body);
                                send_netbios(&mut socket, &resp).await;
                            }
                            0x0001 => {
                                // Session Setup
                                // Accept any setup
                                let mut resp = Vec::new();
                                let mut h = Smb2Header::new();
                                h.command = 0x0001;
                                h.flags = 0x00000001;
                                h.message_id = msg_id;
                                h.process_id = proc_id;
                                h.session_id = 0x10000001; // Assign session ID
                                let h_bytes = h.to_bytes();
                                resp.extend_from_slice(&h_bytes);

                                let body = [9, 0, 0, 0]; // StructureSize 9, Flags 0
                                resp.extend_from_slice(&body);
                                send_netbios(&mut socket, &resp).await;
                            }
                            0x0003 => {
                                // Tree Connect
                                let mut resp = Vec::new();
                                let mut h = Smb2Header::new();
                                h.command = 0x0003;
                                h.flags = 0x00000001;
                                h.message_id = msg_id;
                                h.process_id = proc_id;
                                h.session_id = session_id;
                                h.tree_id = 0x20000001; // Assign tree ID
                                let h_bytes = h.to_bytes();
                                resp.extend_from_slice(&h_bytes);

                                let body = [16, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]; // StructureSize 16
                                resp.extend_from_slice(&body);
                                send_netbios(&mut socket, &resp).await;
                            }
                            0x0009 => {
                                // Write (Data from Implant)
                                // Parse Write Request Body
                                // Header(64) + Body. Body starts at 64.
                                if len < 64 + 48 {
                                    continue;
                                } // Min body size
                                let body = &buf[64..];
                                // Offset is at 16 (u16), Length is at 20 (u32)
                                let offset = u16::from_le_bytes([body[16], body[17]]) as usize;
                                let length =
                                    u32::from_le_bytes([body[20], body[21], body[22], body[23]])
                                        as usize;

                                // Offset is from header start.
                                // Typically offset >= 64 + 48.
                                if offset + length <= len {
                                    let payload = &buf[offset..offset + length];

                                    // Handle WRAITH packet
                                    if let Some(response_data) =
                                        protocol.handle_packet(payload, src.to_string()).await
                                    {
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
                                        let h_bytes = h.to_bytes();
                                        resp.extend_from_slice(&h_bytes);

                                        // Write Response Body (17 bytes)
                                        let mut body = vec![17, 0, 0, 0];
                                        body.extend_from_slice(&(length as u32).to_le_bytes()); // Count written
                                        body.extend_from_slice(&[0; 9]);
                                        resp.extend_from_slice(&body);
                                        send_netbios(&mut socket, &resp).await;
                                    }
                                }
                            }
                            0x0008 => {
                                // Read
                                // Parse Read Request (Length is at offset 4 in body)
                                if len < 64 + 4 {
                                    continue;
                                }
                                let req_len =
                                    u32::from_le_bytes([buf[68], buf[69], buf[70], buf[71]])
                                        as usize;

                                let take_len = std::cmp::min(req_len, pending_data.len());
                                let data_to_send: Vec<u8> =
                                    pending_data.drain(0..take_len).collect();

                                let mut resp = Vec::new();
                                let mut h = Smb2Header::new();
                                h.command = 0x0008;
                                h.flags = 0x00000001;
                                h.message_id = msg_id;
                                h.process_id = proc_id;
                                h.session_id = session_id;
                                let h_bytes = h.to_bytes();
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
                            }
                            _ => {}
                        }
                    }
                });
            }
            Err(e) => tracing::error!("SMB Accept error: {}", e),
        }
    }
}

#[cfg(test)]
fn build_netbios_header(data_len: usize) -> [u8; 4] {
    let len = data_len as u32;
    let mut header = [0u8; 4];
    header[3] = (len & 0xFF) as u8;
    header[2] = ((len >> 8) & 0xFF) as u8;
    header[1] = ((len >> 16) & 0xFF) as u8;
    header
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_smb2_header_new_defaults() {
        let h = Smb2Header::new();
        assert_eq!(h.protocol_id, [0xFE, b'S', b'M', b'B']);
        assert_eq!(h.structure_size, 64);
        assert_eq!(h.command, 0x09); // WRITE
        assert_eq!(h.status, 0);
        assert_eq!(h.credits, 0);
        assert_eq!(h.flags, 0);
        assert_eq!(h.next_command, 0);
        assert_eq!(h.message_id, 0);
        assert_eq!(h.process_id, 0);
        assert_eq!(h.tree_id, 0);
        assert_eq!(h.session_id, 0);
        assert_eq!(h.signature, [0u8; 16]);
    }

    #[test]
    fn test_smb2_header_to_bytes_length() {
        let h = Smb2Header::new();
        let bytes = h.to_bytes();
        assert_eq!(bytes.len(), 64);
    }

    #[test]
    fn test_smb2_header_to_bytes_magic() {
        let h = Smb2Header::new();
        let bytes = h.to_bytes();
        assert_eq!(&bytes[0..4], &[0xFE, b'S', b'M', b'B']);
    }

    #[test]
    fn test_smb2_header_to_bytes_structure_size() {
        let h = Smb2Header::new();
        let bytes = h.to_bytes();
        assert_eq!(u16::from_le_bytes([bytes[4], bytes[5]]), 64);
    }

    #[test]
    fn test_smb2_header_to_bytes_command() {
        let mut h = Smb2Header::new();
        h.command = 0x0001; // Session Setup
        let bytes = h.to_bytes();
        assert_eq!(u16::from_le_bytes([bytes[12], bytes[13]]), 0x0001);
    }

    #[test]
    fn test_smb2_header_to_bytes_flags() {
        let mut h = Smb2Header::new();
        h.flags = 0x00000001; // Response
        let bytes = h.to_bytes();
        assert_eq!(
            u32::from_le_bytes([bytes[16], bytes[17], bytes[18], bytes[19]]),
            0x00000001
        );
    }

    #[test]
    fn test_smb2_header_to_bytes_message_id() {
        let mut h = Smb2Header::new();
        h.message_id = 42;
        let bytes = h.to_bytes();
        assert_eq!(u64::from_le_bytes(bytes[24..32].try_into().unwrap()), 42);
    }

    #[test]
    fn test_smb2_header_to_bytes_session_id() {
        let mut h = Smb2Header::new();
        h.session_id = 0x10000001;
        let bytes = h.to_bytes();
        assert_eq!(
            u64::from_le_bytes(bytes[40..48].try_into().unwrap()),
            0x10000001
        );
    }

    #[test]
    fn test_smb2_header_to_bytes_tree_id() {
        let mut h = Smb2Header::new();
        h.tree_id = 0x20000001;
        let bytes = h.to_bytes();
        assert_eq!(
            u32::from_le_bytes(bytes[36..40].try_into().unwrap()),
            0x20000001
        );
    }

    #[test]
    fn test_smb2_header_to_bytes_process_id() {
        let mut h = Smb2Header::new();
        h.process_id = 1234;
        let bytes = h.to_bytes();
        assert_eq!(u32::from_le_bytes(bytes[32..36].try_into().unwrap()), 1234);
    }

    #[test]
    fn test_smb2_header_to_bytes_signature() {
        let mut h = Smb2Header::new();
        h.signature = [0xAA; 16];
        let bytes = h.to_bytes();
        assert_eq!(&bytes[48..64], &[0xAA; 16]);
    }

    #[test]
    fn test_smb2_header_roundtrip() {
        let mut h = Smb2Header::new();
        h.command = 0x0000; // Negotiate
        h.flags = 0x00000001;
        h.message_id = 100;
        h.process_id = 200;
        h.session_id = 300;
        h.tree_id = 400;
        h.credits = 5;
        h.credit_charge = 1;

        let bytes = h.to_bytes();

        // Verify all fields can be read back
        assert_eq!(&bytes[0..4], &[0xFE, b'S', b'M', b'B']);
        assert_eq!(u16::from_le_bytes([bytes[4], bytes[5]]), 64);
        assert_eq!(u16::from_le_bytes([bytes[6], bytes[7]]), 1); // credit_charge
        assert_eq!(
            u32::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]),
            0
        ); // status
        assert_eq!(u16::from_le_bytes([bytes[12], bytes[13]]), 0x0000); // command
        assert_eq!(u16::from_le_bytes([bytes[14], bytes[15]]), 5); // credits
        assert_eq!(
            u32::from_le_bytes([bytes[16], bytes[17], bytes[18], bytes[19]]),
            1
        ); // flags
        assert_eq!(u64::from_le_bytes(bytes[24..32].try_into().unwrap()), 100); // message_id
        assert_eq!(u32::from_le_bytes(bytes[32..36].try_into().unwrap()), 200); // process_id
        assert_eq!(u32::from_le_bytes(bytes[36..40].try_into().unwrap()), 400); // tree_id
        assert_eq!(u64::from_le_bytes(bytes[40..48].try_into().unwrap()), 300); // session_id
    }

    #[test]
    fn test_smb2_header_negotiate_response() {
        let mut h = Smb2Header::new();
        h.command = 0x0000;
        h.flags = 0x00000001;
        h.credits = 1;
        let bytes = h.to_bytes();

        // Verify it looks like a valid SMB2 negotiate response header
        assert_eq!(&bytes[0..4], &[0xFE, b'S', b'M', b'B']);
        assert_eq!(u16::from_le_bytes([bytes[12], bytes[13]]), 0x0000);
        assert_eq!(
            u32::from_le_bytes([bytes[16], bytes[17], bytes[18], bytes[19]]),
            1
        ); // Response flag
    }

    #[test]
    fn test_smb2_header_copy() {
        let h = Smb2Header::new();
        let h2 = h; // Copy trait
        assert_eq!(h.protocol_id, h2.protocol_id);
        assert_eq!(h.command, h2.command);
    }

    #[test]
    fn test_build_netbios_header_small() {
        let header = build_netbios_header(100);
        assert_eq!(header[0], 0); // Session message type
        assert_eq!(header[1], 0);
        assert_eq!(header[2], 0);
        assert_eq!(header[3], 100);
    }

    #[test]
    fn test_build_netbios_header_large() {
        let header = build_netbios_header(0x010203);
        assert_eq!(header[0], 0);
        assert_eq!(header[1], 0x01);
        assert_eq!(header[2], 0x02);
        assert_eq!(header[3], 0x03);
    }

    #[test]
    fn test_build_netbios_header_zero() {
        let header = build_netbios_header(0);
        assert_eq!(header, [0, 0, 0, 0]);
    }

    #[test]
    fn test_smb2_header_status_field() {
        let mut h = Smb2Header::new();
        h.status = 0xC0000022; // STATUS_ACCESS_DENIED
        let bytes = h.to_bytes();
        assert_eq!(
            u32::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]),
            0xC0000022
        );
    }
}
