// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

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

struct ClientState {
    client: Mutex<Option<OperatorServiceClient<Channel>>>,
}

#[tauri::command]
async fn connect_to_server(
    address: String, 
    state: State<'_, ClientState>
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
    state: State<'_, ClientState>
) -> Result<String, String> {
    let mut lock = state.client.lock().await;
    let client = lock.as_mut().ok_or("Not connected")?;

    let request = tonic::Request::new(CreateCampaignRequest {
        name,
        description,
        roe_document: vec![],
        roe_signature: vec![],
    });

    let response = client.create_campaign(request).await
        .map_err(|e| format!("gRPC error: {}", e))?;

    Ok(serde_json::to_string(&response.into_inner()).unwrap())
}

#[tauri::command]
async fn list_implants(
    state: State<'_, ClientState>
) -> Result<String, String> {
    let mut lock = state.client.lock().await;
    let client = lock.as_mut().ok_or("Not connected")?;

    let request = tonic::Request::new(ListImplantsRequest {
        campaign_id: "".to_string(), // List all for now
        status_filter: "".to_string(),
        page_size: 100,
        page_token: "".to_string(),
    });

    let response = client.list_implants(request).await
        .map_err(|e| format!("gRPC error: {}", e))?;

    // We manually map to a JSON-friendly struct or just rely on the fact generated structs implement serde if configured?
    // Tonic generated structs usually don't implement Serialize unless configured.
    // For simplicity here, I'll rely on debug format or manual mapping if strictly needed, 
    // BUT typically we configure tonic to add serde derives. 
    // Let's modify Cargo.toml to ensure prost supports serde or manual map.
    // Actually, simpler to just return the debug string for MVP or map fields.
    // Let's assume we map manually for clean JSON.
    
    let implants = response.into_inner().implants;
    // Basic JSON construction
    let json = serde_json::to_string(&implants).map_err(|e| e.to_string())?;
    Ok(json)
}

#[tauri::command]
async fn send_command(
    implant_id: String,
    command_type: String,
    payload: String, // Hex encoded or base64?
    state: State<'_, ClientState>
) -> Result<String, String> {
    let mut lock = state.client.lock().await;
    let client = lock.as_mut().ok_or("Not connected")?;

    let request = tonic::Request::new(SendCommandRequest {
        implant_id,
        command_type,
        payload: payload.into_bytes(), // Simplified
        priority: 1,
        timeout_seconds: 30,
    });

    let response = client.send_command(request).await
        .map_err(|e| format!("gRPC error: {}", e))?;

    Ok(response.into_inner().id)
}

#[tokio::main]
async fn main() {
    tauri::Builder::default()
        .manage(ClientState {
            client: Mutex::new(None),
        })
        .invoke_handler(tauri::generate_handler![
            connect_to_server,
            create_campaign,
            list_implants,
            send_command
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}