use dashmap::DashMap;
use std::sync::Arc;
use std::time::SystemTime;
use wraith_crypto::noise::{NoiseHandshake, NoiseTransport};
use wraith_crypto::x25519::PrivateKey;

pub struct TrackedSession {
    pub transport: NoiseTransport,
    pub packet_count: u64,
    pub last_rekey: SystemTime,
}

impl TrackedSession {
    pub fn new(transport: NoiseTransport) -> Self {
        Self {
            transport,
            packet_count: 0,
            last_rekey: SystemTime::now(),
        }
    }

    pub fn should_rekey(&self) -> bool {
        let elapsed = self
            .last_rekey
            .elapsed()
            .unwrap_or(std::time::Duration::from_secs(0));
        self.packet_count >= 1_000_000 || elapsed >= std::time::Duration::from_secs(120)
    }

    pub fn on_packet(&mut self) {
        self.packet_count += 1;
    }

    pub fn on_rekey(&mut self) {
        let _ = self.transport.rekey_dh();
        self.packet_count = 0;
        self.last_rekey = SystemTime::now();
    }
}

#[derive(Clone)]
pub struct SessionManager {
    // Map temporary CIDs (from handshake) to pending handshakes and ratchet key
    pub handshakes: Arc<DashMap<[u8; 8], (NoiseHandshake, PrivateKey)>>,
    // Map session CIDs to established transports
    pub sessions: Arc<DashMap<[u8; 8], TrackedSession>>,
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
        self.sessions.insert(cid, TrackedSession::new(transport));
    }

    pub fn get_session(
        &self,
        cid: &[u8; 8],
    ) -> Option<dashmap::mapref::one::RefMut<'_, [u8; 8], TrackedSession>> {
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

    #[test]
    fn test_session_manager_p2p_links() {
        let manager = SessionManager::new();
        let downstream = [1u8; 8];
        let upstream = [2u8; 8];

        assert!(manager.get_upstream_cid(&downstream).is_none());

        manager.insert_p2p_link(downstream, upstream);
        assert_eq!(manager.get_upstream_cid(&downstream), Some(upstream));

        // Non-existent downstream
        let other = [3u8; 8];
        assert!(manager.get_upstream_cid(&other).is_none());
    }

    #[test]
    fn test_session_manager_get_session_empty() {
        let manager = SessionManager::new();
        let cid = [1u8; 8];
        assert!(manager.get_session(&cid).is_none());
    }

    #[test]
    fn test_session_manager_multiple_handshakes() {
        let manager = SessionManager::new();

        for i in 0..5u8 {
            let cid = [i; 8];
            let keypair = NoiseKeypair::generate().unwrap();
            let handshake = wraith_crypto::noise::NoiseHandshake::new_initiator(&keypair).unwrap();
            let mut rng = SecureRng::new();
            let priv_key = PrivateKey::generate(&mut rng);
            manager.insert_handshake(cid, handshake, priv_key);
        }

        // All 5 should exist
        for i in 0..5u8 {
            let cid = [i; 8];
            assert!(manager.remove_handshake(&cid).is_some());
        }

        // All should now be gone
        for i in 0..5u8 {
            let cid = [i; 8];
            assert!(manager.remove_handshake(&cid).is_none());
        }
    }

    #[test]
    fn test_tracked_session_new() {
        let initiator_keypair = NoiseKeypair::generate().unwrap();
        let responder_keypair = NoiseKeypair::generate().unwrap();

        let mut initiator =
            wraith_crypto::noise::NoiseHandshake::new_initiator(&initiator_keypair).unwrap();
        let mut responder =
            wraith_crypto::noise::NoiseHandshake::new_responder(&responder_keypair).unwrap();

        let msg1 = initiator.write_message(&[]).unwrap();
        let _ = responder.read_message(&msg1).unwrap();
        let msg2 = responder.write_message(&[]).unwrap();
        let _ = initiator.read_message(&msg2).unwrap();
        let msg3 = initiator.write_message(&[]).unwrap();
        let _ = responder.read_message(&msg3).unwrap();

        let mut rng = SecureRng::new();
        let resp_ratchet_priv = PrivateKey::generate(&mut rng);
        let resp_ratchet_pub = resp_ratchet_priv.public_key();

        let transport = initiator
            .into_transport(None, Some(resp_ratchet_pub))
            .unwrap();
        let session = TrackedSession::new(transport);

        assert_eq!(session.packet_count, 0);
        assert!(!session.should_rekey());
    }
}
