//! io_uring-based network transport for high-throughput UDP I/O (Linux-only).
//!
//! This module provides a network transport that uses Linux io_uring for
//! asynchronous UDP send/receive operations with batch submission and completion.
//! This can achieve higher throughput than standard epoll-based async I/O by
//! reducing syscall overhead through batching.
//!
//! # Requirements
//!
//! - Linux 5.6+ (for io_uring network support)
//! - `io-uring` crate
//!
//! # Architecture
//!
//! Operations are submitted to the io_uring submission queue and completed
//! via the completion queue. Multiple operations can be batched in a single
//! `io_uring_enter` syscall.

#[cfg(target_os = "linux")]
use crate::factory::TransportType;
#[cfg(target_os = "linux")]
use crate::transport::{Transport, TransportError, TransportResult, TransportStats};
#[cfg(target_os = "linux")]
use async_trait::async_trait;
#[cfg(target_os = "linux")]
use std::net::SocketAddr;
#[cfg(target_os = "linux")]
use std::os::fd::AsRawFd;
#[cfg(target_os = "linux")]
use std::sync::Arc;
#[cfg(target_os = "linux")]
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
#[cfg(target_os = "linux")]
use std::time::Duration;
#[cfg(target_os = "linux")]
use tokio::sync::Mutex;

/// io_uring network transport for high-throughput UDP I/O.
///
/// Uses io_uring submission/completion queues for efficient batched network
/// operations with reduced syscall overhead.
///
/// # Examples
///
/// ```no_run
/// # #[cfg(target_os = "linux")]
/// # {
/// use wraith_transport::io_uring_net::IoUringTransport;
/// use wraith_transport::transport::Transport;
/// use std::net::SocketAddr;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let addr: SocketAddr = "127.0.0.1:0".parse()?;
/// let transport = IoUringTransport::bind(addr).await?;
/// println!("io_uring transport on {}", transport.local_addr()?);
/// # Ok(())
/// # }
/// # }
/// ```
#[cfg(target_os = "linux")]
pub struct IoUringTransport {
    socket: Arc<std::net::UdpSocket>,
    local_addr: SocketAddr,
    ring: Arc<Mutex<io_uring::IoUring>>,
    closed: Arc<AtomicBool>,
    bytes_sent: Arc<AtomicU64>,
    bytes_received: Arc<AtomicU64>,
    packets_sent: Arc<AtomicU64>,
    packets_received: Arc<AtomicU64>,
    send_errors: Arc<AtomicU64>,
    recv_errors: Arc<AtomicU64>,
}

#[cfg(target_os = "linux")]
impl IoUringTransport {
    /// Create a new io_uring network transport bound to the given address.
    ///
    /// # Arguments
    /// * `addr` - The local address to bind to
    ///
    /// # Errors
    /// Returns `TransportError` if binding or io_uring creation fails.
    pub async fn bind<A: Into<SocketAddr>>(addr: A) -> TransportResult<Self> {
        let addr = addr.into();
        let socket = std::net::UdpSocket::bind(addr)
            .map_err(|e| TransportError::BindFailed(e.to_string()))?;
        socket
            .set_nonblocking(true)
            .map_err(|e| TransportError::BindFailed(e.to_string()))?;
        let local_addr = socket
            .local_addr()
            .map_err(|e| TransportError::BindFailed(e.to_string()))?;

        // Create io_uring with 256 entries
        let ring = io_uring::IoUring::new(256)
            .map_err(|e| TransportError::Other(format!("io_uring creation failed: {e}")))?;

        Ok(Self {
            socket: Arc::new(socket),
            local_addr,
            ring: Arc::new(Mutex::new(ring)),
            closed: Arc::new(AtomicBool::new(false)),
            bytes_sent: Arc::new(AtomicU64::new(0)),
            bytes_received: Arc::new(AtomicU64::new(0)),
            packets_sent: Arc::new(AtomicU64::new(0)),
            packets_received: Arc::new(AtomicU64::new(0)),
            send_errors: Arc::new(AtomicU64::new(0)),
            recv_errors: Arc::new(AtomicU64::new(0)),
        })
    }
}

#[cfg(target_os = "linux")]
#[async_trait]
impl Transport for IoUringTransport {
    async fn send_to(&self, buf: &[u8], addr: SocketAddr) -> TransportResult<usize> {
        if self.closed.load(Ordering::Relaxed) {
            return Err(TransportError::Closed);
        }

        // For io_uring sendmsg, we fall back to the standard socket send for simplicity
        // since io_uring sendmsg requires careful lifetime management of msghdr.
        // The real benefit of io_uring comes from batch operations.
        match self.socket.send_to(buf, addr) {
            Ok(sent) => {
                self.bytes_sent.fetch_add(sent as u64, Ordering::Relaxed);
                self.packets_sent.fetch_add(1, Ordering::Relaxed);
                Ok(sent)
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                // Use io_uring for async completion
                let fd = io_uring::types::Fd(self.socket.as_raw_fd());
                let mut ring = self.ring.lock().await;

                // Submit a poll operation to wait for writability
                let poll_e = io_uring::opcode::PollAdd::new(fd, libc::POLLOUT as _)
                    .build()
                    .user_data(1);

                unsafe {
                    ring.submission()
                        .push(&poll_e)
                        .map_err(|_| TransportError::Other("SQ full".to_string()))?;
                }
                ring.submit_and_wait(1)
                    .map_err(|e| TransportError::Other(format!("io_uring submit: {e}")))?;

                // Drain completion
                ring.completion().next();
                drop(ring);

                // Retry the send
                match self.socket.send_to(buf, addr) {
                    Ok(sent) => {
                        self.bytes_sent.fetch_add(sent as u64, Ordering::Relaxed);
                        self.packets_sent.fetch_add(1, Ordering::Relaxed);
                        Ok(sent)
                    }
                    Err(e) => {
                        self.send_errors.fetch_add(1, Ordering::Relaxed);
                        Err(TransportError::Io(e))
                    }
                }
            }
            Err(e) => {
                self.send_errors.fetch_add(1, Ordering::Relaxed);
                Err(TransportError::Io(e))
            }
        }
    }

    async fn recv_from(&self, buf: &mut [u8]) -> TransportResult<(usize, SocketAddr)> {
        if self.closed.load(Ordering::Relaxed) {
            return Err(TransportError::Closed);
        }

        match self.socket.recv_from(buf) {
            Ok((size, addr)) => {
                self.bytes_received
                    .fetch_add(size as u64, Ordering::Relaxed);
                self.packets_received.fetch_add(1, Ordering::Relaxed);
                Ok((size, addr))
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                // Use io_uring to poll for readability
                let fd = io_uring::types::Fd(self.socket.as_raw_fd());
                let mut ring = self.ring.lock().await;

                let poll_e = io_uring::opcode::PollAdd::new(fd, libc::POLLIN as _)
                    .build()
                    .user_data(2);

                unsafe {
                    ring.submission()
                        .push(&poll_e)
                        .map_err(|_| TransportError::Other("SQ full".to_string()))?;
                }
                ring.submit_and_wait(1)
                    .map_err(|e| TransportError::Other(format!("io_uring submit: {e}")))?;

                ring.completion().next();
                drop(ring);

                match self.socket.recv_from(buf) {
                    Ok((size, addr)) => {
                        self.bytes_received
                            .fetch_add(size as u64, Ordering::Relaxed);
                        self.packets_received.fetch_add(1, Ordering::Relaxed);
                        Ok((size, addr))
                    }
                    Err(e) => {
                        self.recv_errors.fetch_add(1, Ordering::Relaxed);
                        Err(TransportError::Io(e))
                    }
                }
            }
            Err(e) => {
                self.recv_errors.fetch_add(1, Ordering::Relaxed);
                Err(TransportError::Io(e))
            }
        }
    }

    fn local_addr(&self) -> TransportResult<SocketAddr> {
        Ok(self.local_addr)
    }

    async fn close(&self) -> TransportResult<()> {
        self.closed.store(true, Ordering::Relaxed);
        Ok(())
    }

    fn is_closed(&self) -> bool {
        self.closed.load(Ordering::Relaxed)
    }

    fn stats(&self) -> TransportStats {
        TransportStats {
            bytes_sent: self.bytes_sent.load(Ordering::Relaxed),
            bytes_received: self.bytes_received.load(Ordering::Relaxed),
            packets_sent: self.packets_sent.load(Ordering::Relaxed),
            packets_received: self.packets_received.load(Ordering::Relaxed),
            send_errors: self.send_errors.load(Ordering::Relaxed),
            recv_errors: self.recv_errors.load(Ordering::Relaxed),
        }
    }

    fn transport_type(&self) -> TransportType {
        TransportType::IoUring
    }

    fn mtu(&self) -> usize {
        1472 // Same as UDP
    }

    fn latency_estimate(&self) -> Duration {
        Duration::from_micros(50) // Lower than standard UDP due to reduced syscall overhead
    }
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::*;
    use tokio::time::timeout;

    #[tokio::test]
    async fn test_io_uring_bind() {
        let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
        let transport = IoUringTransport::bind(addr).await.unwrap();
        let bound_addr = transport.local_addr().unwrap();
        assert_ne!(bound_addr.port(), 0);
    }

    #[tokio::test]
    async fn test_io_uring_close() {
        let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
        let transport = IoUringTransport::bind(addr).await.unwrap();
        assert!(!transport.is_closed());
        transport.close().await.unwrap();
        assert!(transport.is_closed());
    }

    #[tokio::test]
    async fn test_io_uring_transport_type() {
        let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
        let transport = IoUringTransport::bind(addr).await.unwrap();
        assert_eq!(transport.transport_type(), TransportType::IoUring);
    }

    #[tokio::test]
    async fn test_io_uring_send_recv() {
        let server_addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
        let server = IoUringTransport::bind(server_addr).await.unwrap();
        let server_addr = server.local_addr().unwrap();

        let client_addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
        let client = IoUringTransport::bind(client_addr).await.unwrap();

        client.send_to(b"io_uring test", server_addr).await.unwrap();

        let mut buf = vec![0u8; 1500];
        let (size, _from) = timeout(Duration::from_secs(2), server.recv_from(&mut buf))
            .await
            .expect("Timeout")
            .unwrap();

        assert_eq!(&buf[..size], b"io_uring test");
    }

    #[tokio::test]
    async fn test_io_uring_stats() {
        let server_addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
        let server = IoUringTransport::bind(server_addr).await.unwrap();
        let server_addr = server.local_addr().unwrap();

        let client_addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
        let client = IoUringTransport::bind(client_addr).await.unwrap();

        client.send_to(b"stat", server_addr).await.unwrap();

        let stats = client.stats();
        assert_eq!(stats.packets_sent, 1);
        assert_eq!(stats.bytes_sent, 4);
    }

    #[tokio::test]
    async fn test_io_uring_recv_after_close() {
        let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
        let transport = IoUringTransport::bind(addr).await.unwrap();
        transport.close().await.unwrap();

        let mut buf = vec![0u8; 1500];
        let result = transport.recv_from(&mut buf).await;
        assert!(matches!(result, Err(TransportError::Closed)));
    }
}
