//! Integration tests for v3 Phase 3 TCP transport.

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::timeout;
use wraith_transport::factory::TransportType;
use wraith_transport::tcp::TcpTransport;
use wraith_transport::transport::{Transport, TransportError};

const TEST_TIMEOUT: Duration = Duration::from_secs(5);

/// Helper to create a bound TCP transport on localhost with an OS-assigned port.
async fn new_tcp() -> TcpTransport {
    let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
    TcpTransport::bind(addr).await.unwrap()
}

// ---------------------------------------------------------------------------
// Basic trait method values
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_tcp_transport_type_returns_tcp() {
    let t = new_tcp().await;
    assert_eq!(t.transport_type(), TransportType::Tcp);
}

#[tokio::test]
async fn test_tcp_mtu_is_65535() {
    let t = new_tcp().await;
    assert_eq!(t.mtu(), 65535);
}

#[tokio::test]
async fn test_tcp_latency_estimate() {
    let t = new_tcp().await;
    assert_eq!(t.latency_estimate(), Duration::from_millis(1));
}

#[tokio::test]
async fn test_tcp_does_not_support_migration() {
    let t = new_tcp().await;
    assert!(!t.supports_migration());
}

// ---------------------------------------------------------------------------
// Send / recv round-trip
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_tcp_send_recv_roundtrip() {
    let server = Arc::new(new_tcp().await);
    let server_addr = server.local_addr().unwrap();
    let client = new_tcp().await;

    let srv = server.clone();
    let recv_handle = tokio::spawn(async move {
        let mut buf = vec![0u8; 1500];
        let (n, _from) = timeout(TEST_TIMEOUT, srv.recv_from(&mut buf))
            .await
            .expect("recv timed out")
            .unwrap();
        buf.truncate(n);
        buf
    });

    tokio::time::sleep(Duration::from_millis(50)).await;

    let sent = client
        .send_to(b"Hello TCP integration!", server_addr)
        .await
        .unwrap();
    assert_eq!(sent, 22);

    let received = recv_handle.await.unwrap();
    assert_eq!(&received, b"Hello TCP integration!");
}

// ---------------------------------------------------------------------------
// Multiple concurrent connections
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_tcp_multiple_clients() {
    let server = Arc::new(new_tcp().await);
    let server_addr = server.local_addr().unwrap();

    // We send from two distinct clients sequentially (each triggers a new accept).
    let client_a = new_tcp().await;
    let client_b = new_tcp().await;

    // First message
    let srv = server.clone();
    let h1 = tokio::spawn(async move {
        let mut buf = vec![0u8; 1500];
        let (n, _) = timeout(TEST_TIMEOUT, srv.recv_from(&mut buf))
            .await
            .expect("timeout")
            .unwrap();
        buf.truncate(n);
        buf
    });
    tokio::time::sleep(Duration::from_millis(50)).await;
    client_a.send_to(b"from_a", server_addr).await.unwrap();
    let data_a = h1.await.unwrap();
    assert_eq!(&data_a, b"from_a");

    // Second message from different client
    let srv = server.clone();
    let h2 = tokio::spawn(async move {
        let mut buf = vec![0u8; 1500];
        let (n, _) = timeout(TEST_TIMEOUT, srv.recv_from(&mut buf))
            .await
            .expect("timeout")
            .unwrap();
        buf.truncate(n);
        buf
    });
    tokio::time::sleep(Duration::from_millis(50)).await;
    client_b.send_to(b"from_b", server_addr).await.unwrap();
    let data_b = h2.await.unwrap();
    assert_eq!(&data_b, b"from_b");
}

// ---------------------------------------------------------------------------
// Stats tracking
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_tcp_stats_tracking() {
    let server = Arc::new(new_tcp().await);
    let server_addr = server.local_addr().unwrap();
    let client = new_tcp().await;

    let srv = server.clone();
    let h = tokio::spawn(async move {
        let mut buf = vec![0u8; 1500];
        timeout(TEST_TIMEOUT, srv.recv_from(&mut buf))
            .await
            .expect("timeout")
            .unwrap()
    });
    tokio::time::sleep(Duration::from_millis(50)).await;

    client.send_to(b"stat_check", server_addr).await.unwrap();
    h.await.unwrap();

    let cs = client.stats();
    assert_eq!(cs.packets_sent, 1);
    assert_eq!(cs.bytes_sent, 10);

    let ss = server.stats();
    assert_eq!(ss.packets_received, 1);
    assert_eq!(ss.bytes_received, 10);
}

// ---------------------------------------------------------------------------
// Close behaviour
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_tcp_send_after_close_fails() {
    let t = new_tcp().await;
    t.close().await.unwrap();
    assert!(t.is_closed());

    let result = t.send_to(b"nope", "127.0.0.1:1234".parse().unwrap()).await;
    assert!(matches!(result, Err(TransportError::Closed)));
}

#[tokio::test]
async fn test_tcp_recv_after_close_fails() {
    let t = new_tcp().await;
    t.close().await.unwrap();

    let mut buf = vec![0u8; 1500];
    let result = t.recv_from(&mut buf).await;
    assert!(matches!(result, Err(TransportError::Closed)));
}

// ---------------------------------------------------------------------------
// Large message (close to 64 KiB)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_tcp_large_message_roundtrip() {
    let server = Arc::new(new_tcp().await);
    let server_addr = server.local_addr().unwrap();
    let client = new_tcp().await;

    let payload = vec![0xAB_u8; 60_000];
    let expected = payload.clone();

    let srv = server.clone();
    let h = tokio::spawn(async move {
        let mut buf = vec![0u8; 65536];
        let (n, _) = timeout(TEST_TIMEOUT, srv.recv_from(&mut buf))
            .await
            .expect("timeout")
            .unwrap();
        buf.truncate(n);
        buf
    });
    tokio::time::sleep(Duration::from_millis(50)).await;

    let sent = client.send_to(&payload, server_addr).await.unwrap();
    assert_eq!(sent, 60_000);

    let received = h.await.unwrap();
    assert_eq!(received, expected);
}

// ---------------------------------------------------------------------------
// Property: send size equals recv size
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_tcp_various_payload_sizes() {
    for size in [1, 10, 100, 1000, 8192, 32768] {
        let server = Arc::new(new_tcp().await);
        let server_addr = server.local_addr().unwrap();
        let client = new_tcp().await;

        let payload = vec![0x42_u8; size];

        let srv = server.clone();
        let h = tokio::spawn(async move {
            let mut buf = vec![0u8; 65536];
            let (n, _) = timeout(TEST_TIMEOUT, srv.recv_from(&mut buf))
                .await
                .expect("timeout")
                .unwrap();
            n
        });
        tokio::time::sleep(Duration::from_millis(50)).await;

        let sent = client.send_to(&payload, server_addr).await.unwrap();
        assert_eq!(sent, size, "send size mismatch for payload {size}");

        let recv_size = h.await.unwrap();
        assert_eq!(recv_size, size, "recv size mismatch for payload {size}");
    }
}
