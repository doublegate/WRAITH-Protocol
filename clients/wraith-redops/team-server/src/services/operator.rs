use tonic::{Request, Response, Status};
use std::sync::Arc;
use crate::database::Database;
use crate::wraith::redops::*;
use crate::wraith::redops::operator_service_server::OperatorService;

pub struct OperatorServiceImpl {
    pub db: Arc<Database>,
}

#[tonic::async_trait]
impl OperatorService for OperatorServiceImpl {
    type StreamEventsStream = tokio_stream::wrappers::ReceiverStream<Result<Event, Status>>;
    type DownloadArtifactStream = tokio_stream::wrappers::ReceiverStream<Result<ArtifactChunk, Status>>;

    async fn authenticate(&self, _req: Request<AuthRequest>) -> Result<Response<AuthResponse>, Status> {
        // Mock auth for MVP
        Ok(Response::new(AuthResponse {
            token: "mock_token".to_string(),
            expires_at: Some(prost_types::Timestamp::from(std::time::SystemTime::now() + std::time::Duration::from_secs(3600))),
            operator: Some(Operator {
                id: uuid::Uuid::new_v4().to_string(),
                username: "admin".to_string(),
                display_name: "Admin User".to_string(),
                role: "admin".to_string(),
                last_active: None,
            }),
        }))
    }

    async fn refresh_token(&self, _req: Request<RefreshRequest>) -> Result<Response<AuthResponse>, Status> {
        Err(Status::unimplemented("Not implemented"))
    }

    async fn create_campaign(&self, req: Request<CreateCampaignRequest>) -> Result<Response<Campaign>, Status> {
        let req = req.into_inner();
        let camp = self.db.create_campaign(&req.name, &req.description).await
            .map_err(|e| Status::internal(e.to_string()))?;
        
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

    async fn get_campaign(&self, _req: Request<GetCampaignRequest>) -> Result<Response<Campaign>, Status> {
        Err(Status::unimplemented("Not implemented"))
    }

    async fn list_campaigns(&self, _req: Request<ListCampaignsRequest>) -> Result<Response<ListCampaignsResponse>, Status> {
        let campaigns = self.db.list_campaigns().await
            .map_err(|e| Status::internal(e.to_string()))?;

        let list = campaigns.into_iter().map(|c| Campaign {
            id: c.id.to_string(),
            name: c.name,
            description: c.description.unwrap_or_default(),
            status: c.status.unwrap_or_default(),
            start_date: None,
            end_date: None,
            roe: None,
            implant_count: 0,
            active_implant_count: 0,
        }).collect();

        Ok(Response::new(ListCampaignsResponse {
            campaigns: list,
            next_page_token: "".to_string(),
        }))
    }

    async fn update_campaign(&self, _req: Request<UpdateCampaignRequest>) -> Result<Response<Campaign>, Status> {
        Err(Status::unimplemented("Not implemented"))
    }

    async fn list_implants(&self, _req: Request<ListImplantsRequest>) -> Result<Response<ListImplantsResponse>, Status> {
        let implants = self.db.list_implants().await
            .map_err(|e| Status::internal(e.to_string()))?;

        let list = implants.into_iter().map(|i| Implant {
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
        }).collect();

        Ok(Response::new(ListImplantsResponse {
            implants: list,
            next_page_token: "".to_string(),
        }))
    }

    async fn get_implant(&self, _req: Request<GetImplantRequest>) -> Result<Response<Implant>, Status> {
        Err(Status::unimplemented("Not implemented"))
    }

    async fn kill_implant(&self, _req: Request<KillImplantRequest>) -> Result<Response<()>, Status> {
        Err(Status::unimplemented("Not implemented"))
    }

    async fn send_command(&self, req: Request<SendCommandRequest>) -> Result<Response<Command>, Status> {
        let req = req.into_inner();
        let implant_id = uuid::Uuid::parse_str(&req.implant_id)
            .map_err(|_| Status::invalid_argument("Invalid UUID"))?;
        
        let cmd_id = self.db.queue_command(implant_id, &req.command_type, &req.payload).await
            .map_err(|e| Status::internal(e.to_string()))?;

        Ok(Response::new(Command {
            id: cmd_id.to_string(),
            implant_id: req.implant_id,
            operator_id: "".to_string(), // TODO: extract from context
            command_type: req.command_type,
            payload: req.payload,
            priority: req.priority,
            status: "pending".to_string(),
            created_at: None,
            timeout_seconds: req.timeout_seconds,
        }))
    }

    async fn get_command_result(&self, _req: Request<GetCommandResultRequest>) -> Result<Response<CommandResult>, Status> {
        Err(Status::unimplemented("Not implemented"))
    }

    async fn list_commands(&self, _req: Request<ListCommandsRequest>) -> Result<Response<ListCommandsResponse>, Status> {
        Err(Status::unimplemented("Not implemented"))
    }

    async fn cancel_command(&self, _req: Request<CancelCommandRequest>) -> Result<Response<()>, Status> {
        Err(Status::unimplemented("Not implemented"))
    }

    async fn stream_events(&self, _req: Request<StreamEventsRequest>) -> Result<Response<Self::StreamEventsStream>, Status> {
        Err(Status::unimplemented("Not implemented"))
    }

    async fn list_artifacts(&self, _req: Request<ListArtifactsRequest>) -> Result<Response<ListArtifactsResponse>, Status> {
        Err(Status::unimplemented("Not implemented"))
    }

    async fn download_artifact(&self, _req: Request<DownloadArtifactRequest>) -> Result<Response<Self::DownloadArtifactStream>, Status> {
        Err(Status::unimplemented("Not implemented"))
    }

    async fn list_credentials(&self, _req: Request<ListCredentialsRequest>) -> Result<Response<ListCredentialsResponse>, Status> {
        Err(Status::unimplemented("Not implemented"))
    }

    async fn create_listener(&self, req: Request<CreateListenerRequest>) -> Result<Response<Listener>, Status> {
        let req = req.into_inner();
        let config_json = serde_json::to_value(req.config).map_err(|e| Status::internal(e.to_string()))?;
        
        let listener = self.db.create_listener(&req.name, &req.r#type, &req.bind_address, config_json).await
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

    async fn list_listeners(&self, _req: Request<ListListenersRequest>) -> Result<Response<ListListenersResponse>, Status> {
        let listeners = self.db.list_listeners().await
            .map_err(|e| Status::internal(e.to_string()))?;

        let list = listeners.into_iter().map(|l| Listener {
            id: l.id.to_string(),
            name: l.name,
            r#type: l.r#type,
            bind_address: l.bind_address,
            port: 0, 
            status: l.status,
            config: std::collections::HashMap::new(),
        }).collect();

        Ok(Response::new(ListListenersResponse { listeners: list }))
    }

    async fn start_listener(&self, req: Request<ListenerActionRequest>) -> Result<Response<Listener>, Status> {
        let req = req.into_inner();
        let id = uuid::Uuid::parse_str(&req.id).map_err(|_| Status::invalid_argument("Bad ID"))?;
        
        self.db.update_listener_status(id, "active").await
            .map_err(|e| Status::internal(e.to_string()))?;
            
        Ok(Response::new(Listener::default())) 
    }

    async fn stop_listener(&self, req: Request<ListenerActionRequest>) -> Result<Response<Listener>, Status> {
        let req = req.into_inner();
        let id = uuid::Uuid::parse_str(&req.id).map_err(|_| Status::invalid_argument("Bad ID"))?;
        
        self.db.update_listener_status(id, "stopped").await
            .map_err(|e| Status::internal(e.to_string()))?;
            
        Ok(Response::new(Listener::default()))
    }
}
