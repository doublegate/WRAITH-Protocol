use tokio::net::UdpSocket;
use wraith_crypto::signatures::SigningKey;

// Hardcoded seed for killswitch (In production, this comes from secure storage or hardware token)
const KILL_KEY_SEED: [u8; 32] = *b"kill_switch_master_key_seed_0000";

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
    let key = SigningKey::from_bytes(&KILL_KEY_SEED);
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
    use wraith_crypto::signatures::SigningKey;

    #[test]
    fn test_kill_signal_structure() {
        let key = SigningKey::from_bytes(&KILL_KEY_SEED);
        let msg = b"TEST";
        
        // This just verifies we can sign without panic
        let _sig = key.sign(msg);
    }
}