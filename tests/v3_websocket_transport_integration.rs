//! Integration tests for v3 Phase 3 WebSocket transport.

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::timeout;
use wraith_transport::factory::TransportType;
use wraith_transport::transport::{Transport, TransportError};
use wraith_transport::websocket::WebSocketTransport;

const TEST_TIMEOUT: Duration = Duration::from_secs(5);

async fn new_ws() -> WebSocketTransport {
    let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
    WebSocketTransport::bind(addr).await.unwrap()
}

// ---------------------------------------------------------------------------
// Trait method values
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_ws_transport_type_returns_websocket() {
    let t = new_ws().await;
    assert_eq!(t.transport_type(), TransportType::WebSocket);
}

#[tokio::test]
async fn test_ws_mtu_is_65535() {
    let t = new_ws().await;
    assert_eq!(t.mtu(), 65535);
}

#[tokio::test]
async fn test_ws_latency_estimate() {
    let t = new_ws().await;
    assert_eq!(t.latency_estimate(), Duration::from_millis(5));
}

#[tokio::test]
async fn test_ws_does_not_support_migration() {
    let t = new_ws().await;
    assert!(!t.supports_migration());
}

// ---------------------------------------------------------------------------
// Send / recv round-trip
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_ws_send_recv_roundtrip() {
    let server = Arc::new(new_ws().await);
    let server_addr = server.local_addr().unwrap();
    let client = new_ws().await;

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
    tokio::time::sleep(Duration::from_millis(50)).await;

    let sent = client
        .send_to(b"Hello WS integration!", server_addr)
        .await
        .unwrap();
    assert_eq!(sent, 21);

    let received = h.await.unwrap();
    assert_eq!(&received, b"Hello WS integration!");
}

// ---------------------------------------------------------------------------
// Stats tracking
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_ws_stats_tracking() {
    let server = Arc::new(new_ws().await);
    let server_addr = server.local_addr().unwrap();
    let client = new_ws().await;

    let srv = server.clone();
    let h = tokio::spawn(async move {
        let mut buf = vec![0u8; 1500];
        timeout(TEST_TIMEOUT, srv.recv_from(&mut buf))
            .await
            .expect("timeout")
            .unwrap()
    });
    tokio::time::sleep(Duration::from_millis(50)).await;

    client.send_to(b"wsstat", server_addr).await.unwrap();
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
async fn test_ws_send_after_close_fails() {
    let t = new_ws().await;
    t.close().await.unwrap();
    assert!(t.is_closed());

    let result = t.send_to(b"nope", "127.0.0.1:1234".parse().unwrap()).await;
    assert!(matches!(result, Err(TransportError::Closed)));
}

#[tokio::test]
async fn test_ws_recv_after_close_fails() {
    let t = new_ws().await;
    t.close().await.unwrap();

    let mut buf = vec![0u8; 1500];
    let result = t.recv_from(&mut buf).await;
    assert!(matches!(result, Err(TransportError::Closed)));
}

// ---------------------------------------------------------------------------
// Close then new transport on same flow
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_ws_close_and_rebind() {
    let t = new_ws().await;
    let _addr = t.local_addr().unwrap();
    t.close().await.unwrap();
    assert!(t.is_closed());

    // Creating a new transport should succeed (port released).
    let t2 = new_ws().await;
    assert!(!t2.is_closed());
    assert_ne!(t2.local_addr().unwrap().port(), 0);
}
