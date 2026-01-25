use std::sync::Arc;
use tokio::net::UdpSocket;
use tokio::sync::broadcast;
use crate::wraith::redops::Event;
use crate::governance::GovernanceEngine;
use wraith_crypto::noise::NoiseKeypair;
use crate::services::session::SessionManager;

pub async fn start_dns_listener(
    port: u16,
    _event_tx: broadcast::Sender<Event>,
    governance: Arc<GovernanceEngine>,
    _static_key: NoiseKeypair,
    _sessions: Arc<SessionManager>
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

    let mut buf = [0u8; 1024];

    loop {
        match socket.recv_from(&mut buf).await {
            Ok((len, src)) => {
                if !governance.validate_action(src.ip()) {
                    continue;
                }
                
                // Minimal DNS parsing
                if len > 12 {
                    let domain = parse_dns_query(&buf[..len]);
                    
                    if !governance.validate_domain(&domain) {
                        continue;
                    }

                    tracing::debug!("Received DNS query for {} from {}", domain, src);
                }
            }
            Err(e) => tracing::error!("DNS Recv error: {}", e),
        }
    }
}

fn parse_dns_query(buf: &[u8]) -> String {
    // Skip header (12 bytes)
    let mut pos = 12;
    let mut domain = String::new();
    
    while pos < buf.len() {
        let len = buf[pos] as usize;
        if len == 0 { break; }
        pos += 1;
        if pos + len > buf.len() { break; }
        
        if !domain.is_empty() { domain.push('.'); }
        domain.push_str(&String::from_utf8_lossy(&buf[pos..pos+len]));
        pos += len;
    }
    domain
}