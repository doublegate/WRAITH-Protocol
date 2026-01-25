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
async fn kill_implant(implant_id: String, state: State<'_, ClientState>) -> Result<(), String> {
    let mut lock = state.client.lock().await;
    let client = lock.as_mut().ok_or("Not connected")?;

    client
        .kill_implant(tonic::Request::new(KillImplantRequest { 
            id: implant_id,
            clean_artifacts: false
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
        .start_listener(tonic::Request::new(ListenerActionRequest { id: listener_id }))
        .await
        .map_err(|e| format!("gRPC error: {}", e))?;

    Ok(())
}

#[tauri::command]
async fn stop_listener(listener_id: String, state: State<'_, ClientState>) -> Result<(), String> {
    let mut lock = state.client.lock().await;
    let client = lock.as_mut().ok_or("Not connected")?;

    client
        .stop_listener(tonic::Request::new(ListenerActionRequest { id: listener_id }))
        .await
        .map_err(|e| format!("gRPC error: {}", e))?;

    Ok(())
}

/// Initialize and run the Tauri application
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Initialize logging
    if std::env::var("RUST_LOG").is_err() {
        // SAFETY: This is called at the start of main before any threads are spawned
        unsafe { std::env::set_var("RUST_LOG", "info") };
    }
    tracing_subscriber::fmt::init();

    tracing::info!("Starting WRAITH RedOps Operator Console");
    tracing::warn!("IMPORTANT: This tool is for AUTHORIZED red team operations ONLY");

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_shell::init())
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
            kill_implant,
            start_listener,
            stop_listener
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_module_compiles() {
        // This test verifies that the module compiles correctly
        assert!(true);
    }
}
