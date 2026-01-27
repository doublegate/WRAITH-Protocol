use crate::database::Database;
use crate::governance::GovernanceEngine;
use crate::services::protocol::ProtocolHandler;
use crate::services::session::SessionManager;
use crate::wraith::redops::Event;
use std::sync::Arc;
use tokio::net::UdpSocket;
use tokio::sync::broadcast;
use wraith_crypto::noise::NoiseKeypair;

pub async fn start_dns_listener(
    db: Arc<Database>,
    port: u16,
    event_tx: broadcast::Sender<Event>,
    governance: Arc<GovernanceEngine>,
    static_key: NoiseKeypair,
    session_manager: Arc<SessionManager>,
) {
    let addr = format!("0.0.0.0:{}", port);
    tracing::info!("DNS Listener starting on {}", addr);

    let socket = match UdpSocket::bind(&addr).await {
        Ok(s) => s,
        Err(e) => {
            tracing::error!("Failed to bind DNS socket: {}", e);
            return;
        }
    };

    let _protocol = ProtocolHandler::new(db, session_manager, Arc::new(static_key), event_tx);
    let mut buf = [0u8; 1024];

    loop {
        match socket.recv_from(&mut buf).await {
            Ok((len, src)) => {
                if !governance.validate_action(src.ip()) {
                    continue;
                }

                if len > 12 {
                    let domain = parse_dns_query(&buf[..len]);
                    if !governance.validate_domain(&domain) {
                        continue;
                    }
                    tracing::debug!("Received DNS query for {} from {}", domain, src);
                    // TODO: Hook up protocol handler for TXT records
                }
            }
            Err(e) => tracing::error!("DNS Recv error: {}", e),
        }
    }
}

fn parse_dns_query(buf: &[u8]) -> String {
    let mut pos = 12;
    let mut domain = String::new();
    while pos < buf.len() {
        let len = buf[pos] as usize;
        if len == 0 {
            break;
        }
        pos += 1;
        if pos + len > buf.len() {
            break;
        }
        if !domain.is_empty() {
            domain.push('.');
        }
        domain.push_str(&String::from_utf8_lossy(&buf[pos..pos + len]));
        pos += len;
    }
    domain
}
