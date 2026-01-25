use dashmap::DashMap;
use std::sync::Arc;
use wraith_crypto::noise::{NoiseHandshake, NoiseTransport};

#[derive(Clone)]
pub struct SessionManager {
    // Map temporary CIDs (from handshake) to pending handshakes
    pub handshakes: Arc<DashMap<[u8; 8], NoiseHandshake>>,
    // Map session CIDs to established transports
    pub sessions: Arc<DashMap<[u8; 8], NoiseTransport>>,
}

impl SessionManager {
    pub fn new() -> Self {
        Self {
            handshakes: Arc::new(DashMap::new()),
            sessions: Arc::new(DashMap::new()),
        }
    }

    pub fn insert_handshake(&self, cid: [u8; 8], handshake: NoiseHandshake) {
        self.handshakes.insert(cid, handshake);
    }

    pub fn remove_handshake(&self, cid: &[u8; 8]) -> Option<NoiseHandshake> {
        self.handshakes.remove(cid).map(|(_, h)| h)
    }

    pub fn insert_session(&self, cid: [u8; 8], transport: NoiseTransport) {
        self.sessions.insert(cid, transport);
    }

    pub fn get_session(
        &self,
        cid: &[u8; 8],
    ) -> Option<dashmap::mapref::one::RefMut<'_, [u8; 8], NoiseTransport>> {
        self.sessions.get_mut(cid)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wraith_crypto::noise::NoiseKeypair;

    #[test]
    fn test_session_manager() {
        let manager = SessionManager::new();
        let cid = [1u8; 8];

        let keypair = NoiseKeypair::generate().expect("Test keypair generation failed");
        let handshake = wraith_crypto::noise::NoiseHandshake::new_initiator(&keypair)
            .expect("Test handshake creation failed");

        manager.insert_handshake(cid, handshake);
        assert!(manager.remove_handshake(&cid).is_some());
        assert!(manager.remove_handshake(&cid).is_none());
    }
}
