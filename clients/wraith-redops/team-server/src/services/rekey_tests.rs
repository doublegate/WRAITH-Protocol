#[cfg(test)]
mod tests {
    use super::super::session::TrackedSession;
    use std::time::{Duration, SystemTime};
    use wraith_crypto::noise::{NoiseHandshake, NoiseKeypair};
    use wraith_crypto::x25519::PrivateKey;
    use wraith_crypto::random::SecureRng;

    fn create_dummy_session() -> TrackedSession {
        // Create a real transport for testing
        let initiator_keypair = NoiseKeypair::generate().unwrap();
        let responder_keypair = NoiseKeypair::generate().unwrap();

        let mut initiator = NoiseHandshake::new_initiator(&initiator_keypair).unwrap();
        let mut responder = NoiseHandshake::new_responder(&responder_keypair).unwrap();

        let msg1 = initiator.write_message(&[]).unwrap();
        let _ = responder.read_message(&msg1).unwrap();

        let msg2 = responder.write_message(&[]).unwrap();
        let _ = initiator.read_message(&msg2).unwrap();

        let msg3 = initiator.write_message(&[]).unwrap();
        let _ = responder.read_message(&msg3).unwrap();

        let mut rng = SecureRng::new();
        let resp_ratchet_priv = PrivateKey::generate(&mut rng);
        let resp_ratchet_pub = resp_ratchet_priv.public_key();

        let initiator_transport = initiator
            .into_transport(None, Some(resp_ratchet_pub))
            .unwrap();

        TrackedSession::new(initiator_transport)
    }

    #[test]
    fn test_rekey_trigger_packet_limit() {
        let mut session = create_dummy_session();
        session.packet_count = 999_999;
        assert!(!session.should_rekey(), "Should not rekey at 999,999 packets");
        
        session.on_packet(); // 1,000,000
        assert!(session.should_rekey(), "Should rekey at 1,000,000 packets");
        
        session.on_rekey();
        assert!(!session.should_rekey(), "Should reset after rekey");
        assert_eq!(session.packet_count, 0);
    }

    #[test]
    fn test_rekey_trigger_time_limit() {
        let mut session = create_dummy_session();
        // Simulate time passing: 121 seconds ago
        session.last_rekey = SystemTime::now() - Duration::from_secs(121);
        assert!(session.should_rekey(), "Should rekey after 121 seconds");
        
        session.on_rekey();
        assert!(!session.should_rekey(), "Should reset after rekey");
        // Verify time updated (within tolerance)
        assert!(session.last_rekey.elapsed().unwrap().as_secs() < 1);
    }

    #[test]
    fn test_rekey_counters_increment() {
        let mut session = create_dummy_session();
        session.on_packet();
        session.on_packet();
        assert_eq!(session.packet_count, 2);
    }
}