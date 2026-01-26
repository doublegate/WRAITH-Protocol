use crate::services::operator::OperatorServiceImpl;
use crate::wraith::redops::operator_service_server::OperatorService;
use crate::wraith::redops::{KillImplantRequest};
use tonic::Request;
use std::sync::Arc;
use crate::database::Database;
use crate::governance::GovernanceEngine;
use crate::services::session::SessionManager;
use wraith_crypto::noise::NoiseKeypair;

#[tokio::test]
#[should_panic(expected = "KILLSWITCH_PORT must be set")]
async fn test_kill_implant_panics_without_port() {
    // Ensure the relevant env vars are NOT set
    unsafe {
        std::env::remove_var("KILLSWITCH_PORT");
        std::env::set_var("KILLSWITCH_SECRET", "dummy_secret");
        std::env::set_var("KILLSWITCH_KEY", "0000000000000000000000000000000000000000000000000000000000000000");
        std::env::set_var("HMAC_SECRET", "test_hmac_secret");
        std::env::set_var("MASTER_KEY", "0000000000000000000000000000000000000000000000000000000000000000");
    }

    let pool = sqlx::postgres::PgPoolOptions::new().connect_lazy("postgres://user:pass@localhost/db").unwrap();
    let db = Arc::new(Database::new(pool));
    let (tx, _) = tokio::sync::broadcast::channel(1);
    
    let operator_service = OperatorServiceImpl {
        db,
        event_tx: tx,
        governance: Arc::new(GovernanceEngine::new()),
        static_key: Arc::new(NoiseKeypair::generate().unwrap()),
        sessions: Arc::new(SessionManager::new()),
    };

    let req = Request::new(KillImplantRequest {
        id: uuid::Uuid::new_v4().to_string(),
        clean_artifacts: false,
    });
    
    // This should panic
    let _ = operator_service.kill_implant(req).await;
}

#[tokio::test]
#[should_panic(expected = "KILLSWITCH_SECRET must be set")]
async fn test_kill_implant_panics_without_secret() {
    // Set port but not secret
    unsafe {
        std::env::set_var("KILLSWITCH_PORT", "1234");
        std::env::remove_var("KILLSWITCH_SECRET");
        std::env::set_var("KILLSWITCH_KEY", "0000000000000000000000000000000000000000000000000000000000000000");
        std::env::set_var("HMAC_SECRET", "test_hmac_secret");
        std::env::set_var("MASTER_KEY", "0000000000000000000000000000000000000000000000000000000000000000");
    }

    let pool = sqlx::postgres::PgPoolOptions::new().connect_lazy("postgres://user:pass@localhost/db").unwrap();
    let db = Arc::new(Database::new(pool));
    let (tx, _) = tokio::sync::broadcast::channel(1);

    let operator_service = OperatorServiceImpl {
        db,
        event_tx: tx,
        governance: Arc::new(GovernanceEngine::new()),
        static_key: Arc::new(NoiseKeypair::generate().unwrap()),
        sessions: Arc::new(SessionManager::new()),
    };
    
    let req = Request::new(KillImplantRequest {
        id: uuid::Uuid::new_v4().to_string(),
        clean_artifacts: false,
    });

    // This should panic
    let _ = operator_service.kill_implant(req).await;
}