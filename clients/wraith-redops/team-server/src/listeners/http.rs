use crate::database::Database;
use crate::governance::GovernanceEngine;
use crate::services::protocol::ProtocolHandler;
use crate::services::session::SessionManager;
use crate::wraith::redops::Event;
use axum::{
    Router,
    body::Bytes,
    extract::{ConnectInfo, State},
    response::IntoResponse,
    routing::post,
};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::broadcast;
use wraith_crypto::noise::NoiseKeypair;

#[derive(Clone)]
pub struct AppState {
    pub governance: Arc<GovernanceEngine>,
    pub protocol: ProtocolHandler,
}

pub async fn start_http_listener(
    db: Arc<Database>,
    port: u16,
    event_tx: broadcast::Sender<Event>,
    governance: Arc<GovernanceEngine>,
    static_key: NoiseKeypair,
    session_manager: Arc<SessionManager>,
) {
    let protocol = ProtocolHandler::new(db, session_manager, Arc::new(static_key), event_tx);

    let state = AppState {
        governance,
        protocol,
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

    state
        .protocol
        .handle_packet(&data, addr.to_string())
        .await
        .unwrap_or_default()
}
