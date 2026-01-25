use tokio::net::UdpSocket;

pub async fn broadcast_kill_signal(port: u16, secret: &[u8]) -> std::io::Result<()> {
    let socket = UdpSocket::bind("0.0.0.0:0").await?;
    socket.set_broadcast(true)?;
    
    let target = format!("255.255.255.255:{}", port);
    // Payload: MAGIC + SECRET (Simulated signature)
    let mut payload = Vec::new();
    payload.extend_from_slice(b"WRAITH_KILL");
    payload.extend_from_slice(secret);
    
    tracing::warn!("BROADCASTING KILL SIGNAL to {}", target);
    socket.send_to(&payload, target).await?;
    Ok(())
}
