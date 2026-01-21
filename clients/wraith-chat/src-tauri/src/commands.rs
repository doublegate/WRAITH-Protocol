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
    let db = state.db.lock().await;

    // Get or create ratchet for this peer
    let mut ratchets = state.ratchets.lock().await;
    let ratchet = ratchets.entry(peer_id.clone()).or_insert_with(|| {
        // TODO: Initialize with shared secret from key agreement
        let shared_secret = [0u8; 32]; // Placeholder
        DoubleRatchet::new(&shared_secret, None).unwrap()
    });

    // Encrypt message with Double Ratchet
    let _encrypted = ratchet
        .encrypt(body.as_bytes())
        .map_err(|e| e.to_string())?;

    // Save ratchet state
    let ratchet_json = ratchet.to_json().map_err(|e| e.to_string())?;
    db.save_ratchet_state(&peer_id, &ratchet_json)
        .map_err(|e| e.to_string())?;

    // Create message record
    let message = Message {
        id: 0,
        conversation_id,
        sender_peer_id: state.local_peer_id.lock().await.clone(),
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

    // TODO: Send encrypted message via WRAITH protocol
    // let node = state.node.lock().await;
    // node.send_message(&peer_id, &encrypted)?;

    // Mark as sent (for now, immediately mark as sent)
    // In production, this would be updated after WRAITH protocol confirms delivery

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
    // TODO: Initialize WRAITH node
    log::info!("Starting WRAITH node on {}", listen_addr);

    let mut peer_id = state.local_peer_id.lock().await;
    *peer_id = "local-peer-id-placeholder".to_string(); // TODO: Get from node

    Ok(())
}

#[tauri::command]
pub async fn get_node_status(state: State<'_, Arc<AppState>>) -> Result<NodeStatus, String> {
    let peer_id = state.local_peer_id.lock().await;

    // Get session count from active ratchets (cryptographic sessions)
    let ratchets = state.ratchets.lock().await;
    let session_count = ratchets.len();

    // Get conversation count from database
    let db = state.db.lock().await;
    let active_conversations = db.count_conversations().unwrap_or(0);

    Ok(NodeStatus {
        running: !peer_id.is_empty(),
        local_peer_id: peer_id.clone(),
        session_count,
        active_conversations,
    })
}

// MARK: - Helper Functions

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
