//! Integration tests for v3 Phase 3 TransportFactory.

use std::net::SocketAddr;
use std::time::Duration;
use tokio::time::timeout;
use wraith_transport::factory::{TransportFactory, TransportFactoryConfig, TransportType};

const TEST_TIMEOUT: Duration = Duration::from_secs(5);

fn localhost() -> SocketAddr {
    "127.0.0.1:0".parse().unwrap()
}

// ---------------------------------------------------------------------------
// Factory creates each non-Linux transport type
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_factory_creates_udp() {
    let t = TransportFactory::create(TransportFactoryConfig::udp(localhost()))
        .await
        .unwrap();
    assert_eq!(t.transport_type(), TransportType::Udp);
    assert_ne!(t.local_addr().unwrap().port(), 0);
}

#[tokio::test]
async fn test_factory_creates_tcp() {
    let t = TransportFactory::create(TransportFactoryConfig::tcp(localhost()))
        .await
        .unwrap();
    assert_eq!(t.transport_type(), TransportType::Tcp);
    assert_ne!(t.local_addr().unwrap().port(), 0);
}

#[tokio::test]
async fn test_factory_creates_websocket() {
    let t = TransportFactory::create(TransportFactoryConfig::websocket(localhost()))
        .await
        .unwrap();
    assert_eq!(t.transport_type(), TransportType::WebSocket);
    assert_ne!(t.local_addr().unwrap().port(), 0);
}

#[tokio::test]
async fn test_factory_creates_quic() {
    let t = TransportFactory::create(TransportFactoryConfig::quic(localhost()))
        .await
        .unwrap();
    assert_eq!(t.transport_type(), TransportType::Quic);
    assert_ne!(t.local_addr().unwrap().port(), 0);
}

// ---------------------------------------------------------------------------
// Shorthand constructors
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_factory_create_udp_shorthand() {
    let t = TransportFactory::create_udp(localhost()).await.unwrap();
    assert_eq!(t.transport_type(), TransportType::Udp);
}

#[tokio::test]
async fn test_factory_create_quic_shorthand() {
    let t = TransportFactory::create_quic(localhost()).await.unwrap();
    assert_eq!(t.transport_type(), TransportType::Quic);
}

// ---------------------------------------------------------------------------
// available_transports and is_implemented
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_factory_available_transports() {
    let available = TransportFactory::available_transports();
    assert!(available.contains(&TransportType::Udp));
    assert!(available.contains(&TransportType::Quic));
    assert!(available.contains(&TransportType::Tcp));
    assert!(available.contains(&TransportType::WebSocket));
    // At minimum 4 on any platform
    assert!(available.len() >= 4);
}

#[tokio::test]
async fn test_factory_is_implemented() {
    assert!(TransportFactory::is_implemented(TransportType::Udp));
    assert!(TransportFactory::is_implemented(TransportType::Tcp));
    assert!(TransportFactory::is_implemented(TransportType::WebSocket));
    assert!(TransportFactory::is_implemented(TransportType::Quic));
}

#[cfg(target_os = "linux")]
#[tokio::test]
async fn test_factory_linux_transports_implemented() {
    assert!(TransportFactory::is_implemented(TransportType::IoUring));
    assert!(TransportFactory::is_implemented(TransportType::AfXdp));
}

#[cfg(not(target_os = "linux"))]
#[tokio::test]
async fn test_factory_linux_transports_not_implemented_on_non_linux() {
    assert!(!TransportFactory::is_implemented(TransportType::IoUring));
    assert!(!TransportFactory::is_implemented(TransportType::AfXdp));
}

// ---------------------------------------------------------------------------
// TransportType Display
// ---------------------------------------------------------------------------

#[test]
fn test_transport_type_display() {
    assert_eq!(format!("{}", TransportType::Udp), "UDP");
    assert_eq!(format!("{}", TransportType::Quic), "QUIC");
    assert_eq!(format!("{}", TransportType::Tcp), "TCP");
    assert_eq!(format!("{}", TransportType::WebSocket), "WebSocket");
    assert_eq!(format!("{}", TransportType::IoUring), "io_uring");
    assert_eq!(format!("{}", TransportType::AfXdp), "AF_XDP");
}

#[test]
fn test_transport_type_default_is_udp() {
    assert_eq!(TransportType::default(), TransportType::Udp);
}

// ---------------------------------------------------------------------------
// TransportFactoryConfig builder pattern
// ---------------------------------------------------------------------------

#[test]
fn test_config_default() {
    let config = TransportFactoryConfig::default();
    assert_eq!(config.transport_type, TransportType::Udp);
    assert!(config.recv_buffer_size.is_none());
    assert!(config.send_buffer_size.is_none());
}

#[test]
fn test_config_with_buffer_sizes() {
    let config =
        TransportFactoryConfig::udp(localhost()).with_buffer_sizes(1024 * 1024, 512 * 1024);
    assert_eq!(config.recv_buffer_size, Some(1024 * 1024));
    assert_eq!(config.send_buffer_size, Some(512 * 1024));
}

#[test]
fn test_config_convenience_constructors() {
    let udp = TransportFactoryConfig::udp(localhost());
    assert_eq!(udp.transport_type, TransportType::Udp);

    let tcp = TransportFactoryConfig::tcp(localhost());
    assert_eq!(tcp.transport_type, TransportType::Tcp);

    let ws = TransportFactoryConfig::websocket(localhost());
    assert_eq!(ws.transport_type, TransportType::WebSocket);

    let quic = TransportFactoryConfig::quic(localhost());
    assert_eq!(quic.transport_type, TransportType::Quic);
}

// ---------------------------------------------------------------------------
// Factory-created transports actually work (UDP send/recv)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_factory_created_udp_transport_works() {
    let server = TransportFactory::create_udp(localhost()).await.unwrap();
    let server_addr = server.local_addr().unwrap();
    let client = TransportFactory::create_udp(localhost()).await.unwrap();

    client.send_to(b"factory_works", server_addr).await.unwrap();

    let mut buf = vec![0u8; 1500];
    let (n, from) = timeout(TEST_TIMEOUT, server.recv_from(&mut buf))
        .await
        .expect("timeout")
        .unwrap();

    assert_eq!(&buf[..n], b"factory_works");
    assert_eq!(from, client.local_addr().unwrap());
}
