use sqlx::postgres::PgPoolOptions;
use std::net::SocketAddr;
use std::sync::Arc;
use tonic::transport::Server;
use tracing::info;

// Import generated protos
pub mod wraith {
    pub mod redops {
        tonic::include_proto!("wraith.redops");
    }
}

use wraith::redops::implant_service_server::ImplantServiceServer;
use wraith::redops::operator_service_server::OperatorServiceServer;

mod database;
mod models;
mod services;
mod listeners;
mod utils;
mod governance;
mod builder;

use database::Database;
use governance::GovernanceEngine;
use services::implant::ImplantServiceImpl;
use services::operator::OperatorServiceImpl;
use services::session::SessionManager;
use wraith_crypto::noise::NoiseKeypair;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    // Database connection
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:postgres@localhost/wraith_redops".to_string());

    info!("Connecting to database...");
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await?;

    // Run migrations
    info!("Running database migrations...");
    sqlx::migrate!("./migrations").run(&pool).await?;

    let db = Arc::new(Database::new(pool));
    let governance = Arc::new(GovernanceEngine::new());
    let static_key = NoiseKeypair::generate().expect("Failed to generate static key");
    let sessions = Arc::new(SessionManager::new());

    // Event broadcast channel
    let (event_tx, _rx) = tokio::sync::broadcast::channel(100);

    // Start HTTP Listener for Implants
    let http_db = db.clone();
    let http_event_tx = event_tx.clone();
    let http_governance = governance.clone();
    let http_sessions = sessions.clone();
    let http_key = static_key.clone();
    
    tokio::spawn(async move {
        listeners::http::start_http_listener(http_db, 8080, http_event_tx, http_governance, http_key, http_sessions).await;
    });

    // Start UDP Listener
    let udp_db = db.clone();
    let udp_event_tx = event_tx.clone();
    let udp_governance = governance.clone();
    let udp_sessions = sessions.clone();
    let udp_key = static_key.clone();

    tokio::spawn(async move {
        listeners::udp::start_udp_listener(udp_db, 9999, udp_event_tx, udp_governance, udp_key, udp_sessions).await;
    });

    let addr: SocketAddr = "0.0.0.0:50051".parse()?;
    let operator_service = OperatorServiceImpl {
        db: db.clone(),
        event_tx: event_tx.clone(),
        governance: governance.clone(),
        static_key: Arc::new(static_key.clone()),
        sessions: sessions.clone()
    };
    let implant_service = ImplantServiceImpl { db: db.clone(), event_tx: event_tx.clone() };

    info!("Team Server listening on {}", addr);

    Server::builder()
        .add_service(OperatorServiceServer::new(operator_service))
        .add_service(ImplantServiceServer::new(implant_service))
        .serve(addr)
        .await?;

    Ok(())
}
