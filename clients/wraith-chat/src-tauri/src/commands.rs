// Tauri IPC Commands for WRAITH-Chat

use crate::audio::AudioDevice;
use crate::crypto::{DoubleRatchet, EncryptedMessage};
use crate::database::{
    Contact, Conversation, GroupActivityStats, Message, NewConversation, StorageBreakdown,
};
use crate::group::{GroupInfo, GroupMember, GroupRole, SenderKeyDistribution};
use crate::state::AppState;
use crate::video::{CameraDevice, ScreenSource, VideoResolution};
use crate::video_call::{VideoCallInfo, VideoSource};
use crate::voice_call::CallInfo;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::{AppHandle, Emitter, State};

// MARK: - Contact Commands

#[tauri::command]
pub async fn create_contact(
    state: State<'_, Arc<AppState>>,
    peer_id: String,
    display_name: Option<String>,
    identity_key: Vec<u8>,
) -> Result<i64, String> {
    let db = state.db.lock().await;

    // Generate safety number from peer ID and identity key
    let safety_number = generate_safety_number(&peer_id, &identity_key);

    let contact = Contact {
        id: 0,
        peer_id,
        display_name,
        identity_key,
        safety_number,
        verified: false,
        blocked: false,
        created_at: chrono::Utc::now().timestamp(),
        last_seen: None,
    };

    db.insert_contact(&contact).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_contact(
    state: State<'_, Arc<AppState>>,
    peer_id: String,
) -> Result<Option<Contact>, String> {
    let db = state.db.lock().await;
    db.get_contact(&peer_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn list_contacts(state: State<'_, Arc<AppState>>) -> Result<Vec<Contact>, String> {
    let db = state.db.lock().await;
    db.list_contacts().map_err(|e| e.to_string())
}

// MARK: - Conversation Commands

#[tauri::command]
pub async fn create_conversation(
    state: State<'_, Arc<AppState>>,
    conv_type: String,
    peer_id: Option<String>,
    group_id: Option<String>,
    display_name: Option<String>,
) -> Result<i64, String> {
    let db = state.db.lock().await;

    let new_conv = NewConversation {
        conv_type,
        peer_id,
        group_id,
        display_name,
    };

    db.create_conversation(&new_conv).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_conversation(
    state: State<'_, Arc<AppState>>,
    id: i64,
) -> Result<Option<Conversation>, String> {
    let db = state.db.lock().await;
    db.get_conversation(id).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn list_conversations(
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<Conversation>, String> {
    let db = state.db.lock().await;
    db.list_conversations().map_err(|e| e.to_string())
}

// MARK: - Message Commands

#[tauri::command]
pub async fn send_message(
    state: State<'_, Arc<AppState>>,
    conversation_id: i64,
    peer_id: String,
    body: String,
) -> Result<i64, String> {
    // Parse peer ID from hex
    let peer_id_bytes: [u8; 32] = hex::decode(&peer_id)
        .map_err(|e| format!("Invalid peer ID hex: {}", e))?
        .try_into()
        .map_err(|_| "Peer ID must be 32 bytes")?;

    let db = state.db.lock().await;

    // Get or create ratchet for this peer
    let mut ratchets = state.ratchets.lock().await;
    let ratchet = if let Some(r) = ratchets.get_mut(&peer_id) {
        r
    } else {
        // Try to load from database
        if let Ok(Some(state_json)) = db.load_ratchet_state(&peer_id) {
            let loaded = DoubleRatchet::from_json(&state_json).map_err(|e| e.to_string())?;
            ratchets.insert(peer_id.clone(), loaded);
            ratchets.get_mut(&peer_id).unwrap()
        } else {
            // No existing session - need to establish one first
            return Err(
                "No session established with this peer. Call establish_session first.".to_string(),
            );
        }
    };

    // Encrypt message with Double Ratchet
    let encrypted = ratchet
        .encrypt(body.as_bytes())
        .map_err(|e| e.to_string())?;

    // Save ratchet state
    let ratchet_json = ratchet.to_json().map_err(|e| e.to_string())?;
    db.save_ratchet_state(&peer_id, &ratchet_json)
        .map_err(|e| e.to_string())?;

    // Create message record (before sending, to track it)
    let local_peer_id = state.local_peer_id.lock().await.clone();
    let message = Message {
        id: 0,
        conversation_id,
        sender_peer_id: local_peer_id,
        content_type: "text".to_string(),
        body: Some(body),
        media_path: None,
        media_mime_type: None,
        media_size: None,
        timestamp: chrono::Utc::now().timestamp(),
        sent: false,
        delivered: false,
        read_by_me: true,
        expires_at: None,
        direction: "outgoing".to_string(),
    };

    let message_id = db.insert_message(&message).map_err(|e| e.to_string())?;

    // Serialize encrypted message to send over the wire
    let encrypted_bytes = serde_json::to_vec(&encrypted)
        .map_err(|e| format!("Failed to serialize encrypted message: {}", e))?;

    // Send encrypted message via WRAITH protocol
    let node = state.node.lock().await;
    if node.is_running() {
        match node.send_data(&peer_id_bytes, &encrypted_bytes).await {
            Ok(()) => {
                // Update message as sent
                db.mark_message_sent(message_id)
                    .map_err(|e| e.to_string())?;
                log::debug!("Message {} sent successfully", message_id);
            }
            Err(e) => {
                log::warn!("Failed to send message via WRAITH protocol: {}", e);
                // Message is saved but not marked as sent - can retry later
            }
        }
    } else {
        log::warn!("WRAITH node not running, message saved but not sent");
    }

    Ok(message_id)
}

#[tauri::command]
pub async fn receive_message(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    peer_id: String,
    encrypted_message: EncryptedMessage,
) -> Result<i64, String> {
    let db = state.db.lock().await;

    // Get ratchet for this peer
    let mut ratchets = state.ratchets.lock().await;

    let ratchet = if let Some(ratchet) = ratchets.get_mut(&peer_id) {
        ratchet
    } else {
        // Load from database
        let state_json = db
            .load_ratchet_state(&peer_id)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "No ratchet state found".to_string())?;

        let ratchet = DoubleRatchet::from_json(&state_json).map_err(|e| e.to_string())?;
        ratchets.insert(peer_id.clone(), ratchet);
        ratchets.get_mut(&peer_id).unwrap()
    };

    // Decrypt message
    let plaintext = ratchet
        .decrypt(&encrypted_message)
        .map_err(|e| e.to_string())?;

    let body = String::from_utf8(plaintext).map_err(|e| e.to_string())?;

    // Save updated ratchet state
    let ratchet_json = ratchet.to_json().map_err(|e| e.to_string())?;
    db.save_ratchet_state(&peer_id, &ratchet_json)
        .map_err(|e| e.to_string())?;

    // Find or create conversation
    let conversations = db.list_conversations().map_err(|e| e.to_string())?;
    let conversation_id = if let Some(conv) = conversations
        .iter()
        .find(|c| c.peer_id.as_deref() == Some(&peer_id))
    {
        conv.id
    } else {
        // Create new conversation
        let new_conv = NewConversation {
            conv_type: "direct".to_string(),
            peer_id: Some(peer_id.clone()),
            group_id: None,
            display_name: None,
        };
        db.create_conversation(&new_conv)
            .map_err(|e| e.to_string())?
    };

    // Create message record
    let message = Message {
        id: 0,
        conversation_id,
        sender_peer_id: peer_id.clone(),
        content_type: "text".to_string(),
        body: Some(body),
        media_path: None,
        media_mime_type: None,
        media_size: None,
        timestamp: chrono::Utc::now().timestamp(),
        sent: true,
        delivered: true,
        read_by_me: false,
        expires_at: None,
        direction: "incoming".to_string(),
    };

    let message_id = db.insert_message(&message).map_err(|e| e.to_string())?;

    // Emit event to frontend to update UI
    if let Err(e) = app.emit(
        "message_received",
        serde_json::json!({
            "message_id": message_id,
            "conversation_id": conversation_id,
            "peer_id": peer_id,
            "timestamp": chrono::Utc::now().to_rfc3339(),
        }),
    ) {
        log::warn!("Failed to emit message_received event: {}", e);
    }

    Ok(message_id)
}

#[tauri::command]
pub async fn get_messages(
    state: State<'_, Arc<AppState>>,
    conversation_id: i64,
    limit: i64,
    offset: i64,
) -> Result<Vec<Message>, String> {
    let db = state.db.lock().await;
    db.get_messages(conversation_id, limit, offset)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn mark_as_read(
    state: State<'_, Arc<AppState>>,
    conversation_id: i64,
) -> Result<(), String> {
    let db = state.db.lock().await;
    db.mark_as_read(conversation_id).map_err(|e| e.to_string())
}

// MARK: - Node Commands

#[tauri::command]
pub async fn start_node(
    state: State<'_, Arc<AppState>>,
    listen_addr: String,
) -> Result<(), String> {
    log::info!("Starting WRAITH node on {}", listen_addr);

    let mut node = state.node.lock().await;

    // Parse listen address if provided
    let config = if !listen_addr.is_empty() {
        let addr: std::net::SocketAddr = listen_addr
            .parse()
            .map_err(|e| format!("Invalid listen address: {}", e))?;
        wraith_core::node::NodeConfig {
            listen_addr: addr,
            ..Default::default()
        }
    } else {
        wraith_core::node::NodeConfig::default()
    };

    // Initialize node if not already done
    if node.node().is_none() {
        node.initialize_with_config(config).await?;
    }

    // Start the node
    node.start().await?;

    // Update local peer ID cache
    if let Some(peer_id) = node.peer_id() {
        let mut local_peer_id = state.local_peer_id.lock().await;
        *local_peer_id = peer_id;
    }

    log::info!("WRAITH node started successfully");
    Ok(())
}

#[tauri::command]
pub async fn stop_node(state: State<'_, Arc<AppState>>) -> Result<(), String> {
    log::info!("Stopping WRAITH node");

    let mut node = state.node.lock().await;
    node.stop().await?;

    // Clear local peer ID cache
    let mut local_peer_id = state.local_peer_id.lock().await;
    local_peer_id.clear();

    log::info!("WRAITH node stopped");
    Ok(())
}

#[tauri::command]
pub async fn get_node_status(state: State<'_, Arc<AppState>>) -> Result<NodeStatus, String> {
    let node = state.node.lock().await;
    let peer_id = state.local_peer_id.lock().await;

    // Get session count from WRAITH node
    let session_count = node.active_route_count();

    // Get conversation count from database
    let db = state.db.lock().await;
    let active_conversations = db.count_conversations().unwrap_or(0);

    Ok(NodeStatus {
        running: node.is_running(),
        local_peer_id: peer_id.clone(),
        session_count,
        active_conversations,
    })
}

#[tauri::command]
pub async fn get_peer_id(state: State<'_, Arc<AppState>>) -> Result<String, String> {
    let node = state.node.lock().await;
    node.peer_id()
        .ok_or_else(|| "Node not initialized".to_string())
}

// MARK: - Session Commands

/// Establish an encrypted session with a peer
///
/// This performs a Noise_XX handshake via the WRAITH protocol and initializes
/// a Double Ratchet for forward-secret message encryption.
#[tauri::command]
pub async fn establish_session(
    state: State<'_, Arc<AppState>>,
    peer_id_hex: String,
) -> Result<SessionInfo, String> {
    // Parse peer ID from hex
    let peer_id_bytes: [u8; 32] = hex::decode(&peer_id_hex)
        .map_err(|e| format!("Invalid peer ID hex: {}", e))?
        .try_into()
        .map_err(|_| "Peer ID must be 32 bytes")?;

    // Establish WRAITH session
    let node = state.node.lock().await;
    let session_id = node
        .establish_session(&peer_id_bytes)
        .await
        .map_err(|e| format!("Failed to establish session: {}", e))?;

    // Get the X25519 public key from the node for the Double Ratchet
    let our_x25519_pub = node.x25519_public_key().ok_or("Node not initialized")?;

    // Derive a shared secret for the Double Ratchet from the session ID
    // The session ID is derived from the Noise handshake, so it's a secure source
    let shared_secret = derive_ratchet_secret(&session_id, &peer_id_bytes);

    // Initialize Double Ratchet with the shared secret
    // We're the initiator, so we don't have the remote's DH public key yet
    let ratchet = DoubleRatchet::new(&shared_secret, None)
        .map_err(|e| format!("Failed to create Double Ratchet: {}", e))?;

    // Store ratchet state
    let mut ratchets = state.ratchets.lock().await;
    ratchets.insert(peer_id_hex.clone(), ratchet);

    // Also save to database
    let db = state.db.lock().await;
    let ratchet = ratchets.get(&peer_id_hex).unwrap();
    let ratchet_json = ratchet.to_json().map_err(|e| e.to_string())?;
    db.save_ratchet_state(&peer_id_hex, &ratchet_json)
        .map_err(|e| e.to_string())?;

    log::info!(
        "Established encrypted session with peer {}",
        &peer_id_hex[..16]
    );

    Ok(SessionInfo {
        session_id: hex::encode(session_id),
        peer_id: peer_id_hex,
        our_public_key: hex::encode(our_x25519_pub),
    })
}

/// Initialize a receiving session (when we receive a connection from a peer)
///
/// This is called when we receive a message from a peer we haven't communicated with yet.
#[tauri::command]
pub async fn init_receiving_session(
    state: State<'_, Arc<AppState>>,
    peer_id_hex: String,
    remote_public_key: Vec<u8>,
) -> Result<(), String> {
    // Parse peer ID from hex
    let peer_id_bytes: [u8; 32] = hex::decode(&peer_id_hex)
        .map_err(|e| format!("Invalid peer ID hex: {}", e))?
        .try_into()
        .map_err(|_| "Peer ID must be 32 bytes")?;

    // Derive shared secret (receiver perspective)
    let node = state.node.lock().await;

    // We need a session ID - try to get from existing session
    let session_id = match node.node() {
        Some(n) => {
            // Get session ID from any existing connection
            let sessions = n.active_sessions().await;
            if let Some(sid) = sessions.first() {
                *sid
            } else {
                // Generate a deterministic placeholder from peer ID
                // This will be replaced when the actual session is established
                peer_id_bytes
            }
        }
        None => peer_id_bytes, // Fallback
    };

    let shared_secret = derive_ratchet_secret(&session_id, &peer_id_bytes);

    // Initialize Double Ratchet with remote's public key (we're the responder)
    let ratchet = DoubleRatchet::new(&shared_secret, Some(&remote_public_key))
        .map_err(|e| format!("Failed to create Double Ratchet: {}", e))?;

    // Store ratchet state
    let mut ratchets = state.ratchets.lock().await;
    ratchets.insert(peer_id_hex.clone(), ratchet);

    // Also save to database
    let db = state.db.lock().await;
    let ratchet = ratchets.get(&peer_id_hex).unwrap();
    let ratchet_json = ratchet.to_json().map_err(|e| e.to_string())?;
    db.save_ratchet_state(&peer_id_hex, &ratchet_json)
        .map_err(|e| e.to_string())?;

    log::info!(
        "Initialized receiving session with peer {}",
        &peer_id_hex[..16]
    );

    Ok(())
}

// MARK: - Helper Functions

/// Derive a shared secret for the Double Ratchet from session data
fn derive_ratchet_secret(session_id: &[u8; 32], peer_id: &[u8; 32]) -> [u8; 32] {
    use sha2::{Digest, Sha256};

    let mut hasher = Sha256::new();
    hasher.update(b"wraith-chat-ratchet-secret-v1");
    hasher.update(session_id);
    hasher.update(peer_id);
    let hash = hasher.finalize();

    let mut secret = [0u8; 32];
    secret.copy_from_slice(&hash);
    secret
}

fn generate_safety_number(peer_id: &str, identity_key: &[u8]) -> String {
    use sha2::{Digest, Sha256};

    let mut hasher = Sha256::new();
    hasher.update(peer_id.as_bytes());
    hasher.update(identity_key);
    let hash = hasher.finalize();

    // Format as 12 groups of 5 digits
    let mut safety_number = String::new();
    for (i, chunk) in hash.chunks(2).take(12).enumerate() {
        if i > 0 {
            safety_number.push(' ');
        }
        let value = u16::from_be_bytes([chunk[0], chunk.get(1).copied().unwrap_or(0)]);
        safety_number.push_str(&format!("{:05}", u32::from(value) % 100000));
    }

    safety_number
}

// MARK: - Data Models

#[derive(Debug, serde::Serialize)]
pub struct NodeStatus {
    pub running: bool,
    pub local_peer_id: String,
    pub session_count: usize,
    pub active_conversations: usize,
}

#[derive(Debug, serde::Serialize)]
pub struct SessionInfo {
    pub session_id: String,
    pub peer_id: String,
    pub our_public_key: String,
}

// MARK: - Voice Call Commands (Sprint 17.5)

/// Start a voice call with a peer
#[tauri::command]
pub async fn start_call(
    state: State<'_, Arc<AppState>>,
    peer_id: String,
) -> Result<CallInfo, String> {
    state
        .voice_calls
        .start_call(&peer_id)
        .await
        .map_err(|e| e.to_string())
}

/// Answer an incoming call
#[tauri::command]
pub async fn answer_call(
    state: State<'_, Arc<AppState>>,
    call_id: String,
) -> Result<CallInfo, String> {
    state
        .voice_calls
        .answer_call(&call_id)
        .await
        .map_err(|e| e.to_string())
}

/// Reject an incoming call
#[tauri::command]
pub async fn reject_call(
    state: State<'_, Arc<AppState>>,
    call_id: String,
    reason: Option<String>,
) -> Result<(), String> {
    state
        .voice_calls
        .reject_call(&call_id, &reason.unwrap_or_else(|| "declined".to_string()))
        .await
        .map_err(|e| e.to_string())
}

/// End an active call
#[tauri::command]
pub async fn end_call(
    state: State<'_, Arc<AppState>>,
    call_id: String,
    reason: Option<String>,
) -> Result<(), String> {
    state
        .voice_calls
        .end_call(&call_id, &reason.unwrap_or_else(|| "hangup".to_string()))
        .await
        .map_err(|e| e.to_string())
}

/// Toggle mute on a call
#[tauri::command]
pub async fn toggle_mute(state: State<'_, Arc<AppState>>, call_id: String) -> Result<bool, String> {
    state
        .voice_calls
        .toggle_mute(&call_id)
        .await
        .map_err(|e| e.to_string())
}

/// Toggle speaker on a call
#[tauri::command]
pub async fn toggle_speaker(
    state: State<'_, Arc<AppState>>,
    call_id: String,
) -> Result<bool, String> {
    state
        .voice_calls
        .toggle_speaker(&call_id)
        .await
        .map_err(|e| e.to_string())
}

/// Get information about a call
#[tauri::command]
pub async fn get_call_info(
    state: State<'_, Arc<AppState>>,
    call_id: String,
) -> Result<Option<CallInfo>, String> {
    state
        .voice_calls
        .get_call_info(&call_id)
        .await
        .map_err(|e| e.to_string())
}

/// Get all active calls
#[tauri::command]
pub async fn get_active_calls(state: State<'_, Arc<AppState>>) -> Result<Vec<CallInfo>, String> {
    Ok(state.voice_calls.get_active_calls().await)
}

/// List available audio input devices
#[tauri::command]
pub async fn list_audio_input_devices(
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<AudioDevice>, String> {
    state
        .voice_calls
        .list_input_devices()
        .await
        .map_err(|e| e.to_string())
}

/// List available audio output devices
#[tauri::command]
pub async fn list_audio_output_devices(
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<AudioDevice>, String> {
    state
        .voice_calls
        .list_output_devices()
        .await
        .map_err(|e| e.to_string())
}

/// Set the audio input device
#[tauri::command]
pub async fn set_audio_input_device(
    state: State<'_, Arc<AppState>>,
    device_id: Option<String>,
) -> Result<(), String> {
    state
        .voice_calls
        .set_input_device(device_id)
        .await
        .map_err(|e| e.to_string())
}

/// Set the audio output device
#[tauri::command]
pub async fn set_audio_output_device(
    state: State<'_, Arc<AppState>>,
    device_id: Option<String>,
) -> Result<(), String> {
    state
        .voice_calls
        .set_output_device(device_id)
        .await
        .map_err(|e| e.to_string())
}

// MARK: - Group Messaging Commands (Sprint 17.7)

/// Create a new group
#[tauri::command]
pub async fn create_group(
    state: State<'_, Arc<AppState>>,
    name: String,
    member_peer_ids: Option<Vec<String>>,
) -> Result<GroupInfo, String> {
    let local_peer_id = state.local_peer_id.lock().await.clone();

    let mut group_sessions = state.group_sessions.lock().await;
    let info = group_sessions.create_group(name, local_peer_id.clone(), None);

    // Get our sender key distribution to share with invited members
    let session = group_sessions
        .get_session(&info.group_id)
        .ok_or_else(|| "Failed to get newly created group session".to_string())?;
    let our_distribution = session.get_my_distribution();

    drop(group_sessions);

    // If member_peer_ids provided, send invitations via WRAITH protocol
    if let Some(peer_ids) = member_peer_ids {
        log::info!(
            "Group {} created, inviting {} members",
            info.group_id,
            peer_ids.len()
        );

        // Prepare invitation payload
        let invitation = serde_json::json!({
            "type": "group_invitation",
            "group_id": info.group_id,
            "group_name": info.name,
            "inviter_peer_id": local_peer_id,
            "sender_key_distribution": {
                "generation": our_distribution.generation,
                "chain_key": hex::encode(&our_distribution.chain_key),
                "iteration": our_distribution.iteration,
                "signing_key": hex::encode(&our_distribution.signing_key),
            },
            "timestamp": chrono::Utc::now().to_rfc3339(),
        });
        let invitation_bytes = serde_json::to_vec(&invitation)
            .map_err(|e| format!("Failed to serialize invitation: {}", e))?;

        // Send to each invited peer
        let node = state.node.lock().await;
        if node.is_running() {
            for peer_id_hex in &peer_ids {
                // Parse peer ID from hex
                match hex::decode(peer_id_hex) {
                    Ok(bytes) if bytes.len() == 32 => {
                        let mut peer_id_bytes = [0u8; 32];
                        peer_id_bytes.copy_from_slice(&bytes);

                        match node.send_data(&peer_id_bytes, &invitation_bytes).await {
                            Ok(()) => {
                                log::info!(
                                    "Sent group invitation for {} to peer {}",
                                    info.group_id,
                                    &peer_id_hex[..16]
                                );
                            }
                            Err(e) => {
                                log::warn!(
                                    "Failed to send invitation to {}: {}",
                                    &peer_id_hex[..16],
                                    e
                                );
                            }
                        }
                    }
                    Ok(_) => {
                        log::warn!("Invalid peer ID length for {}", peer_id_hex);
                    }
                    Err(e) => {
                        log::warn!("Invalid peer ID hex {}: {}", peer_id_hex, e);
                    }
                }
            }
        } else {
            log::warn!("WRAITH node not running, group invitations not sent");
        }
    }

    // Also create a conversation for this group
    let db = state.db.lock().await;
    let conv = NewConversation {
        conv_type: "group".to_string(),
        peer_id: None,
        group_id: Some(info.group_id.clone()),
        display_name: Some(info.name.clone()),
    };
    db.create_conversation(&conv).map_err(|e| e.to_string())?;

    Ok(info)
}

/// Get group information
#[tauri::command]
pub async fn get_group_info(
    state: State<'_, Arc<AppState>>,
    group_id: String,
) -> Result<Option<GroupInfo>, String> {
    let group_sessions = state.group_sessions.lock().await;
    Ok(group_sessions.get_session(&group_id).map(|s| s.get_info()))
}

/// Update group settings (name, description, avatar)
#[tauri::command]
pub async fn update_group_settings(
    state: State<'_, Arc<AppState>>,
    group_id: String,
    name: Option<String>,
    description: Option<String>,
    avatar: Option<Vec<u8>>,
) -> Result<GroupInfo, String> {
    let mut group_sessions = state.group_sessions.lock().await;
    let session = group_sessions
        .get_session_mut(&group_id)
        .ok_or_else(|| format!("Group not found: {}", group_id))?;

    // Only admins can update settings
    if !session.am_i_admin() {
        return Err("Only admins can update group settings".to_string());
    }

    session.update_settings(name, description, avatar);

    Ok(session.get_info())
}

/// Add a member to a group
#[tauri::command]
pub async fn add_group_member(
    state: State<'_, Arc<AppState>>,
    group_id: String,
    peer_id: String,
    display_name: Option<String>,
) -> Result<GroupMember, String> {
    let mut group_sessions = state.group_sessions.lock().await;
    let session = group_sessions
        .get_session_mut(&group_id)
        .ok_or_else(|| format!("Group not found: {}", group_id))?;

    // Only admins can add members
    if !session.am_i_admin() {
        return Err("Only admins can add members".to_string());
    }

    // Create a placeholder distribution - in real usage, we'd receive this from the peer
    let placeholder_dist = SenderKeyDistribution {
        generation: 0,
        chain_key: vec![0u8; 32],
        iteration: 0,
        signing_key: vec![0u8; 32],
    };

    session
        .add_member_key(
            &peer_id,
            placeholder_dist,
            display_name.clone(),
            GroupRole::Member,
        )
        .map_err(|e| e.to_string())?;

    // Also add to database
    let db = state.db.lock().await;
    db.conn
        .execute(
            "INSERT OR REPLACE INTO group_members (group_id, peer_id, role, joined_at)
             VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![group_id, peer_id, "member", chrono::Utc::now().timestamp()],
        )
        .map_err(|e| e.to_string())?;

    Ok(GroupMember {
        peer_id: peer_id.clone(),
        display_name,
        role: GroupRole::Member,
        joined_at: chrono::Utc::now().timestamp(),
        key_generation: 0,
    })
}

/// Remove a member from a group
#[tauri::command]
pub async fn remove_group_member(
    state: State<'_, Arc<AppState>>,
    group_id: String,
    peer_id: String,
) -> Result<(), String> {
    let mut group_sessions = state.group_sessions.lock().await;
    let session = group_sessions
        .get_session_mut(&group_id)
        .ok_or_else(|| format!("Group not found: {}", group_id))?;

    // Only admins can remove members
    if !session.am_i_admin() {
        return Err("Only admins can remove members".to_string());
    }

    session.remove_member(&peer_id).map_err(|e| e.to_string())?;

    // Also remove from database
    let db = state.db.lock().await;
    db.conn
        .execute(
            "DELETE FROM group_members WHERE group_id = ?1 AND peer_id = ?2",
            rusqlite::params![group_id, peer_id],
        )
        .map_err(|e| e.to_string())?;

    Ok(())
}

/// Leave a group
#[tauri::command]
pub async fn leave_group(state: State<'_, Arc<AppState>>, group_id: String) -> Result<(), String> {
    let mut group_sessions = state.group_sessions.lock().await;

    // Remove the session
    group_sessions.remove_session(&group_id);

    // Archive the conversation
    let db = state.db.lock().await;
    db.conn
        .execute(
            "UPDATE conversations SET archived = 1 WHERE group_id = ?1",
            rusqlite::params![group_id],
        )
        .map_err(|e| e.to_string())?;

    Ok(())
}

/// Promote a member to admin
#[tauri::command]
pub async fn promote_to_admin(
    state: State<'_, Arc<AppState>>,
    group_id: String,
    peer_id: String,
) -> Result<(), String> {
    let mut group_sessions = state.group_sessions.lock().await;
    let session = group_sessions
        .get_session_mut(&group_id)
        .ok_or_else(|| format!("Group not found: {}", group_id))?;

    // Only admins can promote
    if !session.am_i_admin() {
        return Err("Only admins can promote members".to_string());
    }

    session
        .promote_to_admin(&peer_id)
        .map_err(|e| e.to_string())?;

    // Update database
    let db = state.db.lock().await;
    db.conn
        .execute(
            "UPDATE group_members SET role = 'admin' WHERE group_id = ?1 AND peer_id = ?2",
            rusqlite::params![group_id, peer_id],
        )
        .map_err(|e| e.to_string())?;

    Ok(())
}

/// Demote an admin to member
#[tauri::command]
pub async fn demote_from_admin(
    state: State<'_, Arc<AppState>>,
    group_id: String,
    peer_id: String,
) -> Result<(), String> {
    let mut group_sessions = state.group_sessions.lock().await;
    let session = group_sessions
        .get_session_mut(&group_id)
        .ok_or_else(|| format!("Group not found: {}", group_id))?;

    // Only admins can demote
    if !session.am_i_admin() {
        return Err("Only admins can demote members".to_string());
    }

    session
        .demote_from_admin(&peer_id)
        .map_err(|e| e.to_string())?;

    // Update database
    let db = state.db.lock().await;
    db.conn
        .execute(
            "UPDATE group_members SET role = 'member' WHERE group_id = ?1 AND peer_id = ?2",
            rusqlite::params![group_id, peer_id],
        )
        .map_err(|e| e.to_string())?;

    Ok(())
}

/// Send a message to a group
#[tauri::command]
pub async fn send_group_message(
    state: State<'_, Arc<AppState>>,
    group_id: String,
    body: String,
) -> Result<i64, String> {
    let local_peer_id = state.local_peer_id.lock().await.clone();

    // Encrypt with sender keys
    let mut group_sessions = state.group_sessions.lock().await;
    let session = group_sessions
        .get_session_mut(&group_id)
        .ok_or_else(|| format!("Group not found: {}", group_id))?;

    let encrypted = session
        .encrypt(body.as_bytes())
        .map_err(|e| e.to_string())?;

    drop(group_sessions);

    // Get the conversation for this group
    let db = state.db.lock().await;
    let conv_id: i64 = db
        .conn
        .query_row(
            "SELECT id FROM conversations WHERE group_id = ?1",
            rusqlite::params![group_id],
            |row| row.get(0),
        )
        .map_err(|e| format!("Group conversation not found: {}", e))?;

    // Store message
    let message = Message {
        id: 0,
        conversation_id: conv_id,
        sender_peer_id: local_peer_id,
        content_type: "text".to_string(),
        body: Some(body),
        media_path: None,
        media_mime_type: None,
        media_size: None,
        timestamp: chrono::Utc::now().timestamp(),
        sent: false,
        delivered: false,
        read_by_me: true,
        expires_at: None,
        direction: "outgoing".to_string(),
    };

    let message_id = db.insert_message(&message).map_err(|e| e.to_string())?;

    // Serialize encrypted message for transport
    let encrypted_bytes =
        serde_json::to_vec(&encrypted).map_err(|e| format!("Serialization error: {}", e))?;

    // Send encrypted message to all group members via WRAITH protocol
    let node = state.node.lock().await;
    let local_peer_id = state.local_peer_id.lock().await.clone();

    if node.is_running() {
        // Re-acquire group session to get members
        let group_sessions = state.group_sessions.lock().await;
        if let Some(session) = group_sessions.get_session(&group_id) {
            let members = session.get_members();
            let mut send_count = 0;
            let mut fail_count = 0;

            for member in members {
                // Skip ourselves
                if member.peer_id == local_peer_id {
                    continue;
                }

                // Parse peer ID from hex
                match hex::decode(&member.peer_id) {
                    Ok(bytes) if bytes.len() == 32 => {
                        let mut peer_id_bytes = [0u8; 32];
                        peer_id_bytes.copy_from_slice(&bytes);

                        match node.send_data(&peer_id_bytes, &encrypted_bytes).await {
                            Ok(()) => {
                                send_count += 1;
                                log::debug!(
                                    "Sent group message to member {}",
                                    &member.peer_id[..16]
                                );
                            }
                            Err(e) => {
                                fail_count += 1;
                                log::warn!(
                                    "Failed to send group message to {}: {}",
                                    &member.peer_id[..16],
                                    e
                                );
                            }
                        }
                    }
                    Ok(_) => {
                        log::warn!("Invalid peer ID length for member {}", member.peer_id);
                        fail_count += 1;
                    }
                    Err(e) => {
                        log::warn!("Invalid peer ID hex for member: {}", e);
                        fail_count += 1;
                    }
                }
            }

            log::info!(
                "Group message {} sent to {}/{} members (group: {})",
                message_id,
                send_count,
                send_count + fail_count,
                group_id
            );
        }
    } else {
        log::warn!("WRAITH node not running, group message saved but not sent");
    }

    // Mark as sent (message saved locally regardless of send status)
    db.mark_message_sent(message_id)
        .map_err(|e| e.to_string())?;

    Ok(message_id)
}

/// Get members of a group
#[tauri::command]
pub async fn get_group_members(
    state: State<'_, Arc<AppState>>,
    group_id: String,
) -> Result<Vec<GroupMember>, String> {
    let group_sessions = state.group_sessions.lock().await;
    let session = group_sessions
        .get_session(&group_id)
        .ok_or_else(|| format!("Group not found: {}", group_id))?;

    Ok(session.get_members().into_iter().cloned().collect())
}

/// Manually trigger key rotation for a group
#[tauri::command]
pub async fn rotate_group_keys(
    state: State<'_, Arc<AppState>>,
    group_id: String,
) -> Result<(), String> {
    let mut group_sessions = state.group_sessions.lock().await;
    let session = group_sessions
        .get_session_mut(&group_id)
        .ok_or_else(|| format!("Group not found: {}", group_id))?;

    // Only admins can manually rotate keys
    if !session.am_i_admin() {
        return Err("Only admins can rotate group keys".to_string());
    }

    session.rotate_sender_key();

    // Get the new sender key distribution to share with members
    let new_distribution = session.get_my_distribution();
    let members: Vec<_> = session
        .get_members()
        .iter()
        .map(|m| m.peer_id.clone())
        .collect();
    let local_peer_id = state.local_peer_id.lock().await.clone();

    drop(group_sessions);

    // Distribute new sender key to all members via WRAITH protocol
    let distribution_message = serde_json::json!({
        "type": "sender_key_distribution",
        "group_id": group_id,
        "sender_peer_id": local_peer_id,
        "distribution": {
            "generation": new_distribution.generation,
            "chain_key": hex::encode(&new_distribution.chain_key),
            "iteration": new_distribution.iteration,
            "signing_key": hex::encode(&new_distribution.signing_key),
        },
        "timestamp": chrono::Utc::now().to_rfc3339(),
    });
    let distribution_bytes = serde_json::to_vec(&distribution_message)
        .map_err(|e| format!("Failed to serialize distribution: {}", e))?;

    let node = state.node.lock().await;
    if node.is_running() {
        let mut send_count = 0;
        let mut fail_count = 0;

        for member_peer_id in &members {
            // Skip ourselves
            if member_peer_id == &local_peer_id {
                continue;
            }

            // Parse peer ID from hex
            match hex::decode(member_peer_id) {
                Ok(bytes) if bytes.len() == 32 => {
                    let mut peer_id_bytes = [0u8; 32];
                    peer_id_bytes.copy_from_slice(&bytes);

                    match node.send_data(&peer_id_bytes, &distribution_bytes).await {
                        Ok(()) => {
                            send_count += 1;
                            log::debug!(
                                "Distributed new sender key to member {}",
                                &member_peer_id[..16.min(member_peer_id.len())]
                            );
                        }
                        Err(e) => {
                            fail_count += 1;
                            log::warn!(
                                "Failed to distribute sender key to {}: {}",
                                &member_peer_id[..16.min(member_peer_id.len())],
                                e
                            );
                        }
                    }
                }
                Ok(_) => {
                    log::warn!("Invalid peer ID length for member {}", member_peer_id);
                    fail_count += 1;
                }
                Err(e) => {
                    log::warn!("Invalid peer ID hex for member: {}", e);
                    fail_count += 1;
                }
            }
        }

        log::info!(
            "Rotated keys for group {}: distributed to {}/{} members",
            group_id,
            send_count,
            send_count + fail_count
        );
    } else {
        log::warn!(
            "WRAITH node not running, sender key distribution for group {} not sent",
            group_id
        );
    }

    Ok(())
}

// MARK: - Video Call Commands (Sprint 17.6)

/// Start a video call with a peer
#[tauri::command]
pub async fn start_video_call(
    state: State<'_, Arc<AppState>>,
    peer_id: String,
    enable_video: bool,
) -> Result<VideoCallInfo, String> {
    state
        .video_calls
        .start_video_call(&peer_id, enable_video)
        .await
        .map_err(|e| e.to_string())
}

/// Answer a video call
#[tauri::command]
pub async fn answer_video_call(
    state: State<'_, Arc<AppState>>,
    call_id: String,
    enable_video: bool,
) -> Result<VideoCallInfo, String> {
    state
        .video_calls
        .answer_video_call(&call_id, enable_video)
        .await
        .map_err(|e| e.to_string())
}

/// End a video call
#[tauri::command]
pub async fn end_video_call(
    state: State<'_, Arc<AppState>>,
    call_id: String,
    reason: Option<String>,
) -> Result<(), String> {
    state
        .video_calls
        .end_video_call(&call_id, &reason.unwrap_or_else(|| "hangup".to_string()))
        .await
        .map_err(|e| e.to_string())
}

/// Enable video during a call
#[tauri::command]
pub async fn enable_video(
    state: State<'_, Arc<AppState>>,
    call_id: String,
    source: String,
) -> Result<(), String> {
    let video_source = match source.as_str() {
        "camera" => VideoSource::Camera,
        "screen" => VideoSource::Screen,
        _ => VideoSource::Camera,
    };

    state
        .video_calls
        .enable_video(&call_id, video_source)
        .await
        .map_err(|e| e.to_string())
}

/// Disable video during a call
#[tauri::command]
pub async fn disable_video(state: State<'_, Arc<AppState>>, call_id: String) -> Result<(), String> {
    state
        .video_calls
        .disable_video(&call_id)
        .await
        .map_err(|e| e.to_string())
}

/// Switch video source (camera to screen or vice versa)
#[tauri::command]
pub async fn switch_video_source(
    state: State<'_, Arc<AppState>>,
    call_id: String,
    source: String,
) -> Result<(), String> {
    let video_source = match source.as_str() {
        "camera" => VideoSource::Camera,
        "screen" => VideoSource::Screen,
        _ => return Err("Invalid video source".to_string()),
    };

    state
        .video_calls
        .switch_video_source(&call_id, video_source)
        .await
        .map_err(|e| e.to_string())
}

/// Switch between front and back camera
#[tauri::command]
pub async fn switch_camera(state: State<'_, Arc<AppState>>, call_id: String) -> Result<(), String> {
    state
        .video_calls
        .switch_camera(&call_id)
        .await
        .map_err(|e| e.to_string())
}

/// Toggle audio mute on a video call
#[tauri::command]
pub async fn toggle_video_mute(
    state: State<'_, Arc<AppState>>,
    call_id: String,
) -> Result<bool, String> {
    state
        .video_calls
        .toggle_mute(&call_id)
        .await
        .map_err(|e| e.to_string())
}

/// Get video call information
#[tauri::command]
pub async fn get_video_call_info(
    state: State<'_, Arc<AppState>>,
    call_id: String,
) -> Result<Option<VideoCallInfo>, String> {
    state
        .video_calls
        .get_call_info(&call_id)
        .await
        .map_err(|e| e.to_string())
}

/// Get all active video calls
#[tauri::command]
pub async fn get_active_video_calls(
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<VideoCallInfo>, String> {
    Ok(state.video_calls.get_active_calls().await)
}

/// List available camera devices
#[tauri::command]
pub async fn list_cameras() -> Result<Vec<CameraDevice>, String> {
    crate::video_call::VideoCallManager::list_cameras().map_err(|e| e.to_string())
}

/// List available screen capture sources
#[tauri::command]
pub async fn list_screen_sources() -> Result<Vec<ScreenSource>, String> {
    crate::video_call::VideoCallManager::list_screen_sources().map_err(|e| e.to_string())
}

/// Select a camera device for a call
#[tauri::command]
pub async fn select_camera(
    state: State<'_, Arc<AppState>>,
    call_id: String,
    device_id: String,
) -> Result<(), String> {
    state
        .video_calls
        .select_camera(&call_id, &device_id)
        .await
        .map_err(|e| e.to_string())
}

/// Select a screen capture source for a call
#[tauri::command]
pub async fn select_screen_source(
    state: State<'_, Arc<AppState>>,
    call_id: String,
    source_id: String,
) -> Result<(), String> {
    state
        .video_calls
        .select_screen_source(&call_id, &source_id)
        .await
        .map_err(|e| e.to_string())
}

/// Set video quality for a call
#[tauri::command]
pub async fn set_video_quality(
    state: State<'_, Arc<AppState>>,
    call_id: String,
    quality: String,
) -> Result<(), String> {
    let resolution = match quality.as_str() {
        "ultra_low" | "240p" => VideoResolution::UltraLow,
        "low" | "360p" => VideoResolution::Low,
        "medium" | "480p" => VideoResolution::Medium,
        "hd" | "720p" => VideoResolution::Hd,
        "full_hd" | "1080p" => VideoResolution::FullHd,
        _ => {
            return Err(
                "Invalid video quality. Use: ultra_low, low, medium, hd, full_hd".to_string(),
            );
        }
    };

    state
        .video_calls
        .set_video_quality(&call_id, resolution)
        .await
        .map_err(|e| e.to_string())
}

/// Request a keyframe from remote (after packet loss)
#[tauri::command]
pub async fn request_keyframe(
    state: State<'_, Arc<AppState>>,
    call_id: String,
) -> Result<(), String> {
    state
        .video_calls
        .request_keyframe(&call_id)
        .await
        .map_err(|e| e.to_string())
}

// MARK: - Statistics Commands (Sprint 18.3)

/// Enhanced statistics with detailed metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnhancedStatistics {
    // Message counts
    /// Total messages (sent + received)
    pub total_messages: u64,
    /// Total conversations
    pub total_conversations: u64,
    /// Messages sent by the user
    pub messages_sent: u64,
    /// Messages received from others
    pub messages_received: u64,

    // Time-based message breakdown
    /// Messages sent today
    pub messages_sent_today: u64,
    /// Messages received today
    pub messages_received_today: u64,
    /// Messages sent this week
    pub messages_sent_week: u64,
    /// Messages received this week
    pub messages_received_week: u64,
    /// Messages sent this month
    pub messages_sent_month: u64,
    /// Messages received this month
    pub messages_received_month: u64,

    // Latency metrics
    /// Average message delivery latency in milliseconds
    pub average_latency_ms: Option<f64>,

    // Call statistics
    /// Total voice call duration in seconds
    pub total_voice_call_duration_secs: u64,
    /// Number of voice calls completed
    pub voice_call_count: u64,
    /// Average voice call duration in seconds
    pub average_voice_call_duration_secs: Option<f64>,
    /// Total video call duration in seconds
    pub total_video_call_duration_secs: u64,
    /// Number of video calls completed
    pub video_call_count: u64,
    /// Average video call duration in seconds
    pub average_video_call_duration_secs: Option<f64>,
    /// Total call duration (voice + video) in seconds
    pub total_call_duration_secs: u64,
    /// Average call duration across all calls in seconds
    pub average_call_duration_secs: Option<f64>,

    // Group statistics
    /// Number of groups
    pub total_groups: u64,
    /// Activity statistics per group
    pub group_stats: Vec<GroupActivityStats>,

    // Security metrics
    /// Number of encryption key rotations (Double Ratchet + group keys)
    pub key_rotations: u64,
    /// Number of peer sessions with encryption keys
    pub active_peer_sessions: u64,

    // Storage usage
    /// Storage breakdown by category
    pub storage_bytes: StorageBreakdown,

    // Timestamps
    /// When statistics were generated (RFC 3339)
    pub generated_at: String,
}

/// Get comprehensive chat statistics
#[tauri::command]
pub async fn get_statistics(state: State<'_, Arc<AppState>>) -> Result<EnhancedStatistics, String> {
    let db = state.db.lock().await;
    let now = chrono::Utc::now();

    // Calculate time boundaries
    let today_start = now
        .date_naive()
        .and_hms_opt(0, 0, 0)
        .unwrap()
        .and_utc()
        .timestamp();
    let week_start = (now - chrono::Duration::days(7)).timestamp();
    let month_start = (now - chrono::Duration::days(30)).timestamp();

    // Get message counts
    let total_messages = db.count_messages().map_err(|e| e.to_string())?;
    let messages_sent = db
        .count_messages_by_direction("outgoing")
        .map_err(|e| e.to_string())?;
    let messages_received = db
        .count_messages_by_direction("incoming")
        .map_err(|e| e.to_string())?;

    // Get time-based counts
    let messages_sent_today = db
        .count_messages_since("outgoing", today_start)
        .map_err(|e| e.to_string())?;
    let messages_received_today = db
        .count_messages_since("incoming", today_start)
        .map_err(|e| e.to_string())?;
    let messages_sent_week = db
        .count_messages_since("outgoing", week_start)
        .map_err(|e| e.to_string())?;
    let messages_received_week = db
        .count_messages_since("incoming", week_start)
        .map_err(|e| e.to_string())?;
    let messages_sent_month = db
        .count_messages_since("outgoing", month_start)
        .map_err(|e| e.to_string())?;
    let messages_received_month = db
        .count_messages_since("incoming", month_start)
        .map_err(|e| e.to_string())?;

    // Get conversation counts
    let total_conversations = db.count_conversations().map_err(|e| e.to_string())? as u64;
    let total_groups = db.count_group_conversations().map_err(|e| e.to_string())? as u64;

    // Get group activity stats
    let group_stats = db.get_group_activity_stats().map_err(|e| e.to_string())?;

    // Get storage breakdown
    let storage_bytes = db.get_storage_breakdown().map_err(|e| e.to_string())?;

    // Get peer session count (ratchet states)
    let active_peer_sessions = db.count_ratchet_states().map_err(|e| e.to_string())?;

    // Get runtime statistics from the tracker
    let stats_tracker = &state.statistics;

    Ok(EnhancedStatistics {
        total_messages,
        total_conversations,
        messages_sent,
        messages_received,
        messages_sent_today,
        messages_received_today,
        messages_sent_week,
        messages_received_week,
        messages_sent_month,
        messages_received_month,
        average_latency_ms: stats_tracker.average_latency_ms(),
        total_voice_call_duration_secs: stats_tracker.total_voice_call_duration_secs(),
        voice_call_count: stats_tracker.voice_call_count(),
        average_voice_call_duration_secs: stats_tracker.average_voice_call_duration_secs(),
        total_video_call_duration_secs: stats_tracker.total_video_call_duration_secs(),
        video_call_count: stats_tracker.video_call_count(),
        average_video_call_duration_secs: stats_tracker.average_video_call_duration_secs(),
        total_call_duration_secs: stats_tracker.total_call_duration_secs(),
        average_call_duration_secs: stats_tracker.average_call_duration_secs(),
        total_groups,
        group_stats,
        key_rotations: stats_tracker.key_rotation_count(),
        active_peer_sessions,
        storage_bytes,
        generated_at: now.to_rfc3339(),
    })
}

/// Record a message delivery latency (called internally when message is confirmed delivered)
#[tauri::command]
pub async fn record_message_latency(
    state: State<'_, Arc<AppState>>,
    latency_ms: u64,
) -> Result<(), String> {
    state.statistics.record_latency(latency_ms);
    Ok(())
}

/// Record a completed voice call (called when a voice call ends)
#[tauri::command]
pub async fn record_voice_call_completed(
    state: State<'_, Arc<AppState>>,
    duration_secs: u64,
) -> Result<(), String> {
    state.statistics.record_voice_call(duration_secs);
    Ok(())
}

/// Record a completed video call (called when a video call ends)
#[tauri::command]
pub async fn record_video_call_completed(
    state: State<'_, Arc<AppState>>,
    duration_secs: u64,
) -> Result<(), String> {
    state.statistics.record_video_call(duration_secs);
    Ok(())
}

/// Record an encryption key rotation (called when keys are rotated)
#[tauri::command]
pub async fn record_key_rotation(state: State<'_, Arc<AppState>>) -> Result<(), String> {
    state.statistics.record_key_rotation();
    Ok(())
}
