use tonic::{Request, Response, Status};
use std::sync::Arc;
use crate::database::Database;
use crate::wraith::redops::*;
use crate::wraith::redops::implant_service_server::ImplantService;
use uuid::Uuid;

pub struct ImplantServiceImpl {
    pub db: Arc<Database>,
}

#[tonic::async_trait]
impl ImplantService for ImplantServiceImpl {
    type GetPendingCommandsStream = tokio_stream::wrappers::ReceiverStream<Result<Command, Status>>;
    type DownloadPayloadStream = tokio_stream::wrappers::ReceiverStream<Result<PayloadChunk, Status>>;

    async fn register(&self, req: Request<RegisterRequest>) -> Result<Response<RegisterResponse>, Status> {
        // In a real implementation, we would decrypt the registration payload here.
        // For now, we mock the registration logic.
        let _req = req.into_inner();
        
        let implant_data = crate::models::Implant {
            id: Uuid::new_v4(), // This should be deterministic in prod based on key
            campaign_id: None,
            hostname: Some("unknown".to_string()),
            internal_ip: None,
            external_ip: None,
            os_type: None,
            os_version: None,
            architecture: None,
            username: None,
            domain: None,
            privileges: None,
            implant_version: None,
            first_seen: None,
            last_checkin: None,
            checkin_interval: Some(60),
            jitter_percent: Some(10),
            status: Some("active".to_string()),
            notes: None,
            metadata: None,
        };

        let id = self.db.register_implant(&implant_data).await
            .map_err(|e| Status::internal(e.to_string()))?;

        Ok(Response::new(RegisterResponse {
            implant_id: id.to_string(),
            encrypted_config: vec![], // TODO
            checkin_interval: 60,
            jitter_percent: 10,
        }))
    }

    async fn check_in(&self, req: Request<CheckInRequest>) -> Result<Response<CheckInResponse>, Status> {
        let req = req.into_inner();
        let id = Uuid::parse_str(&req.implant_id)
            .map_err(|_| Status::invalid_argument("Invalid UUID"))?;

        self.db.update_implant_checkin(id).await
            .map_err(|e| Status::internal(e.to_string()))?;

        // Check for pending commands (simple count check for response header)
        // Ideally we'd optimize this
        let cmds = self.db.get_pending_commands(id).await
            .map_err(|e| Status::internal(e.to_string()))?;

        Ok(Response::new(CheckInResponse {
            has_commands: !cmds.is_empty(),
            command_count: cmds.len() as i32,
            next_checkin_seconds: 60,
            metadata: vec![],
        }))
    }

    async fn get_pending_commands(&self, req: Request<GetPendingCommandsRequest>) -> Result<Response<Self::GetPendingCommandsStream>, Status> {
        let req = req.into_inner();
        let id = Uuid::parse_str(&req.implant_id)
            .map_err(|_| Status::invalid_argument("Invalid UUID"))?;

        let cmds = self.db.get_pending_commands(id).await
            .map_err(|e| Status::internal(e.to_string()))?;

        let (tx, rx) = tokio::sync::mpsc::channel(4);
        
        tokio::spawn(async move {
            for cmd in cmds {
                let grpc_cmd = Command {
                    id: cmd.id.to_string(),
                    implant_id: cmd.implant_id.unwrap_or_default().to_string(),
                    operator_id: cmd.operator_id.unwrap_or_default().to_string(),
                    command_type: cmd.command_type,
                    payload: cmd.payload.unwrap_or_default(),
                    priority: cmd.priority.unwrap_or(0),
                    status: cmd.status.unwrap_or_default(),
                    created_at: None,
                    timeout_seconds: cmd.timeout_seconds.unwrap_or(0),
                };
                if tx.send(Ok(grpc_cmd)).await.is_err() {
                    break;
                }
            }
        });

        Ok(Response::new(tokio_stream::wrappers::ReceiverStream::new(rx)))
    }

    async fn submit_result(&self, req: Request<SubmitResultRequest>) -> Result<Response<()>, Status> {
        let req = req.into_inner();
        let cmd_id = Uuid::parse_str(&req.command_id)
            .map_err(|_| Status::invalid_argument("Invalid Command ID"))?;

        // In real impl, decrypt `encrypted_result` using session key
        // For MVP, we assume payload is cleartext for now or generic blob
        self.db.update_command_result(cmd_id, &req.encrypted_result).await
            .map_err(|e| Status::internal(e.to_string()))?;

        Ok(Response::new(()))
    }

    async fn upload_artifact(&self, _req: Request<tonic::Streaming<ArtifactChunk>>) -> Result<Response<UploadArtifactResponse>, Status> {
        Err(Status::unimplemented("Not implemented"))
    }

    async fn download_payload(&self, _req: Request<DownloadPayloadRequest>) -> Result<Response<Self::DownloadPayloadStream>, Status> {
        Err(Status::unimplemented("Not implemented"))
    }
}
