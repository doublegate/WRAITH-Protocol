use crate::database::Database;
use crate::governance::GovernanceEngine;
use crate::services::listener::ListenerManager;
use crate::services::operator::OperatorServiceImpl;
use crate::services::session::SessionManager;
use crate::wraith::redops::KillImplantRequest;
use crate::wraith::redops::operator_service_server::OperatorService;
use serial_test::serial;
use std::sync::Arc;
use tonic::Request;
use wraith_crypto::noise::NoiseKeypair;

#[tokio::test]
#[serial]
#[should_panic(expected = "KILLSWITCH_PORT must be set")]
async fn test_kill_implant_panics_without_port() {
    // Ensure the relevant env vars are NOT set
    unsafe {
        std::env::remove_var("KILLSWITCH_PORT");
        std::env::set_var("KILLSWITCH_SECRET", "dummy_secret");
        std::env::set_var(
            "KILLSWITCH_KEY",
            "0000000000000000000000000000000000000000000000000000000000000000",
        );
        std::env::set_var("HMAC_SECRET", "test_hmac_secret");
        std::env::set_var(
            "MASTER_KEY",
            "0000000000000000000000000000000000000000000000000000000000000000",
        );
    }

    let pool = sqlx::postgres::PgPoolOptions::new()
        .connect_lazy("postgres://user:pass@localhost/db")
        .unwrap();
    let db = Arc::new(Database::new(pool));
    let (tx, _) = tokio::sync::broadcast::channel(1);
    let governance = Arc::new(GovernanceEngine::new());
    let sessions = Arc::new(SessionManager::new());
    let static_key = Arc::new(NoiseKeypair::generate().unwrap());

    let listener_manager = Arc::new(ListenerManager::new(
        db.clone(),
        governance.clone(),
        sessions.clone(),
        static_key.clone(),
        tx.clone(),
    ));

    let operator_service = OperatorServiceImpl {
        db,
        event_tx: tx,
        listener_manager,
        powershell_manager: Arc::new(crate::services::powershell::PowerShellManager::new()),
    };

    let req = Request::new(KillImplantRequest {
        id: uuid::Uuid::new_v4().to_string(),
        clean_artifacts: false,
    });

    // This should panic
    let _ = operator_service.kill_implant(req).await;
}

#[tokio::test]
#[serial]
#[should_panic(expected = "KILLSWITCH_SECRET must be set")]
async fn test_kill_implant_panics_without_secret() {
    // Set port but not secret
    unsafe {
        std::env::set_var("KILLSWITCH_PORT", "1234");
        std::env::remove_var("KILLSWITCH_SECRET");
        std::env::set_var(
            "KILLSWITCH_KEY",
            "0000000000000000000000000000000000000000000000000000000000000000",
        );
        std::env::set_var("HMAC_SECRET", "test_hmac_secret");
        std::env::set_var(
            "MASTER_KEY",
            "0000000000000000000000000000000000000000000000000000000000000000",
        );
    }

    let pool = sqlx::postgres::PgPoolOptions::new()
        .connect_lazy("postgres://user:pass@localhost/db")
        .unwrap();
    let db = Arc::new(Database::new(pool));
    let (tx, _) = tokio::sync::broadcast::channel(1);
    let governance = Arc::new(GovernanceEngine::new());
    let sessions = Arc::new(SessionManager::new());
    let static_key = Arc::new(NoiseKeypair::generate().unwrap());

    let listener_manager = Arc::new(ListenerManager::new(
        db.clone(),
        governance.clone(),
        sessions.clone(),
        static_key.clone(),
        tx.clone(),
    ));

    let operator_service = OperatorServiceImpl {
        db,
        event_tx: tx,
        listener_manager,
        powershell_manager: Arc::new(crate::services::powershell::PowerShellManager::new()),
    };

    let req = Request::new(KillImplantRequest {
        id: uuid::Uuid::new_v4().to_string(),
        clean_artifacts: false,
    });

    // This should panic
    let _ = operator_service.kill_implant(req).await;
}
