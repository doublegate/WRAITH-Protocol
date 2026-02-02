use std::env;
use tokio::net::UdpSocket;
use wraith_crypto::signatures::SigningKey;

pub async fn broadcast_kill_signal(port: u16, secret_msg: &[u8]) -> std::io::Result<()> {
    let socket = UdpSocket::bind("0.0.0.0:0").await?;
    socket.set_broadcast(true)?;

    let target = format!("255.255.255.255:{}", port);

    // Construct Payload: [MAGIC: 8] + [TIMESTAMP: 8] + [SECRET: N]
    // MAGIC "WRAITH_K" is 8 bytes
    let magic = b"WRAITH_K";
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let mut data = Vec::new();
    data.extend_from_slice(magic);
    data.extend_from_slice(&timestamp.to_be_bytes());
    data.extend_from_slice(secret_msg);

    // Sign it
    let seed_hex = env::var("KILLSWITCH_KEY").expect("KILLSWITCH_KEY env var must be set");
    let seed = hex::decode(&seed_hex).expect("Failed to decode KILLSWITCH_KEY");
    let key_bytes: [u8; 32] = seed.try_into().expect("KILLSWITCH_KEY must be 32 bytes");

    let key = SigningKey::from_bytes(&key_bytes);
    let signature = key.sign(&data);

    // Final Payload: [SIGNATURE: 64] + [DATA]
    let mut payload = Vec::new();
    payload.extend_from_slice(signature.as_bytes());
    payload.extend_from_slice(&data);

    tracing::warn!("BROADCASTING SIGNED KILL SIGNAL to {}", target);
    socket.send_to(&payload, target).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use wraith_crypto::signatures::VerifyingKey;

    #[test]
    fn test_kill_signal_structure() {
        let dummy_key = "0000000000000000000000000000000000000000000000000000000000000000";
        unsafe {
            std::env::set_var("KILLSWITCH_KEY", dummy_key);
        }

        let seed = hex::decode(dummy_key).unwrap();
        let key_bytes: [u8; 32] = seed.try_into().unwrap();
        let key = SigningKey::from_bytes(&key_bytes);
        let msg = b"TEST";

        // This just verifies we can sign without panic
        let _sig = key.sign(msg);
    }

    #[test]
    fn test_kill_signal_payload_construction() {
        let magic = b"WRAITH_K";
        let secret_msg = b"KILL_ALL";

        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let mut data = Vec::new();
        data.extend_from_slice(magic);
        data.extend_from_slice(&timestamp.to_be_bytes());
        data.extend_from_slice(secret_msg);

        // Verify structure: [MAGIC: 8] + [TIMESTAMP: 8] + [SECRET: N]
        assert_eq!(&data[0..8], magic);
        let extracted_ts = u64::from_be_bytes(data[8..16].try_into().unwrap());
        assert_eq!(extracted_ts, timestamp);
        assert_eq!(&data[16..], secret_msg);
        assert_eq!(data.len(), 8 + 8 + secret_msg.len());
    }

    #[test]
    fn test_kill_signal_signature_verification() {
        let key_hex = "0000000000000000000000000000000000000000000000000000000000000000";
        let seed = hex::decode(key_hex).unwrap();
        let key_bytes: [u8; 32] = seed.try_into().unwrap();
        let signing_key = SigningKey::from_bytes(&key_bytes);

        let data = b"test_payload";
        let signature = signing_key.sign(data);

        // Verify the signature is valid
        let verifying_key = signing_key.verifying_key();
        assert!(verifying_key.verify(data, &signature).is_ok());
    }

    #[test]
    fn test_kill_signal_signature_wrong_data_fails() {
        let key_hex = "0000000000000000000000000000000000000000000000000000000000000000";
        let seed = hex::decode(key_hex).unwrap();
        let key_bytes: [u8; 32] = seed.try_into().unwrap();
        let signing_key = SigningKey::from_bytes(&key_bytes);

        let data = b"correct_data";
        let signature = signing_key.sign(data);

        let verifying_key = signing_key.verifying_key();
        assert!(verifying_key.verify(b"wrong_data", &signature).is_err());
    }

    #[test]
    fn test_kill_signal_full_payload_format() {
        let key_hex = "0101010101010101010101010101010101010101010101010101010101010101";
        let seed = hex::decode(key_hex).unwrap();
        let key_bytes: [u8; 32] = seed.try_into().unwrap();
        let signing_key = SigningKey::from_bytes(&key_bytes);

        let magic = b"WRAITH_K";
        let timestamp: u64 = 1700000000;
        let secret_msg = b"EMERGENCY";

        let mut data = Vec::new();
        data.extend_from_slice(magic);
        data.extend_from_slice(&timestamp.to_be_bytes());
        data.extend_from_slice(secret_msg);

        let signature = signing_key.sign(&data);

        // Final payload: [SIGNATURE: 64] + [DATA]
        let mut payload = Vec::new();
        payload.extend_from_slice(signature.as_bytes());
        payload.extend_from_slice(&data);

        // Verify structure
        assert_eq!(payload.len(), 64 + 8 + 8 + secret_msg.len());

        // Parse back and verify
        let sig_bytes = &payload[..64];
        let data_bytes = &payload[64..];
        assert_eq!(&data_bytes[0..8], magic);

        let verifying_key = signing_key.verifying_key();
        let parsed_sig = wraith_crypto::signatures::Signature::from_slice(sig_bytes).unwrap();
        assert!(verifying_key.verify(data_bytes, &parsed_sig).is_ok());
    }

    #[test]
    fn test_kill_signal_magic_constant() {
        let magic = b"WRAITH_K";
        assert_eq!(magic.len(), 8);
        assert_eq!(magic, b"WRAITH_K");
    }
}
