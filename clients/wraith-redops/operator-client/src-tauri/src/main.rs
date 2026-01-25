// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

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

/// Apply Wayland workarounds for Linux desktop environments.
///
/// This function handles two common issues with WebKitGTK on Linux:
/// 1. Wayland Error 71 (Protocol error) on KDE Plasma 6
/// 2. GBM (Generic Buffer Management) errors with hardware acceleration
#[cfg(target_os = "linux")]
fn apply_wayland_workarounds() {
    use std::env;

    // Workaround for Wayland Error 71 (Protocol error) on KDE Plasma 6
    // See: https://github.com/tauri-apps/tauri/issues/10702
    //      https://github.com/tauri-apps/tao/issues/977
    //
    // WebKitGTK has compatibility issues with Wayland on KDE Plasma 6, causing
    // "Error 71 (Protocol error) dispatching to Wayland display" crashes.
    // This is an upstream issue blocked by tao/webkit2gtk compatibility.
    //
    // Solution: Automatically fallback to X11 via XWayland if:
    // 1. We're on Linux
    // 2. GDK_BACKEND is not already set (respect user preference)
    // 3. We're in a Wayland session
    // 4. We're on KDE Plasma 6 (or any Wayland compositor with issues)

    // Only set GDK_BACKEND if not already configured by user
    if env::var("GDK_BACKEND").is_err() {
        // Check if we're in a Wayland session
        if let Ok(session_type) = env::var("XDG_SESSION_TYPE") {
            if session_type == "wayland" {
                // Check for KDE Plasma (common source of Error 71)
                let is_kde = env::var("KDE_SESSION_VERSION").is_ok()
                    || env::var("KDE_FULL_SESSION").is_ok()
                    || env::var("DESKTOP_SESSION")
                        .map(|s| s.contains("plasma") || s.contains("kde"))
                        .unwrap_or(false);

                if is_kde {
                    eprintln!(
                        "Detected KDE Plasma on Wayland - forcing X11 backend to avoid Error 71"
                    );
                    eprintln!("See: https://github.com/tauri-apps/tauri/issues/10702");
                    // SAFETY: We're in main() before any threads are spawned,
                    // so there's no risk of data races with other threads reading env vars
                    unsafe {
                        env::set_var("GDK_BACKEND", "x11");
                    }
                } else {
                    // For other Wayland compositors, prefer Wayland but fallback to X11
                    // This allows GDK to try Wayland first, then X11 if issues occur
                    // SAFETY: We're in main() before any threads are spawned,
                    // so there's no risk of data races with other threads reading env vars
                    unsafe {
                        env::set_var("GDK_BACKEND", "wayland,x11");
                    }
                }
            }
        }
    }

    // Workaround for GBM (Generic Buffer Management) errors
    // See: https://github.com/tauri-apps/tauri/issues/13493
    //      https://github.com/winfunc/opcode/issues/26
    //
    // WebKitGTK's hardware-accelerated compositing can fail with:
    // "Failed to create GBM buffer of size WxH: Invalid argument"
    //
    // This occurs due to incompatibility between WebKitGTK, Mesa, and GPU drivers
    // (especially NVIDIA). Disabling compositing mode forces WebKit to use a
    // simpler, more compatible rendering path.
    if env::var("WEBKIT_DISABLE_COMPOSITING_MODE").is_err() {
        // SAFETY: We're in main() before any threads are spawned
        unsafe {
            env::set_var("WEBKIT_DISABLE_COMPOSITING_MODE", "1");
        }
    }
}

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

    let campaign_json: CampaignJson = response.into_inner().into();
    serde_json::to_string(&campaign_json).map_err(|e| e.to_string())
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

    let implants: Vec<ImplantJson> = response.into_inner().implants
        .into_iter()
        .map(|i| i.into())
        .collect();

    serde_json::to_string(&implants).map_err(|e| e.to_string())
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
    // Apply Wayland workarounds before initializing Tauri
    #[cfg(target_os = "linux")]
    apply_wayland_workarounds();

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