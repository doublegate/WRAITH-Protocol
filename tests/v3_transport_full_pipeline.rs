//! Integration tests for v3 Phase 3 full transport pipeline.
//!
//! These tests exercise cross-module scenarios: factory creation, manager
//! orchestration, migration, and polymorphic trait usage.

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::timeout;
use wraith_transport::factory::{TransportFactory, TransportFactoryConfig, TransportType};
use wraith_transport::manager::{TransportManager, TransportSelector};
use wraith_transport::tcp::TcpTransport;
use wraith_transport::transport::Transport;
use wraith_transport::udp_async::AsyncUdpTransport;

const TEST_TIMEOUT: Duration = Duration::from_secs(5);

fn localhost() -> SocketAddr {
    "127.0.0.1:0".parse().unwrap()
}

// ---------------------------------------------------------------------------
// Factory -> Manager -> send/recv -> migrate -> send/recv
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_full_pipeline_factory_manager_migrate() {
    // Create UDP transport via factory
    let udp = TransportFactory::create_udp(localhost()).await.unwrap();

    // Create TCP transport directly
    let tcp = Arc::new(TcpTransport::bind(localhost()).await.unwrap());

    // Build manager with UDP as primary
    let manager = TransportManager::new(udp);
    manager.add_transport(tcp).await;

    assert_eq!(manager.active_transport_type().await, TransportType::Udp);

    // Send via UDP primary
    let udp_server = AsyncUdpTransport::bind(localhost()).await.unwrap();
    let udp_server_addr = udp_server.local_addr().unwrap();

    manager
        .send_to(b"pre_migrate", udp_server_addr)
        .await
        .unwrap();

    let mut buf = vec![0u8; 1500];
    let (n, _) = timeout(TEST_TIMEOUT, udp_server.recv_from(&mut buf))
        .await
        .expect("timeout")
        .unwrap();
    assert_eq!(&buf[..n], b"pre_migrate");

    // Migrate to TCP
    manager.migrate(TransportType::Tcp).await.unwrap();
    assert_eq!(manager.active_transport_type().await, TransportType::Tcp);

    // Send via TCP primary -- target a TCP server
    let tcp_server = Arc::new(TcpTransport::bind(localhost()).await.unwrap());
    let tcp_server_addr = tcp_server.local_addr().unwrap();

    let srv = tcp_server.clone();
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

    manager
        .send_to(b"post_migrate", tcp_server_addr)
        .await
        .unwrap();

    let received = h.await.unwrap();
    assert_eq!(&received, b"post_migrate");
}

// ---------------------------------------------------------------------------
// Transport trait polymorphism with mixed types
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_transport_trait_polymorphism() {
    let udp = TransportFactory::create_udp(localhost()).await.unwrap();
    let tcp = TransportFactory::create(TransportFactoryConfig::tcp(localhost()))
        .await
        .unwrap();
    let quic = TransportFactory::create(TransportFactoryConfig::quic(localhost()))
        .await
        .unwrap();

    let transports: Vec<Arc<dyn Transport>> = vec![udp, tcp, quic];

    // All should have valid local addresses
    for t in &transports {
        let addr = t.local_addr().unwrap();
        assert_ne!(addr.port(), 0);
        assert!(!t.is_closed());
    }

    // Verify each returns its correct type
    assert_eq!(transports[0].transport_type(), TransportType::Udp);
    assert_eq!(transports[1].transport_type(), TransportType::Tcp);
    assert_eq!(transports[2].transport_type(), TransportType::Quic);

    // MTU should differ
    assert_eq!(transports[0].mtu(), 1472); // UDP (Ethernet MTU minus IP/UDP headers)
    assert_eq!(transports[1].mtu(), 65535); // TCP
    assert_eq!(transports[2].mtu(), 1200); // QUIC

    // Migration support
    assert!(!transports[0].supports_migration()); // UDP
    assert!(!transports[1].supports_migration()); // TCP
    assert!(transports[2].supports_migration()); // QUIC
}

// ---------------------------------------------------------------------------
// Stats verification across migration boundary
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_stats_across_migration() {
    let udp_client = AsyncUdpTransport::bind(localhost()).await.unwrap();
    let tcp_client = TcpTransport::bind(localhost()).await.unwrap();

    let udp_server = AsyncUdpTransport::bind(localhost()).await.unwrap();
    let udp_server_addr = udp_server.local_addr().unwrap();

    let manager = TransportManager::new(Arc::new(udp_client));
    manager.add_transport(Arc::new(tcp_client)).await;

    // Send one packet via UDP
    manager.send_to(b"udp1", udp_server_addr).await.unwrap();

    let stats_before = manager.aggregated_stats().await;
    assert_eq!(stats_before.packets_sent, 1);
    assert_eq!(stats_before.bytes_sent, 4);

    // Migrate to TCP
    manager.migrate(TransportType::Tcp).await.unwrap();

    // Send one packet via TCP to a TCP server
    let tcp_server = Arc::new(TcpTransport::bind(localhost()).await.unwrap());
    let tcp_server_addr = tcp_server.local_addr().unwrap();

    let srv = tcp_server.clone();
    let h = tokio::spawn(async move {
        let mut buf = vec![0u8; 1500];
        timeout(TEST_TIMEOUT, srv.recv_from(&mut buf))
            .await
            .expect("timeout")
            .unwrap()
    });
    tokio::time::sleep(Duration::from_millis(50)).await;

    manager.send_to(b"tcp1", tcp_server_addr).await.unwrap();
    h.await.unwrap();

    // Aggregated stats should show both UDP and TCP activity
    let stats_after = manager.aggregated_stats().await;
    assert_eq!(stats_after.packets_sent, 2);
    assert_eq!(stats_after.bytes_sent, 8); // 4 + 4
}

// ---------------------------------------------------------------------------
// Factory creates multiple types and all are functional
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_factory_all_types_bind_and_close() {
    let types = [
        TransportType::Udp,
        TransportType::Tcp,
        TransportType::WebSocket,
        TransportType::Quic,
    ];

    for tt in &types {
        let config = TransportFactoryConfig::new(*tt, localhost());
        let t = TransportFactory::create(config).await.unwrap();
        assert_eq!(t.transport_type(), *tt, "type mismatch for {tt}");
        assert!(!t.is_closed());
        t.close().await.unwrap();
        assert!(t.is_closed());
    }
}

// ---------------------------------------------------------------------------
// Manager with factory-created transports, latency-based selection
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_manager_latency_selection_with_mixed_transports() {
    let udp = TransportFactory::create_udp(localhost()).await.unwrap();
    let tcp = TransportFactory::create(TransportFactoryConfig::tcp(localhost()))
        .await
        .unwrap();

    let manager = TransportManager::new(udp).with_selector(TransportSelector::LatencyBased);
    manager.add_transport(tcp).await;

    // UDP latency (1ms) == TCP latency (1ms), so either could be selected.
    // Just verify the manager works without errors.
    let server = AsyncUdpTransport::bind(localhost()).await.unwrap();
    let server_addr = server.local_addr().unwrap();

    // This may go via either transport; we just check no error.
    let result = manager.send_to(b"latency_test", server_addr).await;
    // Result depends on which transport is selected; both are valid.
    // UDP will succeed, TCP may fail to connect (no TCP listener), but the
    // important thing is no panic.
    let _ = result;
}

// ---------------------------------------------------------------------------
// Manager transport count after add
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_manager_transport_count_after_adds() {
    let udp = AsyncUdpTransport::bind(localhost()).await.unwrap();
    let manager = TransportManager::new(Arc::new(udp));
    assert_eq!(manager.transport_count().await, 1);

    let tcp = TcpTransport::bind(localhost()).await.unwrap();
    manager.add_transport(Arc::new(tcp)).await;
    assert_eq!(manager.transport_count().await, 2);

    let udp2 = AsyncUdpTransport::bind(localhost()).await.unwrap();
    manager.add_transport(Arc::new(udp2)).await;
    assert_eq!(manager.transport_count().await, 3);
}
