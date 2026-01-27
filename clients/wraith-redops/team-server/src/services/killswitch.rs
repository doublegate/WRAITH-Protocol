use tokio::net::UdpSocket;
use wraith_crypto::signatures::SigningKey;
use std::env;

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
}