//! Integration tests for v3 Phase 3 TransportManager.

use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;
use tokio::time::timeout;
use wraith_transport::factory::TransportType;
use wraith_transport::manager::{MigrationEvent, TransportManager, TransportSelector};
use wraith_transport::tcp::TcpTransport;
use wraith_transport::transport::{Transport, TransportError};
use wraith_transport::udp_async::AsyncUdpTransport;

const TEST_TIMEOUT: Duration = Duration::from_secs(5);

async fn new_udp() -> AsyncUdpTransport {
    AsyncUdpTransport::bind("127.0.0.1:0".parse::<SocketAddr>().unwrap())
        .await
        .unwrap()
}

// ---------------------------------------------------------------------------
// Creation
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_manager_creation_with_single_transport() {
    let udp = new_udp().await;
    let manager = TransportManager::new(Arc::new(udp));
    assert_eq!(manager.transport_count().await, 1);
    assert!(!manager.is_closed());
    assert!(manager.supports_migration());
}

#[tokio::test]
async fn test_manager_creation_with_multiple_transports() {
    let udp = new_udp().await;
    let tcp = TcpTransport::bind("127.0.0.1:0".parse::<SocketAddr>().unwrap())
        .await
        .unwrap();
    let manager = TransportManager::new(Arc::new(udp));
    manager.add_transport(Arc::new(tcp)).await;
    assert_eq!(manager.transport_count().await, 2);
}

// ---------------------------------------------------------------------------
// Send / recv via primary transport (UDP)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_manager_send_recv_via_primary() {
    let server = new_udp().await;
    let server_addr = server.local_addr().unwrap();

    let client_udp = new_udp().await;
    let manager = TransportManager::new(Arc::new(client_udp));

    manager.send_to(b"manager_test", server_addr).await.unwrap();

    let mut buf = vec![0u8; 1500];
    let (n, _) = timeout(TEST_TIMEOUT, server.recv_from(&mut buf))
        .await
        .expect("timeout")
        .unwrap();
    assert_eq!(&buf[..n], b"manager_test");
}

// ---------------------------------------------------------------------------
// Active transport type
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_manager_active_transport_type_default_udp() {
    let udp = new_udp().await;
    let manager = TransportManager::new(Arc::new(udp));
    assert_eq!(manager.active_transport_type().await, TransportType::Udp);
}

// ---------------------------------------------------------------------------
// Migration
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_manager_migrate_udp_to_tcp() {
    let udp = new_udp().await;
    let tcp = TcpTransport::bind("127.0.0.1:0".parse::<SocketAddr>().unwrap())
        .await
        .unwrap();

    let manager = TransportManager::new(Arc::new(udp));
    manager.add_transport(Arc::new(tcp)).await;

    assert_eq!(manager.active_transport_type().await, TransportType::Udp);

    manager.migrate(TransportType::Tcp).await.unwrap();
    assert_eq!(manager.active_transport_type().await, TransportType::Tcp);
}

#[tokio::test]
async fn test_manager_migrate_to_nonexistent_type_fails() {
    let udp = new_udp().await;
    let manager = TransportManager::new(Arc::new(udp));

    let result = manager.migrate(TransportType::Quic).await;
    assert!(result.is_err());
}

// ---------------------------------------------------------------------------
// Migration event tracking
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_manager_migration_events_on_success() {
    let udp = new_udp().await;
    let tcp = TcpTransport::bind("127.0.0.1:0".parse::<SocketAddr>().unwrap())
        .await
        .unwrap();

    let manager = TransportManager::new(Arc::new(udp));
    manager.add_transport(Arc::new(tcp)).await;

    let started = Arc::new(AtomicUsize::new(0));
    let completed = Arc::new(AtomicUsize::new(0));
    let started_c = started.clone();
    let completed_c = completed.clone();

    manager
        .set_migration_callback(Arc::new(move |event| match event {
            MigrationEvent::Started { .. } => {
                started_c.fetch_add(1, Ordering::Relaxed);
            }
            MigrationEvent::Completed { .. } => {
                completed_c.fetch_add(1, Ordering::Relaxed);
            }
            _ => {}
        }))
        .await;

    manager.migrate(TransportType::Tcp).await.unwrap();

    assert_eq!(started.load(Ordering::Relaxed), 1);
    assert_eq!(completed.load(Ordering::Relaxed), 1);
}

#[tokio::test]
async fn test_manager_migration_events_on_failure() {
    let udp = new_udp().await;
    let manager = TransportManager::new(Arc::new(udp));

    let event_count = Arc::new(AtomicUsize::new(0));
    let failed_count = Arc::new(AtomicUsize::new(0));
    let ec = event_count.clone();
    let fc = failed_count.clone();

    manager
        .set_migration_callback(Arc::new(move |event| {
            ec.fetch_add(1, Ordering::Relaxed);
            if matches!(event, MigrationEvent::Failed { .. }) {
                fc.fetch_add(1, Ordering::Relaxed);
            }
        }))
        .await;

    let _ = manager.migrate(TransportType::Quic).await;

    // Started + Failed = 2 events
    assert_eq!(event_count.load(Ordering::Relaxed), 2);
    assert_eq!(failed_count.load(Ordering::Relaxed), 1);
}

// ---------------------------------------------------------------------------
// Aggregated stats
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_manager_aggregated_stats() {
    let server = new_udp().await;
    let server_addr = server.local_addr().unwrap();

    let client_udp = new_udp().await;
    let manager = TransportManager::new(Arc::new(client_udp));

    manager.send_to(b"agg", server_addr).await.unwrap();
    manager.send_to(b"stats", server_addr).await.unwrap();

    let stats = manager.aggregated_stats().await;
    assert_eq!(stats.packets_sent, 2);
    assert_eq!(stats.bytes_sent, 8); // 3 + 5
}

// ---------------------------------------------------------------------------
// Selector strategies
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_manager_round_robin_selector() {
    let udp1 = new_udp().await;
    let udp2 = new_udp().await;

    let server = new_udp().await;
    let server_addr = server.local_addr().unwrap();

    let manager =
        TransportManager::new(Arc::new(udp1)).with_selector(TransportSelector::RoundRobin);
    manager.add_transport(Arc::new(udp2)).await;

    // Both sends should succeed (round-robin between two transports).
    manager.send_to(b"rr1", server_addr).await.unwrap();
    manager.send_to(b"rr2", server_addr).await.unwrap();
}

#[tokio::test]
async fn test_manager_latency_based_selector() {
    let udp = new_udp().await;
    let server = new_udp().await;
    let server_addr = server.local_addr().unwrap();

    let manager =
        TransportManager::new(Arc::new(udp)).with_selector(TransportSelector::LatencyBased);
    manager.send_to(b"lat", server_addr).await.unwrap();
}

#[tokio::test]
async fn test_manager_throughput_based_selector() {
    let udp = new_udp().await;
    let server = new_udp().await;
    let server_addr = server.local_addr().unwrap();

    let manager =
        TransportManager::new(Arc::new(udp)).with_selector(TransportSelector::ThroughputBased);
    manager.send_to(b"tput", server_addr).await.unwrap();
}

#[tokio::test]
async fn test_selector_default_is_primary() {
    assert_eq!(TransportSelector::default(), TransportSelector::Primary);
}

// ---------------------------------------------------------------------------
// Close behaviour
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_manager_close_closes_all_transports() {
    let udp = new_udp().await;
    let manager = TransportManager::new(Arc::new(udp));
    manager.close().await.unwrap();
    assert!(manager.is_closed());

    let result = manager
        .send_to(b"nope", "127.0.0.1:1234".parse().unwrap())
        .await;
    assert!(matches!(result, Err(TransportError::Closed)));
}
