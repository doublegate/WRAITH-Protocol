//! Integration tests for v3 Phase 3 QUIC transport.

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::timeout;
use wraith_transport::factory::TransportType;
use wraith_transport::quic::{QuicConfig, QuicTransport};
use wraith_transport::transport::{Transport, TransportError};

const TEST_TIMEOUT: Duration = Duration::from_secs(10);

async fn new_quic() -> QuicTransport {
    let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
    QuicTransport::bind(addr).await.unwrap()
}

// ---------------------------------------------------------------------------
// Trait method values
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_quic_transport_type_returns_quic() {
    let t = new_quic().await;
    assert_eq!(t.transport_type(), TransportType::Quic);
}

#[tokio::test]
async fn test_quic_supports_migration_true() {
    let t = new_quic().await;
    assert!(t.supports_migration());
}

#[tokio::test]
async fn test_quic_mtu_is_1200() {
    let t = new_quic().await;
    assert_eq!(t.mtu(), 1200);
}

#[tokio::test]
async fn test_quic_latency_estimate() {
    let t = new_quic().await;
    assert_eq!(t.latency_estimate(), Duration::from_micros(500));
}

// ---------------------------------------------------------------------------
// QuicConfig defaults
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_quic_config_defaults() {
    let config = QuicConfig::default();
    assert!(config.accept_self_signed);
    assert!(config.server_cert_chain.is_empty());
    assert!(config.server_key_der.is_empty());
    assert_eq!(config.keep_alive, Some(Duration::from_secs(15)));
}

// ---------------------------------------------------------------------------
// Send / recv round-trip
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_quic_send_recv_roundtrip() {
    let server = Arc::new(new_quic().await);
    let server_addr = server.local_addr().unwrap();
    let client = new_quic().await;

    let srv = server.clone();
    let h = tokio::spawn(async move {
        let mut buf = vec![0u8; 1500];
        let (n, _) = timeout(TEST_TIMEOUT, srv.recv_from(&mut buf))
            .await
            .expect("timeout")
            .unwrap();
        buf.truncate(n);
        buf
    });
    tokio::time::sleep(Duration::from_millis(100)).await;

    let sent = client
        .send_to(b"Hello QUIC integration!", server_addr)
        .await
        .unwrap();
    assert_eq!(sent, 23);

    let received = h.await.unwrap();
    assert_eq!(&received, b"Hello QUIC integration!");
}

// ---------------------------------------------------------------------------
// Stats tracking
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_quic_stats_tracking() {
    let server = Arc::new(new_quic().await);
    let server_addr = server.local_addr().unwrap();
    let client = new_quic().await;

    let srv = server.clone();
    let h = tokio::spawn(async move {
        let mut buf = vec![0u8; 1500];
        timeout(TEST_TIMEOUT, srv.recv_from(&mut buf))
            .await
            .expect("timeout")
            .unwrap()
    });
    tokio::time::sleep(Duration::from_millis(100)).await;

    client.send_to(b"qstats", server_addr).await.unwrap();
    h.await.unwrap();

    let cs = client.stats();
    assert_eq!(cs.packets_sent, 1);
    assert_eq!(cs.bytes_sent, 6);

    let ss = server.stats();
    assert_eq!(ss.packets_received, 1);
    assert_eq!(ss.bytes_received, 6);
}

// ---------------------------------------------------------------------------
// Close behaviour
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_quic_send_after_close_fails() {
    let t = new_quic().await;
    t.close().await.unwrap();
    assert!(t.is_closed());

    let result = t.send_to(b"nope", "127.0.0.1:1234".parse().unwrap()).await;
    assert!(matches!(result, Err(TransportError::Closed)));
}

#[tokio::test]
async fn test_quic_recv_after_close_fails() {
    let t = new_quic().await;
    t.close().await.unwrap();

    let mut buf = vec![0u8; 1500];
    let result = t.recv_from(&mut buf).await;
    assert!(matches!(result, Err(TransportError::Closed)));
}

// ---------------------------------------------------------------------------
// Multiple sequential messages
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_quic_multiple_sequential_messages() {
    // Each send_to + recv_from is an independent QUIC connection (due to the
    // accept-per-recv design), so we test two sequential round-trips.
    for i in 0..2 {
        let server = Arc::new(new_quic().await);
        let server_addr = server.local_addr().unwrap();
        let client = new_quic().await;

        let payload = format!("msg_{i}");
        let expected = payload.clone();

        let srv = server.clone();
        let h = tokio::spawn(async move {
            let mut buf = vec![0u8; 1500];
            let (n, _) = timeout(TEST_TIMEOUT, srv.recv_from(&mut buf))
                .await
                .expect("timeout")
                .unwrap();
            buf.truncate(n);
            String::from_utf8(buf).unwrap()
        });
        tokio::time::sleep(Duration::from_millis(100)).await;

        client
            .send_to(payload.as_bytes(), server_addr)
            .await
            .unwrap();

        let received = h.await.unwrap();
        assert_eq!(received, expected);
    }
}

// ---------------------------------------------------------------------------
// Bind with custom config
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_quic_bind_with_custom_config() {
    let config = QuicConfig {
        accept_self_signed: true,
        keep_alive: Some(Duration::from_secs(30)),
        ..QuicConfig::default()
    };
    let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
    let t = QuicTransport::bind_with_config(addr, config).await.unwrap();
    assert!(!t.is_closed());
    assert_ne!(t.local_addr().unwrap().port(), 0);
}
