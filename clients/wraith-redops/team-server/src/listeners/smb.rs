use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::broadcast;
use crate::wraith::redops::Event;
use crate::governance::GovernanceEngine;
use wraith_crypto::noise::NoiseKeypair;
use crate::services::session::SessionManager;
use crate::database::Database;
use crate::services::protocol::ProtocolHandler;

pub async fn start_smb_listener(
    db: Arc<Database>,
    port: u16,
    event_tx: broadcast::Sender<Event>,
    governance: Arc<GovernanceEngine>,
    static_key: NoiseKeypair,
    session_manager: Arc<SessionManager>
) {
    let addr = format!("0.0.0.0:{}", port);
    tracing::info!("SMB (TCP Simulation) Listener starting on {}", addr);
    
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
                tracing::info!("New SMB connection from {}", src);
                
                let protocol = protocol.clone();
                tokio::spawn(async move {
                    let mut buf = [0u8; 4096];
                    loop {
                        // Simple echo/handler for MVP
                        // In real SMB, this is complex.
                        use tokio::io::{AsyncReadExt, AsyncWriteExt};
                        match socket.read(&mut buf).await {
                            Ok(0) => break,
                            Ok(n) => {
                                // Protocol handle
                                if let Some(resp) = protocol.handle_packet(&buf[..n], src.to_string()).await {
                                    if let Err(e) = socket.write_all(&resp).await {
                                        tracing::error!("SMB write error: {}", e);
                                        break;
                                    }
                                }
                            }
                            Err(_) => break,
                        }
                    }
                });
            }
            Err(e) => tracing::error!("SMB Accept error: {}", e),
        }
    }
}
