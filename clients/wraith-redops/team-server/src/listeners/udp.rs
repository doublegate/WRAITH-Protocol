use crate::database::Database;
use crate::governance::GovernanceEngine;
use crate::services::protocol::ProtocolHandler;
use crate::services::session::SessionManager;
use crate::wraith::redops::Event;
use std::sync::Arc;
use tokio::sync::broadcast;
use wraith_crypto::noise::NoiseKeypair;

pub async fn start_udp_listener(
    db: Arc<Database>,
    port: u16,
    event_tx: broadcast::Sender<Event>,
    governance: Arc<GovernanceEngine>,
    static_key: NoiseKeypair,
    session_manager: Arc<SessionManager>,
) {
    let addr = format!("0.0.0.0:{}", port);
    tracing::info!("UDP Listener starting on {}", addr);

    let socket = match tokio::net::UdpSocket::bind(&addr).await {
        Ok(s) => Arc::new(s),
        Err(e) => {
            tracing::error!("Failed to bind UDP socket: {}", e);
            return;
        }
    };

    let protocol = ProtocolHandler::new(db, session_manager, Arc::new(static_key), event_tx);

    let mut buf = [0u8; 65535];

    loop {
        match socket.recv_from(&mut buf).await {
            Ok((len, src)) => {
                // Governance
                if !governance.validate_action(src.ip()) {
                    continue;
                }

                let data = buf[..len].to_vec();
                let protocol = protocol.clone();
                let socket = socket.clone();

                // Spawn a task to handle the packet to avoid blocking the loop
                tokio::spawn(async move {
                    if let Some(resp) = protocol.handle_packet(&data, src.to_string()).await
                        && let Err(e) = socket.send_to(&resp, src).await
                    {
                        tracing::error!("Failed to send UDP response: {}", e);
                    }
                });
            }
            Err(e) => tracing::error!("UDP Recv error: {}", e),
        }
    }
}
