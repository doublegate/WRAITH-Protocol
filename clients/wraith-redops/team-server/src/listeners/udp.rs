use crate::services::session::SessionManager;
use std::sync::Arc;
use tokio::sync::broadcast;
use crate::wraith::redops::Event;
use crate::governance::GovernanceEngine;
use wraith_crypto::noise::NoiseKeypair;

pub async fn start_udp_listener(
    port: u16,
    _event_tx: broadcast::Sender<Event>,
    governance: Arc<GovernanceEngine>,
    _static_key: NoiseKeypair,
    _sessions: Arc<SessionManager>
) {
    let addr = format!("0.0.0.0:{}", port);
    tracing::info!("UDP Listener starting on {}", addr);
    
    let socket = match tokio::net::UdpSocket::bind(&addr).await {
        Ok(s) => s,
        Err(e) => {
            tracing::error!("Failed to bind UDP socket: {}", e);
            return;
        }
    };

    let mut buf = [0u8; 65535];

    loop {
        match socket.recv_from(&mut buf).await {
            Ok((_len, src)) => {
                // Governance
                if !governance.validate_action(src.ip()) {
                    continue;
                }
                
                // For MVP, we just log reception. 
                // Full integration requires sharing the `handle_packet` logic from http.rs (refactoring common logic).
                tracing::debug!("Received UDP packet from {}", src);
            }
            Err(e) => tracing::error!("UDP Recv error: {}", e),
        }
    }
}
