use crate::database::Database;
use crate::governance::GovernanceEngine;
use crate::wraith::redops::Event;
use axum::{
    Router,
    body::Bytes,
    extract::{ConnectInfo, State},
    response::IntoResponse,
    routing::post,
};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::broadcast;
use wraith_crypto::noise::{HandshakePhase, NoiseHandshake, NoiseKeypair, NoiseTransport};

#[derive(Clone)]
pub struct AppState {
    pub db: Arc<Database>,
    pub event_tx: broadcast::Sender<Event>,
    pub governance: Arc<GovernanceEngine>,
    pub static_key: Arc<NoiseKeypair>,
    // Map temporary CIDs (from handshake) to pending handshakes
    pub handshakes: Arc<DashMap<[u8; 8], NoiseHandshake>>,
    // Map session CIDs to established transports
    pub sessions: Arc<DashMap<[u8; 8], NoiseTransport>>,
}

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

// Helper to extract CID from packet
fn extract_cid(data: &[u8]) -> Option<[u8; 8]> {
    if data.len() < 8 {
        return None;
    }
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
) {
    let state = AppState {
        db,
        event_tx,
        governance,
        static_key: Arc::new(static_key),
        handshakes: Arc::new(DashMap::new()),
        sessions: Arc::new(DashMap::new()),
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
                app.into_make_service_with_connect_info::<SocketAddr>(),
            )
            .await
            {
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

        let response_payload = match handshake.read_message(payload) {
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

        // We need a way to map the next request (Msg 3) to this state.
        // In UDP WRAITH, Msg 2 contains a new CID derived from keys.
        // We haven't derived keys yet.
        // Snow doesn't expose ephemeral public key easily to derive CID?
        // We'll generate a random temporary CID for the handshake continuation.
        // For Msg 2 response, we prepend THIS temp CID so client knows where to send Msg 3.
        // Wait, WRAITH spec says Responder sends "Connection ID (8 bytes): Derived from ee".
        // Snow state allows `get_handshake_hash`?
        // For MVP integration, I'll use a random CID.

        let temp_cid = uuid::Uuid::new_v4().as_bytes()[0..8].try_into().unwrap();
        state.handshakes.insert(temp_cid, handshake);

        // Response: Temp CID + Msg 2
        let mut response = Vec::new();
        response.extend_from_slice(&temp_cid);
        response.extend_from_slice(&msg2);
        return response;
    }

    // 2. Handshake Continue (Msg 3)
    if let Some((_, mut handshake)) = state.handshakes.remove(&cid) {
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

        // In WRAITH, we derive session keys and CID.
        // Here we'll just use the same Temp CID as the Session CID for simplicity
        // or derive a new one. Let's use the same CID for the session.
        // (In real WRAITH, CID rotates, but for this integration we stick to stable CID for HTTP).
        state.sessions.insert(cid, transport);

        // Return empty ack (Msg 4 equivalent/Transport established)
        // Client can now send Data.
        let mut response = Vec::new();
        response.extend_from_slice(&cid);
        return response;
    }

    // 3. Data Transport
    if let Some(mut transport) = state.sessions.get_mut(&cid) {
        // Decrypt
        let plaintext = match transport.read_message(payload) {
            Ok(pt) => pt,
            Err(e) => {
                tracing::error!("Decryption failed: {}", e);
                return Vec::new();
            }
        };

        // Handle Inner Frame (WraithFrame)
        // For now, assume it's just the JSON body we used before, or a Frame struct.
        // Implant sends: serialized WraithFrame.
        // We need to parse it.
        // Since I don't have wraith-core::frame::Frame available easily (I need to import it),
        // I'll just assume the payload IS the data for the beacon logic (skip frame header parsing for this step
        // or parse it manually if simple).
        // My Implant sends: Header (28 bytes) + JSON.
        // I'll skip 28 bytes.

        if plaintext.len() > 28 {
            let inner_payload = &plaintext[28..];

            // Process Beacon (Legacy Logic)
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
                // TODO: Get real tasks
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
