use axum::{
    extract::{State, ConnectInfo},
    routing::post,
    response::IntoResponse,
    body::Bytes,
    Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::net::SocketAddr;
use crate::database::Database;
use tokio::sync::broadcast;
use crate::wraith::redops::Event;
use crate::governance::GovernanceEngine;
use wraith_crypto::noise::{NoiseKeypair, NoiseHandshake};
use crate::services::session::SessionManager;

#[derive(Debug, Deserialize, Serialize)]
pub struct BeaconData {
    pub id: String,
    pub hostname: String,
    pub username: String,
}

#[derive(Debug, Serialize)]
pub struct BeaconResponse {
    pub tasks: Vec<String>,
}

#[derive(Clone)]
pub struct AppState {
    pub db: Arc<Database>,
    pub event_tx: broadcast::Sender<Event>,
    pub governance: Arc<GovernanceEngine>,
    pub static_key: Arc<NoiseKeypair>,
    pub session_manager: Arc<SessionManager>,
}

// Helper to extract CID from packet
fn extract_cid(data: &[u8]) -> Option<[u8; 8]> {
    if data.len() < 8 { return None; }
    let mut cid = [0u8; 8];
    cid.copy_from_slice(&data[0..8]);
    Some(cid)
}

pub async fn start_http_listener(
    db: Arc<Database>, 
    port: u16, 
    event_tx: broadcast::Sender<Event>, 
    governance: Arc<GovernanceEngine>,
    static_key: NoiseKeypair,
    session_manager: Arc<SessionManager>
) {
    let state = AppState { 
        db, 
        event_tx, 
        governance,
        static_key: Arc::new(static_key),
        session_manager,
    };
    
    let app = Router::new()
        .route("/api/v1/beacon", post(handle_beacon))
        .with_state(state);

    let addr = std::net::SocketAddr::from(([0, 0, 0, 0], port));
    tracing::info!("HTTP Listener starting on {}", addr);

    match tokio::net::TcpListener::bind(addr).await {
        Ok(listener) => {
            if let Err(e) = axum::serve(
                listener, 
                app.into_make_service_with_connect_info::<SocketAddr>()
            ).await {
                tracing::error!("HTTP Listener error: {}", e);
            }
        }
        Err(e) => tracing::error!("Failed to bind HTTP listener to {}: {}", addr, e),
    }
}

async fn handle_beacon(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    body: Bytes,
) -> impl IntoResponse {
    // Governance
    if !state.governance.validate_action(addr.ip()) {
        return Vec::new();
    }

    let data = body.to_vec();
    let cid = match extract_cid(&data) {
        Some(c) => c,
        None => return Vec::new(),
    };
    
    let payload = &data[8..]; // Skip CID

    // 1. Handshake Init (Msg 1)
    if cid == [0xFF; 8] {
        tracing::debug!("Received Handshake Msg 1 from {}", addr);
        let mut handshake = match NoiseHandshake::new_responder(&state.static_key) {
            Ok(h) => h,
            Err(e) => {
                tracing::error!("Handshake init failed: {}", e);
                return Vec::new();
            }
        };
        
        let _response_payload = match handshake.read_message(payload) {
            Ok(p) => p, // Should be empty
            Err(e) => {
                tracing::error!("Read msg 1 failed: {}", e);
                return Vec::new();
            }
        };
        
        // Generate Msg 2
        let msg2 = match handshake.write_message(&[]) {
            Ok(m) => m,
            Err(e) => {
                tracing::error!("Write msg 2 failed: {}", e);
                return Vec::new();
            }
        };
        
        let temp_cid = uuid::Uuid::new_v4().as_bytes()[0..8].try_into().unwrap();
        state.session_manager.insert_handshake(temp_cid, handshake);
        
        // Response: Temp CID + Msg 2
        let mut response = Vec::new();
        response.extend_from_slice(&temp_cid);
        response.extend_from_slice(&msg2);
        return response;
    }
    
    // 2. Handshake Continue (Msg 3)
    if let Some(mut handshake) = state.session_manager.remove_handshake(&cid) {
        tracing::debug!("Received Handshake Msg 3");
        let _payload = match handshake.read_message(payload) {
            Ok(p) => p,
            Err(e) => {
                tracing::error!("Read msg 3 failed: {}", e);
                return Vec::new();
            }
        };
        
        // Handshake complete
        let transport = match handshake.into_transport() {
            Ok(t) => t,
            Err(e) => {
                tracing::error!("Into transport failed: {}", e);
                return Vec::new();
            }
        };
        
        state.session_manager.insert_session(cid, transport);
        
        // Return empty ack (Msg 4 equivalent/Transport established)
        let mut response = Vec::new();
        response.extend_from_slice(&cid);
        return response;
    }
    
    // 3. Data Transport
    if let Some(mut transport) = state.session_manager.get_session(&cid) {
        // Decrypt
        let plaintext = match transport.read_message(payload) {
            Ok(pt) => pt,
            Err(e) => {
                tracing::error!("Decryption failed: {}", e);
                return Vec::new();
            }
        };
        
        // Handle Inner Frame (WraithFrame)
        // Skip header (28 bytes)
        if plaintext.len() > 28 {
            let inner_payload = &plaintext[28..];
            
            // Process Beacon
            if let Ok(beacon) = serde_json::from_slice::<BeaconData>(inner_payload) {
                 tracing::debug!("Beacon Checkin: {}", beacon.id);
                 let _ = state.event_tx.send(Event {
                    id: uuid::Uuid::new_v4().to_string(),
                    r#type: "beacon_checkin".to_string(),
                    timestamp: Some(prost_types::Timestamp::from(std::time::SystemTime::now())),
                    campaign_id: "".to_string(),
                    implant_id: beacon.id.clone(),
                    data: std::collections::HashMap::new(),
                });
                
                // Response: Encrypt JSON tasks
                // In real impl, get tasks from DB
                let resp_json = serde_json::to_vec(&BeaconResponse { tasks: vec![] }).unwrap();
                
                // Wrap in Frame (Mock header)
                let mut frame = vec![0u8; 28]; // Empty header
                frame.extend_from_slice(&resp_json);
                
                // Encrypt
                let ciphertext = match transport.write_message(&frame) {
                    Ok(ct) => ct,
                    Err(_) => return Vec::new(),
                };
                
                let mut response = Vec::new();
                response.extend_from_slice(&cid);
                response.extend_from_slice(&ciphertext);
                return response;
            }
        }
    }

    // Unknown CID or failure
    Vec::new()
}