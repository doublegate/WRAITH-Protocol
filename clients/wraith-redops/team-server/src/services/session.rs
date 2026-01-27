use dashmap::DashMap;
use std::sync::Arc;
use wraith_crypto::noise::{NoiseHandshake, NoiseTransport};
use wraith_crypto::x25519::PrivateKey;

#[derive(Clone)]
pub struct SessionManager {
    // Map temporary CIDs (from handshake) to pending handshakes and ratchet key
    pub handshakes: Arc<DashMap<[u8; 8], (NoiseHandshake, PrivateKey)>>,
    // Map session CIDs to established transports
    pub sessions: Arc<DashMap<[u8; 8], NoiseTransport>>,
    // Map downstream CID to upstream CID for mesh routing
    pub p2p_links: Arc<DashMap<[u8; 8], [u8; 8]>>,
}

impl SessionManager {
    pub fn new() -> Self {
        Self {
            handshakes: Arc::new(DashMap::new()),
            sessions: Arc::new(DashMap::new()),
            p2p_links: Arc::new(DashMap::new()),
        }
    }

    #[allow(dead_code)]
    pub fn insert_p2p_link(&self, downstream: [u8; 8], upstream: [u8; 8]) {
        self.p2p_links.insert(downstream, upstream);
    }

    pub fn get_upstream_cid(&self, downstream: &[u8; 8]) -> Option<[u8; 8]> {
        self.p2p_links.get(downstream).map(|cid| *cid)
    }

    pub fn insert_handshake(&self, cid: [u8; 8], handshake: NoiseHandshake, priv_key: PrivateKey) {
        self.handshakes.insert(cid, (handshake, priv_key));
    }

    pub fn remove_handshake(&self, cid: &[u8; 8]) -> Option<(NoiseHandshake, PrivateKey)> {
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
    use wraith_crypto::random::SecureRng;

    #[test]
    fn test_session_manager() {
        let manager = SessionManager::new();
        let cid = [1u8; 8];

        let keypair = NoiseKeypair::generate().expect("Test keypair generation failed");
        let handshake = wraith_crypto::noise::NoiseHandshake::new_initiator(&keypair)
            .expect("Test handshake creation failed");

        let mut rng = SecureRng::new();
        let priv_key = PrivateKey::generate(&mut rng);

        manager.insert_handshake(cid, handshake, priv_key);
        assert!(manager.remove_handshake(&cid).is_some());
        assert!(manager.remove_handshake(&cid).is_none());
    }
}
