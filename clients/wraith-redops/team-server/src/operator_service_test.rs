#[cfg(test)]
mod tests {
    use crate::database::Database;
    use crate::services::operator::OperatorServiceImpl;
    use crate::wraith::redops::*;
    use crate::wraith::redops::operator_service_server::OperatorService;
    use sqlx::PgPool;
    use std::sync::Arc;
    use tokio::sync::broadcast;
    use wraith_crypto::noise::NoiseKeypair;
    use tonic::Request;
    use uuid::Uuid;

    async fn setup_db() -> Arc<Database> {
        // Trigger recompile for new migrations
        let db_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| "postgres://postgres:postgres@localhost:5432/postgres".to_string());
        
        unsafe {
            std::env::set_var("HMAC_SECRET", "0000000000000000000000000000000000000000000000000000000000000000"); // 64 hex chars = 32 bytes
            std::env::set_var("MASTER_KEY", "0000000000000000000000000000000000000000000000000000000000000000");
            std::env::set_var("JWT_SECRET", "test_jwt_secret_key_must_be_long_enough");
            std::env::set_var("KILLSWITCH_SECRET", "test_killswitch_secret");
            std::env::set_var("KILLSWITCH_PORT", "5000");
        }

        let pool = PgPool::connect(&db_url).await.expect("Failed to connect to Postgres");
        
        let schema_name = format!("test_{}", Uuid::new_v4().to_string().replace("-", ""));
        sqlx::query(&format!("CREATE SCHEMA {}", schema_name)).execute(&pool).await.unwrap();
        
        let schema_url = format!("{}?options=-c%20search_path%3D{}", db_url, schema_name);
        let schema_pool = PgPool::connect(&schema_url).await.unwrap();
        sqlx::migrate!("./migrations").run(&schema_pool).await.unwrap();
        
        Arc::new(Database::new(schema_pool))
    }

    fn create_service(db: Arc<Database>) -> OperatorServiceImpl {
        let (event_tx, _) = broadcast::channel(100);
        let governance = Arc::new(crate::governance::GovernanceEngine::new());
        let sessions = Arc::new(crate::services::session::SessionManager::new());
        let static_key = Arc::new(NoiseKeypair::generate().unwrap());
        
        OperatorServiceImpl {
            db: db.clone(),
            event_tx: event_tx.clone(),
            governance: governance.clone(),
            static_key: static_key.clone(),
            sessions: sessions.clone(),
            listener_manager: Arc::new(crate::services::listener::ListenerManager::new(
                db.clone(),
                governance,
                sessions,
                static_key,
                event_tx,
            )),
        }
    }

    fn auth_req<T>(data: T, user_id: &str) -> Request<T> {
        let mut req = Request::new(data);
        req.extensions_mut().insert(crate::utils::Claims {
            sub: user_id.to_string(),
            role: "admin".to_string(),
            exp: 0,
        });
        req
    }

    #[tokio::test]
    async fn test_operator_service_comprehensive() {
        let db = setup_db().await;
        let service = create_service(db.clone());
        let op_id = Uuid::new_v4();
        
        // Setup mock operator in DB for auth tests
        sqlx::query("INSERT INTO operators (id, username, public_key, role) VALUES ($1, $2, $3, $4)")
            .bind(op_id)
            .bind("testuser")
            .bind(vec![0u8; 32])
            .bind("admin")
            .execute(db.pool()).await.unwrap();

        // 1. Campaign Tests
        let camp_resp = service.create_campaign(auth_req(CreateCampaignRequest {
            name: "Test".to_string(),
            description: "Desc".to_string(),
            roe_document: vec![],
            roe_signature: vec![],
        }, &op_id.to_string())).await.unwrap().into_inner();
        let camp_id = camp_resp.id;

        service.get_campaign(Request::new(GetCampaignRequest { id: camp_id.clone() })).await.unwrap();
        service.list_campaigns(Request::new(ListCampaignsRequest { page_size: 10, page_token: "".to_string(), status_filter: "".to_string() })).await.unwrap();
        service.update_campaign(Request::new(UpdateCampaignRequest {
            id: camp_id.clone(),
            name: "New".to_string(),
            description: "".to_string(),
            status: "active".to_string(),
        })).await.unwrap();

        // 2. Implant Tests
        let imp_id = Uuid::new_v4();
        sqlx::query("INSERT INTO implants (id, campaign_id, hostname, status) VALUES ($1, $2, $3, $4)")
            .bind(imp_id)
            .bind(Uuid::parse_str(&camp_id).unwrap())
            .bind("host1")
            .bind("active")
            .execute(db.pool()).await.unwrap();

        service.get_implant(Request::new(GetImplantRequest { id: imp_id.to_string() })).await.unwrap();
        service.list_implants(Request::new(ListImplantsRequest { campaign_id: camp_id.clone(), page_size: 10, page_token: "".to_string(), status_filter: "".to_string() })).await.unwrap();

        // 3. Command Tests
        let cmd_resp = service.send_command(auth_req(SendCommandRequest {
            implant_id: imp_id.to_string(),
            command_type: "shell".to_string(),
            payload: "whoami".to_string().into_bytes(),
            priority: 1,
            timeout_seconds: 60,
        }, &op_id.to_string())).await.unwrap().into_inner();
        let cmd_id = cmd_resp.id;

        service.list_commands(Request::new(ListCommandsRequest { implant_id: imp_id.to_string(), page_size: 10, page_token: "".to_string(), status_filter: "".to_string() })).await.unwrap();
        service.cancel_command(Request::new(CancelCommandRequest { command_id: cmd_id.clone() })).await.unwrap();

        // 4. Listener Tests
        let list_resp = service.create_listener(Request::new(CreateListenerRequest {
            name: "L1".to_string(),
            r#type: "http".to_string(),
            bind_address: "0.0.0.0".to_string(),
            port: 8080,
            config: std::collections::HashMap::new(),
        })).await.unwrap().into_inner();
        let list_id = list_resp.id;

        service.list_listeners(Request::new(ListListenersRequest {})).await.unwrap();
        // start/stop might fail if port is taken, but we test the RPC logic
        let _ = service.start_listener(Request::new(ListenerActionRequest { id: list_id.clone() })).await;
        let _ = service.stop_listener(Request::new(ListenerActionRequest { id: list_id.clone() })).await;

        // 5. Artifacts & Credentials
        service.list_artifacts(Request::new(ListArtifactsRequest { campaign_id: camp_id.clone(), implant_id: "".to_string(), page_size: 10, page_token: "".to_string() })).await.unwrap();
        service.list_credentials(Request::new(ListCredentialsRequest { campaign_id: camp_id.clone(), implant_id: "".to_string(), page_size: 10, page_token: "".to_string(), credential_type: "".to_string() })).await.unwrap();

        // 6. Attack Chains & Playbooks
        let chain_resp = service.create_attack_chain(Request::new(CreateAttackChainRequest {
            name: "Chain1".to_string(),
            description: "Desc".to_string(),
            steps: vec![ChainStepRequest {
                step_order: 1,
                technique_id: "T1003".to_string(),
                command_type: "shell".to_string(),
                payload: "dir".to_string(),
                description: "step1".to_string(),
            }],
        })).await.unwrap().into_inner();
        let chain_id = chain_resp.id;

        service.get_attack_chain(Request::new(GetAttackChainRequest { id: chain_id.clone() })).await.unwrap();
        service.list_attack_chains(Request::new(ListAttackChainsRequest { page_size: 10, page_token: "".to_string() })).await.unwrap();
        service.execute_attack_chain(Request::new(ExecuteAttackChainRequest { chain_id: chain_id.clone(), implant_id: imp_id.to_string() })).await.unwrap();

        service.list_playbooks(Request::new(ListPlaybooksRequest {})).await.unwrap();
        
        // 7. Persistence
        service.list_persistence(Request::new(ListPersistenceRequest { implant_id: imp_id.to_string() })).await.unwrap();
    }
}
