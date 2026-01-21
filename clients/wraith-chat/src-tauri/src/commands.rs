// Tauri IPC Commands for WRAITH-Chat

use crate::crypto::{DoubleRatchet, EncryptedMessage};
use crate::database::{Contact, Conversation, Message, NewConversation};
use crate::state::AppState;
use std::sync::Arc;
use tauri::State;

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
        sender_peer_id: peer_id,
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

    // TODO: Emit event to frontend to update UI

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
