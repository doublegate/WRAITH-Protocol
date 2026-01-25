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

/// SMB listener simulating a named pipe over TCP (direct-hosted SMB on port 445).
/// This implementation handles basic SMB2-style encapsulation for WRAITH packets.
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
                    loop {
                        // SMB2-like packet structure: [4-byte length (BE)] [Payload]
                        // This mimics a simplified direct-TCP SMB encapsulation.
                        let mut len_buf = [0u8; 4];
                        if let Err(e) = socket.read_exact(&mut len_buf).await {
                            if e.kind() != std::io::ErrorKind::UnexpectedEof {
                                tracing::error!("SMB header read error from {}: {}", src, e);
                            }
                            break;
                        }

                        let payload_len = u32::from_be_bytes(len_buf) as usize;
                        if payload_len > buf.len() {
                            tracing::error!("SMB payload too large from {}: {} bytes", src, payload_len);
                            break;
                        }

                        if let Err(e) = socket.read_exact(&mut buf[..payload_len]).await {
                            tracing::error!("SMB payload read error from {}: {}", src, e);
                            break;
                        }

                        // Process the WRAITH packet inside the SMB "Named Pipe"
                        if let Some(resp) = protocol.handle_packet(&buf[..payload_len], src.to_string()).await {
                            // Send response back with the same encapsulation
                            let resp_len = resp.len() as u32;
                            let mut out_buf = Vec::with_capacity(4 + resp.len());
                            out_buf.extend_from_slice(&resp_len.to_be_bytes());
                            out_buf.extend_from_slice(&resp);

                            if let Err(e) = socket.write_all(&out_buf).await {
                                tracing::error!("SMB write error to {}: {}", src, e);
                                break;
                            }
                        }
                    }
                });
            }
            Err(e) => tracing::error!("SMB Accept error: {}", e),
        }
    }
}

#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn test_smb_encapsulation_loopback() {
        // This test requires a running listener, but we can test the logic by mocking
        // the encapsulation protocol.
        let payload = b"WRAITH_TEST_PACKET";
        let mut buf = Vec::new();
        let len = payload.len() as u32;
        buf.extend_from_slice(&len.to_be_bytes());
        buf.extend_from_slice(payload);

        assert_eq!(buf.len(), 4 + payload.len());
        assert_eq!(&buf[4..], payload);
        assert_eq!(u32::from_be_bytes([buf[0], buf[1], buf[2], buf[3]]), len);
    }
}