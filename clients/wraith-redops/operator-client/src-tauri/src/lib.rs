//! WRAITH RedOps Operator Console
//!
//! A Tauri desktop application for red team operations.
//! Provides a GUI interface for managing campaigns, implants, and commands.
//!
//! ## Features
//! - Campaign management
//! - Implant monitoring
//! - Command execution
//! - Real-time status updates
//!
//! ## Architecture
//! - Frontend: React 19 + TypeScript + Tailwind CSS v4
//! - Backend: Tauri 2.x + Rust
//! - Communication: gRPC to team server

use serde::{Deserialize, Serialize};
use tauri::State;
use tokio::sync::Mutex;
use tonic::transport::Channel;

// Import generated protos
pub mod wraith {
    pub mod redops {
        tonic::include_proto!("wraith.redops");
    }
}
use wraith::redops::operator_service_client::OperatorServiceClient;
use wraith::redops::*;

// JSON-friendly wrapper types for frontend communication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CampaignJson {
    pub id: String,
    pub name: String,
    pub description: String,
    pub status: String,
    pub implant_count: i32,
    pub active_implant_count: i32,
}

impl From<Campaign> for CampaignJson {
    fn from(c: Campaign) -> Self {
        Self {
            id: c.id,
            name: c.name,
            description: c.description,
            status: c.status,
            implant_count: c.implant_count,
            active_implant_count: c.active_implant_count,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImplantJson {
    pub id: String,
    pub campaign_id: String,
    pub hostname: String,
    pub internal_ip: String,
    pub external_ip: String,
    pub os_type: String,
    pub os_version: String,
    pub architecture: String,
    pub username: String,
    pub domain: String,
    pub privileges: String,
    pub implant_version: String,
    pub checkin_interval: i32,
    pub jitter_percent: i32,
    pub status: String,
}

impl From<Implant> for ImplantJson {
    fn from(i: Implant) -> Self {
        Self {
            id: i.id,
            campaign_id: i.campaign_id,
            hostname: i.hostname,
            internal_ip: i.internal_ip,
            external_ip: i.external_ip,
            os_type: i.os_type,
            os_version: i.os_version,
            architecture: i.architecture,
            username: i.username,
            domain: i.domain,
            privileges: i.privileges,
            implant_version: i.implant_version,
            checkin_interval: i.checkin_interval,
            jitter_percent: i.jitter_percent,
            status: i.status,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListenerJson {
    pub id: String,
    pub name: String,
    pub type_: String,
    pub bind_address: String,
    pub port: i32,
    pub status: String,
}

impl From<Listener> for ListenerJson {
    fn from(l: Listener) -> Self {
        Self {
            id: l.id,
            name: l.name,
            type_: l.r#type,
            bind_address: l.bind_address,
            port: l.port,
            status: l.status,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandJson {
    pub id: String,
    pub implant_id: String,
    pub command_type: String,
    pub status: String,
    pub payload_preview: String,
}

impl From<Command> for CommandJson {
    fn from(c: Command) -> Self {
        Self {
            id: c.id,
            implant_id: c.implant_id,
            command_type: c.command_type,
            status: c.status,
            payload_preview: String::from_utf8_lossy(&c.payload).to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandResultJson {
    pub id: String,
    pub output: String,
    pub exit_code: i32,
    pub error_message: String,
}

impl From<CommandResult> for CommandResultJson {
    fn from(r: CommandResult) -> Self {
        Self {
            id: r.id,
            output: String::from_utf8_lossy(&r.output).to_string(),
            exit_code: r.exit_code,
            error_message: r.error_message,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactJson {
    pub id: String,
    pub filename: String,
    pub size: i64,
}

impl From<Artifact> for ArtifactJson {
    fn from(a: Artifact) -> Self {
        Self {
            id: a.id,
            filename: a.filename,
            size: a.file_size,
        }
    }
}

struct ClientState {
    client: Mutex<Option<OperatorServiceClient<Channel>>>,
}

#[tauri::command]
async fn connect_to_server(
    address: String,
    state: State<'_, ClientState>,
) -> Result<String, String> {
    let endpoint = if address.starts_with("http") {
        address
    } else {
        format!("http://{}", address)
    };

    let client = OperatorServiceClient::connect(endpoint)
        .await
        .map_err(|e| format!("Failed to connect: {}", e))?;

    let mut lock = state.client.lock().await;
    *lock = Some(client);

    Ok("Connected successfully".to_string())
}

#[tauri::command]
async fn create_campaign(
    name: String,
    description: String,
    state: State<'_, ClientState>,
) -> Result<String, String> {
    let mut lock = state.client.lock().await;
    let client = lock.as_mut().ok_or("Not connected")?;

    let request = tonic::Request::new(CreateCampaignRequest {
        name,
        description,
        roe_document: vec![],
        roe_signature: vec![],
    });

    let response = client
        .create_campaign(request)
        .await
        .map_err(|e| format!("gRPC error: {}", e))?;

    let campaign_json: CampaignJson = response.into_inner().into();
    serde_json::to_string(&campaign_json).map_err(|e| e.to_string())
}

#[tauri::command]
async fn list_implants(state: State<'_, ClientState>) -> Result<String, String> {
    let mut lock = state.client.lock().await;
    let client = lock.as_mut().ok_or("Not connected")?;

    let request = tonic::Request::new(ListImplantsRequest {
        campaign_id: String::new(), // List all for now
        status_filter: String::new(),
        page_size: 100,
        page_token: String::new(),
    });

    let response = client
        .list_implants(request)
        .await
        .map_err(|e| format!("gRPC error: {}", e))?;

    let implants: Vec<ImplantJson> = response
        .into_inner()
        .implants
        .into_iter()
        .map(|i| i.into())
        .collect();

    serde_json::to_string(&implants).map_err(|e| e.to_string())
}

#[tauri::command]
async fn send_command(
    implant_id: String,
    command_type: String,
    payload: String,
    state: State<'_, ClientState>,
) -> Result<String, String> {
    let mut lock = state.client.lock().await;
    let client = lock.as_mut().ok_or("Not connected")?;

    let request = tonic::Request::new(SendCommandRequest {
        implant_id,
        command_type,
        payload: payload.into_bytes(),
        priority: 1,
        timeout_seconds: 30,
    });

    let response = client
        .send_command(request)
        .await
        .map_err(|e| format!("gRPC error: {}", e))?;

    Ok(response.into_inner().id)
}

#[tauri::command]
async fn list_campaigns(state: State<'_, ClientState>) -> Result<String, String> {
    let mut lock = state.client.lock().await;
    let client = lock.as_mut().ok_or("Not connected")?;

    let response = client
        .list_campaigns(tonic::Request::new(ListCampaignsRequest {
            status_filter: "".to_string(),
            page_size: 100,
            page_token: "".to_string(),
        }))
        .await
        .map_err(|e| format!("gRPC error: {}", e))?;

    let campaigns: Vec<CampaignJson> = response
        .into_inner()
        .campaigns
        .into_iter()
        .map(|c| c.into())
        .collect();

    serde_json::to_string(&campaigns).map_err(|e| e.to_string())
}

#[tauri::command]
async fn list_listeners(state: State<'_, ClientState>) -> Result<String, String> {
    let mut lock = state.client.lock().await;
    let client = lock.as_mut().ok_or("Not connected")?;

    let response = client
        .list_listeners(tonic::Request::new(ListListenersRequest {}))
        .await
        .map_err(|e| format!("gRPC error: {}", e))?;

    let listeners: Vec<ListenerJson> = response
        .into_inner()
        .listeners
        .into_iter()
        .map(|l| l.into())
        .collect();

    serde_json::to_string(&listeners).map_err(|e| e.to_string())
}

#[tauri::command]
async fn create_listener(
    name: String,
    type_: String,
    bind_address: String,
    port: i32,
    state: State<'_, ClientState>,
) -> Result<String, String> {
    let mut lock = state.client.lock().await;
    let client = lock.as_mut().ok_or("Not connected")?;

    let response = client
        .create_listener(tonic::Request::new(CreateListenerRequest {
            name,
            r#type: type_,
            bind_address,
            port,
            config: std::collections::HashMap::new(),
        }))
        .await
        .map_err(|e| format!("gRPC error: {}", e))?;

    let l: ListenerJson = response.into_inner().into();
    serde_json::to_string(&l).map_err(|e| e.to_string())
}

#[tauri::command]
async fn list_commands(
    implant_id: String,
    state: State<'_, ClientState>,
) -> Result<String, String> {
    let mut lock = state.client.lock().await;
    let client = lock.as_mut().ok_or("Not connected")?;

    let response = client
        .list_commands(tonic::Request::new(ListCommandsRequest {
            implant_id,
            status_filter: "".to_string(),
            page_size: 100,
            page_token: "".to_string(),
        }))
        .await
        .map_err(|e| format!("gRPC error: {}", e))?;

    let cmds: Vec<CommandJson> = response
        .into_inner()
        .commands
        .into_iter()
        .map(|c| c.into())
        .collect();

    serde_json::to_string(&cmds).map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_command_result(
    command_id: String,
    state: State<'_, ClientState>,
) -> Result<String, String> {
    let mut lock = state.client.lock().await;
    let client = lock.as_mut().ok_or("Not connected")?;

    let response = client
        .get_command_result(tonic::Request::new(GetCommandResultRequest { command_id }))
        .await
        .map_err(|e| format!("gRPC error: {}", e))?;

    let res: CommandResultJson = response.into_inner().into();
    serde_json::to_string(&res).map_err(|e| e.to_string())
}

#[tauri::command]
async fn list_artifacts(state: State<'_, ClientState>) -> Result<String, String> {
    let mut lock = state.client.lock().await;
    let client = lock.as_mut().ok_or("Not connected")?;

    let response = client
        .list_artifacts(tonic::Request::new(ListArtifactsRequest {
            implant_id: "".to_string(),
            campaign_id: "".to_string(),
            page_size: 100,
            page_token: "".to_string(),
        }))
        .await
        .map_err(|e| format!("gRPC error: {}", e))?;

    let artifacts: Vec<ArtifactJson> = response
        .into_inner()
        .artifacts
        .into_iter()
        .map(|a| a.into())
        .collect();

    serde_json::to_string(&artifacts).map_err(|e| e.to_string())
}

#[tauri::command]
async fn download_artifact(
    artifact_id: String,
    save_path: String,
    state: State<'_, ClientState>,
) -> Result<String, String> {
    let mut lock = state.client.lock().await;
    let client = lock.as_mut().ok_or("Not connected")?;

    let mut stream = client
        .download_artifact(tonic::Request::new(DownloadArtifactRequest { artifact_id }))
        .await
        .map_err(|e| format!("gRPC error: {}", e))?
        .into_inner();

    let mut file = tokio::fs::File::create(&save_path)
        .await
        .map_err(|e| e.to_string())?;

    use tokio::io::AsyncWriteExt;
    while let Some(chunk) = stream.message().await.map_err(|e| e.to_string())? {
        file.write_all(&chunk.data)
            .await
            .map_err(|e| e.to_string())?;
    }

    Ok("Download complete".to_string())
}

#[tauri::command]
async fn update_campaign(
    id: String,
    name: String,
    description: String,
    status: String,
    state: State<'_, ClientState>,
) -> Result<String, String> {
    let mut lock = state.client.lock().await;
    let client = lock.as_mut().ok_or("Not connected")?;

    let response = client
        .update_campaign(tonic::Request::new(UpdateCampaignRequest {
            id,
            name,
            description,
            status,
        }))
        .await
        .map_err(|e| format!("gRPC error: {}", e))?;

    let c: CampaignJson = response.into_inner().into();
    serde_json::to_string(&c).map_err(|e| e.to_string())
}

#[tauri::command]
async fn kill_implant(implant_id: String, state: State<'_, ClientState>) -> Result<(), String> {
    let mut lock = state.client.lock().await;
    let client = lock.as_mut().ok_or("Not connected")?;

    client
        .kill_implant(tonic::Request::new(KillImplantRequest {
            id: implant_id,
            clean_artifacts: false,
        }))
        .await
        .map_err(|e| format!("gRPC error: {}", e))?;

    Ok(())
}

#[tauri::command]
async fn start_listener(listener_id: String, state: State<'_, ClientState>) -> Result<(), String> {
    let mut lock = state.client.lock().await;
    let client = lock.as_mut().ok_or("Not connected")?;

    client
        .start_listener(tonic::Request::new(ListenerActionRequest {
            id: listener_id,
        }))
        .await
        .map_err(|e| format!("gRPC error: {}", e))?;

    Ok(())
}

#[tauri::command]
async fn stop_listener(listener_id: String, state: State<'_, ClientState>) -> Result<(), String> {
    let mut lock = state.client.lock().await;
    let client = lock.as_mut().ok_or("Not connected")?;

    client
        .stop_listener(tonic::Request::new(ListenerActionRequest {
            id: listener_id,
        }))
        .await
        .map_err(|e| format!("gRPC error: {}", e))?;

    Ok(())
}

#[tauri::command]
async fn create_phishing(
    type_: String,
    c2_url: String,
    save_path: String,
    method: Option<String>,
    state: State<'_, ClientState>,
) -> Result<String, String> {
    let mut lock = state.client.lock().await;
    let client = lock.as_mut().ok_or("Not connected")?;

    let req = tonic::Request::new(GeneratePhishingRequest {
        r#type: type_,
        c2_url,
        template_name: "".to_string(),
        method: method.unwrap_or_default(),
    });

    let mut stream = client
        .generate_phishing(req)
        .await
        .map_err(|e| format!("gRPC error: {}", e))?
        .into_inner();

    let mut file = tokio::fs::File::create(&save_path)
        .await
        .map_err(|e| e.to_string())?;
    use tokio::io::AsyncWriteExt;
    while let Some(chunk) = stream.message().await.map_err(|e| e.to_string())? {
        file.write_all(&chunk.data)
            .await
            .map_err(|e| e.to_string())?;
    }

    Ok("Phishing payload generated".to_string())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistenceItemJson {
    pub id: String,
    pub implant_id: String,
    pub method: String,
    pub details: String,
}

impl From<PersistenceItem> for PersistenceItemJson {
    fn from(p: PersistenceItem) -> Self {
        Self {
            id: p.id,
            implant_id: p.implant_id,
            method: p.method,
            details: p.details,
        }
    }
}

#[tauri::command]
async fn list_persistence(
    implant_id: String,
    state: State<'_, ClientState>,
) -> Result<String, String> {
    let mut lock = state.client.lock().await;
    let client = lock.as_mut().ok_or("Not connected")?;

    let response = client
        .list_persistence(tonic::Request::new(ListPersistenceRequest { implant_id }))
        .await
        .map_err(|e| format!("gRPC error: {}", e))?;

    let items: Vec<PersistenceItemJson> = response
        .into_inner()
        .items
        .into_iter()
        .map(|i| i.into())
        .collect();
    serde_json::to_string(&items).map_err(|e| e.to_string())
}

#[tauri::command]
async fn remove_persistence(id: String, state: State<'_, ClientState>) -> Result<(), String> {
    let mut lock = state.client.lock().await;
    let client = lock.as_mut().ok_or("Not connected")?;

    client
        .remove_persistence(tonic::Request::new(RemovePersistenceRequest { id }))
        .await
        .map_err(|e| format!("gRPC error: {}", e))?;
    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CredentialJson {
    pub id: String,
    pub implant_id: String,
    pub source: String,
    pub credential_type: String,
    pub domain: String,
    pub username: String,
}

impl From<Credential> for CredentialJson {
    fn from(c: Credential) -> Self {
        Self {
            id: c.id,
            implant_id: c.implant_id,
            source: c.source,
            credential_type: c.credential_type,
            domain: c.domain,
            username: c.username,
        }
    }
}

#[tauri::command]
async fn list_credentials(state: State<'_, ClientState>) -> Result<String, String> {
    let mut lock = state.client.lock().await;
    let client = lock.as_mut().ok_or("Not connected")?;

    let response = client
        .list_credentials(tonic::Request::new(ListCredentialsRequest {
            campaign_id: "".to_string(),
            implant_id: "".to_string(),
            credential_type: "".to_string(),
            page_size: 100,
            page_token: "".to_string(),
        }))
        .await
        .map_err(|e| format!("gRPC error: {}", e))?;

    let creds: Vec<CredentialJson> = response
        .into_inner()
        .credentials
        .into_iter()
        .map(|c| c.into())
        .collect();
    serde_json::to_string(&creds).map_err(|e| e.to_string())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainStepJson {
    pub id: String,
    pub step_order: i32,
    pub technique_id: String,
    pub command_type: String,
    pub payload: String,
    pub description: String,
}

impl From<ChainStep> for ChainStepJson {
    fn from(s: ChainStep) -> Self {
        Self {
            id: s.id,
            step_order: s.step_order,
            technique_id: s.technique_id,
            command_type: s.command_type,
            payload: s.payload,
            description: s.description,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttackChainJson {
    pub id: String,
    pub name: String,
    pub description: String,
    pub steps: Vec<ChainStepJson>,
}

impl From<AttackChain> for AttackChainJson {
    fn from(c: AttackChain) -> Self {
        Self {
            id: c.id,
            name: c.name,
            description: c.description,
            steps: c.steps.into_iter().map(|s| s.into()).collect(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainStepInput {
    pub step_order: i32,
    pub technique_id: String,
    pub command_type: String,
    pub payload: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaybookJson {
    pub id: String,
    pub name: String,
    pub description: String,
    pub content: String,
}

impl From<Playbook> for PlaybookJson {
    fn from(p: Playbook) -> Self {
        Self {
            id: p.id,
            name: p.name,
            description: p.description,
            content: p.content_json,
        }
    }
}

#[tauri::command]
async fn create_attack_chain(
    name: String,
    description: String,
    steps: Vec<ChainStepInput>,
    state: State<'_, ClientState>,
) -> Result<String, String> {
    let mut lock = state.client.lock().await;
    let client = lock.as_mut().ok_or("Not connected")?;

    let req_steps = steps
        .into_iter()
        .map(|s| ChainStepRequest {
            step_order: s.step_order,
            technique_id: s.technique_id,
            command_type: s.command_type,
            payload: s.payload,
            description: s.description,
        })
        .collect();

    let request = tonic::Request::new(CreateAttackChainRequest {
        name,
        description,
        steps: req_steps,
    });

    let response = client
        .create_attack_chain(request)
        .await
        .map_err(|e| format!("gRPC error: {}", e))?;
    let chain: AttackChainJson = response.into_inner().into();
    serde_json::to_string(&chain).map_err(|e| e.to_string())
}

#[tauri::command]
async fn list_attack_chains(state: State<'_, ClientState>) -> Result<String, String> {
    let mut lock = state.client.lock().await;
    let client = lock.as_mut().ok_or("Not connected")?;

    let response = client
        .list_attack_chains(tonic::Request::new(ListAttackChainsRequest {
            page_size: 100,
            page_token: "".to_string(),
        }))
        .await
        .map_err(|e| format!("gRPC error: {}", e))?;

    let chains: Vec<AttackChainJson> = response
        .into_inner()
        .chains
        .into_iter()
        .map(|c| c.into())
        .collect();
    serde_json::to_string(&chains).map_err(|e| e.to_string())
}

#[tauri::command]
async fn execute_attack_chain(
    chain_id: String,
    implant_id: String,
    state: State<'_, ClientState>,
) -> Result<(), String> {
    let mut lock = state.client.lock().await;
    let client = lock.as_mut().ok_or("Not connected")?;

    client
        .execute_attack_chain(tonic::Request::new(ExecuteAttackChainRequest {
            chain_id,
            implant_id,
        }))
        .await
        .map_err(|e| format!("gRPC error: {}", e))?;
    Ok(())
}

#[tauri::command]
async fn refresh_token(token: String, state: State<'_, ClientState>) -> Result<String, String> {
    let mut lock = state.client.lock().await;
    let client = lock.as_mut().ok_or("Not connected")?;
    let response = client
        .refresh_token(tonic::Request::new(RefreshRequest { token }))
        .await
        .map_err(|e| format!("gRPC error: {}", e))?;
    Ok(response.into_inner().token)
}

#[tauri::command]
async fn get_campaign(id: String, state: State<'_, ClientState>) -> Result<String, String> {
    let mut lock = state.client.lock().await;
    let client = lock.as_mut().ok_or("Not connected")?;
    let response = client
        .get_campaign(tonic::Request::new(GetCampaignRequest { id }))
        .await
        .map_err(|e| format!("gRPC error: {}", e))?;
    let c: CampaignJson = response.into_inner().into();
    serde_json::to_string(&c).map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_implant(id: String, state: State<'_, ClientState>) -> Result<String, String> {
    let mut lock = state.client.lock().await;
    let client = lock.as_mut().ok_or("Not connected")?;
    let response = client
        .get_implant(tonic::Request::new(GetImplantRequest { id }))
        .await
        .map_err(|e| format!("gRPC error: {}", e))?;
    let i: ImplantJson = response.into_inner().into();
    serde_json::to_string(&i).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cancel_command(command_id: String, state: State<'_, ClientState>) -> Result<(), String> {
    let mut lock = state.client.lock().await;
    let client = lock.as_mut().ok_or("Not connected")?;
    client
        .cancel_command(tonic::Request::new(CancelCommandRequest { command_id }))
        .await
        .map_err(|e| format!("gRPC error: {}", e))?;
    Ok(())
}

#[tauri::command]
async fn generate_implant(
    platform: String,
    arch: String,
    format: String,
    c2_url: String,
    sleep_interval: i32,
    save_path: String,
    state: State<'_, ClientState>,
) -> Result<String, String> {
    let mut lock = state.client.lock().await;
    let client = lock.as_mut().ok_or("Not connected")?;

    let req = tonic::Request::new(GenerateImplantRequest {
        platform,
        arch,
        format,
        c2_url,
        sleep_interval,
    });

    let mut stream = client
        .generate_implant(req)
        .await
        .map_err(|e| format!("gRPC error: {}", e))?
        .into_inner();

    let mut file = tokio::fs::File::create(&save_path)
        .await
        .map_err(|e| e.to_string())?;
    use tokio::io::AsyncWriteExt;
    while let Some(chunk) = stream.message().await.map_err(|e| e.to_string())? {
        file.write_all(&chunk.data)
            .await
            .map_err(|e| e.to_string())?;
    }

    Ok("Implant generated".to_string())
}

#[tauri::command]
async fn list_playbooks(state: State<'_, ClientState>) -> Result<String, String> {
    let mut lock = state.client.lock().await;
    let client = lock.as_mut().ok_or("Not connected")?;
    let response = client
        .list_playbooks(tonic::Request::new(ListPlaybooksRequest {}))
        .await
        .map_err(|e| format!("gRPC error: {}", e))?;

    let playbooks: Vec<PlaybookJson> = response
        .into_inner()
        .playbooks
        .into_iter()
        .map(|p| p.into())
        .collect();
    serde_json::to_string(&playbooks).map_err(|e| e.to_string())
}

#[tauri::command]
async fn instantiate_playbook(
    playbook_id: String,
    name_override: String,
    state: State<'_, ClientState>,
) -> Result<String, String> {
    let mut lock = state.client.lock().await;
    let client = lock.as_mut().ok_or("Not connected")?;
    let response = client
        .instantiate_playbook(tonic::Request::new(InstantiatePlaybookRequest {
            playbook_id,
            name_override,
        }))
        .await
        .map_err(|e| format!("gRPC error: {}", e))?;

    let chain: AttackChainJson = response.into_inner().into();
    serde_json::to_string(&chain).map_err(|e| e.to_string())
}

#[derive(Clone, Serialize)]
struct StreamEventPayload {
    id: String,
    #[serde(rename = "type")]
    type_: String,
    implant_id: String,
    data: std::collections::HashMap<String, String>,
}

#[tauri::command]
async fn stream_events(app: tauri::AppHandle, state: State<'_, ClientState>) -> Result<(), String> {
    let mut lock = state.client.lock().await;
    let client = lock.as_mut().ok_or("Not connected")?;
    let mut stream_client = client.clone();

    let response = stream_client
        .stream_events(tonic::Request::new(StreamEventsRequest {
            campaign_id: "".to_string(),
            event_types: vec![],
        }))
        .await
        .map_err(|e| format!("gRPC error: {}", e))?;

    let mut stream = response.into_inner();

    tauri::async_runtime::spawn(async move {
        use tauri::Emitter;
        use tauri_plugin_notification::NotificationExt;
        while let Ok(Some(event)) = stream.message().await {
            let payload = StreamEventPayload {
                id: event.id.clone(),
                type_: event.r#type.clone(),
                implant_id: event.implant_id.clone(),
                data: event.data.clone(),
            };
            let _ = app.emit("server-event", payload);

            // Desktop Notification for important events
            if event.r#type == "implant_checkin" || event.r#type == "command_complete" {
                let _ = app
                    .notification()
                    .builder()
                    .title("WRAITH RedOps")
                    .body(format!("Event: {} from {}", event.r#type, event.implant_id))
                    .show();
            }
        }
    });

    Ok(())
}

#[tauri::command]
async fn get_attack_chain(id: String, state: State<'_, ClientState>) -> Result<String, String> {
    let mut lock = state.client.lock().await;
    let client = lock.as_mut().ok_or("Not connected")?;

    let response = client
        .get_attack_chain(tonic::Request::new(GetAttackChainRequest { id }))
        .await
        .map_err(|e| format!("gRPC error: {}", e))?;

    let chain: AttackChainJson = response.into_inner().into();
    serde_json::to_string(&chain).map_err(|e| e.to_string())
}

#[tauri::command]
async fn set_powershell_profile(
    implant_id: String,
    profile_script: String,
    state: State<'_, ClientState>,
) -> Result<(), String> {
    let mut lock = state.client.lock().await;
    let client = lock.as_mut().ok_or("Not connected")?;

    client
        .set_power_shell_profile(tonic::Request::new(SetProfileRequest {
            implant_id,
            profile_script,
        }))
        .await
        .map_err(|e| format!("gRPC error: {}", e))?;
    Ok(())
}

#[tauri::command]
async fn get_powershell_profile(
    implant_id: String,
    state: State<'_, ClientState>,
) -> Result<String, String> {
    let mut lock = state.client.lock().await;
    let client = lock.as_mut().ok_or("Not connected")?;

    let response = client
        .get_power_shell_profile(tonic::Request::new(GetProfileRequest { implant_id }))
        .await
        .map_err(|e| format!("gRPC error: {}", e))?;

    Ok(response.into_inner().profile_script)
}

/// Initialize and run the Tauri application
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Initialize logging with tracing-subscriber's default env filter
    // Uses RUST_LOG if set, otherwise defaults to info level
    use tracing_subscriber::EnvFilter;
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    tracing_subscriber::fmt().with_env_filter(filter).init();

    tracing::info!("Starting WRAITH RedOps Operator Console");
    tracing::warn!("IMPORTANT: This tool is for AUTHORIZED red team operations ONLY");

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_notification::init())
        .manage(ClientState {
            client: Mutex::new(None),
        })
        .invoke_handler(tauri::generate_handler![
            connect_to_server,
            create_campaign,
            list_implants,
            send_command,
            list_campaigns,
            list_listeners,
            create_listener,
            list_commands,
            get_command_result,
            list_artifacts,
            download_artifact,
            update_campaign,
            kill_implant,
            start_listener,
            stop_listener,
            create_phishing,
            list_persistence,
            remove_persistence,
            list_credentials,
            create_attack_chain,
            list_attack_chains,
            execute_attack_chain,
            get_attack_chain,
            refresh_token,
            get_campaign,
            get_implant,
            cancel_command,
            generate_implant,
            list_playbooks,
            instantiate_playbook,
            stream_events,
            set_powershell_profile,
            get_powershell_profile
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ipc_types_serialization() {
        // Campaign
        let campaign = CampaignJson {
            id: "123".to_string(),
            name: "Op".to_string(),
            description: "Desc".to_string(),
            status: "active".to_string(),
            implant_count: 5,
            active_implant_count: 2,
        };
        let json = serde_json::to_string(&campaign).unwrap();
        assert!(json.contains("implant_count"));

        // Listener
        let listener = ListenerJson {
            id: "456".to_string(),
            name: "HTTP".to_string(),
            type_: "http".to_string(),
            bind_address: "0.0.0.0".to_string(),
            port: 8080,
            status: "running".to_string(),
        };
        let json = serde_json::to_string(&listener).unwrap();
        assert!(json.contains("bind_address"));
    }
}
