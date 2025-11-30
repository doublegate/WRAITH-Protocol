//! UDP socket fallback transport.
//!
//! This module provides a standard UDP socket-based transport for systems
//! without AF_XDP support or when kernel bypass is not needed.
//!
//! Features:
//! - Non-blocking I/O
//! - Configurable socket buffer sizes
//! - Cross-platform support (Linux, macOS, Windows)
//! - Target throughput: >1 Gbps on gigabit links

use socket2::{Domain, Protocol, Socket, Type};
use std::io;
use std::net::{SocketAddr, UdpSocket};

/// UDP transport for systems without AF_XDP support
///
/// Provides a reliable fallback transport using standard UDP sockets
/// with optimized buffer sizes and non-blocking operation.
pub struct UdpTransport {
    socket: UdpSocket,
    recv_buf: Vec<u8>,
    recv_buffer_size: usize,
    send_buffer_size: usize,
}

impl UdpTransport {
    /// Create a new UDP transport bound to the given address
    ///
    /// # Arguments
    /// * `addr` - The local address to bind to. Use "0.0.0.0:0" for automatic port selection.
    ///
    /// # Examples
    /// ```no_run
    /// use wraith_transport::udp::UdpTransport;
    /// use std::net::SocketAddr;
    ///
    /// let addr: SocketAddr = "127.0.0.1:40000".parse().unwrap();
    /// let transport = UdpTransport::bind(addr).unwrap();
    /// println!("Listening on {}", transport.local_addr().unwrap());
    /// ```
    pub fn bind<A: Into<SocketAddr>>(addr: A) -> io::Result<Self> {
        let addr = addr.into();

        // Use socket2 for advanced socket options
        let domain = if addr.is_ipv4() {
            Domain::IPV4
        } else {
            Domain::IPV6
        };

        let socket2 = Socket::new(domain, Type::DGRAM, Some(Protocol::UDP))?;

        // Enable non-blocking mode for async-compatible I/O
        socket2.set_nonblocking(true)?;

        // Increase buffer sizes for high-throughput operation
        // Target: 2MB buffers for sustained high-rate transfers
        socket2.set_recv_buffer_size(2 * 1024 * 1024)?; // 2MB RX buffer
        socket2.set_send_buffer_size(2 * 1024 * 1024)?; // 2MB TX buffer

        // Store the actual buffer sizes set by the kernel
        let recv_buffer_size = socket2.recv_buffer_size()?;
        let send_buffer_size = socket2.send_buffer_size()?;

        // Bind to address
        socket2.bind(&addr.into())?;

        // Convert to std::net::UdpSocket
        let socket: UdpSocket = socket2.into();

        // Allocate receive buffer (64KB for jumbo frame support)
        let recv_buf = vec![0u8; 65536];

        Ok(Self {
            socket,
            recv_buf,
            recv_buffer_size,
            send_buffer_size,
        })
    }

    /// Receive a packet from the socket
    ///
    /// Returns the number of bytes received and the sender's address.
    /// In non-blocking mode, returns `WouldBlock` if no data is available.
    ///
    /// # Examples
    /// ```no_run
    /// # use wraith_transport::udp::UdpTransport;
    /// # use std::net::SocketAddr;
    /// let mut transport = UdpTransport::bind("127.0.0.1:40000".parse::<SocketAddr>().unwrap()).unwrap();
    ///
    /// match transport.recv_from() {
    ///     Ok((size, from)) => {
    ///         println!("Received {} bytes from {}", size, from);
    ///         let data = transport.recv_buffer();
    ///         // Process data[..size]
    ///     }
    ///     Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
    ///         // No data available
    ///     }
    ///     Err(e) => eprintln!("Receive error: {}", e),
    /// }
    /// ```
    pub fn recv_from(&mut self) -> io::Result<(usize, SocketAddr)> {
        self.socket.recv_from(&mut self.recv_buf)
    }

    /// Send a packet to the specified address
    ///
    /// Returns the number of bytes sent. In non-blocking mode,
    /// may return `WouldBlock` if the send buffer is full.
    ///
    /// # Arguments
    /// * `buf` - The data to send
    /// * `addr` - The destination address
    ///
    /// # Examples
    /// ```no_run
    /// # use wraith_transport::udp::UdpTransport;
    /// # use std::net::SocketAddr;
    /// let transport = UdpTransport::bind("127.0.0.1:0".parse::<SocketAddr>().unwrap()).unwrap();
    /// let remote: SocketAddr = "127.0.0.1:50000".parse().unwrap();
    ///
    /// match transport.send_to(b"Hello, WRAITH!", remote) {
    ///     Ok(sent) => println!("Sent {} bytes", sent),
    ///     Err(e) => eprintln!("Send error: {}", e),
    /// }
    /// ```
    pub fn send_to(&self, buf: &[u8], addr: SocketAddr) -> io::Result<usize> {
        self.socket.send_to(buf, addr)
    }

    /// Get a reference to the receive buffer
    ///
    /// This buffer is reused across all `recv_from()` calls.
    /// The valid data range is determined by the size returned from `recv_from()`.
    pub fn recv_buffer(&self) -> &[u8] {
        &self.recv_buf
    }

    /// Get a mutable reference to the receive buffer
    ///
    /// Allows modification of the buffer for zero-copy processing.
    pub fn recv_buffer_mut(&mut self) -> &mut [u8] {
        &mut self.recv_buf
    }

    /// Get the local address this socket is bound to
    ///
    /// # Examples
    /// ```no_run
    /// # use wraith_transport::udp::UdpTransport;
    /// # use std::net::SocketAddr;
    /// let transport = UdpTransport::bind("127.0.0.1:0".parse::<SocketAddr>().unwrap()).unwrap();
    /// let addr = transport.local_addr().unwrap();
    /// println!("Bound to {}", addr);
    /// ```
    pub fn local_addr(&self) -> io::Result<SocketAddr> {
        self.socket.local_addr()
    }

    /// Set the socket to blocking or non-blocking mode
    ///
    /// By default, sockets are created in non-blocking mode.
    pub fn set_nonblocking(&self, nonblocking: bool) -> io::Result<()> {
        self.socket.set_nonblocking(nonblocking)
    }

    /// Get the receive buffer size in bytes
    pub fn recv_buffer_size(&self) -> io::Result<usize> {
        Ok(self.recv_buffer_size)
    }

    /// Get the send buffer size in bytes
    pub fn send_buffer_size(&self) -> io::Result<usize> {
        Ok(self.send_buffer_size)
    }

    /// Set the time-to-live (TTL) for outgoing packets
    pub fn set_ttl(&self, ttl: u32) -> io::Result<()> {
        self.socket.set_ttl(ttl)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_udp_bind() {
        let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
        let transport = UdpTransport::bind(addr).unwrap();
        let bound_addr = transport.local_addr().unwrap();
        assert_ne!(bound_addr.port(), 0);
        assert!(bound_addr.is_ipv4());
    }

    #[test]
    fn test_udp_buffer_sizes() {
        let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
        let transport = UdpTransport::bind(addr).unwrap();

        // Buffer sizes should be non-zero
        // Note: Actual sizes are system-dependent and may be capped by kernel limits
        let recv_size = transport.recv_buffer_size().unwrap();
        let send_size = transport.send_buffer_size().unwrap();

        // On most systems, the kernel sets some reasonable minimum (typically 256KB+)
        assert!(recv_size > 0, "Receive buffer size should be non-zero");
        assert!(send_size > 0, "Send buffer size should be non-zero");

        // Typically at least the system minimum (often 128-256KB)
        assert!(
            recv_size >= 128 * 1024,
            "Receive buffer should be >= 128KB, got {}",
            recv_size
        );
        assert!(
            send_size >= 128 * 1024,
            "Send buffer should be >= 128KB, got {}",
            send_size
        );
    }

    #[test]
    fn test_udp_send_recv() {
        let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
        let mut server = UdpTransport::bind(addr).unwrap();
        let server_addr = server.local_addr().unwrap();

        let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
        let client = UdpTransport::bind(addr).unwrap();

        // Send from client to server
        let sent = client.send_to(b"Hello, WRAITH!", server_addr).unwrap();
        assert_eq!(sent, 14);

        // Give packet time to arrive
        std::thread::sleep(Duration::from_millis(10));

        // Receive on server
        let (recv_size, from_addr) = server.recv_from().unwrap();
        assert_eq!(recv_size, 14);
        assert_eq!(&server.recv_buffer()[..recv_size], b"Hello, WRAITH!");
        assert_eq!(from_addr, client.local_addr().unwrap());
    }

    #[test]
    fn test_udp_nonblocking() {
        let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
        let mut transport = UdpTransport::bind(addr).unwrap();

        // Should return WouldBlock immediately when no data available
        let result = transport.recv_from();
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().kind(), io::ErrorKind::WouldBlock);
    }

    #[test]
    fn test_udp_large_packet() {
        let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
        let mut server = UdpTransport::bind(addr).unwrap();
        let server_addr = server.local_addr().unwrap();

        let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
        let client = UdpTransport::bind(addr).unwrap();

        // Send a large packet (close to MTU)
        let large_data = vec![0xAA; 1400];
        let sent = client.send_to(&large_data, server_addr).unwrap();
        assert_eq!(sent, 1400);

        std::thread::sleep(Duration::from_millis(10));

        let (recv_size, _) = server.recv_from().unwrap();
        assert_eq!(recv_size, 1400);
        assert_eq!(&server.recv_buffer()[..recv_size], &large_data[..]);
    }

    #[test]
    fn test_udp_ttl() {
        let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
        let transport = UdpTransport::bind(addr).unwrap();
        transport.set_ttl(64).unwrap();
        // TTL is set, no easy way to verify without sending packets
    }

    #[test]
    fn test_udp_multiple_packets() {
        let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
        let mut server = UdpTransport::bind(addr).unwrap();
        let server_addr = server.local_addr().unwrap();

        let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
        let client = UdpTransport::bind(addr).unwrap();

        // Send multiple packets
        for i in 0..10 {
            let data = format!("Packet {}", i);
            client.send_to(data.as_bytes(), server_addr).unwrap();
        }

        std::thread::sleep(Duration::from_millis(50));

        // Receive all packets
        let mut count = 0;
        loop {
            match server.recv_from() {
                Ok((size, _)) => {
                    assert!(size > 0);
                    count += 1;
                }
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => break,
                Err(e) => panic!("Unexpected error: {}", e),
            }
        }

        // Should receive all 10 packets (or close to it, UDP is lossy on localhost usually reliable)
        assert!(count >= 8, "Received only {} packets out of 10", count);
    }
}
